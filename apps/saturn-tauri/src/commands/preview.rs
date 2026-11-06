// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::super::*;

/// Result of real, measured motion tracking. Every confidence is the mean
/// ZNCC of the RANSAC inlier patches (0..1); dropped ticks honestly report
/// frames where fewer than 4 patches matched well enough to fit a
/// homography — they are never interpolated.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionTrackResult {
    pub points: Vec<saturn_core::TrackPoint>,
    pub ticks_dropped: Vec<i64>,
    /// Mean RANSAC forward-transfer error over tracked frames (pixels in
    /// the decode resolution the tracker ran at).
    pub mean_reprojection_error_px: f64,
    pub track_width_px: u32,
    pub track_height_px: u32,
    pub frames_sampled: usize,
}

/// Track one region of a media asset across a bounded frame window with
/// measured patch matching + RANSAC homography, replacing the synthetic
/// sine/cosine data this command used to return.
///
/// Coordinate mapping: `start_x`/`start_y` are normalized 0..1 references
/// into the frame; returned `TrackPoint` x/y are normalized the same way.
/// Frames decode at a bounded resolution (long edge ≤ 960 px) so long media
/// cannot exhaust memory — normalized outputs are resolution-invariant and
/// the reprojection error is reported in that decode resolution.
#[tauri::command]
pub async fn analyze_motion_tracking(
    state: State<'_, AppState>,
    media_id: u64,
    start_x: f64,
    start_y: f64,
    num_frames: Option<u32>,
    search_radius: Option<u32>,
) -> Result<MotionTrackResult, String> {
    if !(0.0..=1.0).contains(&start_x) || !(0.0..=1.0).contains(&start_y) {
        return Err(format!(
            "start ({start_x}, {start_y}) must be normalized coordinates in 0..1"
        ));
    }
    // Snapshot asset metadata under the engine lock; never hold the lock
    // across decoding or tracking.
    let (source, fps) = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_owned())?;
        let asset = engine
            .document()
            .project
            .media
            .iter()
            .find(|asset| asset.id == media_id)
            .ok_or_else(|| format!("Media asset {media_id} was not found"))?;
        match &asset.kind {
            MediaKind::Video | MediaKind::Image => {}
            other => {
                return Err(format!(
                    "analyze_motion_tracking: {other:?} media has no video frames"
                ));
            }
        }
        if !asset.path.is_file() {
            return Err(format!(
                "analyze_motion_tracking: source file is missing: {}",
                asset.path.display()
            ));
        }
        (asset.path.clone(), asset.fps)
    };

    let frames_wanted = num_frames.unwrap_or(24).clamp(2, 48);
    let search_radius = search_radius.unwrap_or(12).clamp(4, 40);

    tauri::async_runtime::spawn_blocking(move || {
        // One-frame step from the media fps (fallback 30 fps when absent).
        let step_ms = fps
            .filter(|fps| *fps > 0.0)
            .map(|fps| (1000.0 / fps).round() as i64)
            .unwrap_or(33)
            .max(1);
        let decode = |tick: Tick| -> Result<saturn_perception::SampledFrame, String> {
            let (width, height, rgb) =
                saturn_render::frame_extractor::decode_frame_rgb(&source, tick)?;
            // Bounded-resolution tracking window (see doc comment).
            let mut frame = saturn_perception::shot_detect::SampledFrame {
                tick,
                width,
                height,
                rgb,
            };
            frame = saturn_perception::downscale_frame_rgb(&frame, 960);
            Ok(frame)
        };

        let mut frames = Vec::with_capacity(frames_wanted as usize);
        for index in 0..frames_wanted {
            let tick = Tick(index as i64 * step_ms);
            frames.push(decode(tick).map_err(|error| {
                format!("analyze_motion_tracking: could not decode a frame at tick {}: {error}", tick.0)
            })?);
        }
        let first = &frames[0];
        // Region: a centered block ~25% of the frame on each axis, snapped
        // to sane bounds so the 3×3 patch grid always fits.
        let track_w = ((first.width as f64 / 4.0).round() as u32).clamp(96, 240);
        let track_h = ((first.height as f64 / 4.0).round() as u32).clamp(96, 240);
        let center_x = (start_x * first.width as f64).round() as f32;
        let center_y = (start_y * first.height as f64).round() as f32;
        let region_x = (center_x - track_w as f32 / 2.0)
            .clamp(0.0, (first.width - track_w) as f32);
        let region_y = (center_y - track_h as f32 / 2.0)
            .clamp(0.0, (first.height - track_h) as f32);

        let tracked = saturn_perception::track_region_frames(
            &frames,
            (region_x, region_y, track_w, track_h),
            search_radius,
        )?;

        let reference_center = [
            (region_x + track_w as f32 / 2.0) as f64,
            (region_y + track_h as f32 / 2.0) as f64,
        ];
        let mut points = Vec::with_capacity(tracked.len());
        let mut ticks_dropped = Vec::new();
        let mut reprojection_sum = 0.0_f64;
        let mut tracked_count = 0_usize;
        for (frame, entry) in frames.iter().zip(&tracked) {
            match entry {
                Some(region) => {
                    let mapped = saturn_perception::apply_homography(
                        &region.homography,
                        reference_center,
                    );
                    points.push(saturn_core::TrackPoint {
                        time: frame.tick,
                        x: (mapped[0] / first.width as f64).clamp(0.0, 1.0),
                        y: (mapped[1] / first.height as f64).clamp(0.0, 1.0),
                        confidence: region.confidence as f64,
                    });
                    reprojection_sum += region.reprojection_error_px;
                    tracked_count += 1;
                }
                None => ticks_dropped.push(frame.tick.0),
            }
        }
        Ok(MotionTrackResult {
            points,
            ticks_dropped,
            mean_reprojection_error_px: if tracked_count > 0 {
                reprojection_sum / tracked_count as f64
            } else {
                0.0
            },
            track_width_px: track_w,
            track_height_px: track_h,
            frames_sampled: frames.len(),
        })
    })
    .await
    .map_err(|error| format!("motion tracking task failed: {error}"))?
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
