// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Video scope telemetry for the AI feedback loop (Packet 1.2).
//!
//! Computes a [`ScopeTelemetryReport`] from a raw RGB frame: luma IRE
//! percentiles, highlight-clip and shadow-crush percentages, RGB parade
//! medians, vectorscope saturation, and — as *chroma telemetry only* — the
//! skin-tone-line metrics. All numbers are direct measurements of the
//! analyzed pixels; there is deliberately **no invented aggregate "quality"
//! score** (plan-review constraint §4.4).
//!
//! Skin tone (plan-review constraint §4.5): measured as a **chroma-band**
//! statistic relative to the broadcast flesh-tone line reference, never by
//! any ethnicity label and never with group-specific IRE targets. The
//! reported mean IRE is what to compare against *your reference shot's*
//! measured value.
//!
//! The file is named `scopes_gpu.rs` per the plan, but the analysis is a
//! deterministic O(n) histogram pass on the CPU: for telemetry-sized frames
//! (a few megapixels) this is exact, reproducible across machines, and fast
//! enough that a GPU path would add nondeterminism, not speed. The report
//! type stays the place to plug a wgpu implementation later.

use saturn_core::Tick;
use serde::{Deserialize, Serialize};

/// The flesh-tone line on a standard vectorscope: +123° from the +U axis
/// (U = Cb−128 to the right, V = Cr−128 up, CCW positive). This is the
/// *reference target* the deviation is measured against; it is user-judgeable
/// telemetry, not a correctness verdict.
pub const SKIN_TONE_LINE_DEG: f32 = 123.0;

/// Luma at or above this IRE counts as clipped (raw Y ≥ ~254). 8-bit RGB
/// quantizes 100 IRE to raw 255, so the ceiling is the only observable
/// "over 100 IRE" state.
pub const CLIP_THRESHOLD_IRE: f32 = 99.5;
/// Luma at or below this IRE counts as crushed (raw Y ≤ ~1).
pub const CRUSH_THRESHOLD_IRE: f32 = 0.5;

/// A region of interest in analyzed-frame pixels. Clamped to the frame; an
/// empty or outside frame ROI is a typed error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Roi {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Roi {
    fn clamped(self, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        // A ROI fully outside the frame is a caller error, not something to
        // silently relocate; partial overlaps clamp to the intersection.
        if self.x >= width || self.y >= height {
            return None;
        }
        let x = self.x.min(width.saturating_sub(1));
        let y = self.y.min(height.saturating_sub(1));
        let w = self.w.min(width - x);
        let h = self.h.min(height - y);
        if w == 0 || h == 0 {
            None
        } else {
            Some((x, y, w, h))
        }
    }
}

/// Measurable scope telemetry for one frame (or ROI of a frame).
///
/// Every field is a direct pixel measurement. `skin_tone_*` fields are
/// chroma-band statistics against the [`SKIN_TONE_LINE_DEG`] reference —
/// compare them with a reference shot's numbers; they never gate anything
/// on their own.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeTelemetryReport {
    pub at_tick: Tick,
    pub width: u32,
    pub height: u32,
    pub roi: Option<Roi>,
    /// True when the analysis sampled the frame (stride > 1) to stay inside
    /// the `MAX_ANALYSIS_PIXELS` budget; histogram statistics stay exact
    /// within sampling error, stated so callers can judge confidence.
    pub sampled: bool,
    pub luma_ire_min: f32,
    pub luma_ire_max: f32,
    pub luma_ire_p10: f32,
    pub luma_ire_p50: f32,
    pub luma_ire_p90: f32,
    /// % of analyzed pixels with luma ≥ [`CLIP_THRESHOLD_IRE`].
    pub clipped_highlights_pct: f32,
    /// % of analyzed pixels with luma ≤ [`CRUSH_THRESHOLD_IRE`].
    pub crushed_shadows_pct: f32,
    /// Per-channel median, 0..100 IRE scale (R, G, B order) — the parade.
    pub rgb_parade_medians: [f32; 3],
    /// Mean HSV saturation, 0..100.
    pub vectorscope_mean_sat_pct: f32,
    /// 90th-percentile HSV saturation, 0..100 — the "punchiest" decile.
    pub vectorscope_p90_sat_pct: f32,
    /// Mean chroma angle over the whole frame, degrees 0..360 (0° = +U).
    pub chroma_angle_mean_deg: f32,
    /// % of pixels inside the skin-tone chroma band (the detection basis —
    /// small values mean the skin metrics are on few pixels).
    pub skin_pixel_pct: f32,
    /// `true` when ≥ [`SKIN_DETECT_MIN_PIXEL_PCT`] of pixels fall in the
    /// skin-tone chroma band.
    pub skin_tone_detected: bool,
    /// Circular difference between the mean chroma angle of skin-band pixels
    /// and the 123° flesh-tone reference, in degrees (signed, −180..180).
    pub skin_tone_iline_deviation_deg: Option<f32>,
    /// Mean luma IRE of skin-band pixels — compare against a reference shot,
    /// not against any group-specific target (plan-review §4.5).
    pub skin_tone_ire_mean: Option<f32>,
}

/// Above this analyzed-pixel count the analysis samples with a stride.
/// ~4 Mpixels keep a full report in a few milliseconds.
pub const MAX_ANALYSIS_PIXELS: u64 = 4_000_000;
/// Minimum share of skin-band pixels for `skin_tone_detected`.
pub const SKIN_DETECT_MIN_PIXEL_PCT: f32 = 0.5;
/// The classic YCbCr skin-chroma band (broadcast heuristic): Cb 77..127,
/// Cr 133..173. A *chroma* band — luma, hue objects (wood, sand) can enter
/// it, which is why the pixel share is reported alongside.
pub const SKIN_CB_RANGE: (f32, f32) = (77.0, 127.0);
pub const SKIN_CR_RANGE: (f32, f32) = (133.0, 173.0);

fn percentile_from_histogram(histogram: &[u64; 256], total: u64, which: f32) -> f32 {
    let target = (which / 100.0) * (total as f32);
    let mut cumulative = 0u64;
    for (value, &count) in histogram.iter().enumerate() {
        cumulative += count;
        if (cumulative as f32) >= target {
            return value as f32 * 100.0 / 255.0;
        }
    }
    100.0
}

fn median_from_histogram(histogram: &[u64; 256], total: u64) -> f32 {
    percentile_from_histogram(histogram, total, 50.0)
}

/// HSV saturation of one RGB triple, 0..100.
fn saturation_pct(r: u8, g: u8, b: u8) -> f32 {
    let max = r.max(g).max(b) as f32;
    if max <= 0.0 {
        return 0.0;
    }
    let min = r.min(g).min(b) as f32;
    (max - min) / max * 100.0
}

/// Analyze a tightly packed RGB frame (`rgb.len() == width * height * 3`).
/// `roi` restricts the analysis to a sub-rectangle of frame pixels.
pub fn analyze_rgb_scopes(
    rgb: &[u8],
    width: u32,
    height: u32,
    roi: Option<Roi>,
    at_tick: Tick,
) -> Result<ScopeTelemetryReport, String> {
    if width == 0 || height == 0 {
        return Err("Scope analysis needs a non-empty frame".into());
    }
    let expected = width as usize * height as usize * 3;
    if rgb.len() < expected {
        return Err(format!(
            "Frame buffer is too small: {} bytes for {}x{} RGB (need {expected})",
            rgb.len(),
            width,
            height
        ));
    }
    let (x0, y0, roi_w, roi_h) = match roi {
        Some(roi) => roi.clamped(width, height).ok_or_else(|| {
            format!("ROI {roi:?} is empty after clamping to the {width}x{height} frame")
        })?,
        None => (0, 0, width, height),
    };

    // Stride sampling keeps the pass bounded on huge frames while staying a
    // uniform, spatially spread sample (documented via `sampled`).
    let roi_pixels = roi_w as u64 * roi_h as u64;
    let stride = ((roi_pixels / MAX_ANALYSIS_PIXELS.max(1)).max(1) as u32)
        .next_power_of_two()
        .max(1);
    let sampled = stride > 1;

    let mut luma_histogram = [0u64; 256];
    let mut channel_histograms = [[0u64; 256]; 3];
    let mut clip = 0u64;
    let mut crush = 0u64;
    let mut sat_sum = 0u64; // saturation × 100, integer-fixed
    let mut sat_histogram = [0u64; 101];
    let mut angle_sin = 0f64;
    let mut angle_cos = 0f64;
    let mut skin_count = 0u64;
    let mut skin_angle_sin = 0f64;
    let mut skin_angle_cos = 0f64;
    let mut skin_ire_sum = 0f64;
    let mut analyzed = 0u64;

    for row in (y0..y0 + roi_h).step_by(stride as usize) {
        for col in (x0..x0 + roi_w).step_by(stride as usize) {
            let index = (row as usize * width as usize + col as usize) * 3;
            let (r, g, b) = (rgb[index], rgb[index + 1], rgb[index + 2]);
            // Rec. 709 luma for modern-scope telemetry (not Rec. 601 luma,
            // which the chroma plane below uses, matching vectorscope math).
            let y_raw = (0.212_6 * r as f32 + 0.715_2 * g as f32 + 0.072_2 * b as f32).round();
            let y_raw = y_raw.clamp(0.0, 255.0) as u8;
            let ire = y_raw as f32 * 100.0 / 255.0;
            luma_histogram[y_raw as usize] += 1;
            if ire >= CLIP_THRESHOLD_IRE {
                clip += 1;
            }
            if ire <= CRUSH_THRESHOLD_IRE {
                crush += 1;
            }
            channel_histograms[0][r as usize] += 1;
            channel_histograms[1][g as usize] += 1;
            channel_histograms[2][b as usize] += 1;

            let sat = saturation_pct(r, g, b);
            sat_sum += (sat * 100.0).round() as u64;
            sat_histogram[(sat.round() as usize).min(100)] += 1;

            // Rec. 601 chroma plane (the vectorscope's convention).
            let cb = (128.0 - 0.168_736 * r as f32 - 0.331_264 * g as f32 + 0.5 * b as f32).clamp(0.0, 255.0);
            let cr = (128.0 + 0.5 * r as f32 - 0.418_688 * g as f32 - 0.081_312 * b as f32).clamp(0.0, 255.0);
            let (u, v) = (cb - 128.0, cr - 128.0);
            let magnitude = (u.hypot(v)) as f64;
            if magnitude > 1.0 {
                let u = u as f64;
                let v = v as f64;
                let sin = v / magnitude;
                let cos = u / magnitude;
                angle_sin += sin;
                angle_cos += cos;
                let in_skin_band =
                    cb >= SKIN_CB_RANGE.0 && cb <= SKIN_CB_RANGE.1 && cr >= SKIN_CR_RANGE.0 && cr <= SKIN_CR_RANGE.1;
                if in_skin_band {
                    skin_count += 1;
                    skin_angle_sin += sin;
                    skin_angle_cos += cos;
                    skin_ire_sum += ire as f64;
                }
            }
            analyzed += 1;
        }
    }

    if analyzed == 0 {
        return Err("Scope analysis sampled zero pixels".into());
    }

    let analyzed_f = analyzed as f64;
    let sat_values = |which: f32| -> f32 {
        let target = (which / 100.0) * analyzed_f as f32;
        let mut cumulative = 0f64;
        for (value, &count) in sat_histogram.iter().enumerate() {
            cumulative += count as f64;
            if cumulative >= f64::from(target) {
                return value as f32;
            }
        }
        100.0
    };

    let skin_pixel_pct = skin_count as f64 / analyzed_f * 100.0; // f64, cast below
    let (skin_deviation, skin_ire_mean) = if skin_count > 0 {
        let mean_angle = skin_angle_sin.atan2(skin_angle_cos).to_degrees();
        let mean_angle = if mean_angle < 0.0 { mean_angle + 360.0 } else { mean_angle };
        let mut deviation = mean_angle - f64::from(SKIN_TONE_LINE_DEG);
        if deviation > 180.0 {
            deviation -= 360.0;
        }
        if deviation < -180.0 {
            deviation += 360.0;
        }
        (Some(deviation as f32), Some((skin_ire_sum / skin_count as f64) as f32))
    } else {
        (None, None)
    };
    let chroma_angle_mean = {
        let angle = angle_sin.atan2(angle_cos).to_degrees();
        if angle < 0.0 { angle + 360.0 } else { angle }
    };

    Ok(ScopeTelemetryReport {
        at_tick,
        width,
        height,
        roi,
        sampled,
        luma_ire_min: percentile_from_histogram(&luma_histogram, analyzed, 0.0),
        luma_ire_max: percentile_from_histogram(&luma_histogram, analyzed, 100.0),
        luma_ire_p10: percentile_from_histogram(&luma_histogram, analyzed, 10.0),
        luma_ire_p50: percentile_from_histogram(&luma_histogram, analyzed, 50.0),
        luma_ire_p90: percentile_from_histogram(&luma_histogram, analyzed, 90.0),
        clipped_highlights_pct: (clip as f64 / analyzed_f * 100.0) as f32,
        crushed_shadows_pct: (crush as f64 / analyzed_f * 100.0) as f32,
        rgb_parade_medians: [
            median_from_histogram(&channel_histograms[0], analyzed),
            median_from_histogram(&channel_histograms[1], analyzed),
            median_from_histogram(&channel_histograms[2], analyzed),
        ],
        vectorscope_mean_sat_pct: (sat_sum as f64 / analyzed_f / 100.0) as f32,
        vectorscope_p90_sat_pct: sat_values(90.0),
        chroma_angle_mean_deg: chroma_angle_mean as f32,
        skin_pixel_pct: skin_pixel_pct as f32,
        skin_tone_detected: skin_pixel_pct >= f64::from(SKIN_DETECT_MIN_PIXEL_PCT),
        skin_tone_iline_deviation_deg: skin_deviation,
        skin_tone_ire_mean: skin_ire_mean,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
        vec![rgb; (width * height) as usize].into_iter().flatten().collect()
    }

    #[test]
    fn clip_gate_synthetic_white_frame_flags_over_95_percent() {
        // Phase 1 exit criterion: a synthetic clipped-white frame.
        let frame = solid(320, 240, [255, 255, 255]);
        let report =
            analyze_rgb_scopes(&frame, 320, 240, None, Tick(0)).expect("white frame analyzes");
        assert!(
            report.clipped_highlights_pct > 95.0,
            "clipped_highlights_pct must flag the white frame: {}",
            report.clipped_highlights_pct
        );
        assert_eq!(report.luma_ire_p50, 100.0);
        assert!(!report.sampled);
    }

    #[test]
    fn crushed_black_frame_flags_shadow_crush() {
        let frame = solid(160, 90, [0, 0, 0]);
        let report =
            analyze_rgb_scopes(&frame, 160, 90, None, Tick(0)).expect("black frame analyzes");
        assert!(report.crushed_shadows_pct > 95.0);
        assert_eq!(report.luma_ire_p50, 0.0);
        assert_eq!(report.vectorscope_mean_sat_pct, 0.0, "black is unsaturated");
    }

    #[test]
    fn luma_ramp_percentiles_track_the_ramp() {
        // A horizontal 0..255 ramp: the p-values should sit at ~10/50/90 IRE.
        let width: u32 = 256;
        let height: u32 = 8;
        let mut frame = Vec::with_capacity((width * height * 3) as usize);
        for col in 0..width {
            let value = ((col * 255) / (width - 1)) as u8;
            frame.extend_from_slice(&[value, value, value]);
        }
        for _ in 0..height - 1 {
            frame.extend_from_within(0..(width * 3) as usize);
        }
        let report =
            analyze_rgb_scopes(&frame, width, height, None, Tick(0)).expect("ramp analyzes");
        assert!((report.luma_ire_p10 - 10.0).abs() < 2.0, "p10 = {}", report.luma_ire_p10);
        assert!((report.luma_ire_p50 - 50.0).abs() < 2.0, "p50 = {}", report.luma_ire_p50);
        assert!((report.luma_ire_p90 - 90.0).abs() < 2.0, "p90 = {}", report.luma_ire_p90);
    }

    #[test]
    fn skin_chroma_block_is_detected_on_the_reference_line() {
        // RGB chosen so the Rec.601 chroma vector points exactly along the
        // 123° flesh-tone reference: a typical mid skin tone.
        let width = 64;
        let height = 64;
        let frame = solid(width, height, [185, 138, 121]);
        let report = analyze_rgb_scopes(&frame, width, height, None, Tick(0)).expect("skin frame");
        assert!(report.skin_tone_detected, "skin band pixels = {}%", report.skin_pixel_pct);
        let deviation = report.skin_tone_iline_deviation_deg.expect("deviation reported");
        assert!(deviation.abs() < 2.0, "on-line skin must read ~0°: {deviation}");
        // Mid skin tone sits around 50..60 IRE with Rec.709 luma.
        let ire = report.skin_tone_ire_mean.expect("skin IRE reported");
        assert!((40.0..=70.0).contains(&ire), "skin IRE mean = {ire}");
    }

    #[test]
    fn roi_restricts_the_analysis() {
        let width = 64u32;
        let height = 64u32;
        let mut frame = solid(width, height, [0, 0, 0]);
        // A white quadrant inside a black frame.
        for row in 0..32u32 {
            for col in 32..64u32 {
                let index = ((row * width + col) * 3) as usize;
                frame[index..index + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let all = analyze_rgb_scopes(&frame, width, height, None, Tick(0)).unwrap();
        let quadrant = analyze_rgb_scopes(
            &frame,
            width,
            height,
            Some(Roi { x: 32, y: 0, w: 32, h: 32 }),
            Tick(0),
        )
        .unwrap();
        assert!(all.clipped_highlights_pct < 60.0, "mixed frame is half white");
        assert!(quadrant.clipped_highlights_pct > 95.0, "ROI is pure white");
    }

    #[test]
    fn saturation_separates_gray_from_saturated_color() {
        let gray = solid(32, 32, [128, 128, 128]);
        let red = solid(32, 32, [220, 40, 40]);
        let gray_report = analyze_rgb_scopes(&gray, 32, 32, None, Tick(0)).unwrap();
        let red_report = analyze_rgb_scopes(&red, 32, 32, None, Tick(0)).unwrap();
        assert!(gray_report.vectorscope_mean_sat_pct < 1.0);
        assert!(red_report.vectorscope_mean_sat_pct > 70.0);
        // A saturated red frame's chroma angle should point near the red
        // target (~109° from +U in the Rec.601 plane).
        assert!(
            (red_report.chroma_angle_mean_deg - 108.6).abs() < 2.0,
            "red chroma angle = {}",
            red_report.chroma_angle_mean_deg
        );
    }

    #[test]
    fn invalid_buffers_and_rois_are_typed_errors() {
        assert!(analyze_rgb_scopes(&[0u8; 10], 8, 8, None, Tick(0)).is_err());
        assert!(analyze_rgb_scopes(&[0u8; 0], 0, 0, None, Tick(0)).is_err());
        let frame = solid(16, 16, [128; 3]);
        let outside = analyze_rgb_scopes(
            &frame,
            16,
            16,
            Some(Roi { x: 100, y: 100, w: 10, h: 10 }),
            Tick(0),
        );
        assert!(outside.is_err());
    }
}
