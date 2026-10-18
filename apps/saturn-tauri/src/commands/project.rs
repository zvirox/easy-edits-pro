use super::super::*;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAvailability {
    media_id: u64,
    missing: bool,
}

#[tauri::command]
pub fn execute_command(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequest,
) -> Result<Value, String> {
    let command_id = request.id.clone();
    let result = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .execute(request)
        .map_err(|error| error.to_string())?;

    if matches!(
        command_id.as_str(),
        "file.open" | "project.new" | "file.save"
    ) {
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
    ) {
        let engine = state.0.lock().map_err(|_| "Editor state is unavailable")?;
        register_project_media(&app, engine.document());
    }
    Ok(result)
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
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    Ok(engine
        .document()
        .project
        .media
        .iter()
        .map(|media| MediaAvailability {
            media_id: media.id,
            missing: !media.path.is_file(),
        })
        .collect())
}
