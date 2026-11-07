// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! CPU reference evaluation for the Packet 3.2 colour-node chain.
//!
//! Deliberately written as a direct mirror of the WGSL in
//! [`crate::color_nodes_gpu`] (same op order, same formulas);
//! GPU-vs-CPU agreement is asserted to per-op tolerances in
//! `tests/color_pipeline.rs`. Values are f64 here; the GPU runs f32, so
//! the tolerances cover f32 rounding, not semantic drift.

use crate::color_nodes_gpu::{
    EvalNode, FilmPlan, ResolvedWindow, WindowId, KIND_FILM_GRAIN, KIND_HDR_ZONES,
    KIND_HSL_QUALIFIER, KIND_LUT_3D, KIND_PRIMARY_WHEELS, KIND_RGB_CURVES, KIND_WARPER,
    MAX_CURVE_POINTS,
};

// --- CPU reference evaluation ----------------------------------------------

/// `smoothstep(e0, e1, x)` — bit-exact with the WGSL builtin definition.
pub fn smoothstep_f64(e0: f64, e1: f64, x: f64) -> f64 {
    if e0 == e1 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Soft band weight: 1 inside the plateau, falling to 0 at `half` distance.
/// `softness` in 0..1 carves the plateau out of the width.
pub fn band_weight_f64(dist: f64, half: f64, soft: f64) -> f64 {
    let plateau = (half * (1.0 - soft)).max(0.0);
    if soft <= 0.0 {
        if dist <= half {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - smoothstep_f64(plateau, half, dist)
    }
}

/// Rec.709 luma of the eval-domain triple (documented anchor coefficients).
pub fn luma_f64(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Hue/sat/lightness (HSL) of an RGB triple; hue in turns 0..1.
pub fn rgb_to_hsl_f64(c: [f64; 3]) -> (f64, f64, f64) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d <= 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let hue = if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    } / 6.0;
    (hue.rem_euclid(1.0), s.clamp(0.0, 1.0), l)
}

fn hue_to_rgb_f64(p: f64, q: f64, t: f64) -> f64 {
    let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// HSL back to RGB; the exact inverse of the conversion above.
pub fn hsl_to_rgb_f64(h: f64, s: f64, l: f64) -> [f64; 3] {
    if s <= 0.0 {
        return [l; 3];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    [
        hue_to_rgb_f64(p, q, h + 1.0 / 3.0),
        hue_to_rgb_f64(p, q, h),
        hue_to_rgb_f64(p, q, h - 1.0 / 3.0),
    ]
}

/// Shortest signed hue distance in turns, -0.5..0.5.
pub fn hue_distance_f64(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(1.0);
    if d > 0.5 {
        d - 1.0
    } else {
        d
    }
}

/// Deterministic per-pixel grain noise in -1..1 from integer coordinates
/// (integer hash — identical results in WGSL and Rust by construction).
pub fn grain_noise_f64(px: i64, py: i64, frame: u32) -> f64 {
    let mut h = (px as u32)
        .wrapping_mul(0x9E37_79B1)
        ^ (py as u32).wrapping_mul(0x85EB_CA6B)
        ^ frame.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FE0_0001);
    h ^= h >> 15;
    h = h.wrapping_mul(0x9E37_79B1);
    h ^= h >> 16;
    // 24-bit mantissa keeps the mapping to -1..1 exact in f32.
    let unit = (h >> 8) as f64 / 16_777_216.0;
    unit * 2.0 - 1.0
}

/// Monotone cubic (Fritsch–Carlson) evaluation through ascending-x points.
/// Same construction in WGSL.
pub fn monotone_spline_f64(points: &[[f64; 2]], x: f64) -> f64 {
    let n = points.len();
    debug_assert!(n >= 2);
    if x <= points[0][0] {
        return points[0][1];
    }
    if x >= points[n - 1][0] {
        return points[n - 1][1];
    }
    let mut seg = 0usize;
    for i in 0..n - 1 {
        if x >= points[i][0] && x < points[i + 1][0] {
            seg = i;
            break;
        }
    }
    let dx = (points[seg + 1][0] - points[seg][0]).max(1e-9);
    let slope0 = if seg == 0 {
        (points[1][1] - points[0][1]) / dx
    } else {
        (points[seg + 1][1] - points[seg - 1][1]) / (points[seg + 1][0] - points[seg - 1][0])
    };
    let slope1 = if seg + 2 == n {
        (points[seg + 1][1] - points[seg][1]) / dx
    } else {
        (points[seg + 2][1] - points[seg][1]) / (points[seg + 2][0] - points[seg][0])
    };
    // Fritsch–Carlson limiter on the two segment-end tangents.
    let sec = (points[seg + 1][1] - points[seg][1]) / dx;
    let (m0, m1) = if sec <= 0.0 {
        (0.0, 0.0)
    } else {
        let limit = 3.0 * sec;
        (
            slope0.clamp(-limit, limit),
            slope1.clamp(-limit, limit),
        )
    };
    let t = (x - points[seg][0]) / dx;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * points[seg][1] + h10 * dx * m0 + h01 * points[seg + 1][1] + h11 * dx * m1
}

/// Tetrahedral LUT lookup on a flat `size^3` RGB grid (r fastest).
pub fn tetra_lut_f64(size: usize, grid: &[f64], c: [f64; 3]) -> [f64; 3] {
    let scale = (size - 1) as f64;
    let cs = [
        c[0].clamp(0.0, 1.0) * scale,
        c[1].clamp(0.0, 1.0) * scale,
        c[2].clamp(0.0, 1.0) * scale,
    ];
    let i = [
        cs[0].floor() as usize,
        cs[1].floor() as usize,
        cs[2].floor() as usize,
    ];
    let f = [
        cs[0] - i[0] as f64,
        cs[1] - i[1] as f64,
        cs[2] - i[2] as f64,
    ];
    let max = size - 1;
    let corner = |p: [usize; 3]| {
        let k = (p[2].min(max) * size * size + p[1].min(max) * size + p[0].min(max)) * 3;
        [grid[k], grid[k + 1], grid[k + 2]]
    };
    // Sort the three fractional parts descending; the tetrahedron steps one
    // axis at a time in that order. Ties are interpolated continuously, so
    // the tie order does not change the result.
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| f[b].partial_cmp(&f[a]).unwrap());
    let c0 = corner(i);
    let mut result = c0;
    let mut prev = c0;
    let mut steps = [0usize; 3];
    for &axis in order.iter() {
        let weight = f[axis];
        debug_assert!(
            weight >= 0.0,
            "descending sort must yield non-negative weights"
        );
        // The tetrahedron steps are cumulative: C1 steps one axis, C2 the
        // first two, C3 all three.
        steps[axis] += 1;
        let next = corner([
            i[0] + steps[0],
            i[1] + steps[1],
            i[2] + steps[2],
        ]);
        for ch in 0..3 {
            // Telescoped tetrahedral weights: the largest fraction pairs with
            // the first axis step, and so on (continuous across ties).
            result[ch] += weight * (next[ch] - prev[ch]);
        }
        prev = next;
    }
    result
}
// --- Chain reference --------------------------------------------------------

/// Per-pixel reference of the whole node chain on one sample. `uv` is the
/// post-weave sample coordinate in UV (the weave offset itself is applied by
/// the caller when sampling the frame; see the WGSL entry for the GPU-side).
pub fn eval_reference(
    mut c: [f64; 3],
    uv: [f64; 2],
    pixel: [i64; 2],
    nodes: &[EvalNode],
    film: &FilmPlan,
    frame: u32,
) -> [f64; 3] {
    for node in nodes {
        if !node.enabled {
            continue;
        }
        // Power windows gate the whole node: product of member weights.
        let mut gate = 1.0f64;
        for w in &node.windows {
            gate *= window_weight_f64(w, uv);
            if gate <= 0.0 {
                break;
            }
        }
        if gate <= 0.0 {
            continue;
        }
        c = match node.kind {
            KIND_PRIMARY_WHEELS => apply_wheels(c, &node.data, gate),
            KIND_HDR_ZONES => apply_zones(c, &node.data, gate),
            KIND_RGB_CURVES => apply_curves(c, &node.data, gate),
            KIND_LUT_3D => apply_lut(c, &node.data, gate),
            KIND_HSL_QUALIFIER => apply_hsl(c, &node.data, gate),
            KIND_WARPER => apply_warper(c, &node.data, gate),
            KIND_FILM_GRAIN => apply_grain(c, pixel, frame, &node.data, gate),
            _ => c,
        };
    }
    // Whole-frame halation spill — strengths destructured for clarity; the
    // spill itself arrives blended from the blur pre-pass in the pipeline.
    let _ = film.halation;
    c
}

fn apply_dot(c: [f64; 3], a: [f64; 3], k: f64) -> [f64; 3] {
    [c[0] + a[0] * k, c[1] + a[1] * k, c[2] + a[2] * k]
}

/// Wheels word order: [exposure, gamma, gain r g b l, offset r g b l, lift r g b l]
fn apply_wheels(mut c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    let exposure = d[0] as f64 * gate;
    c = [
        c[0] * 2f64.powf(exposure),
        c[1] * 2f64.powf(exposure),
        c[2] * 2f64.powf(exposure),
    ];
    // gain
    for ch in 0..3 {
        let g = 1.0 + (d[2 + ch] as f64 + d[5] as f64) * gate;
        c[ch] *= g.max(0.0);
    }
    // gamma (clamped at 0 first — documented)
    let p = 2f64.powf(-(d[1] as f64) * gate);
    for ch in 0..3 {
        c[ch] = c[ch].max(0.0).powf(p);
    }
    // offset
    for ch in 0..3 {
        c[ch] += (d[6 + ch] as f64 + d[9] as f64) * gate;
    }
    // shadow-weighted lift
    let shade = (1.0 - luma_f64(c)).max(0.0) * gate;
    apply_dot(c, [d[10] as f64, d[11] as f64, d[12] as f64], shade)
}

/// Zones word order: 6 x [center, width, r g b l]
fn apply_zones(c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    let l = luma_f64(c).max(1e-6).log2();
    let mut acc = [0.0f64; 3];
    for z in 0..6 {
        let base = z * 6;
        let center = d[base] as f64;
        let width = d[base + 1] as f64;
        let w = band_weight_f64((l - center).abs(), width / 2.0, 0.0) * gate;
        for ch in 0..3 {
            acc[ch] += (d[base + 2 + ch] as f64 + d[base + 5] as f64) * w;
        }
    }
    [c[0] + acc[0], c[1] + acc[1], c[2] + acc[2]]
}

/// Curves word order: 4 curves x [count, 8 (x, y) pairs]; master, red,
/// green, blue. `count` excludes padding. The window gate interpolates
/// between the input value and the curved value (a full replace would make
/// window gating a no-op).
fn apply_curves(c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    let stride = 1 + MAX_CURVE_POINTS * 2;
    let read_curve = |curve: usize| -> Vec<[f64; 2]> {
        let base = curve * stride;
        let count = (d[base] as usize).min(MAX_CURVE_POINTS);
        (0..count)
            .map(|i| {
                let o = base + 1 + i * 2;
                [d[o] as f64, d[o + 1] as f64]
            })
            .collect()
    };
    let mut curved = c;
    // Master: same spline applied to every channel in parallel.
    for ch in 0..3 {
        let pts = read_curve(0);
        if pts.len() >= 2 {
            let v = c[ch].clamp(0.0, 1.0);
            curved[ch] = monotone_spline_f64(&pts, v);
        }
    }
    // Per-channel curves.
    for ch in 0..3 {
        let pts = read_curve(1 + ch);
        if pts.len() >= 2 {
            let v = curved[ch].clamp(0.0, 1.0);
            curved[ch] = monotone_spline_f64(&pts, v);
        }
    }
    [
        c[0] + (curved[0] - c[0]) * gate,
        c[1] + (curved[1] - c[1]) * gate,
        c[2] + (curved[2] - c[2]) * gate,
    ]
}

/// LUT word order: [size] + size^3 RGB triples (r fastest).
fn apply_lut(c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    if gate <= 0.0 {
        return c;
    }
    let size = d[0] as usize;
    let grid: Vec<f64> = d[1..].iter().map(|v| *v as f64).collect();
    let out = tetra_lut_f64(size, &grid, c);
    [
        c[0] + (out[0] - c[0]) * gate,
        c[1] + (out[1] - c[1]) * gate,
        c[2] + (out[2] - c[2]) * gate,
    ]
}

/// HSL word order: [enabled, hue_center, hue_width, hue_softness, sat_center,
/// sat_width, sat_softness, luma_center, luma_width, luma_softness,
/// hue_shift, sat_boost, exposure_shift, invert]
fn apply_hsl(c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    if d[0] <= 0.0 {
        return c;
    }
    let (hue, sat, l) = rgb_to_hsl_f64(c);
    let hue_w = band_weight_f64(hue_distance_f64(hue, d[1] as f64).abs(), d[2] as f64 / 2.0, d[3] as f64);
    let sat_w = band_weight_f64((sat - d[4] as f64).abs(), d[5] as f64 / 2.0, d[6] as f64);
    let luma_w = band_weight_f64(
        (luma_f64(c).clamp(0.0, 1.0) - d[7] as f64).abs(),
        d[8] as f64 / 2.0,
        d[9] as f64,
    );
    let mut w = hue_w * sat_w * luma_w * gate;
    if d[13] > 0.5 {
        w = 1.0 - w;
    }
    if w <= 0.0 {
        return c;
    }
    let hue2 = (hue + d[10] as f64 * w).rem_euclid(1.0);
    let sat2 = (sat * (1.0 + (d[11] as f64 - 1.0) * w)).clamp(0.0, 1.0);
    let mut rgb2 = hsl_to_rgb_f64(hue2, sat2, l);
    let exposure = 2f64.powf(d[12] as f64 * w);
    for ch in 0..3 {
        rgb2[ch] *= exposure;
    }
    [
        c[0] + (rgb2[0] - c[0]) * 1.0,
        c[1] + (rgb2[1] - c[1]) * 1.0,
        c[2] + (rgb2[2] - c[2]) * 1.0,
    ]
}

/// Warper word order: [count] + count x [hue, sat, d_hue, d_sat].
fn apply_warper(c: [f64; 3], d: &[f32], gate: f64) -> [f64; 3] {
    if gate <= 0.0 {
        return c;
    }
    let count = d[0] as usize;
    if count == 0 {
        return c;
    }
    let (hue, sat, l) = rgb_to_hsl_f64(c);
    let mut wsum = 0.0f64;
    let mut dh = 0.0f64;
    let mut ds = 0.0f64;
    for p in 0..count {
        let base = 1 + p * 4;
        let ph = d[base] as f64;
        let ps = d[base + 1] as f64;
        let dist_sq = hue_distance_f64(hue, ph).powi(2) + (sat - ps).powi(2);
        let w = 1.0 / dist_sq.max(1e-4);
        wsum += w;
        dh += w * d[base + 2] as f64;
        ds += w * d[base + 3] as f64;
    }
    let hue2 = (hue + dh / wsum).rem_euclid(1.0);
    let sat2 = (sat + ds / wsum).clamp(0.0, 1.0);
    let rgb2 = hsl_to_rgb_f64(hue2, sat2, l);
    [
        c[0] + (rgb2[0] - c[0]) * gate,
        c[1] + (rgb2[1] - c[1]) * gate,
        c[2] + (rgb2[2] - c[2]) * gate,
    ]
}

/// Film word order: [strength, weave_amplitude_uv, _, _]. Grain is added on
/// the eval-domain luma: n * strength * 0.10 * sqrt(luma) (documented model;
/// the 0.10 scale is a design constant, not a measured claim).
fn apply_grain(
    mut c: [f64; 3],
    pixel: [i64; 2],
    frame: u32,
    d: &[f32],
    gate: f64,
) -> [f64; 3] {
    let strength = d[0] as f64 * gate;
    if strength <= 0.0 {
        return c;
    }
    let n = grain_noise_f64(pixel[0], pixel[1], frame);
    let amp = strength * 0.10 * luma_f64(c).max(0.01).sqrt();
    for ch in 0..3 {
        c[ch] += n * amp;
    }
    c
}

/// Window coverage in UV. The static transform maps the pixel around the
/// window pivot first: `p' = pivot + R(θ)·(uv - pivot) / scale`, with θ the
/// clockwise (y-down) rotation in degrees — identical in the WGSL path.
pub fn window_weight_f64(w: &ResolvedWindow, uv: [f64; 2]) -> f64 {
    let pivot = [w.pivot[0] as f64, w.pivot[1] as f64];
    let scale = w.scale as f64;
    let theta = w.rotation_deg as f64 * std::f64::consts::PI / 180.0;
    let (s, cth) = theta.sin_cos();
    let rel = [uv[0] - pivot[0], uv[1] - pivot[1]];
    let scaled = [rel[0] / scale, rel[1] / scale];
    // CW in y-down: x' = x·cos − y·sin, y' = x·sin + y·cos.
    let rotated = [scaled[0] * cth - scaled[1] * s, scaled[0] * s + scaled[1] * cth];
    let mapped = [pivot[0] + rotated[0], pivot[1] + rotated[1]];
    weight_for_shape(w, mapped)
}

fn weight_for_shape(w: &ResolvedWindow, uv: [f64; 2]) -> f64 {
    let mut distance = match w.shape {
        WindowId::Rect => {
            let dx = ((uv[0] - w.center[0] as f64) / w.half_size[0] as f64).abs();
            let dy = ((uv[1] - w.center[1] as f64) / w.half_size[1] as f64).abs();
            dx.max(dy)
        }
        WindowId::Ellipse => {
            let dx = (uv[0] - w.center[0] as f64) / w.half_size[0] as f64;
            let dy = (uv[1] - w.center[1] as f64) / w.half_size[1] as f64;
            (dx * dx + dy * dy).sqrt()
        }
        WindowId::Polygon => {
            // Normalised signed distance: inside → 1 - frac, outside → the
            // distance to the nearest edge normalised by its length.
            polygon_distance(w, uv)
        }
    };
    if w.inverted {
        distance = -distance;
    }
    let weight = if w.softness <= 0.0 {
        if distance <= 1.0 {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - smoothstep_f64(1.0 - w.softness as f64, 1.0, distance)
    };
    weight * w.opacity as f64
}

/// Polygon coverage: 1 inside, 0 outside, softness is not symmetric for
/// polygons in this packet — the hard edge uses the point-in-polygon test
/// and the soft falloff blends the inside weight with the outside by the
/// normalised nearest-edge distance (documented).
fn polygon_distance(w: &ResolvedWindow, uv: [f64; 2]) -> f64 {
    let pts: Vec<[f64; 2]> = w.polygon.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    let n = pts.len();
    let mut inside = false;
    let mut nearest = f64::INFINITY;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        // ray cast
        if (a[1] > uv[1]) != (b[1] > uv[1]) {
            let x_int = a[0] + (uv[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if uv[0] < x_int {
                inside = !inside;
            }
        }
        // distance to segment
        let ab = [b[0] - a[0], b[1] - a[1]];
        let ap = [uv[0] - a[0], uv[1] - a[1]];
        let ab2 = ab[0] * ab[0] + ab[1] * ab[1];
        let t = if ab2 > 0.0 {
            (ap[0] * ab[0] + ap[1] * ab[1]).clamp(0.0, 1.0) / ab2
        } else {
            0.0
        };
        let proj = [a[0] + ab[0] * t, a[1] + ab[1] * t];
        let d = ((uv[0] - proj[0]).powi(2) + (uv[1] - proj[1]).powi(2)).sqrt();
        nearest = nearest.min(d);
    }
    // Normalise: nearest edge distance relative to window bbox scale —
    // documented normalisation (mean edgehalf extent).
    let mut scale = 0.0f64;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        scale += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    }
    scale /= n as f64;
    let norm = nearest / scale.max(1e-6);
    if inside {
        1.0 - norm.min(1.0)
    } else {
        1.0 + norm.min(1.0)
    }
}
