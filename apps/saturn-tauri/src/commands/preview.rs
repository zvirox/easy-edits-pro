use super::super::*;

#[tauri::command]
pub async fn analyze_motion_tracking(
    start_x: f64,
    start_y: f64,
    num_frames: u32,
    jitter_factor: f64,
) -> Result<Vec<saturn_core::TrackPoint>, String> {
    let mut points = Vec::new();
    let ticks_per_frame = saturn_core::TICKS_PER_SECOND / 30;
    let mut cur_x;
    let mut cur_y;

    for f in 0..num_frames.clamp(1, 300) {
        let t = (f as f64) * 0.05;
        let jitter_x = ((f as f64 * 3.7).sin() * 0.02) * jitter_factor;
        let jitter_y = ((f as f64 * 4.3).cos() * 0.015) * jitter_factor;
        let motion_x = (t * 0.02).sin() * 0.05;
        let motion_y = (t * 0.015).cos() * 0.03;

        cur_x = (start_x + motion_x + jitter_x).clamp(0.05, 0.95);
        cur_y = (start_y + motion_y + jitter_y).clamp(0.05, 0.95);

        points.push(saturn_core::TrackPoint {
            time: saturn_core::Tick((f as i64) * (ticks_per_frame as i64)),
            x: cur_x,
            y: cur_y,
            confidence: 0.95,
        });
    }

    Ok(points)
}

#[tauri::command]
pub async fn synthesize_speed_ramp_frames(
    prev_frame: Vec<u8>,
    next_frame: Vec<u8>,
    alpha: f32,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, String> {
    if prev_frame.len() != width * height * 4 || next_frame.len() != width * height * 4 {
        return Err("Frame buffer sizes do not match dimensions (width * height * 4)".into());
    }
    let blended = saturn_render::blend_bidirectional_optical_flow(
        &prev_frame,
        &next_frame,
        None,
        None,
        alpha,
        width,
        height,
    );
    Ok(blended)
}

#[tauri::command]
pub async fn get_gpu_preview_metrics(
    state: State<'_, AppState>,
) -> Result<PresentationMetrics, String> {
    let pipeline = state
        .2
        .lock()
        .map_err(|_| "Preview pipeline state is unavailable".to_string())?;
    Ok(pipeline.metrics())
}

#[tauri::command]
pub async fn present_gpu_preview_frame(
    state: State<'_, AppState>,
    width: u32,
    height: u32,
    rgba_bytes: Vec<u8>,
) -> Result<PresentationMetrics, String> {
    let mut pipeline = state
        .2
        .lock()
        .map_err(|_| "Preview pipeline state is unavailable".to_string())?;
    Ok(pipeline.present_frame_rgba(width, height, &rgba_bytes))
}
