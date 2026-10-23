use crate::AppState;
use saturn_core::ProjectDocument;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryMetadata {
    pub saved_at: u64,
    pub project_name: String,
    pub source_path: Option<String>,
}

pub fn recovery_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("recovery");
    Ok((
        directory.join("autosave.saturn"),
        directory.join("autosave.json"),
    ))
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
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("Could not write recovery details: {error}"))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("Could not finalize recovery details: {error}"))
}

pub fn autosave_once(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    if !engine.is_dirty() {
        return Ok(());
    }
    let document = engine.document();
    let (project_path, metadata_path) = recovery_paths(app)?;
    let directory = project_path
        .parent()
        .ok_or_else(|| "Recovery folder has no parent".to_string())?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create recovery folder: {error}"))?;
    document.save_atomic(&project_path)?;
    let metadata = RecoveryMetadata {
        saved_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        project_name: document.project.name.clone(),
        source_path: engine
            .current_path()
            .map(|path| path.to_string_lossy().into_owned()),
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
    read_recovery_state(&project_path, &metadata_path)
}

fn read_recovery_state(project_path: &Path, metadata_path: &Path) -> Result<Value, String> {
    if !project_path.is_file() {
        return Ok(json!({"available": false}));
    }
    let document = ProjectDocument::load(project_path)?;
    let metadata = std::fs::read(metadata_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let modified = std::fs::metadata(project_path)
        .ok()
        .and_then(|item| item.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
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
    let metadata = std::fs::read(metadata_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let source_path = metadata
        .and_then(|item| item.source_path)
        .map(PathBuf::from);
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    engine
        .restore_recovery(document, source_path)
        .map_err(|error| error.to_string())?;
    crate::register_project_media(&app, engine.document());
    Ok(())
}

#[tauri::command]
pub fn discard_autosave(app: AppHandle) {
    remove_recovery_files(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_metadata_round_trips_with_camel_case_keys() {
        let metadata = RecoveryMetadata {
            saved_at: 1_760_000_000,
            project_name: "Crash documentary".into(),
            source_path: Some("/assets/crash-documentary.saturn".into()),
        };
        let bytes = serde_json::to_vec_pretty(&metadata).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"savedAt\""), "{text}");
        assert!(text.contains("\"projectName\""), "{text}");
        assert!(text.contains("\"sourcePath\""), "{text}");
        let parsed: RecoveryMetadata = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.saved_at, metadata.saved_at);
        assert_eq!(parsed.project_name, metadata.project_name);
        assert_eq!(parsed.source_path, metadata.source_path);
    }

    #[test]
    fn recovery_metadata_writes_atomically_and_reads_back_cleanly() {
        let folder = tempfile::tempdir().unwrap();
        let metadata_path = folder.path().join("autosave.json");
        let metadata = RecoveryMetadata {
            saved_at: 42,
            project_name: "Atomic".into(),
            source_path: None,
        };
        save_recovery_metadata(&metadata_path, &metadata).unwrap();
        // The temporary staging file must never leak next to the final one.
        let leftovers = std::fs::read_dir(folder.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp"))
            .count();
        assert_eq!(leftovers, 0, "no .tmp staging files may survive");
        let parsed: RecoveryMetadata =
            serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
        assert_eq!(parsed, metadata);
    }

    #[test]
    fn recovery_state_reports_the_snapshot_then_reports_nothing_after_removal() {
        let folder = tempfile::tempdir().unwrap();
        let project_path = folder.path().join("autosave.saturn");
        let metadata_path = folder.path().join("autosave.json");

        // No files yet: the welcome screen must show no recovery offer.
        let state = read_recovery_state(&project_path, &metadata_path).unwrap();
        assert_eq!(state["available"], json!(false));

        // A crashed autosave appears with the metadata's naming and timestamp.
        let document = ProjectDocument::new("Crashed project");
        document.save_atomic(&project_path).unwrap();
        let metadata = RecoveryMetadata {
            saved_at: 1_760_000_000,
            project_name: "Crashed project".into(),
            source_path: Some("/assets/crashed.saturn".into()),
        };
        save_recovery_metadata(&metadata_path, &metadata).unwrap();
        let state = read_recovery_state(&project_path, &metadata_path).unwrap();
        assert_eq!(state["available"], json!(true));
        assert_eq!(state["projectName"], json!("Crashed project"));
        assert_eq!(state["savedAt"], json!(metadata.saved_at));

        // If the snapshot is gone, there is nothing to offer even if metadata
        // alone still lingers.
        std::fs::remove_file(&project_path).unwrap();
        let state = read_recovery_state(&project_path, &metadata_path).unwrap();
        assert_eq!(state["available"], json!(false));
    }
}
