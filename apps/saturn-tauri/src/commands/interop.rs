use super::super::*;

#[tauri::command]
pub async fn export_timeline_interop(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
    path: Option<String>,
) -> Result<String, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let doc = engine.document();
    let content = match format.to_lowercase().as_str() {
        "edl" | "cmx3600" => export_cmx3600_edl(doc)?,
        "fcpxml" | "fcp" => export_fcpxml(doc)?,
        "premiere_xml" | "premiere" | "xml" | "xmeml" => export_xmeml_xml(doc)?,
        "otio" | "opentimelineio" => export_otio(doc)?,
        _ => {
            return Err(format!(
                "Unsupported interchange format: {format}. Supported: 'edl', 'fcpxml', 'xmeml' (premiere_xml alias), 'otio'"
            ));
        }
    };

    if let Some(target_path) = path {
        let validated_path = validate_write_path(&app, &target_path, Some(50 * 1024 * 1024)) // 50 MB limit
            .map_err(|e| format!("Invalid export path: {e}"))?;
        std::fs::write(&validated_path, &content)
            .map_err(|e| format!("Could not write timeline interchange export: {e}"))?;
    }

    Ok(content)
}

#[tauri::command]
pub async fn import_timeline_interop(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
    content: Option<String>,
    path: Option<String>,
    project_name: Option<String>,
) -> Result<Value, String> {
    let raw_str = if let Some(c) = content {
        c
    } else if let Some(p) = path {
        let validated_path = validate_read_path(&app, &p, Some(50 * 1024 * 1024)) // 50 MB limit
            .map_err(|e| format!("Invalid import file path: {e}"))?;
        std::fs::read_to_string(&validated_path)
            .map_err(|e| format!("Could not read interchange file: {e}"))?
    } else {
        return Err("Either file path or content must be provided for timeline import".to_string());
    };

    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;

    let cmd_name = match format.to_lowercase().as_str() {
        "edl" | "cmx3600" => "interop.import_edl",
        "fcpxml" | "fcp" => "interop.import_fcpxml",
        "premiere_xml" | "premiere" | "xml" | "xmeml" => "interop.import_xmeml_xml",
        "otio" | "opentimelineio" => "interop.import_otio",
        _ => return Err(format!("Unsupported interchange format: {format}")),
    };

    let payload = if cmd_name == "interop.import_edl" {
        json!({
            "edl": raw_str,
            "project_name": project_name,
        })
    } else if cmd_name == "interop.import_otio" {
        json!({
            "content": raw_str,
            "project_name": project_name,
        })
    } else {
        json!({
            "xml": raw_str,
        })
    };

    let res = engine
        .execute(CommandRequest::new(cmd_name, payload))
        .map_err(|e| e.to_string())?;
    Ok(res)
}

/// Reveal a filesystem path in the system file manager (the reference
/// editor's "Reveal in/// Finder"). Opens the containing directory so the file itself stays visible;
/// plain shell-out, no plugin dependency.
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    let directory = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| format!("Path has no parent directory: {path}"))?
    };
    // Canonicalize so the opened path is a real, existing directory (this
    // also resolves `..` and symlink chains before they reach xdg-open).
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("Directory not accessible: {error}"))?;
    if !directory.is_dir() {
        return Err(format!("Directory not found: {}", directory.display()));
    }
    std::process::Command::new("xdg-open")
        .arg(&directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open file manager: {error}"))
}
