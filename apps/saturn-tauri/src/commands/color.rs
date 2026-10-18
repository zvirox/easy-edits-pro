#[tauri::command]
pub async fn load_lut_cube_file(path: String) -> Result<saturn_render::lut::Lut3DInfo, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read LUT file `{path}`: {e}"))?;
    let lut = saturn_render::lut::Lut3D::parse_cube(&content)?;
    Ok(lut.info())
}

#[tauri::command]
pub async fn get_lut_preset_cube(preset_id: String) -> Result<String, String> {
    saturn_render::lut::Lut3D::generate_preset_cube(&preset_id, 17)
}
