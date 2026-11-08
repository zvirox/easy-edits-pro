// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::super::*;
use tauri::Manager;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAvailability {
    media_id: u64,
    missing: bool,
}

#[tauri::command]
pub async fn execute_command(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequest,
) -> Result<Value, String> {
    if matches!(request.id.as_str(), "timeline.asymmetric_trim" | "edit.match_cut_align" | "timeline.beat_grid_quantize") {
        let state = AppState(
            Arc::clone(&state.0),
            Arc::clone(&state.1),
            Arc::clone(&state.2),
        );
        return tauri::async_runtime::spawn_blocking(move || {
            execute_command_with_state(&app, &state, request)
        })
        .await
        .map_err(|e| format!("Edit preparation task failed: {e}"))?;
    }
    execute_command_with_state(&app, state.inner(), request)
}

/// Execute a command using the application's shared editor state. Used by Tauri IPC and the
/// same-user MCP bridge so both interfaces edit the exact same project and undo history.
pub fn execute_command_with_state(
    app: &AppHandle,
    state: &AppState,
    request: CommandRequest,
) -> Result<Value, String> {
    let command_id = request.id.clone();
    // Write-policy gate (audit priority item 5): engine commands that take a
    // raw `path` destination are checked against the allowed roots and the
    // session write grants BEFORE dispatch, and their parameter is rewritten
    // to the normalized spelling so the engine writes the canonical route.
    // file.open/project.new also invalidate session grants (project switch).
    let mut request = request;
    let telemetry = if command_id == "edit.match_cut_align"
        && request.params.get("expected_digest").is_none()
    {
        let document = state.0.lock().map_err(|_| "Editor state is unavailable")?.document().clone();
        let edit = saturn_render::match_cut::prepare(&document, &request.params)?;
        request = edit.command;
        Some(edit.telemetry)
    } else if command_id == "timeline.beat_grid_quantize"
        && request.params.get("expected_digest").is_none()
    {
        let document = state.0.lock().map_err(|_| "Editor state is unavailable")?.document().clone();
        let edit = saturn_render::beat_grid::prepare(&document, &request.params)?;
        request = edit.command;
        Some(edit.telemetry)
    } else { None };
    let mut prepared = if command_id == "timeline.asymmetric_trim"
        && request.params.get("expected_digest").is_none()
    {
        let (document, path) = {
            let guard = state.0.lock().map_err(|_| "Editor state is unavailable")?;
            (
                guard.document().clone(),
                guard.current_path().map(std::path::Path::to_path_buf),
            )
        };
        let edit =
            saturn_render::asymmetric_trim::prepare(&document, path.as_deref(), &request.params)?;
        request = edit.command.clone();
        Some(edit)
    } else {
        None
    };
    if let Some(param) = engine_write_target_param(&command_id) {
        let target = request
            .params
            .get(param)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("{command_id} requires a string '{param}'"))?;
        let project_dir = {
            let guard = state
                .0
                .lock()
                .map_err(|_| "Editor state is unavailable".to_string())?;
            guard
                .current_path()
                .and_then(std::path::Path::parent)
                .map(std::path::Path::to_path_buf)
        };
        let normalized = enforce_write_target(app, &target, project_dir.as_deref())?;
        if let Some(params) = request.params.as_object_mut() {
            params.insert(
                param.to_string(),
                Value::String(normalized.to_string_lossy().into_owned()),
            );
        }
    }
    if matches!(
        command_id.as_str(),
        "file.open" | "project.new" | "file.close" | "file.closeProject"
    ) {
        app.state::<WriteGrants>().clear();
    }
    let result = {
        // catch_unwind inside the lock scope: a panicking command used to
        // poison the state mutex and brick every later command ("Editor
        // state is unavailable"). A panic is now reported as a plain error
        // and the guard still drops normally.
        let mut guard = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| guard.execute(request)))
            .map_err(|payload| {
                let detail = if let Some(message) = payload.downcast_ref::<&str>() {
                    (*message).to_string()
                } else if let Some(message) = payload.downcast_ref::<String>() {
                    message.clone()
                } else {
                    "internal engine panic".to_string()
                };
                eprintln!("Engine command '{command_id}' panicked: {detail}");
                format!("Internal engine error in '{command_id}': {detail}")
            })
            .and_then(|inner| inner.map_err(|error| error.to_string()))?
    };

    if let Some(edit) = prepared.as_mut() {
        edit.commit();
    }
    if matches!(command_id.as_str(), "file.open" | "project.new" | "file.save") {
        recovery::remove_recovery_files(&app);
    }
    let mutating = command_catalogue()
        .iter()
        .find(|command| command.id == command_id)
        .is_some_and(|command| command.mutating);
    if mutating && !matches!(command_id.as_str(), "file.open" | "file.save") {
        let is_dirty = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable")?
            .is_dirty();
        if is_dirty {
            let autosave_app = app.clone();
            let autosave_state = AppState(
                Arc::clone(&state.0),
                Arc::clone(&state.1),
                Arc::clone(&state.2),
            );
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = recovery::autosave_once(&autosave_app, &autosave_state) {
                    eprintln!("Autosave failed: {error}");
                }
            });
        } else {
            recovery::remove_recovery_files(&app);
        }
    }
    if matches!(
        command_id.as_str(),
        "file.open"
            | "project.new"
            | "project.add_media"
            | "project.relink_media"
            | "project.import_media"
            | "timeline.asymmetric_trim"
    ) {
        let engine = state.0.lock().map_err(|_| "Editor state is unavailable")?;
        register_project_media(&app, engine.document());
    }
    Ok(if let Some(telemetry) = telemetry {
        json!({"applied": result, "telemetry": telemetry})
    } else { result })
}

/// Record a UI-dialog-picked destination as a session write grant (planning
/// §6f policy). The dialog pick is the consent act; the grant makes exactly
/// this normalized path writable for the rest of the session. There is no
/// bridge method for this — only the UI (or the harness env bypass) can
/// create grants.
#[tauri::command]
pub fn register_write_grant(app: AppHandle, path: String) -> Result<(), String> {
    app.state::<WriteGrants>()
        .grant(std::path::Path::new(&path))
}

/// Directory of the currently open project file, if any — one of the
/// allowed write roots for the write policy (planning.md §6f).
pub fn current_project_dir(state: &AppState) -> Option<std::path::PathBuf> {
    state
        .0
        .lock()
        .ok()?
        .current_path()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
}

#[tauri::command]
pub fn command_status(state: State<'_, AppState>, id: String) -> Result<CommandStatus, String> {
    state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .command_status(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_command_catalogue() -> Vec<saturn_core::CommandSpec> {
    command_catalogue().to_vec()
}

#[tauri::command]
pub fn check_media_availability(
    state: State<'_, AppState>,
) -> Result<Vec<MediaAvailability>, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    // The engine owns missing-media truth; the shell only reshapes it.
    // Re-running the check keeps the stored `is_missing` flags in step with
    // the disk before the flags are read.
    engine
        .execute(CommandRequest::new("media.check_missing", Value::Null))
        .map_err(|error| error.to_string())?;
    Ok(engine
        .document()
        .project
        .media
        .iter()
        .filter(|media| media.kind != MediaKind::Title && media.kind != MediaKind::CompoundSequence)
        .map(|media| MediaAvailability {
            media_id: media.id,
            missing: media.is_missing,
        })
        .collect())
}
