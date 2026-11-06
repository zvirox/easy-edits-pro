// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Shot detection and cut-continuity measurement (Packet 2.2).
//!
//! Everything here is deterministic, model-free image/audio math over frames
//! the caller decodes (via `saturn-render`). Honesty constraints from the
//! plan review §4.4 shape this module:
//!
//! - "Eye-trace" needs a saliency model, so what is reported is a cheap
//!   **luminance-centroid proxy** (`focal_centroid_distance_norm`), labelled
//!   as such — it is *not* claimed to be gaze.
//! - The 180° rule needs subject/face orientation, which no model in this
//!   packet provides, so `screen_direction_preserved` is always `None`
//!   rather than a guess.
//! - The plan's `overall_murch_score` (0–100 "invisible Hollywood cut") is
//!   an invented scalar with no ground truth; the combined value here is
//!   named `heuristic_continuity_score`, documented as unvalidated, and it
//!   never gates anything by itself.
//! - Gradual transitions (dissolves/fades) are **not** detected — only hard
//!   cuts; callers get that stated in a warning rather than discovering it.

use saturn_core::Tick;
use serde::{Deserialize, Serialize};

/// One decoded RGB8 frame the caller sampled from a source. `tick` is the
/// frame's position in the coordinate space the caller works in (media time
/// for analysis, timeline time for cut evaluation).
#[derive(Clone, Debug)]
pub struct SampledFrame {
    pub tick: Tick,
    pub width: u32,
    pub height: u32,
    /// RGB8, row-major, 3 bytes per pixel (`len == w*h*3`).
    pub rgb: Vec<u8>,
}

impl SampledFrame {
    fn luma(&self, x: u32, y: u32) -> u8 {
        let i = ((y * self.width + x) * 3) as usize;
        // Rec.601 luma on 0..255; matches the IRE-0..100 mapping used below.
        let (r, g, b) = (self.rgb[i] as u32, self.rgb[i + 1] as u32, self.rgb[i + 2] as u32);
        ((r * 299 + g * 587 + b * 114) / 1000) as u8
    }
}

/// Tunables for hard-cut detection; defaults suit ~2–4 fps sampling.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShotDetectOptions {
    /// Composite frame-difference score above this is a cut (0..1 scale).
    #[serde(default = "default_cut_threshold")]
    pub cut_threshold: f32,
    /// Minimum ticks between reported cuts (hysteresis against flicker).
    #[serde(default = "default_min_shot_gap_ticks")]
    pub min_shot_gap_ticks: i64,
}

fn default_cut_threshold() -> f32 {
    0.32
}
fn default_min_shot_gap_ticks() -> i64 {
    // 0.5 s of ticks by default — cuts closer than this are usually noise
    // at analysis sampling rates.
    Tick::from_millis(500).0
}

impl Default for ShotDetectOptions {
    fn default() -> Self {
        Self {
            cut_threshold: default_cut_threshold(),
            min_shot_gap_ticks: default_min_shot_gap_ticks(),
        }
    }
}

/// A detected (hard) shot boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShotBoundary {
    /// Tick of the first frame of the **new** shot, in the caller's space.
    pub tick: Tick,
    /// Composite difference score at the boundary (heuristic, 0..1).
    pub score: f32,
}

/// Detect hard shot boundaries across sampled frames.
///
/// Score per consecutive pair = 0.6·χ²(luma histogram) + 0.4·mean block
/// difference (downscaled 16×16 grid). Weights are arbitrary heuristics
/// (plan §4.4) — the *score* is meant for ranking, the boolean cut decision
/// is the threshold. Dissolves/fades are not detected.
pub fn detect_shot_boundaries(
    frames: &[SampledFrame],
    options: &ShotDetectOptions,
) -> Vec<ShotBoundary> {
    let mut cuts = Vec::new();
    let mut last_cut_tick: Option<i64> = None;
    for pair in frames.windows(2) {
        let score = frame_difference_score(&pair[0], &pair[1]);
        if score < options.cut_threshold {
            continue;
        }
        let tick = pair[1].tick.0;
        if let Some(last) = last_cut_tick {
            if tick - last < options.min_shot_gap_ticks {
                continue;
            }
        }
        last_cut_tick = Some(tick);
        cuts.push(ShotBoundary {
            tick: Tick(tick),
            score,
        });
    }
    cuts
}

/// Composite frame difference in 0..1 (clamped). Pure — unit-testable.
pub fn frame_difference_score(a: &SampledFrame, b: &SampledFrame) -> f32 {
    if a.width != b.width || a.height != b.height || a.rgb.len() != b.rgb.len() {
        return 0.0; // mismatched geometry cannot be compared honestly
    }
    let hist = 0.6 * histogram_distance(&frame_luma_histogram(a), &frame_luma_histogram(b));
    let blocks = 0.4 * block_difference(a, b);
    (hist + blocks).clamp(0.0, 1.0)
}

fn frame_luma_histogram(frame: &SampledFrame) -> [u64; 32] {
    let mut bins = [0_u64; 32];
    for y in 0..frame.height {
        for x in 0..frame.width {
            bins[(frame.luma(x, y) >> 3) as usize] += 1;
        }
    }
    bins
}

/// [1,2,1]/4 kernel over the histogram: a one-bin global shift then shares
/// most of its mass with the original bin instead of producing a maximal
/// χ² distance (uniform-frame exposure wiggles are not cuts).
fn smooth_histogram(bins: &[u64; 32]) -> [f64; 32] {
    let mut out = [0.0_f64; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let prev = if i > 0 { bins[i - 1] } else { 0 };
        let next = if i + 1 < 32 { bins[i + 1] } else { 0 };
        *slot = (prev + 2 * bins[i] + next) as f64 / 4.0;
    }
    out
}

/// Symmetric χ² distance over smoothed histograms, 0..1.
fn histogram_distance(a: &[u64; 32], b: &[u64; 32]) -> f32 {
    let (a, b) = (smooth_histogram(a), smooth_histogram(b));
    let total_a: f64 = a.iter().sum();
    let total_b: f64 = b.iter().sum();
    if total_a <= 0.0 || total_b <= 0.0 {
        return 0.0;
    }
    let mut chi = 0.0_f64;
    for (bin_a, bin_b) in a.iter().zip(b.iter()) {
        let fa = bin_a / total_a;
        let fb = bin_b / total_b;
        if fa + fb > 0.0 {
            chi += (fa - fb).powi(2) / (fa + fb);
        }
    }
    (0.5 * chi).min(1.0) as f32
}

/// Mean absolute luma difference over a coarse block grid, normalized to
/// 0..1 (mean |Δluma| over all sampled pixels ÷ 255).
fn block_difference(a: &SampledFrame, b: &SampledFrame) -> f32 {
    const GRID: u32 = 16;
    let blocks_x = GRID.min(a.width);
    let blocks_y = GRID.min(a.height);
    let (mut total, mut samples) = (0.0_f64, 0_u64);
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let sx = bx * a.width / blocks_x;
            let sy = by * a.height / blocks_y;
            let bw = (a.width / blocks_x).max(1);
            let bh = (a.height / blocks_y).max(1);
            // Keep sampling bounded (~≤64 luma lookups per block).
            let step = (bw.max(bh) / 8).max(1);
            for y in (sy..sy + bh).step_by(step as usize) {
                for x in (sx..sx + bw).step_by(step as usize) {
                    let d = (a.luma(x, y) as i64 - b.luma(x, y) as i64).unsigned_abs();
                    total += d as f64;
                    samples += 1;
                }
            }
        }
    }
    if samples == 0 {
        return 0.0;
    }
    ((total / samples as f64) / 255.0).min(1.0) as f32
}

// ---------------------------------------------------------------------------
// Cut continuity (review §4.4: measurable components + labelled heuristic)
// ---------------------------------------------------------------------------

/// Continuity measurements across one hard cut, in the caller's tick space.
///
/// Field names keep the plan's `CutContinuityScore` idea, but every invented
/// element is renamed/annulled per plan-review §4.4 — see module docs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CutContinuityScore {
    pub cut_tick: Tick,
    /// |ΔIRE| between the outgoing and incoming frame mean luma (0..1
    /// Rec.601 scale, i.e. IRE/100).
    pub luma_jump_ire: f32,
    /// CIEDE2000 between the two frames' *mean colors* — deliberately the
    /// cheap mean-color ΔE, not a pixelwise map (documented approximation).
    pub chromatic_delta_e: f32,
    /// Distance (0..1) between the frames' luminance centroids — a proxy
    /// for subject re-framing, **not** measured gaze/eye-trace.
    pub focal_centroid_distance_norm: f32,
    /// Angular difference (deg, 0..180) between the mean block-matching
    /// motion direction on each side of the cut; `None` when either side is
    /// near-static (< 0.5 px mean displacement) — a meaningless angle then.
    pub optical_flow_angle_diff_deg: Option<f32>,
    /// **Always `None` in this packet**: the 180°/screen-direction rule
    /// needs a face/subject orientation model that is not implemented.
    /// Reported honestly instead of guessed from color.
    pub screen_direction_preserved: Option<bool>,
    /// Heuristic pop at the audio boundary (dB above the local
    /// sample-difference scale); `None` when the cut carries no audio.
    pub audio_boundary_pop_db: Option<f32>,
    /// Weighted combination of the measured components, 0..1 (1 = smooth).
    /// **Unvalidated heuristic** — the weights are arbitrary (§4.4); check
    /// against labelled exemplar cuts before using it as an automated gate.
    pub heuristic_continuity_score: f32,
    /// Rule-based advice strings derived from the measurements.
    pub recommendations: Vec<String>,
}

/// Inputs for [`evaluate_cut_continuity`] beyond the two boundary frames.
#[derive(Clone, Debug, Default)]
pub struct CutContext<'a> {
    /// Mono 16 kHz PCM ending on the last sample **before** the cut
    /// (~100 ms) — or empty when the cut has no audio underneath.
    pub audio_before: &'a [f32],
    /// Mono 16 kHz PCM starting on the first sample **after** the cut.
    pub audio_after: &'a [f32],
    /// Second-to-last frame of the *outgoing* shot. Without it the outgoing
    /// motion direction is unmeasurable (matching frames across a hard cut
    /// measures the cut, not the action), so flow stays `None`.
    pub outgoing_prev: Option<&'a SampledFrame>,
    /// Second frame of the *incoming* shot — same reasoning as above.
    pub incoming_next: Option<&'a SampledFrame>,
}

/// Evaluate the continuity of one hard cut between the outgoing last frame
/// and the incoming first frame. Pure/deterministic.
pub fn evaluate_cut_continuity(
    outgoing: &SampledFrame,
    incoming: &SampledFrame,
    context: &CutContext<'_>,
    cut_tick: Tick,
) -> Result<CutContinuityScore, String> {
    if outgoing.width != incoming.width || outgoing.height != incoming.height {
        return Err("Cut evaluation needs both frames at the same geometry".into());
    }
    let luma_out = frame_mean_luma(outgoing) / 255.0;
    let luma_in = frame_mean_luma(incoming) / 255.0;
    let luma_jump_ire = (luma_out - luma_in).abs() as f32;

    let lab_out = mean_lab(outgoing);
    let lab_in = mean_lab(incoming);
    let chromatic_delta_e = crate::color_metrics::ciede2000(lab_out, lab_in) as f32;

    let focal_centroid_distance_norm =
        luminance_centroid_distance(outgoing, incoming);

    let optical_flow_angle_diff_deg =
        mean_motion_angle_diff(outgoing, incoming, context);

    let audio_boundary_pop_db = audio_pop_db(context.audio_before, context.audio_after);

    let (heuristic_continuity_score, recommendations) = combine_components(
        luma_jump_ire,
        chromatic_delta_e,
        focal_centroid_distance_norm,
        optical_flow_angle_diff_deg,
        audio_boundary_pop_db,
    );

    Ok(CutContinuityScore {
        cut_tick,
        luma_jump_ire,
        chromatic_delta_e,
        focal_centroid_distance_norm,
        optical_flow_angle_diff_deg,
        screen_direction_preserved: None, // needs an orientation model (§4.4)
        audio_boundary_pop_db,
        heuristic_continuity_score,
        recommendations,
    })
}

fn frame_mean_luma(frame: &SampledFrame) -> f64 {
    let mut total = 0_u64;
    for y in (0..frame.height).step_by(stride_for(frame.height)) {
        for x in (0..frame.width).step_by(stride_for(frame.width)) {
            total += frame.luma(x, y) as u64;
        }
    }
    let sampled = stride_samples(frame.height) * stride_samples(frame.width);
    let sampled = sampled.max(1);
    total as f64 / sampled as f64
}

fn stride_for(dim: u32) -> usize {
    if dim > 512 { 4 } else if dim > 256 { 2 } else { 1 }
}
fn stride_samples(dim: u32) -> u64 {
    let stride = stride_for(dim) as u64;
    (dim as u64).div_ceil(stride)
}

/// ITU-R BT.601 RGB (0..255) → CIE L\*a\*b\* (D65) — see `color_metrics`.
fn rgb_to_lab(r: u8, g: u8, b: u8) -> [f64; 3] {
    crate::color_metrics::rgb_to_lab(r, g, b)
}

/// Mean L\*a\*b\* over a strided sample of the frame. A coarse but stated
/// approximation: ΔE of the mean, not a mean of pixel ΔEs.
fn mean_lab(frame: &SampledFrame) -> [f64; 3] {
    let mut acc = [0.0_f64; 3];
    let mut count = 0_u64;
    let sy = stride_for(frame.height);
    let sx = stride_for(frame.width);
    for y in (0..frame.height).step_by(sy) {
        for x in (0..frame.width).step_by(sx) {
            let i = ((y * frame.width + x) * 3) as usize;
            let lab = rgb_to_lab(frame.rgb[i], frame.rgb[i + 1], frame.rgb[i + 2]);
            for (a, v) in acc.iter_mut().zip(lab) {
                *a += v;
            }
            count += 1;
        }
    }
    let count = count.max(1) as f64;
    [acc[0] / count, acc[1] / count, acc[2] / count]
}

/// Distance between the two frames' luminance centroids, normalized by the
/// frame diagonal (0..1.414 for corner-to-corner; callers see the raw norm).
fn luminance_centroid_distance(a: &SampledFrame, b: &SampledFrame) -> f32 {
    let centroid = |frame: &SampledFrame| -> (f64, f64) {
        let mut sum_w = 0.0;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let sy = stride_for(frame.height);
        let sx = stride_for(frame.width);
        for y in (0..frame.height).step_by(sy) {
            for x in (0..frame.width).step_by(sx) {
                // Contrast-weighted: brighter-than-mean pixels count as
                // salient mass (cheap, stated proxy — not a saliency model).
                let w = frame.luma(x, y).saturating_sub(100) as f64;
                sum_x += w * x as f64;
                sum_y += w * y as f64;
                sum_w += w;
            }
        }
        if sum_w <= 0.0 {
            (frame.width as f64 / 2.0, frame.height as f64 / 2.0)
        } else {
            (sum_x / sum_w, sum_y / sum_w)
        }
    };
    let (ax, ay) = centroid(a);
    let (bx, by) = centroid(b);
    let dx = ax - bx;
    let dy = ay - by;
    let diagonal = ((a.width * a.width + a.height * a.height) as f64).sqrt();
    ((dx * dx + dy * dy).sqrt() / diagonal) as f32
}

/// Difference between the mean motion direction *within* the outgoing shot
/// (prev → last frame) and *within* the incoming shot (first → next frame).
/// `None` unless both neighbor frames are supplied and both sides show
/// measurable motion (> 0.5 px mean displacement) — a static side has no
/// meaningful direction to compare.
fn mean_motion_angle_diff(
    outgoing: &SampledFrame,
    incoming: &SampledFrame,
    context: &CutContext<'_>,
) -> Option<f32> {
    let (angle_before, mag_before) = context
        .outgoing_prev
        .and_then(|prev| mean_block_motion(prev, outgoing))?;
    let (angle_after, mag_after) = context
        .incoming_next
        .and_then(|next| mean_block_motion(incoming, next))?;
    if mag_before < 0.5 || mag_after < 0.5 {
        return None;
    }
    let raw = (angle_before - angle_after).abs().rem_euclid(360.0);
    Some(raw.min(360.0 - raw) as f32)
}

/// Circular mean direction (deg) and magnitude (px) of block displacement
/// between `a` (treated as reference) and `b`. Pure.
fn mean_block_motion(reference: &SampledFrame, next: &SampledFrame) -> Option<(f64, f64)> {
    if reference.width != next.width || reference.height != next.height {
        return None;
    }
    const BLOCK: u32 = 16;
    const SEARCH: i32 = 8;
    // Ties are broken toward the smallest displacement (center-out search
    // order): a frame-constant dimension (e.g. motion only along x) would
    // otherwise pull every block to an arbitrary dy=−8 vote.
    let mut offsets: Vec<(i32, i32)> = Vec::with_capacity(((2 * SEARCH + 1) * (2 * SEARCH + 1)) as usize);
    for dy in -SEARCH..=SEARCH {
        for dx in -SEARCH..=SEARCH {
            offsets.push((dy, dx));
        }
    }
    offsets.sort_by_key(|(dy, dx)| (dy.unsigned_abs(), dx.unsigned_abs()));
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    let mut sum_mag = 0.0;
    let mut blocks = 0_u64;
    let mut by_it = 0;
    while by_it + BLOCK <= reference.height {
        let mut bx_it = 0;
        while bx_it + BLOCK <= reference.width {
            let (mut best, mut best_dx, mut best_dy) = (u64::MAX, 0_i32, 0_i32);
            for (dy, dx) in &offsets {
                let x0 = bx_it as i64 + *dx as i64;
                let y0 = by_it as i64 + *dy as i64;
                if x0 < 0 || y0 < 0 || x0 + BLOCK as i64 > next.width as i64 || y0 + BLOCK as i64 > next.height as i64 {
                    continue;
                }
                let mut ssd = 0_u64;
                for yy in 0..BLOCK {
                    for xx in (0..BLOCK).step_by(4) {
                        let px = (x0 + xx as i64) as u32;
                        let py = (y0 + yy as i64) as u32;
                        let q = reference.luma(bx_it + xx, by_it + yy) as i64;
                        let p = next.luma(px, py) as i64;
                        ssd += (q - p).unsigned_abs();
                    }
                }
                if ssd < best {
                    best = ssd;
                    best_dx = *dx;
                    best_dy = *dy;
                }
            }
            if best != u64::MAX {
                let (dx, dy) = (best_dx as f64, best_dy as f64);
                let mag = (dx * dx + dy * dy).sqrt();
                if mag > 0.0 {
                    sum_sin += dy.atan2(dx).sin() * mag;
                    sum_cos += dy.atan2(dx).cos() * mag;
                    sum_mag += mag;
                }
                blocks += 1;
            }
            bx_it += BLOCK;
        }
        by_it += BLOCK;
    }
    if blocks == 0 || (sum_mag / blocks as f64) < 0.05 {
        return None;
    }
    Some((sum_sin.atan2(sum_cos).to_degrees().rem_euclid(360.0), sum_mag / blocks as f64))
}

/// Peak sample-to-sample jump across the join, in dB above the 95th-
/// percentile local difference (a pop is a jump *out of family* with the
/// surrounding audio). `None` for silence/short sides.
fn audio_pop_db(before: &[f32], after: &[f32]) -> Option<f32> {
    if before.len() < 16 || after.len() < 16 {
        return None;
    }
    let local_scale = |samples: &[f32]| -> f64 {
        let mut deltas: Vec<f64> = samples.windows(2).map(|w| (w[1] - w[0]).abs() as f64).collect();
        deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95 = deltas[(deltas.len() as f64 * 0.95) as usize % deltas.len().max(1)];
        p95.max(1e-6)
    };
    let scale = ((local_scale(before) + local_scale(after)) / 2.0).max(1e-6);
    let join_jump = (after[0] - before[before.len() - 1]).abs() as f64;
    // Silence on one side means no join artifact is measurable.
    let rms = |s: &[f32]| s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64;
    if rms(before) < 1e-6 || rms(after) < 1e-6 {
        return None;
    }
    Some((20.0 * (join_jump / scale).log10()) as f32)
}

/// Weighted 0..1 combination (1 = smooth) + thresholded advice strings.
/// The weights are **arbitrary** (§4.4) and picked to make the failure
/// modes equally visible; they are not validated against human judgement.
fn combine_components(
    luma_jump: f32,
    delta_e: f32,
    centroid_shift: f32,
    flow_angle_diff: Option<f32>,
    audio_pop_db: Option<f32>,
) -> (f32, Vec<String>) {
    let mut recommendations = Vec::new();
    let luma_penalty = (luma_jump * 2.5).min(1.0);
    if luma_jump > 0.25 {
        recommendations.push(
            "Large brightness jump across the cut — consider a flash/L-cut or re-exposing one side"
                .into(),
        );
    }
    let delta_e_penalty = (delta_e / 25.0).min(1.0);
    if delta_e > 15.0 {
        recommendations.push(
            "Strong color cast change (mean ΔE2000 high) — match the grade across the cut".into(),
        );
    }
    let centroid_penalty = (centroid_shift * 1.5).min(1.0);
    if centroid_shift > 0.5 {
        recommendations.push(
            "Luminance centroid moves across the cut (proxy) — check the subject placement".into(),
        );
    }
    let motion_penalty = flow_angle_diff.map(|a| ((a - 0.0).abs() / 90.0).min(1.0)).unwrap_or(0.0);
    if let Some(angle) = flow_angle_diff {
        if angle > 90.0 {
            recommendations.push(
                "Motion direction roughly reverses across the cut — check the 180°/screen direction manually (no face model in this build)".into(),
            );
        }
    }
    let audio_penalty = audio_pop_db.map(|db| ((db - 6.0) / 18.0).clamp(0.0, 1.0)).unwrap_or(0.0);
    if let Some(db) = audio_pop_db {
        if db > 12.0 {
            recommendations
                .push(format!("Audio boundary pop ≈{db:.0} dB above local dynamics — add a crossfade"));
        }
    }
    let score = 1.0
        - (0.25 * luma_penalty
            + 0.25 * delta_e_penalty
            + 0.2 * centroid_penalty
            + 0.15 * motion_penalty
            + 0.15 * audio_penalty);
    (score.clamp(0.0, 1.0), recommendations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(tick_ms: i64, w: u32, h: u32, (r, g, b): (u8, u8, u8)) -> SampledFrame {
        SampledFrame {
            tick: Tick::from_millis(tick_ms),
            width: w,
            height: h,
            rgb: vec![r, g, b].repeat((w * h) as usize),
        }
    }

    /// Horizontal luma gradient spanning many histogram bins — the realistic
    /// textured-frame case (a uniform frame in *every* bin is a degenerate
    /// input for histogram distance and is disclaimed in the module docs).
    fn gradient(tick_ms: i64, w: u32, h: u32, offset: i32) -> SampledFrame {
        let mut rgb = vec![0_u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let v = ((x as i32 * 200 / w as i32) + 28 + offset).clamp(0, 255) as u8;
                let i = ((y * w + x) * 3) as usize;
                rgb[i] = v;
                rgb[i + 1] = v;
                rgb[i + 2] = v;
            }
        }
        SampledFrame {
            tick: Tick::from_millis(tick_ms),
            width: w,
            height: h,
            rgb,
        }
    }

    #[test]
    fn identical_and_similar_frames_score_low() {
        let a = gradient(0, 160, 90, 0);
        let same = gradient(500, 160, 90, 0);
        assert!(frame_difference_score(&a, &same) < 1e-6);
        // A ±6 luma wiggle (family-relation/brightness micro-shift) is not a cut.
        let nth = gradient(1000, 160, 90, 6);
        let score = frame_difference_score(&a, &nth);
        assert!(score < 0.32, "similar textured frames got {score}");
        // A real hard cut between dark and bright textures still trips it.
        let cut = frame_difference_score(&gradient(0, 160, 90, -28), &gradient(1, 160, 90, 200));
        assert!(cut > 0.32, "dark→bright texture cut got {cut}");
    }

    #[test]
    fn hard_cut_scores_high_and_is_detected() {
        let black = solid(0, 64, 48, (0, 0, 0));
        let white = solid(500, 64, 48, (255, 255, 255));
        assert!(frame_difference_score(&black, &white) > 0.8);
        let frames = vec![black, white, solid(1000, 64, 48, (255, 255, 255))];
        let cuts = detect_shot_boundaries(&frames, &ShotDetectOptions::default());
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].tick, Tick::from_millis(500));
    }

    #[test]
    fn cut_report_flags_a_black_to_white_cut_and_rates_a_matched_cut_smooth() {
        let black = solid(0, 64, 48, (10, 10, 10));
        let white = solid(500, 64, 48, (245, 245, 245));
        let report = evaluate_cut_continuity(
            &black,
            &white,
            &CutContext::default(),
            Tick::from_millis(500),
        )
        .unwrap();
        assert!(report.luma_jump_ire > 0.8);
        assert!(report.screen_direction_preserved.is_none());
        assert!(report.audio_boundary_pop_db.is_none());
        assert!(!report.recommendations.is_empty(), "black→white gets advice");
        let matched_delta = evaluate_cut_continuity(
            &solid(0, 64, 48, (120, 120, 120)),
            &solid(500, 64, 48, (121, 122, 119)),
            &CutContext::default(),
            Tick::from_millis(500),
        )
        .unwrap();
        assert!(matched_delta.heuristic_continuity_score > report.heuristic_continuity_score);
        assert!(matched_delta.heuristic_continuity_score > 0.9);
    }

    #[test]
    fn audio_pop_measured_only_when_both_sides_sound() {
        let pre = [0.0_f32; 32];
        let post = [0.0_f32; 32];
        assert!(audio_pop_db(&pre, &post).is_none(), "silence has no pop");
        let mut lo = vec![0.01_f32; 200];
        let mut hi = vec![0.9_f32; 200];
        lo[199] = 0.01;
        hi[0] = 0.9;
        let pop = audio_pop_db(&lo, &hi).expect("measurable pop");
        assert!(pop > 6.0, "got {pop} dB");
    }

    /// Full-frame vertical stripes shifted by `dx` pixels: every block has
    /// an unambiguous displacement, so angle comparisons are not dominated
    /// by mostly-black partial blocks.
    fn striped(tick_ms: i64, w: u32, h: u32, dx: i32) -> SampledFrame {
        let mut rgb = vec![0_u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let v = if ((x as i32 - dx).rem_euclid(24)) < 12 { 230 } else { 20 };
                let i = ((y * w + x) * 3) as usize;
                rgb[i] = v;
                rgb[i + 1] = v;
                rgb[i + 2] = v;
            }
        }
        SampledFrame {
            tick: Tick::from_millis(tick_ms),
            width: w,
            height: h,
            rgb,
        }
    }

    #[test]
    fn motion_angle_diff_measures_within_shot_directions_only() {
        const W: u32 = 96;
        const H: u32 = 64;
        // Outgoing motion: content moves +4 px (dx 0 → 4). Incoming motion:
        // same +4 (dx 4 → 8) → matched action, small angle diff.
        let outgoing_prev = striped(450, W, H, 0);
        let outgoing_last = striped(500, W, H, 4);
        let incoming_first = striped(500, W, H, 4);
        let incoming_next = striped(550, W, H, 8);
        let matched = evaluate_cut_continuity(
            &outgoing_last,
            &incoming_first,
            &CutContext {
                outgoing_prev: Some(&outgoing_prev),
                incoming_next: Some(&incoming_next),
                ..CutContext::default()
            },
            Tick::from_millis(500),
        )
        .unwrap();
        let matched_angle = matched.optical_flow_angle_diff_deg.expect("matched measurable");
        assert!(matched_angle < 45.0, "got {matched_angle}");

        // Reversed incoming motion (dx 4 → 0, i.e. content −4) → large diff.
        let incoming_next_rev = striped(550, W, H, 0);
        let reversed = evaluate_cut_continuity(
            &outgoing_last,
            &incoming_first,
            &CutContext {
                outgoing_prev: Some(&outgoing_prev),
                incoming_next: Some(&incoming_next_rev),
                ..CutContext::default()
            },
            Tick::from_millis(500),
        )
        .unwrap();
        let angle = reversed.optical_flow_angle_diff_deg.expect("reversed measurable");
        assert!(angle > 135.0, "got {angle}");

        // Without neighbor frames the flow is honestly None, not guessed.
        let no_flow = evaluate_cut_continuity(
            &outgoing_last,
            &incoming_first,
            &CutContext::default(),
            Tick::from_millis(500),
        )
        .unwrap();
        assert!(no_flow.optical_flow_angle_diff_deg.is_none());
    }
}

