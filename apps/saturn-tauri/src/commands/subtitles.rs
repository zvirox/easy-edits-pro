use super::super::*;

/// Read a caption file (SRT/WebVTT) so the frontend can hand the raw text to
/// `caption.import`. Parsing lives in the engine so the UI, CLI, and MCP
/// share one validator.
#[tauri::command]
pub async fn read_text_file(app: AppHandle, path: String) -> Result<String, String> {
    let validated_path = validate_read_path(&app, &path, Some(10 * 1024 * 1024))
        .map_err(|error| format!("Invalid caption file path: {error}"))?;
    std::fs::read_to_string(&validated_path)
        .map_err(|error| format!("Could not read caption file: {error}"))
}

#[tauri::command]
pub async fn import_subtitles_file(
    app: AppHandle,
    path: String,
) -> Result<Vec<saturn_core::project::SubtitleItem>, String> {
    let validated_path = validate_read_path(&app, &path, Some(10 * 1024 * 1024)) // 10 MB limit for subtitle files
        .map_err(|e| format!("Invalid subtitle file path: {e}"))?;
    let content = std::fs::read_to_string(&validated_path)
        .map_err(|e| format!("Could not read subtitles file: {e}"))?;
    saturn_core::subtitles::detect_and_parse_subtitles(&content)
}

#[tauri::command]
pub async fn export_subtitles_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    format: String,
) -> Result<(), String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    // Cues come from the default (first) caption track (schema v13).
    let subtitles = engine
        .document()
        .project
        .sequence
        .caption_tracks
        .first()
        .map(|track| &track.cues[..])
        .unwrap_or(&[]);
    let text = if format.eq_ignore_ascii_case("vtt") || path.ends_with(".vtt") {
        saturn_core::subtitles::export_vtt(subtitles)
    } else {
        saturn_core::subtitles::export_srt(subtitles)
    };
    let validated_path = validate_write_path(&app, &path, Some(10 * 1024 * 1024))
        .map_err(|e| format!("Invalid subtitle output path: {e}"))?;
    std::fs::write(&validated_path, text).map_err(|e| format!("Could not write subtitles: {e}"))
}
