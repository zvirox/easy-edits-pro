use super::super::*;

#[tauri::command]
pub async fn load_lut_cube_file(
    app: AppHandle,
    path: String,
) -> Result<saturn_render::lut::Lut3DInfo, String> {
    let validated_path = validate_read_path(&app, &path, Some(5 * 1024 * 1024)) // 5 MB limit for LUT files
        .map_err(|e| format!("Invalid LUT file path: {e}"))?;
    let content = std::fs::read_to_string(&validated_path)
        .map_err(|e| format!("Could not read LUT file: {e}"))?;
    let lut = saturn_render::lut::Lut3D::parse_cube(&content)?;
    Ok(lut.info())
}

#[tauri::command]
pub async fn get_lut_preset_cube(preset_id: String) -> Result<String, String> {
    saturn_render::lut::Lut3D::generate_preset_cube(&preset_id, 17)
}
