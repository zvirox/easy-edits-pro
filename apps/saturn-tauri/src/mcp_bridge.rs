//! Same-user local bridge for MCP requests to the active desktop editor session.

use std::io;

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

use crate::{AppState, commands::execute_command_with_state};

const MAX_REQUEST_BYTES: usize = 1_048_576;

pub fn start(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = serve(app, state).await {
            eprintln!("MCP local bridge stopped: {error}");
        }
    });
}

/// Binds the bridge's Unix socket with the desktop-lifecycle rules:
/// a socket file nobody answers is stale (previous instance crashed) and is
/// removed; a live listener means another editor owns the socket and startup
/// fails; the created socket is readable only by the current user.
async fn prepare_bridge_socket(path: &std::path::Path) -> io::Result<UnixListener> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    {
        use std::os::unix::fs::PermissionsExt;
        if parent.ends_with("easy-edits-pro/runtime") {
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    if path.exists() {
        match UnixStream::connect(path).await {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("another editor already owns MCP socket {}", path.display()),
                ));
            }
            Err(_) => std::fs::remove_file(path)?,
        }
    }

    let listener = UnixListener::bind(path)?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(listener)
}

async fn serve(app: AppHandle, state: AppState) -> io::Result<()> {
    let path = saturn_core::local_control::mcp_socket_path();
    let listener = prepare_bridge_socket(&path).await?;
    eprintln!("Easy Edits Pro MCP bridge listening at {}", path.display());

    loop {
        let (stream, _) = listener.accept().await?;
        let app = app.clone();
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, app, state).await {
                eprintln!("MCP bridge request failed: {error}");
            }
        });
    }
}

async fn handle_connection(stream: UnixStream, app: AppHandle, state: AppState) -> io::Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 4096];
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        if request.len() + count > MAX_REQUEST_BYTES {
            let response = json!({"error": "MCP bridge request exceeds 1 MiB"});
            writer.write_all(response.to_string().as_bytes()).await?;
            writer.write_all(b"\n").await?;
            return Ok(());
        }
        request.extend_from_slice(&chunk[..count]);
        if request.contains(&b'\n') {
            break;
        }
    }

    let response = match serde_json::from_slice::<Value>(&request) {
        Ok(request) => dispatch(request, &app, &state).await,
        Err(error) => json!({"error": format!("invalid bridge request: {error}")}),
    };
    writer.write_all(response.to_string().as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.shutdown().await
}

async fn dispatch(request: Value, app: &AppHandle, state: &AppState) -> Value {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
    let result = match method {
        "command_list" => command_list(state, &params),
        "command_run" => {
            let id = params.get("id").and_then(Value::as_str);
            let Some(id) = id else {
                return json!({"error": "command_run requires a string id"});
            };
            let command_params = params.get("params").cloned().unwrap_or_else(|| json!({}));
            match execute_command_with_state(
                app,
                state,
                saturn_core::CommandRequest::new(id, command_params),
            ) {
                Ok(value) => {
                    let _ = app.emit("mcp_project_changed", ());
                    Ok(value)
                }
                Err(error) => Err(error),
            }
        }
        "doc_inspect" => inspect_document(state),
        "sequence_inspect" => inspect_document(state).and_then(|value| {
            value
                .pointer("/project/project/sequence")
                .cloned()
                .ok_or_else(|| "active sequence not found".to_owned())
        }),
        "render_project" | "render_start" => start_render(state, &params),
        "render_status" => render_status(state, &params),
        "render_cancel" => render_cancel(state, &params),
        "media_search" => media_search(state, &params),
        "media_preview" => media_preview(state, &params),
        _ => Err(format!("unknown bridge method: {method}")),
    };
    match result {
        Ok(value) => json!({"result": value}),
        Err(error) => json!({"error": error}),
    }
}

fn command_list(state: &AppState, params: &Value) -> Result<Value, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_owned())?;
    let filter = params
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let enabled_only = params
        .get("enabled_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let commands = saturn_core::command_catalogue()
        .iter()
        .filter_map(|spec| {
            let status = engine.command_status(spec.id).ok()?;
            let matches = filter.is_empty()
                || spec.id.to_lowercase().contains(&filter)
                || spec.label.to_lowercase().contains(&filter);
            (matches && (!enabled_only || status.enabled)).then(|| {
                json!({
                    "id": spec.id,
                    "label": spec.label,
                    "menu": spec.menu,
                    "shortcut": spec.shortcut,
                    "mutating": spec.mutating,
                    "enabled": status.enabled,
                    "reason": status.reason,
                })
            })
        })
        .collect::<Vec<_>>();
    Ok(Value::Array(commands))
}

fn inspect_document(state: &AppState) -> Result<Value, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_owned())?;
    Ok(json!({
        "project": engine.document(),
        "path": engine.current_path().map(|path| path.to_string_lossy().to_string()),
        "is_dirty": engine.is_dirty(),
    }))
}

fn start_render(state: &AppState, params: &Value) -> Result<Value, String> {
    let output = params
        .get("output_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "render_project requires output_path".to_owned())?;
    let output_path = std::path::PathBuf::from(output);
    let explicit_range = params.get("range").map(|range| {
        let start = range.get("start_ticks").and_then(Value::as_i64)
            .ok_or_else(|| "range.start_ticks must be a non-negative integer".to_owned())?;
        let end = range.get("end_ticks").and_then(Value::as_i64)
            .ok_or_else(|| "range.end_ticks must be an integer greater than start_ticks".to_owned())?;
        if start < 0 || end <= start {
            return Err("Render range must have non-negative start_ticks and end_ticks greater than start_ticks".to_owned());
        }
        Ok(saturn_render::RenderRange { start: saturn_core::Tick(start), end: saturn_core::Tick(end) })
    }).transpose()?;
    if explicit_range.is_some()
        && params
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Err("Pass either range or use_mark_range, not both".into());
    }
    let (document, range) = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_owned())?;
        let document = engine.document().clone();
        let range = if let Some(range) = explicit_range {
            Some(range)
        } else if params
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            match (
                document.project.sequence.mark_in,
                document.project.sequence.mark_out,
            ) {
                (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
                _ => {
                    return Err(
                        "Set both In and Out points before rendering the marked range".into(),
                    );
                }
            }
        } else {
            None
        };
        (document, range)
    };
    let settings = params
        .get("settings")
        .filter(|value| value.is_object())
        .map(|value| serde_json::from_value(value.clone()).map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or_else(|| saturn_render::RenderSettings::from_project(&document));
    let rendered_output = output_path.clone();
    let label = format!(
        "MCP render: {}",
        output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );
    let kind = saturn_core::JobKind::ExportRendering {
        output_path: output_path.clone(),
    };
    let (id, _) = state.1.submit_detached(
        label,
        kind,
        saturn_core::JobPriority::Immediate,
        move |ctx| {
            saturn_render::render_project_range_with_control(
                &document,
                &rendered_output,
                range,
                settings,
                |progress| {
                    ctx.update_progress(progress.fraction_percent as f32 / 100.0, progress.phase)
                },
                || ctx.is_cancelled(),
            )?;
            Ok(Some(rendered_output.to_string_lossy().into_owned()))
        },
    );
    Ok(json!({ "job_id": id.0, "state": "queued", "output_path": output_path }))
}

fn render_status(state: &AppState, params: &Value) -> Result<Value, String> {
    let id = params
        .get("job_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "render_status requires an unsigned job_id".to_owned())?;
    state
        .1
        .get_job(saturn_core::JobId(id))
        .map(|job| serde_json::to_value(job).unwrap_or(Value::Null))
        .ok_or_else(|| format!("Render job {id} was not found"))
}

fn render_cancel(state: &AppState, params: &Value) -> Result<Value, String> {
    let id = params
        .get("job_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "render_cancel requires an unsigned job_id".to_owned())?;
    let cancelled = state.1.cancel(saturn_core::JobId(id));
    Ok(json!({ "job_id": id, "cancellation_requested": cancelled }))
}

fn media_search(state: &AppState, params: &Value) -> Result<Value, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_owned())?;
    let query = params
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let kind = params
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(100)
        .clamp(1, 500) as usize;
    let media = engine
        .document()
        .project
        .media
        .iter()
        .filter(|item| {
            (query.is_empty()
                || item.name.to_lowercase().contains(&query)
                || item.path.to_string_lossy().to_lowercase().contains(&query))
                && (kind.is_empty() || format!("{:?}", item.kind).to_lowercase() == kind)
        })
        .take(limit)
        .collect::<Vec<_>>();
    serde_json::to_value(media).map_err(|error| error.to_string())
}

fn media_preview(state: &AppState, params: &Value) -> Result<Value, String> {
    let id = params
        .get("media_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "media_preview requires an unsigned media_id".to_owned())?;
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_owned())?;
    media_preview_payload(engine.document(), id)
}

/// Shared shape for media_preview: no image payload is fabricated. Until
/// still-frame extraction exists, the tool reports the absence honestly
/// instead of returning a source-path placeholder that clients would mistake
/// for a preview.
fn media_preview_payload(document: &saturn_core::ProjectDocument, id: u64) -> Result<Value, String> {
    let asset = document
        .project
        .media
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| format!("Media asset {id} was not found"))?;
    let asset = serde_json::to_value(asset).map_err(|error| error.to_string())?;
    let placements = document
        .project
        .sequence
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .clips
                .iter()
                .filter(move |clip| clip.media_id == id)
                .map(move |clip| {
                    json!({
                        "track_id": track.id,
                        "track_name": track.name,
                        "clip": clip,
                    })
                })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "media": asset,
        "timeline_placements": placements,
        "still_frame": {
            "available": false,
            "reason": "still-frame extraction is not implemented; no image payload exists yet"
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_preview_reports_honest_absence_of_still_frames() {
        let mut engine = saturn_core::EditorEngine::default();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.new",
                json!({"name": "MCP preview"}),
            ))
            .unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                json!({"name": "clip.mp4", "path": "/assets/clip.mp4", "kind": "video"}),
            ))
            .unwrap();
        let document = engine.document().clone();
        let media_id = document.project.media[0].id;

        let payload = media_preview_payload(&document, media_id).unwrap();
        assert!(payload.get("preview_path").is_none(), "no source-path placeholder");
        assert_eq!(payload["still_frame"]["available"], false);
        assert_eq!(
            payload["media"]["id"],
            serde_json::to_value(media_id).unwrap()
        );

        // Unknown ids stay a typed error, never a fabricated preview.
        assert!(media_preview_payload(&document, 999).is_err());
    }

    #[tokio::test]
    async fn bridge_socket_prep_covers_stale_live_and_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("bridge.sock");

        // A stale socket file (no listener answers) is removed and replaced.
        std::fs::write(&path, b"not a socket").unwrap();
        let listener = prepare_bridge_socket(&path).await.expect("stale socket recovery");
        drop(listener);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600,
            "bridge socket is private to the user"
        );
        // Dropping a listener leaves its socket file behind - that stale file
        // is exactly what the crash-recovery path must clear on next start.
        let _ = std::fs::remove_file(&path);

        // A live listener owns the socket: the second editor must refuse to
        // start its bridge instead of stealing it.
        let owner = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let error = prepare_bridge_socket(&path).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
        drop(owner);
        let _ = std::fs::remove_file(&path);

        // Fresh bind works, creating parent directories as needed.
        let nested = folder.path().join("deep/new/dir");
        let fresh = prepare_bridge_socket(&nested.join("bridge.sock"))
            .await
            .expect("fresh bind with created parents");
        drop(fresh);
    }
}
