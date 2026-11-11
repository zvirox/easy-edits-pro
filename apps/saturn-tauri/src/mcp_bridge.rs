// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Same-user local bridge for MCP requests to the active desktop editor session.

use std::io;
use std::sync::Arc;

use saturn_core::catalogue::policy::{SessionPolicy, all_profiles};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

use crate::{AppState, commands::execute_command_with_state};

#[path = "mcp_bridge_media.rs"]
mod media;
use media::*;

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
    let mut initial_policy = SessionPolicy::default();
    if std::env::var(crate::write_grants::BYPASS_ENV).is_ok() {
        initial_policy.granted_tiers.insert(saturn_core::catalogue::Tier::Filesystem);
        initial_policy.granted_tiers.insert(saturn_core::catalogue::Tier::Destructive);
    }
    if let Ok(tiers_str) = std::env::var("SATURN_MCP_TIERS") {
        for t in tiers_str.split(',') {
            if let Some(tier) = saturn_core::catalogue::Tier::parse(t.trim()) {
                initial_policy.granted_tiers.insert(tier);
            }
        }
    }
    if let Ok(profiles_str) = std::env::var("SATURN_MCP_PROFILES") {
        if profiles_str.trim().eq_ignore_ascii_case("all") {
            for p in saturn_core::catalogue::PROFILES {
                let _ = initial_policy.load_profile(p);
            }
        } else {
            for p in profiles_str.split(',') {
                let _ = initial_policy.load_profile(p.trim());
            }
        }
    }
    let policy = Arc::new(tokio::sync::RwLock::new(initial_policy));
    eprintln!("Easy Edits Pro MCP bridge listening at {}", path.display());

    loop {
        let (stream, _) = listener.accept().await?;
        let app = app.clone();
        let state = state.clone();
        let policy = policy.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, app, state, policy).await {
                eprintln!("MCP bridge request failed: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: UnixStream,
    app: AppHandle,
    state: AppState,
    policy: Arc<tokio::sync::RwLock<SessionPolicy>>,
) -> io::Result<()> {
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
        Ok(request) => dispatch(request, &app, &state, &policy).await,
        Err(error) => json!({"error": format!("invalid bridge request: {error}")}),
    };
    writer.write_all(response.to_string().as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.shutdown().await
}

fn check_bridge_tool(method: &str, policy: &SessionPolicy) -> Result<(), String> {
    use saturn_core::catalogue::Tier::{Edit, Filesystem, Read};
    let (tiers, profiles): (&[saturn_core::catalogue::Tier], &[&str]) = match method {
        "session.stop" | "session.status" => (&[Read], &["core"]),
        "session.resume" => (&[Edit], &["core"]),
        "history.undo_change" => (&[Edit], &["core"]),
        "color_read_scopes" => (&[Read], &["colour", "review"]),
        "color_match_shot_to_reference" => (&[Edit], &["colour"]),
        "audio_read_telemetry" => (&[Read], &["audio", "review"]),
        "audio_process" => (&[Edit], &["audio"]),
        "edit_match_cut_align" => (&[Edit], &["fine-cut", "rough-cut"]),
        "timeline_beat_grid_quantize" => (&[Edit], &["fine-cut", "speed-motion"]),
        "media_semantic_query" => (&[Read], &["ingest", "rough-cut", "review"]),
        "edit_evaluate_cut_continuity" => (&[Read], &["fine-cut", "review"]),
        "verify.run" | "verify_run" => (&[Read], &["review"]),
        "render_preview_clip_proxy" => (&[Edit, Filesystem], &["ingest", "review"]),
        "render_project" | "render_start" => (&[Edit, Filesystem], &["export"]),
        "render_status" => (&[Read], &["export"]),
        "render_cancel" => (&[Edit], &["export", "review"]),
        _ => return Ok(()),
    };
    policy.check_tool(method, tiers, profiles).map_err(|e| e.to_string())
}

async fn dispatch(
    request: Value,
    app: &AppHandle,
    state: &AppState,
    policy: &Arc<tokio::sync::RwLock<SessionPolicy>>,
) -> Value {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    if method == "tools.list_profiles" || method == "tools_list_profiles" {
        let pol = policy.read().await;
        let catalogue = saturn_core::command_catalogue();
        let profiles = all_profiles().iter().map(|info| {
            let count = catalogue.iter().filter(|c| c.profiles.contains(&info.name)).count();
            json!({ "name": info.name, "description": info.description, "candidate_capabilities": info.candidate_capabilities, "active": pol.active_profiles.contains(info.name), "command_count": count })
        }).collect::<Vec<_>>();
        let mut granted = pol.granted_tiers.iter().map(|t| t.as_str()).collect::<Vec<_>>();
        granted.sort();
        let mut active = pol.active_profiles.iter().cloned().collect::<Vec<_>>();
        active.sort();
        return json!({
            "result": { "mode": pol.mode.as_str(), "active_profiles": active, "granted_tiers": granted, "profiles": profiles }
        });
    }

    if method == "tools.load_profile" || method == "tools_load_profile" {
        let profile = match params.get("profile").and_then(Value::as_str) {
            Some(p) => p,
            None => return json!({"error": "tools.load_profile requires string `profile`"}),
        };
        let mut pol = policy.write().await;
        let was_new = match pol.load_profile(profile) {
            Ok(new) => new,
            Err(e) => return json!({"error": e.to_string()}),
        };
        let mut active = pol.active_profiles.iter().cloned().collect::<Vec<_>>();
        active.sort();
        return json!({
            "result": {
                "ok": true,
                "profile": profile,
                "newly_activated": was_new,
                "active_profiles": active,
                "message": format!("Profile '{profile}' is active. Associated commands and tools are now available."),
            }
        });
    }

    {
        let pol = policy.read().await;
        if let Err(err) = check_bridge_tool(method, &pol) {
            return json!({"error": err});
        }
    }

    let result = match method {
        "command_list" => command_list(state, &params),
        "command_batch" => command_batch(app, state, policy, &params).await,
        "session.stop" => session_stop(state, &params),
        "session.resume" => session_resume(state, &params),
        "session.status" => session_status(state, &params),
        "history.undo_change" => history_undo_change(app, state, &params),
        "edit_match_cut_align" => edit_match_cut_align(app, state, &params).await,
        "timeline_beat_grid_quantize" => timeline_beat_grid_quantize(app, state, &params).await,
        "command_run" => {
            let id = params.get("id").and_then(Value::as_str);
            let Some(id) = id else {
                return json!({"error": "command_run requires a string id"});
            };
            let spec = saturn_core::command_catalogue().iter().find(|c| c.id == id);
            if let Some(spec) = spec {
                let pol = policy.read().await;
                if let Err(err) = pol.check_command(spec) {
                    return json!({"error": err.to_string()});
                }
            }
            let command_params = params.get("params").cloned().unwrap_or_else(|| json!({}));
            if id == "timeline.beat_grid_quantize" && command_params.get("expected_digest").is_none() {
                return match timeline_beat_grid_quantize(app, state, &command_params).await {
                    Ok(value) => json!({"result": value}),
                    Err(error) => json!({"error": error}),
                };
            }
            if id == "edit.match_cut_align" && command_params.get("expected_digest").is_none() {
                return match edit_match_cut_align(app, state, &command_params).await {
                    Ok(value) => json!({"result": value}),
                    Err(error) => json!({"error": error}),
                };
            }
            let mut req = saturn_core::CommandRequest::new(id, command_params);
            if let Some(rationale) = params.get("rationale").and_then(Value::as_str) {
                req = req.with_rationale(rationale.to_string());
            }
            if let Some(confidence) = params.get("confidence").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                req = req.with_confidence(confidence);
            }
            match execute_command_with_state(
                app,
                state,
                req,
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
        "render_project" | "render_start" => start_render(&app, state, &params),
        "render_status" => render_status(state, &params),
        "render_cancel" => render_cancel(state, &params),
        "media_search" => media_search(state, &params),
        "media_preview" => media_preview(state, &params).await,
        "vision_extract_frame_sheet" => vision_extract_frame_sheet(state, &params).await,
        "render_preview_clip_proxy" => render_preview_clip_proxy(state, &params).await,
        "color_read_scopes" => color_read_scopes(state, &params).await,
        "audio_process" => audio_process(app, state, &params).await,
        "color_match_shot_to_reference" => {
            color_match_shot_to_reference(app, state, &params).await
        }
        "audio_read_telemetry" => audio_read_telemetry(state, &params).await,
        "media_semantic_query" => media_semantic_query(state, &params).await,
        "edit_evaluate_cut_continuity" => edit_evaluate_cut_continuity(state, &params).await,
        "verify.run" | "verify_run" => verify_run(state, &params).await,
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
                    "tier": spec.tier.as_str(),
                    "tiers": spec.tiers.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
                    "profiles": spec.profiles,
                })
            })
        })
        .collect::<Vec<_>>();
    Ok(Value::Array(commands))
}

async fn command_batch(
    app: &AppHandle,
    state: &AppState,
    policy: &Arc<tokio::sync::RwLock<SessionPolicy>>,
    params: &Value,
) -> Result<Value, String> {
    let steps = params
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| "command_batch requires `steps` array".to_owned())?;
    let atomic = params.get("atomic").and_then(Value::as_bool).unwrap_or(false);
    let stop_on_error = params.get("stop_on_error").and_then(Value::as_bool).unwrap_or(true);
    let ctx: saturn_core::ActorContext = params
        .get("context")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    if atomic {
        let cmd_steps = steps
            .iter()
            .map(|step| {
                let id = step.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                let p = step.get("params").cloned().unwrap_or_else(|| json!({}));
                let mut req = saturn_core::CommandRequest::new(id, p);
                if let Some(rationale) = step.get("rationale").and_then(Value::as_str) {
                    req = req.with_rationale(rationale.to_string());
                }
                if let Some(confidence) = step.get("confidence").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                    req = req.with_confidence(confidence);
                }
                req
            })
            .collect::<Vec<_>>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let res = guard.execute_transaction(cmd_steps, &ctx).map_err(|e| e.to_string())?;
        let _ = app.emit("mcp_project_changed", ());
        Ok(res)
    } else {
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut results = Vec::new();
        for step in steps {
            let Some(id) = step.get("id").and_then(Value::as_str) else {
                failed += 1;
                results.push(json!({"ok": false, "error": "step requires string `id`"}));
                if stop_on_error { break; }
                continue;
            };
            let spec = saturn_core::command_catalogue().iter().find(|cmd| cmd.id == id);
            if let Some(spec) = spec {
                let pol = policy.read().await;
                if let Err(err) = pol.check_command(spec) {
                    failed += 1;
                    results.push(json!({"ok": false, "error": err.to_string()}));
                    if stop_on_error { break; }
                    continue;
                }
            }
            let step_params = step.get("params").cloned().unwrap_or_else(|| json!({}));
            let mut req = saturn_core::CommandRequest::new(id, step_params);
            if let Some(rationale) = step.get("rationale").and_then(Value::as_str) {
                req = req.with_rationale(rationale.to_string());
            }
            if let Some(confidence) = step.get("confidence").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                req = req.with_confidence(confidence);
            }
            match execute_command_with_state(app, state, req) {
                Ok(val) => {
                    completed += 1;
                    results.push(json!({"ok": true, "result": val}));
                }
                Err(err) => {
                    failed += 1;
                    results.push(json!({"ok": false, "error": err}));
                    if stop_on_error { break; }
                }
            }
        }
        let _ = app.emit("mcp_project_changed", ());
        Ok(json!({ "completed": completed, "failed": failed, "results": results }))
    }
}

fn session_stop(state: &AppState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "session.stop requires string `session_id`".to_owned())?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let generation = guard.stop_session(session_id);
    Ok(json!({ "session_id": session_id, "stopped": true, "generation": generation }))
}

fn session_resume(state: &AppState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "session.resume requires string `session_id`".to_owned())?;
    let caller: saturn_core::ActorId = params
        .get("caller")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(saturn_core::ActorId::Human);
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let generation = guard.resume_session(session_id, &caller).map_err(|e| e.to_string())?;
    Ok(json!({ "session_id": session_id, "resumed": true, "generation": generation }))
}

fn session_status(state: &AppState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "session.status requires string `session_id`".to_owned())?;
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let is_stopped = guard.is_session_stopped(session_id);
    let session = guard.sessions.get(session_id);
    Ok(json!({
        "session_id": session_id,
        "is_stopped": is_stopped,
        "generation": session.map(|s| s.generation).unwrap_or(1),
        "actor": session.map(|s| s.actor.clone()).unwrap_or(saturn_core::ActorId::Agent("bridge".into())),
        "active_branch": session.map(|s| s.active_branch.clone()).unwrap_or_else(|| saturn_core::branch::MAIN_BRANCH.to_string()),
        "playhead": session.map(|s| s.playhead.0).unwrap_or(0),
        "locks": guard.document().locks.locks.iter().filter(|l| l.owner.session_id.as_deref() == Some(session_id)).collect::<Vec<_>>(),
    }))
}

fn history_undo_change(app: &AppHandle, state: &AppState, params: &Value) -> Result<Value, String> {
    let change_id = params
        .get("change_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "history.undo_change requires integer `change_id`".to_owned())?;
    let ctx: saturn_core::ActorContext = params
        .get("context")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let res = guard.undo_change(change_id, &ctx).map_err(|e| e.to_string())?;
    let _ = app.emit("mcp_project_changed", ());
    Ok(res)
}

#[cfg(test)]
#[path = "mcp_bridge_tests.rs"]
mod tests;
