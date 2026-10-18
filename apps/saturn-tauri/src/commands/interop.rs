use super::super::*;

#[tauri::command]
pub async fn export_timeline_interop(
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
        "premiere_xml" | "premiere" | "xml" | "xmeml" => export_premiere_xml(doc)?,
        "otio" | "opentimelineio" => export_otio(doc)?,
        _ => {
            return Err(format!(
                "Unsupported interchange format: {format}. Supported: 'edl', 'fcpxml', 'premiere_xml', 'otio'"
            ));
        }
    };

    if let Some(target_path) = path {
        std::fs::write(&target_path, &content).map_err(|e| {
            format!("Could not write timeline interchange export to `{target_path}`: {e}")
        })?;
    }

    Ok(content)
}

#[tauri::command]
pub async fn import_timeline_interop(
    state: State<'_, AppState>,
    format: String,
    content: Option<String>,
    path: Option<String>,
    project_name: Option<String>,
) -> Result<Value, String> {
    let raw_str = if let Some(c) = content {
        c
    } else if let Some(p) = path {
        std::fs::read_to_string(&p)
            .map_err(|e| format!("Could not read interchange file `{p}`: {e}"))?
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
        "premiere_xml" | "premiere" | "xml" | "xmeml" => "interop.import_premiere_xml",
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
