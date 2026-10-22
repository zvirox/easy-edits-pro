use super::super::*;

/// Read a marker CSV file so the frontend can hand it to `marker.import_csv`.
/// Parsing lives in the engine so the UI, CLI, and MCP share one validator.
#[tauri::command]
pub async fn read_markers_file(app: AppHandle, path: String) -> Result<String, String> {
    let validated_path = validate_read_path(&app, &path, Some(4 * 1024 * 1024))
        .map_err(|error| format!("Invalid marker file path: {error}"))?;
    std::fs::read_to_string(&validated_path)
        .map_err(|error| format!("Could not read marker file: {error}"))
}

/// Write marker CSV produced by `marker.export_csv`.
#[tauri::command]
pub async fn write_markers_file(app: AppHandle, path: String, csv: String) -> Result<(), String> {
    if csv.len() > 4 * 1024 * 1024 {
        return Err("Marker CSV is larger than the 4 MB limit".into());
    }
    let validated_path = validate_write_path(&app, &path, Some(4 * 1024 * 1024))
        .map_err(|error| format!("Invalid marker output path: {error}"))?;
    std::fs::write(&validated_path, csv).map_err(|error| format!("Could not write markers: {error}"))
}
