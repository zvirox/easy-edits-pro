use super::super::*;

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
    let subtitles = &engine.document().project.sequence.subtitles;
    let text = if format.eq_ignore_ascii_case("vtt") || path.ends_with(".vtt") {
        saturn_core::subtitles::export_vtt(subtitles)
    } else {
        saturn_core::subtitles::export_srt(subtitles)
    };
    let validated_path = validate_write_path(&app, &path, Some(10 * 1024 * 1024))
        .map_err(|e| format!("Invalid subtitle output path: {e}"))?;
    std::fs::write(&validated_path, text).map_err(|e| format!("Could not write subtitles: {e}"))
}
