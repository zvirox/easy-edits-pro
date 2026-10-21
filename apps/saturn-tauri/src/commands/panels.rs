//! Phase 4 floating panel engine: per-workspace layout persistence.
//!
//! Panel layouts are stored as viewport fractions (0-1) so a layout made on a
//! laptop stays valid on a larger display. Each workspace owns one JSON file in
//! the app data directory; the frontend clamps on load and the editor writes
//! only through these commands, so a layout can never escape its directory.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const MAX_WORKSPACE_LABEL: usize = 32;

fn layouts_directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))
        .map(|root| root.join("panel-layouts"))
}

/// Workspace labels feed a file name; keep them short and boring.
fn safe_workspace_label(label: &str) -> Result<&str, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_WORKSPACE_LABEL {
        return Err("workspace label must be 1-32 characters".into());
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("workspace label may only contain letters, digits, '-' and '_'".into());
    }
    Ok(trimmed)
}

fn layout_path(app: &AppHandle, workspace: &str) -> Result<PathBuf, String> {
    let label = safe_workspace_label(workspace)?;
    Ok(layouts_directory(app)?.join(format!("{label}.json")))
}

/// Persist one workspace's layout. `layout` is already-serialised JSON from the
/// renderer; it is stored verbatim so validation stays in one place. Unknown or
/// corrupt files fail loudly on load instead of poisoning the editor.
#[tauri::command]
pub fn save_panel_layout(app: AppHandle, workspace: String, layout: serde_json::Value) -> Result<(), String> {
    if !layout.is_object() {
        return Err("panel layout must be a JSON object".into());
    }
    let path = layout_path(&app, &workspace)?;
    let directory = path
        .parent()
        .ok_or_else(|| "Panel layout path has no parent".to_string())?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;

    let bytes = serde_json::to_vec_pretty(&layout).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("Could not write panel layout: {error}"))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("Could not finalise panel layout: {error}"))
}

/// Load one workspace's layout, or `None` when nothing was saved yet.
#[tauri::command]
pub fn load_panel_layout(app: AppHandle, workspace: String) -> Result<Option<serde_json::Value>, String> {
    let path = layout_path(&app, &workspace)?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let layout = serde_json::from_str(&contents)
                .map_err(|error| format!("Panel layout for {workspace} is corrupt: {error}"))?;
            Ok(Some(layout))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not read panel layout: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_workspace_labels() {
        assert_eq!(safe_workspace_label("edit").unwrap(), "edit");
        assert_eq!(safe_workspace_label(" fairlight ").unwrap(), "fairlight");
        assert_eq!(safe_workspace_label("a-1_b").unwrap(), "a-1_b");
    }

    #[test]
    fn rejects_path_shaped_labels() {
        assert!(safe_workspace_label("..").is_err());
        assert!(safe_workspace_label("../etc/passwd").is_err());
        assert!(safe_workspace_label("").is_err());
        assert!(safe_workspace_label("edit workspace!").is_err());
    }

    #[test]
    fn rejects_over_long_labels() {
        let long = "x".repeat(MAX_WORKSPACE_LABEL + 1);
        assert!(safe_workspace_label(&long).is_err());
    }
}
