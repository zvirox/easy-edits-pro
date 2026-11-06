// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Measured patch tracking and RANSAC homography (Packet 2.3).
//!
//! Replaces the synthetic sine/cosine "motion tracking" in the preview
//! command: every point returned by these functions comes from actually
//! comparing pixels across decoded frames.
//!
//! - [`track_patch_displacement`]: exhaustive SSD search of a reference
//!   patch in a bounded search window, with parabolic sub-pixel refinement
//!   and a ZNCC (zero-mean normalized cross-correlation) match score in
//!   0..1 that is *reported*, never invented. Translation-only: rotation
//!   and scale drift reduce the score instead of being absorbed silently.
//! - [`homography_from_point_pairs`]: normalized DLT over ≥4 point pairs
//!   (exactly 4 gives the exact projective map).
//! - [`ransac_homography`]: deterministic RANSAC (seeded LCG) over point
//!   pairs with a pixel threshold, returning the consensus homography and
//!   its inliers.
//! - [`track_region_frames`]: the composition the motion-tracking command
//!   uses — a 3×3 grid of patches over the selected region is matched from
//!   the reference frame into every later frame, and a per-frame homography
//!   is RANSAC-fitted over the patch correspondences.

use crate::shot_detect::SampledFrame;
use saturn_core::Tick;

/// One tracked patch position. `confidence` is the ZNCC match score mapped
/// to 0..1 (1 = the tracked content matched linearly and completely); it is
/// measured, and low values mean "trust the point less" — never a synthetic
/// default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchTrackPoint {
    pub tick: Tick,
    /// Full-resolution pixel coordinates of the patch center in this frame.
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

/// Track a square patch (side `patch_px`, centered at `start`) through
/// `frames`, exhaustively searching `±search_radius` pixels around the
/// previous position per frame. Fails honestly when the start patch falls
/// outside the first frame or the search window cannot fit.
pub fn track_patch_displacement(
    frames: &[SampledFrame],
    start: (f32, f32),
    patch_px: u32,
    search_radius: u32,
) -> Result<Vec<PatchTrackPoint>, String> {
    let first = frames.first().ok_or_else(|| {
        "track_patch_displacement needs at least one decoded frame".to_owned()
    })?;
    if patch_px < 4 || patch_px % 2 != 0 {
        return Err(format!(
            "patch_px must be an even number ≥ 4 (got {patch_px})"
        ));
    }
    let half = (patch_px / 2) as i64;
    let (sx, sy) = (start.0.round() as i64, start.1.round() as i64);
    if !window_fits(sx, sy, half, first.width, first.height) {
        return Err(format!(
            "track start ({}, {}) with a {patch_px}px patch lies outside the \
             {}×{} frame",
            start.0, start.1, first.width, first.height
        ));
    }
    let reference = patch_luma(first, sx, sy, half);
    let reference_stats = patch_stats(&reference);

    let mut tracks = Vec::with_capacity(frames.len());
    // Frame 0 is the reference itself: exact position, full confidence.
    tracks.push(PatchTrackPoint {
        tick: first.tick,
        x: sx as f32,
        y: sy as f32,
        confidence: 1.0,
    });
    for frame in frames.iter().skip(1) {
        if frame.width != first.width || frame.height != first.height {
            return Err(format!(
                "frame at tick {} is {}×{}, expected the reference's {}×{} \
                 (all frames must share the source resolution)",
                frame.tick.0,
                frame.width,
                frame.height,
                first.width,
                first.height
            ));
        }
        let matched = match_patch_exhaustive(
            &reference,
            &reference_stats,
            (sx, sy),
            frame,
            half,
            search_radius as i64,
        )?;
        tracks.push(PatchTrackPoint {
            tick: frame.tick,
            x: matched.0 as f32,
            y: matched.1 as f32,
            confidence: matched.2 as f32,
        });
    }
    Ok(tracks)
}

/// Integer BT.601 luma of a square patch centered at (cx, cy).
fn patch_luma(frame: &SampledFrame, cx: i64, cy: i64, half: i64) -> Vec<u8> {
    let w = frame.width as i64;
    let side = (half * 2 + 1) as usize;
    let mut out = Vec::with_capacity(side * side);
    for y in (cy - half)..=(cy + half) {
        for x in (cx - half)..=(cx + half) {
            let idx = (y * w + x) as usize * 3;
            let r = frame.rgb[idx] as u32;
            let g = frame.rgb[idx + 1] as u32;
            let b = frame.rgb[idx + 2] as u32;
            out.push(((r * 299 + g * 587 + b * 114) / 1000) as u8);
        }
    }
    out
}

struct PatchStats {
    mean: f64,
    std: f64,
}

fn patch_stats(patch: &[u8]) -> PatchStats {
    let n = patch.len() as f64;
    let mean = patch.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = patch
        .iter()
        .map(|&v| (v as f64 - mean) * (v as f64 - mean))
        .sum::<f64>()
        / n;
    PatchStats {
        mean,
        std: var.sqrt().max(1e-9),
    }
}

fn ssd(a: &[u8], b: &[u8]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| {
            let d = x as f64 - y as f64;
            d * d
        })
        .sum()
}

fn window_fits(cx: i64, cy: i64, half: i64, width: u32, height: u32) -> bool {
    cx - half >= 0
        && cy - half >= 0
        && (cx + half) < width as i64
        && (cy + half) < height as i64
}

/// Match a reference patch (already extracted) into `target`, searching
/// exhaustively around `around`. Returns the sub-pixel-refined center
/// `(x, y)` and the ZNCC confidence. Errors when no window fits.
fn match_patch_exhaustive(
    reference: &[u8],
    reference_stats: &PatchStats,
    around: (i64, i64),
    target: &SampledFrame,
    half: i64,
    radius: i64,
) -> Result<(f64, f64, f64), String> {
    let base_x = around.0.clamp(half, target.width as i64 - half - 1);
    let base_y = around.1.clamp(half, target.height as i64 - half - 1);
    let ssd_at = |dx: i64, dy: i64| -> Option<f64> {
        let wx = base_x + dx;
        let wy = base_y + dy;
        if !window_fits(wx, wy, half, target.width, target.height) {
            return None;
        }
        let candidate = patch_luma(target, wx, wy, half);
        Some(ssd(reference, &candidate))
    };
    let mut best = (0_i64, 0_i64, f64::INFINITY);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if let Some(score) = ssd_at(dx, dy) {
                if score < best.2 {
                    best = (dx, dy, score);
                }
            }
        }
    }
    if !best.2.is_finite() {
        return Err(format!(
            "no {radius}px search window fit inside the {}×{} target frame",
            target.width, target.height
        ));
    }
    // Parabolic sub-pixel refinement on each axis using the SSD neighbours
    // of the integer minimum; falls back to the integer position when a
    // neighbour leaves the frame or the curvature is non-positive.
    let mut sub_x = best.0 as f64;
    let mut sub_y = best.1 as f64;
    let left = ssd_at(best.0 - 1, best.1).unwrap_or(f64::INFINITY);
    let right = ssd_at(best.0 + 1, best.1).unwrap_or(f64::INFINITY);
    let denom_x = left + right - 2.0 * best.2;
    if denom_x > 0.0 {
        let shift = (left - right) / (2.0 * denom_x);
        if shift.abs() <= 0.5 {
            sub_x += shift;
        }
    }
    let up = ssd_at(best.0, best.1 - 1).unwrap_or(f64::INFINITY);
    let down = ssd_at(best.0, best.1 + 1).unwrap_or(f64::INFINITY);
    let denom_y = up + down - 2.0 * best.2;
    if denom_y > 0.0 {
        let shift = (up - down) / (2.0 * denom_y);
        if shift.abs() <= 0.5 {
            sub_y += shift;
        }
    }
    let cx = (base_x as f64 + sub_x).clamp(half as f64, target.width as f64 - half as f64 - 1.0);
    let cy = (base_y as f64 + sub_y).clamp(half as f64, target.height as f64 - half as f64 - 1.0);

    // ZNCC at the matched patch → 0..1 confidence (measured; a flat patch
    // has no texture and collapses the denominator → confidence ~0).
    let matched = patch_luma(target, cx as i64, cy as i64, half);
    let matched_stats = patch_stats(&matched);
    let mut zncc = 0.0_f64;
    for (r, m) in reference.iter().zip(matched.iter()) {
        zncc += (*r as f64 - reference_stats.mean) * (*m as f64 - matched_stats.mean);
    }
    let denom = reference_stats.std * matched_stats.std * reference.len() as f64;
    let zncc = if denom > 1e-9 { zncc / denom } else { 0.0 };
    Ok((cx, cy, zncc.clamp(0.0, 1.0)))
}

/// Exact/least-squares homography (normalized DLT, Hartley-normalized)
/// mapping `src` → `dst`. Needs ≥ 4 non-degenerate pairs. Returns the 3×3
/// matrix in row-major order with h[8] normalized to 1.
pub fn homography_from_point_pairs(pairs: &[([f64; 2], [f64; 2])]) -> Option<[f64; 9]> {
    if pairs.len() < 4 {
        return None;
    }
    let normalize = |points: Vec<[f64; 2]>| -> ([f64; 9], Vec<[f64; 2]>) {
        let n = points.len() as f64;
        let cx = points.iter().map(|p| p[0]).sum::<f64>() / n;
        let cy = points.iter().map(|p| p[1]).sum::<f64>() / n;
        let mean_dist = points
            .iter()
            .map(|p| ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt())
            .sum::<f64>()
            / n;
        let scale = if mean_dist > 1e-9 {
            (2.0_f64).sqrt() / mean_dist
        } else {
            1.0
        };
        let transform = [
            scale, 0.0, -scale * cx, 0.0, scale, -scale * cy, 0.0, 0.0, 1.0,
        ];
        let normalized = points
            .iter()
            .map(|p| [scale * (p[0] - cx), scale * (p[1] - cy)])
            .collect();
        (transform, normalized)
    };
    let (t_src, src): (_, Vec<[f64; 2]>) = normalize(pairs.iter().map(|p| p.0).collect());
    let (t_dst, dst): (_, Vec<[f64; 2]>) = normalize(pairs.iter().map(|p| p.1).collect());

    // 2n×9 DLT constraint matrix, then the 9×9 normal equations N = AᵗA.
    let rows = pairs.len() * 2;
    let mut a = vec![0.0_f64; rows * 9];
    for (i, (s, d)) in src.iter().zip(&dst).enumerate() {
        let (sx, sy) = (s[0], s[1]);
        let (dx, dy) = (d[0], d[1]);
        let r0 = 2 * i;
        let r1 = 2 * i + 1;
        // Row 0: [0 0 0  −sx −sy −1  dy·sx dy·sy dy]
        a[r0 * 9 + 3] = -sx;
        a[r0 * 9 + 4] = -sy;
        a[r0 * 9 + 5] = -1.0;
        a[r0 * 9 + 6] = dy * sx;
        a[r0 * 9 + 7] = dy * sy;
        a[r0 * 9 + 8] = dy;
        // Row 1: [sx sy 1  0 0 0  −dx·sx −dx·sy −dx]
        a[r1 * 9 + 0] = sx;
        a[r1 * 9 + 1] = sy;
        a[r1 * 9 + 2] = 1.0;
        a[r1 * 9 + 6] = -dx * sx;
        a[r1 * 9 + 7] = -dx * sy;
        a[r1 * 9 + 8] = -dx;
    }
    let mut n = [[0.0_f64; 9]; 9];
    for r in 0..9 {
        for c in 0..9 {
            n[r][c] = (0..rows).map(|k| a[k * 9 + r] * a[k * 9 + c]).sum();
        }
    }
    // Solve for the null vector with the gauge h[8] = 1: for every row r,
    // Σ_{c<8} N[r][c]·h[c] = −N[r][8]. A singular system means degenerate
    // correspondences → honest None.
    let mut m = [[0.0_f64; 8]; 8];
    let mut b = [0.0_f64; 8];
    for r in 0..8 {
        for c in 0..8 {
            m[r][c] = n[r][c];
        }
        b[r] = -n[r][8];
    }
    let h8 = solve_linear_8(&mut m, &mut b)?;
    let mut h = [0.0_f64; 9];
    h[..8].copy_from_slice(&h8);
    h[8] = 1.0;

    // Denormalize: H = T_dst⁻¹ · h · T_src.
    let inv_dst = invert3(&t_dst)?;
    let mut result = multiply3(&inv_dst, &multiply3(&h, &t_src));
    if result[8].abs() < 1e-12 {
        return None;
    }
    let h88 = result[8];
    for value in &mut result {
        *value /= h88;
    }
    Some(result)
}

/// Gauss elimination with partial pivoting on an 8×8 system; singular
/// (pivot below 1e-10) inputs return `None`.
fn solve_linear_8(m: &mut [[f64; 8]; 8], b: &mut [f64; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let mut pivot_row = col;
        for r in col + 1..8 {
            if m[r][col].abs() > m[pivot_row][col].abs() {
                pivot_row = r;
            }
        }
        if m[pivot_row][col].abs() < 1e-10 {
            return None;
        }
        m.swap(col, pivot_row);
        b.swap(col, pivot_row);
        let pivot = m[col][col];
        for r in col + 1..8 {
            let factor = m[r][col] / pivot;
            if factor == 0.0 {
                continue;
            }
            for c in col..8 {
                m[r][c] -= factor * m[col][c];
            }
            b[r] -= factor * b[col];
        }
    }
    let mut out = [0.0_f64; 8];
    for row in (0..8).rev() {
        let mut sum = b[row];
        for c in row + 1..8 {
            sum -= m[row][c] * out[c];
        }
        out[row] = sum / m[row][row];
    }
    Some(out)
}

/// Row-major 3×3 inverse via the adjugate; singular matrices → None.
fn invert3(m: &[f64; 9]) -> Option<[f64; 9]> {
    let (a, b, c, d, e, f, g, h, i) = (m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8]);
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-12 {
        return None;
    }
    Some([
        (e * i - f * h) / det,
        (c * h - b * i) / det,
        (b * f - c * e) / det,
        (f * g - d * i) / det,
        (a * i - c * g) / det,
        (c * d - a * f) / det,
        (d * h - e * g) / det,
        (b * g - a * h) / det,
        (a * e - b * d) / det,
    ])
}

fn multiply3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0_f64; 9];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 3 + c] = (0..3).map(|k| a[r * 3 + k] * b[k * 3 + c]).sum();
        }
    }
    out
}

/// Apply a row-major homography to a point. Points through the vanish line
/// map to infinities (callers treat that as "unusable measurement").
pub fn apply_homography(h: &[f64; 9], point: [f64; 2]) -> [f64; 2] {
    let w = h[6] * point[0] + h[7] * point[1] + h[8];
    if w.abs() < 1e-12 {
        return [f64::INFINITY, f64::INFINITY];
    }
    [
        (h[0] * point[0] + h[1] * point[1] + h[2]) / w,
        (h[3] * point[0] + h[4] * point[1] + h[5]) / w,
    ]
}

/// Outcome of a RANSAC fit: the consensus homography, which pairs were
/// inliers, and the mean forward-transfer error over those inliers.
#[derive(Clone, Debug, PartialEq)]
pub struct RansacOutcome {
    pub homography: [f64; 9],
    pub inliers: Vec<usize>,
    pub mean_inlier_error_px: f64,
}

/// Deterministic RANSAC over point pairs. `iterations` minimal samples of 4
/// pairs are scored by counting forward-transfer errors below
/// `threshold_px`; the best consensus set wins (ties → lower mean error).
/// Returns `None` when no consensus reaches 4 inliers.
pub fn ransac_homography(
    pairs: &[([f64; 2], [f64; 2])],
    iterations: u32,
    threshold_px: f64,
    seed: u64,
) -> Option<RansacOutcome> {
    if pairs.len() < 4 {
        return None;
    }
    let mut rng = seed | 1;
    let mut next = move || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (rng >> 33) as usize
    };
    let mut best: Option<RansacOutcome> = None;
    for _ in 0..iterations {
        let i0 = next() % pairs.len();
        let mut i1 = next() % pairs.len();
        while i1 == i0 {
            i1 = next() % pairs.len();
        }
        let mut i2 = next() % pairs.len();
        while i2 == i0 || i2 == i1 {
            i2 = next() % pairs.len();
        }
        let mut i3 = next() % pairs.len();
        while i3 == i0 || i3 == i1 || i3 == i2 {
            i3 = next() % pairs.len();
        }
        // Degeneracy guard: a minimal sample must be in general position —
        // any 3 collinear src points make the 4-point homography ambiguous.
        // Check all four triplets, not just one.
        let twice_area = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
            (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        };
        let triplet_collinear = |i: usize, j: usize, k: usize| {
            twice_area(pairs[i].0, pairs[j].0, pairs[k].0).abs() < 1e-6
        };
        if triplet_collinear(i0, i1, i2)
            || triplet_collinear(i0, i1, i3)
            || triplet_collinear(i0, i2, i3)
            || triplet_collinear(i1, i2, i3)
        {
            continue;
        }
        let sample_pairs = [pairs[i0], pairs[i1], pairs[i2], pairs[i3]];
        let candidate = match homography_from_point_pairs(&sample_pairs) {
            Some(h) => h,
            None => continue,
        };
        let mut inliers = Vec::with_capacity(pairs.len());
        let mut error_sum = 0.0_f64;
        for (idx, (src, dst)) in pairs.iter().enumerate() {
            let projected = apply_homography(&candidate, *src);
            if projected[0].is_finite() && projected[1].is_finite() {
                let error = ((projected[0] - dst[0]).powi(2)
                    + (projected[1] - dst[1]).powi(2))
                .sqrt();
                if error <= threshold_px {
                    inliers.push(idx);
                    error_sum += error;
                }
            }
        }
        let candidate_wins = match &best {
            None => inliers.len() >= 4,
            Some(current) => {
                inliers.len() > current.inliers.len()
                    || (inliers.len() == current.inliers.len()
                        && inliers.len() >= 4
                        && error_sum / (inliers.len() as f64)
                            < current.mean_inlier_error_px)
            }
        };
        if candidate_wins {
            best = Some(RansacOutcome {
                homography: candidate,
                mean_inlier_error_px: error_sum / inliers.len() as f64,
                inliers,
            });
        }
    }
    best
}

/// Mean forward-transfer error of `homography` over the given inlier pairs.
fn mean_transfer_error(
    homography: &[f64; 9],
    pairs: &[([f64; 2], [f64; 2])],
    inliers: &[usize],
) -> f64 {
    if inliers.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    for &idx in inliers {
        let (src, dst) = pairs[idx];
        let projected = apply_homography(homography, src);
        sum += ((projected[0] - dst[0]).powi(2) + (projected[1] - dst[1]).powi(2)).sqrt();
    }
    sum / inliers.len() as f64
}

/// Per-frame region tracking result. `homography` maps reference-frame
/// coordinates into this frame's coordinates; `None` entries mean fewer
/// than 4 patches matched well enough to fit a homography (reported, not
/// interpolated).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackedRegion {
    pub tick: Tick,
    pub homography: [f64; 9],
    /// Mean RANSAC forward-transfer error over inlier patches (pixels).
    pub reprojection_error_px: f64,
    /// Mean ZNCC confidence of the inlier patches (0..1, measured).
    pub confidence: f64,
    /// Inlier share of the patch grid (0..1).
    pub inlier_ratio: f64,
}

/// Track a rectangular region `(x, y, w, h)` (pixels, reference frame)
/// through `frames` using a 3×3 grid of 32px patches: every patch is
/// matched from its REFERENCE position into each later frame (non-
/// incremental, so one bad match cannot compound), and a RANSAC homography
/// over the resulting correspondences captures rotation/perspective the
/// translation-only patch matcher cannot represent.
pub fn track_region_frames(
    frames: &[SampledFrame],
    region: (f32, f32, u32, u32),
    search_radius: u32,
) -> Result<Vec<Option<TrackedRegion>>, String> {
    let first = frames.first().ok_or_else(|| {
        "track_region_frames needs at least one decoded frame".to_owned()
    })?;
    let (rx, ry, rw, rh) = (region.0, region.1, region.2, region.3);
    if rw == 0
        || rh == 0
        || rx < 0.0
        || ry < 0.0
        || rx + rw as f32 > first.width as f32
        || ry + rh as f32 > first.height as f32
    {
        return Err(format!(
            "track region ({rx}, {ry}, {rw}, {rh}) is outside the {}×{} frame",
            first.width, first.height
        ));
    }
    const PATCH: u32 = 32;
    let half = (PATCH / 2) as i64;
    // 3×3 grid of patch centers strictly inside the region.
    let mut centers: Vec<[f64; 2]> = Vec::with_capacity(9);
    for gy in 0..3_u32 {
        for gx in 0..3_u32 {
            let cx = (rx + rw as f32 * (gx as f32 + 0.5) / 3.0).round() as i64;
            let cy = (ry + rh as f32 * (gy as f32 + 0.5) / 3.0).round() as i64;
            let cx = cx.clamp(half, first.width as i64 - half - 1);
            let cy = cy.clamp(half, first.height as i64 - half - 1);
            centers.push([cx as f64, cy as f64]);
        }
    }
    let seed = ((rx.to_bits() as u64) << 32)
        ^ (ry.to_bits() as u64)
        ^ ((rw as u64) << 16)
        ^ (rh as u64)
        ^ 0x9e37_79b9_7f4a_7c15;

    // Pre-extract the reference patches (every later frame matches them).
    let reference_patches: Vec<(Vec<u8>, PatchStats)> = centers
        .iter()
        .map(|c| {
            let patch = patch_luma(first, c[0] as i64, c[1] as i64, half);
            let stats = patch_stats(&patch);
            (patch, stats)
        })
        .collect();

    let mut results = Vec::with_capacity(frames.len());
    let identity = homography_from_point_pairs(
        &centers
            .iter()
            .map(|&c| (c, c))
            .collect::<Vec<_>>(),
    )
    .ok_or_else(|| "reference frame patch grid was degenerate".to_owned())?;
    results.push(Some(TrackedRegion {
        tick: first.tick,
        homography: identity,
        reprojection_error_px: 0.0,
        confidence: 1.0,
        inlier_ratio: 1.0,
    }));

    for frame in frames.iter().skip(1) {
        if frame.width != first.width || frame.height != first.height {
            return Err(format!(
                "frame at tick {} is {}×{}, expected the reference's {}×{}",
                frame.tick.0, frame.width, frame.height, first.width, first.height
            ));
        }
        let mut pairs: Vec<([f64; 2], [f64; 2])> = Vec::with_capacity(centers.len());
        let mut confidences: Vec<f64> = Vec::with_capacity(centers.len());
        for (patch_idx, &center) in centers.iter().enumerate() {
            let (patch, stats) = &reference_patches[patch_idx];
            if let Ok((x, y, confidence)) = match_patch_exhaustive(
                patch,
                stats,
                (center[0] as i64, center[1] as i64),
                frame,
                half,
                search_radius as i64,
            ) {
                if confidence >= 0.35 {
                    pairs.push((center, [x, y]));
                    confidences.push(confidence);
                }
            }
        }
        if pairs.len() < 4 {
            results.push(None);
            continue;
        }
        let outcome = ransac_homography(&pairs, 200, 2.0, seed)
            .and_then(|consensus| {
                // Refit on the consensus set (one least-squares pass).
                let inlier_pairs: Vec<_> =
                    consensus.inliers.iter().map(|&i| pairs[i]).collect();
                let refined =
                    homography_from_point_pairs(&inlier_pairs)?;
                let refined_outcome = RansacOutcome {
                    homography: refined,
                    mean_inlier_error_px: mean_transfer_error(
                        &refined,
                        &pairs,
                        &consensus.inliers,
                    ),
                    inliers: consensus.inliers.clone(),
                };
                // Keep whichever fits the inliers better.
                if refined_outcome.mean_inlier_error_px
                    <= consensus.mean_inlier_error_px
                {
                    Some(refined_outcome)
                } else {
                    Some(consensus)
                }
            });
        results.push(outcome.map(|outcome| TrackedRegion {
            tick: frame.tick,
            homography: outcome.homography,
            reprojection_error_px: outcome.mean_inlier_error_px,
            confidence: outcome.inliers.iter().map(|&i| confidences[i]).sum::<f64>()
                / outcome.inliers.len() as f64,
            inlier_ratio: outcome.inliers.len() as f64 / centers.len() as f64,
        }));
    }
    Ok(results)
}

/// Downscale a frame so its longest edge is at most `max_edge` (never
/// upscales), with the same deterministic bilinear resampler the model
/// pipelines use. Region tracking runs at this bound so a bounded frame
/// window cannot exhaust memory on 4K media; coordinates scale linearly
/// back to the source resolution.
pub fn downscale_frame_rgb(frame: &SampledFrame, max_edge: u32) -> SampledFrame {
    let longest = frame.width.max(frame.height);
    if longest <= max_edge.max(1) {
        return frame.clone();
    }
    let scale = max_edge as f32 / longest as f32;
    let dw = ((frame.width as f32 * scale).round() as u32).max(1);
    let dh = ((frame.height as f32 * scale).round() as u32).max(1);
    let rgb = crate::matting_ort::bilinear_resize_rgb(
        &frame.rgb,
        frame.width,
        frame.height,
        dw,
        dh,
    );
    SampledFrame {
        tick: frame.tick,
        width: dw,
        height: dh,
        rgb,
    }
}

#[cfg(test)]
#[path = "motion_track_tests.rs"]
mod tests;
