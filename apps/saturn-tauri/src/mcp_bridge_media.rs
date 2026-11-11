// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Media, render, and analysis handlers for the desktop MCP bridge.

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use crate::{AppState, commands::execute_command_with_state};

pub(super) fn inspect_document(state: &AppState) -> Result<Value, String> {
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

pub(super) fn start_render(app: &AppHandle, state: &AppState, params: &Value) -> Result<Value, String> {
    let output = params
        .get("output_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "render_project requires output_path".to_owned())?;
    let project_dir = crate::commands::current_project_dir(state);
    let output_path = crate::enforce_write_target(app, output, project_dir.as_deref())?;
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

pub(super) fn render_status(state: &AppState, params: &Value) -> Result<Value, String> {
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

pub(super) fn render_cancel(state: &AppState, params: &Value) -> Result<Value, String> {
    let id = params
        .get("job_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "render_cancel requires an unsigned job_id".to_owned())?;
    let cancelled = state.1.cancel(saturn_core::JobId(id));
    Ok(json!({ "job_id": id, "cancellation_requested": cancelled }))
}

pub(super) fn media_search(state: &AppState, params: &Value) -> Result<Value, String> {
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
    let val = serde_json::to_value(media).map_err(|error| error.to_string())?;
    let mut tracker = saturn_core::OriginTracker::from_document(engine.document());
    Ok(tracker.wrap_media_search(val))
}

pub(super) fn document_snapshot(state: &AppState) -> Result<saturn_core::ProjectDocument, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_owned())?;
    Ok(engine.document().clone())
}

fn media_id_from(params: &Value) -> Result<u64, String> {
    params
        .get("media_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "media_preview requires an unsigned media_id".to_owned())
}

async fn blocking_payload(
    task: impl FnOnce() -> Result<Value, String> + Send + 'static,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("preview task failed: {error}"))?
}

pub(super) async fn media_preview(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let media_id = media_id_from(params)?;
    let poster_tick = params
        .get("poster_tick")
        .and_then(Value::as_i64)
        .map(saturn_core::Tick);
    let payload = blocking_payload(move || {
        saturn_render::preview_tools::media_preview_payload(&document, media_id, poster_tick)
    })
    .await?;
    let doc = document_snapshot(state)?;
    let mut tracker = saturn_core::OriginTracker::from_document(&doc);
    Ok(tracker.wrap_media_preview(payload))
}

pub(super) async fn vision_extract_frame_sheet(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        saturn_render::preview_tools::frame_sheet_payload(&document, &params)
    })
    .await
}

pub(super) async fn render_preview_clip_proxy(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        saturn_render::preview_tools::preview_clip_proxy_payload(&document, &params)
    })
    .await
}

pub(super) async fn edit_match_cut_align(app: &AppHandle, state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        saturn_render::match_cut::prepare(&document, &params)
    }).await.map_err(|e| format!("Match-cut analysis task failed: {e}"))??;
    let applied = execute_command_with_state(app, state, prepared.command)?;
    let _ = app.emit("mcp_project_changed", ());
    Ok(json!({"applied": applied, "telemetry": prepared.telemetry}))
}

pub(super) async fn timeline_beat_grid_quantize(app: &AppHandle, state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        saturn_render::beat_grid::prepare(&document, &params)
    }).await.map_err(|e| format!("Beat-grid analysis task failed: {e}"))??;
    let applied = execute_command_with_state(app, state, prepared.command)?;
    let _ = app.emit("mcp_project_changed", ());
    Ok(json!({"applied": applied, "telemetry": prepared.telemetry}))
}

pub(super) async fn color_read_scopes(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        saturn_render::preview_tools::color_scopes_payload(&document, &params)
    })
    .await
}

pub(super) async fn color_match_shot_to_reference(
    app: &AppHandle,
    state: &AppState,
    params: &Value,
) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    let payload = blocking_payload(move || {
        saturn_render::shot_match_driver::match_shot_payload(&document, &params)
    })
    .await?;
    let command = payload
        .get("command")
        .cloned()
        .ok_or_else(|| "shot match driver returned no command".to_owned())?;
    let id = command
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "shot match driver command has no id".to_owned())?
        .to_owned();
    let command_params = command.get("params").cloned().unwrap_or(json!({}));
    let applied = execute_command_with_state(
        app,
        state,
        saturn_core::CommandRequest::new(id, command_params),
    )?;
    let _ = app.emit("mcp_project_changed", ());
    let telemetry = payload.get("telemetry").cloned().unwrap_or(json!({}));
    Ok(json!({
        "applied": applied,
        "nodeId": applied.get("nodeId").cloned().unwrap_or(Value::Null),
        "telemetry": telemetry,
    }))
}

pub(super) async fn audio_process(app: &AppHandle, state: &AppState, params: &Value) -> Result<Value, String> {
    let (document, project_path) = {
        let engine = state.0.lock().map_err(|e| e.to_string())?;
        (engine.document().clone(), engine.current_path().map(std::path::Path::to_path_buf))
    };
    let params = params.clone();
    let mut edit = tauri::async_runtime::spawn_blocking(move || {
        let operation = params.get("operation").and_then(Value::as_str).ok_or("operation is required")?;
        saturn_render::audio_edit_driver::prepare(&document, project_path.as_deref(), operation, &params)
    }).await.map_err(|e| format!("Audio processing task failed: {e}"))??;
    let applied = execute_command_with_state(app, state, edit.command.clone())?;
    edit.commit();
    let _ = app.emit("mcp_project_changed", ());
    Ok(json!({"applied": applied, "telemetry": edit.telemetry}))
}

pub(super) async fn audio_read_telemetry(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        saturn_render::preview_tools::audio_telemetry_payload(&document, &params)
    })
    .await
}

pub(super) async fn media_semantic_query(state: &AppState, params: &Value) -> Result<Value, String> {
    let (document, project_path) = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_owned())?;
        (
            engine.document().clone(),
            engine.current_path().map(ToOwned::to_owned),
        )
    };
    let params = params.clone();
    let payload = blocking_payload(move || {
        saturn_render::preview_tools::semantic_query_payload(
            &document,
            project_path.as_deref(),
            &params,
        )
    })
    .await?;
    let doc = document_snapshot(state)?;
    let mut tracker = saturn_core::OriginTracker::from_document(&doc);
    Ok(tracker.wrap_semantic_query(payload))
}

pub(super) async fn edit_evaluate_cut_continuity(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        saturn_render::preview_tools::cut_continuity_payload(&document, &params)
    })
    .await
}

pub(super) async fn verify_run(state: &AppState, params: &Value) -> Result<Value, String> {
    let document = document_snapshot(state)?;
    let params = params.clone();
    blocking_payload(move || {
        let mut verify_params: saturn_core::verification::VerifyParams =
            serde_json::from_value(params.clone())
                .map_err(|e| format!("Invalid verify parameters: {e}"))?;

        if verify_params.audio_metrics.is_none() {
            if let Ok(audio) = saturn_render::preview_tools::audio_telemetry_payload(&document, &params) {
                verify_params.audio_metrics = Some(audio);
            }
        }
        if verify_params.video_metrics.is_none() {
            if let Ok(video) = saturn_render::preview_tools::color_scopes_payload(&document, &params) {
                verify_params.video_metrics = Some(video);
            }
        }

        let report = saturn_core::verification::run_verification(&document, &verify_params)?;
        serde_json::to_value(&report).map_err(|e| e.to_string())
    })
    .await
}
