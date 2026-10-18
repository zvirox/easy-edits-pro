use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State};
use saturn_core::ProjectDocument;
use crate::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryMetadata {
    pub saved_at: u64,
    pub project_name: String,
    pub source_path: Option<String>,
}

pub fn recovery_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let directory = app.path().app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("recovery");
    Ok((directory.join("autosave.saturn"), directory.join("autosave.json")))
}

pub fn remove_recovery_files(app: &AppHandle) {
    if let Ok((project, metadata)) = recovery_paths(app) {
        let _ = std::fs::remove_file(project);
        let _ = std::fs::remove_file(metadata);
    }
}

pub fn save_recovery_metadata(path: &Path, metadata: &RecoveryMetadata) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| format!("Could not write recovery details: {error}"))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("Could not finalize recovery details: {error}"))
}

pub fn autosave_once(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    if !engine.is_dirty() { return Ok(()); }
    let document = engine.document();
    let (project_path, metadata_path) = recovery_paths(app)?;
    let directory = project_path.parent().ok_or_else(|| "Recovery folder has no parent".to_string())?;
    std::fs::create_dir_all(directory).map_err(|error| format!("Could not create recovery folder: {error}"))?;
    document.save_atomic(&project_path)?;
    let metadata = RecoveryMetadata {
        saved_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        project_name: document.project.name.clone(),
        source_path: engine.current_path().map(|path| path.to_string_lossy().into_owned()),
    };
    save_recovery_metadata(&metadata_path, &metadata)
}

pub fn start_autosave(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(error) = autosave_once(&app, &state) {
                eprintln!("Autosave failed: {error}");
            }
        }
    });
}

#[tauri::command]
pub fn recovery_status(app: AppHandle) -> Result<Value, String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    if !project_path.is_file() {
        return Ok(json!({"available": false}));
    }
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(&metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let modified = std::fs::metadata(&project_path).ok().and_then(|item| item.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|duration| duration.as_secs());
    Ok(json!({
        "available": true,
        "projectName": metadata.as_ref().map(|item| item.project_name.clone()).unwrap_or(document.project.name),
        "savedAt": metadata.map(|item| item.saved_at).or(modified).unwrap_or_default(),
    }))
}

#[tauri::command]
pub fn restore_autosave(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let source_path = metadata.and_then(|item| item.source_path).map(PathBuf::from);
    state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?
        .restore_recovery(document, source_path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn discard_autosave(app: AppHandle) {
    remove_recovery_files(&app);
}
