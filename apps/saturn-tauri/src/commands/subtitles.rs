use super::super::*;

#[tauri::command]
pub async fn import_subtitles_file(
    path: String,
) -> Result<Vec<saturn_core::project::SubtitleItem>, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read subtitles file `{path}`: {e}"))?;
    saturn_core::subtitles::detect_and_parse_subtitles(&content)
}

#[tauri::command]
pub async fn export_subtitles_file(
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
    std::fs::write(&path, text).map_err(|e| format!("Could not write subtitles to `{path}`: {e}"))
}
