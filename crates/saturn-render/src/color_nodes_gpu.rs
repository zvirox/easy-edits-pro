// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! WGSL colour-node chain evaluation (AI-native plan Packet 3.2).
//!
//! **Evaluation domain.** Every node in a clip's [`ColorNode`] chain is
//! evaluated on **scene-linear ACEScg (AP1)** triples, regardless of the
//! project's grading-view selection: ACEScct is a viewing/grading transform
//! applied elsewhere in the pipeline, so exposure stops stay physically
//! meaningful. The chain entry point samples the source at a gate-weave
//! offset (see [`crate::film_emulation::gate_weave_offset`]).
//!
//! **Semantics are single-sourced.** The CPU reference in this module and
//! the WGSL program implement the same documented order:
//! exposure → gain → gamma (clamped) → offset → shadow-weighted lift, additive
//! luminance-banded HDR zones, monotone-cubic RGB curves, tetrahedral 3D LUT,
//! HSL qualifier + correction, hue/sat landmark warp, power-window gating
//! (product of window weights), then per-pixel film grain. Halation runs as a
//! separate bright-pass/blur pair in [`crate::film_emulation`] (its result is
//! bound and added by the chain).
//!
//! **Hardening (plan §10.2.3).** WGSL is pre-parsed with naga before a
//! pipeline is created, dispatches run inside a `wgpu` validation error
//! scope, and every failure surfaces as a typed error — never a panic.

use saturn_core::project::{
    ColorNode, ColorNodeKind, HdrZonesPayload, Lut3DPayload, PowerWindow, PrimaryWheelsPayload,
    RgbCurves, WindowShape,
};

/// Node chain limits (documented; the engine renders these as typed errors
/// rather than silently truncating).
pub const MAX_NODES: usize = 16;
pub const MAX_WINDOWS_PER_NODE: usize = 4;
pub const MAX_CURVE_POINTS: usize = 8;
pub const MAX_WARPER_POINTS: usize = 64;
/// Blob budget in u32 words (nodes + windows + payload data).
pub const MAX_BLOB_WORDS: usize = 8192;

#[derive(Debug, thiserror::Error)]
pub enum ColorNodesError {
    #[error("colour node chain exceeds the {0} node limit")]
    TooManyNodes(usize),
    #[error("node {0} exceeds the {1} window limit")]
    TooManyWindows(u64, usize),
    #[error("packed node blob needs {0} words, over the {1}-word budget")]
    BlobTooLarge(usize, usize),
    #[error("node {0} has more than {1} curve points")]
    TooManyCurvePoints(u64, usize),
    #[error("node {0} has more than {1} warper points")]
    TooManyWarperPoints(u64, usize),
    #[error("node {0} LUT payload could not be decoded: {1}")]
    LutDecode(u64, String),
    #[error("node {0} payload is invalid: {1}")]
    Payload(u64, String),
    #[error("WGSL failed naga pre-validation: {0}")]
    WgslInvalid(String),
    #[error("GPU pipeline creation failed: {0}")]
    Pipeline(String),
    #[error("GPU dispatch failed (validation scope): {0}")]
    Dispatch(String),
    #[error("GPU context error: {0}")]
    Context(#[from] crate::gpu_context::GpuContextError),
}

/// A colour node resolved into flat f32 payload words in the documented
/// per-kind order. Built from the persisted schema node.
#[derive(Clone, Debug)]
pub struct EvalNode {
    pub kind: u32,
    pub enabled: bool,
    /// Payload words (packed per kind; see `pack_node_data`).
    pub data: Vec<f32>,
    /// Resolved windows (flattened during blob packing).
    pub windows: Vec<ResolvedWindow>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FilmPlan {
    pub halation: f32,
    pub grain: f32,
    pub weave_mm: f64,
}

/// A window resolved for evaluation: shape + the static transform baked in
/// as pivot/scale/rotation (applied identically by the CPU and WGSL paths).
#[derive(Clone, Debug)]
pub struct ResolvedWindow {
    pub shape: WindowId,
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub softness: f32,
    pub inverted: bool,
    pub opacity: f32,
    /// Transform pivot in UV, uniform scale, rotation in degrees (CW on
    /// screen, y-down).
    pub pivot: [f32; 2],
    pub scale: f32,
    pub rotation_deg: f32,
    /// Polygon points (shape = polygon), clip-relative UV.
    pub polygon: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowId {
    Rect = 0,
    Ellipse = 1,
    Polygon = 2,
}

pub const KIND_PRIMARY_WHEELS: u32 = 0;
pub const KIND_HDR_ZONES: u32 = 1;
pub const KIND_RGB_CURVES: u32 = 2;
pub const KIND_LUT_3D: u32 = 3;
pub const KIND_HSL_QUALIFIER: u32 = 4;
pub const KIND_WARPER: u32 = 5;
pub const KIND_FILM_GRAIN: u32 = 6;

/// Decode a node's stored payload into typed values. Failures are typed
/// errors — persisted payloads were canonicalised by the engine at write
/// time, so an undeodable payload means a hand-edited or foreign project.
fn decode_primary(node: &ColorNode) -> Result<PrimaryWheelsPayload, ColorNodesError> {
    serde_json::from_value(node.payload.clone())
        .map_err(|e| ColorNodesError::Payload(node.id, e.to_string()))
}

fn decode_zones(node: &ColorNode) -> Result<HdrZonesPayload, ColorNodesError> {
    serde_json::from_value(node.payload.clone())
        .map_err(|e| ColorNodesError::Payload(node.id, e.to_string()))
}

/// rgb_curves and hsl_qualifier reuse the effects-model shapes.
fn decode_curves(node: &ColorNode) -> Result<RgbCurves, ColorNodesError> {
    serde_json::from_value(node.payload.clone())
        .map_err(|e| ColorNodesError::Payload(node.id, e.to_string()))
}

/// Builds the evaluation plan from the clip's stored node chain.
pub fn build_plan(nodes: &[ColorNode]) -> Result<(Vec<EvalNode>, FilmPlan), ColorNodesError> {
    if nodes.len() > MAX_NODES {
        return Err(ColorNodesError::TooManyNodes(nodes.len()));
    }
    let mut evals = Vec::new();
    let mut film = FilmPlan::default();
    for node in nodes {
        if node.windows.len() > MAX_WINDOWS_PER_NODE {
            return Err(ColorNodesError::TooManyWindows(node.id, node.windows.len()));
        }
        let mut windows = Vec::new();
        for window in &node.windows {
            windows.push(resolve_window(window)?);
        }
        let (kind, data) = match node.kind {
            ColorNodeKind::PrimaryWheels => (
                KIND_PRIMARY_WHEELS,
                pack_primary_wheels(&decode_primary(node)?),
            ),
            ColorNodeKind::HdrZones => (KIND_HDR_ZONES, pack_zones(&decode_zones(node)?)),
            ColorNodeKind::RgbCurves => {
                let curves = decode_curves(node)?;
                let data = pack_curves(&curves)
                    .ok_or(ColorNodesError::TooManyCurvePoints(node.id, MAX_CURVE_POINTS))?;
                (KIND_RGB_CURVES, data)
            }
            ColorNodeKind::Lut3D => {
                let lut: Lut3DPayload = serde_json::from_value(node.payload.clone())
                    .map_err(|e| ColorNodesError::LutDecode(node.id, e.to_string()))?;
                (KIND_LUT_3D, pack_lut(node.id, &lut)?)
            }
            ColorNodeKind::HslQualifier => (
                KIND_HSL_QUALIFIER,
                pack_hsl(&node.payload).map_err(|e| ColorNodesError::Payload(node.id, e))?,
            ),
            ColorNodeKind::ColorWarperGrid => {
                let points: Vec<saturn_core::project::WarperPoint> =
                    serde_json::from_value(node.payload["points"].clone())
                        .map_err(|e| ColorNodesError::Payload(node.id, e.to_string()))?;
                if points.len() > MAX_WARPER_POINTS {
                    return Err(ColorNodesError::TooManyWarperPoints(node.id, points.len()));
                }
                (KIND_WARPER, pack_warper(&points))
            }
            ColorNodeKind::FilmPrintEmulation => {
                let film_payload: saturn_core::project::FilmPrintPayload =
                    serde_json::from_value(node.payload.clone())
                        .map_err(|e| ColorNodesError::Payload(node.id, e.to_string()))?;
                film.halation = film_payload.halation as f32;
                film.grain = film_payload.grain as f32;
                if film_payload.weave_mm > 0.0 {
                    film.weave_mm = film_payload.weave_mm;
                }
                // The per-pixel grain rides on the chain; halation rides on
                // the blur pre-pass; weave on the source sample offset.
                (KIND_FILM_GRAIN, pack_film(&film_payload))
            }
        };
        evals.push(EvalNode { kind, enabled: node.enabled, data, windows });
    }
    Ok((evals, film))
}

fn resolve_window(window: &PowerWindow) -> Result<ResolvedWindow, ColorNodesError> {
    Ok(match &window.shape {
        WindowShape::Rect { center, half_size } => ResolvedWindow {
            shape: WindowId::Rect,
            center: [center[0] as f32, center[1] as f32],
            half_size: [half_size[0] as f32, half_size[1] as f32],
            softness: window.softness as f32,
            inverted: window.inverted,
            opacity: window.opacity as f32,
            pivot: [window.transform.pos_x as f32, window.transform.pos_y as f32],
            scale: window.transform.scale as f32,
            rotation_deg: window.transform.rotation_deg as f32,
            polygon: Vec::new(),
        },
        WindowShape::Ellipse { center, half_size } => ResolvedWindow {
            shape: WindowId::Ellipse,
            center: [center[0] as f32, center[1] as f32],
            half_size: [half_size[0] as f32, half_size[1] as f32],
            softness: window.softness as f32,
            inverted: window.inverted,
            opacity: window.opacity as f32,
            pivot: [window.transform.pos_x as f32, window.transform.pos_y as f32],
            scale: window.transform.scale as f32,
            rotation_deg: window.transform.rotation_deg as f32,
            polygon: Vec::new(),
        },
        WindowShape::Polygon { points } => {
            if points.len() < 3 {
                return Err(ColorNodesError::Payload(
                    window.id,
                    "polygon window needs at least 3 points".into(),
                ));
            }
            ResolvedWindow {
                shape: WindowId::Polygon,
                center: [0.0, 0.0],
                half_size: [0.0, 0.0],
                softness: window.softness as f32,
                inverted: window.inverted,
                opacity: window.opacity as f32,
                polygon: points
                    .iter()
                    .map(|p| [p[0] as f32, p[1] as f32])
                    .collect(),
            pivot: [window.transform.pos_x as f32, window.transform.pos_y as f32],
            scale: window.transform.scale as f32,
            rotation_deg: window.transform.rotation_deg as f32,
        }
        }
    })
}

// --- Payload packing (documented per-kind word order) -----------------------

fn pack_wheel(w: &saturn_core::project::Wheel, out: &mut Vec<f32>) {
    out.extend_from_slice(&[w.red as f32, w.green as f32, w.blue as f32, w.luma as f32]);
}

fn pack_primary_wheels(p: &PrimaryWheelsPayload) -> Vec<f32> {
    let mut v = vec![p.exposure_stops as f32, p.gamma as f32];
    pack_wheel(&p.gain, &mut v);
    pack_wheel(&p.offset, &mut v);
    pack_wheel(&p.lift, &mut v);
    v
}

fn pack_zones(z: &HdrZonesPayload) -> Vec<f32> {
    let mut v = Vec::new();
    for zone in [
        &z.black,
        &z.shadow,
        &z.dark_mid,
        &z.light_mid,
        &z.highlight,
        &z.white,
    ] {
        v.extend_from_slice(&[zone.center as f32, zone.width as f32]);
        pack_wheel(&zone.wheel, &mut v);
    }
    v
}

/// Curves: 4 curves x [count, then MAX_CURVE_POINTS (x, y) pairs],
/// zero-padded; count drives how much of the array the spline uses.
fn pack_curves(c: &RgbCurves) -> Option<Vec<f32>> {
    let mut v = Vec::new();
    for curve in [&c.master, &c.red, &c.green, &c.blue] {
        if curve.len() > MAX_CURVE_POINTS {
            return None;
        }
        v.push(curve.len() as f32);
        for p in curve {
            v.push(p.x as f32);
            v.push(p.y as f32);
        }
        for _ in curve.len()..MAX_CURVE_POINTS {
            v.extend_from_slice(&[0.0, 0.0]);
        }
    }
    Some(v)
}

/// LUT: [size] then size^3 RGB f32 triples, r fastest (matches the stored
/// base64 layout and the .cube convention).
fn pack_lut(node_id: u64, lut: &Lut3DPayload) -> Result<Vec<f32>, ColorNodesError> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&lut.data_b64)
        .map_err(|e| ColorNodesError::LutDecode(node_id, e.to_string()))?;
    let expected = (lut.size * lut.size * lut.size * 3 * 4) as usize;
    if bytes.len() != expected {
        return Err(ColorNodesError::LutDecode(
            node_id,
            format!("expected {expected} bytes for size {}, got {}", lut.size, bytes.len()),
        ));
    }
    let mut v = vec![lut.size as f32];
    for chunk in bytes.chunks_exact(4) {
        v.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }
    Ok(v)
}

/// HSL qualifier: selection bands + correction, normalised by `enabled` at
/// eval time (the qualifier payload itself carries no `blur_radius`; that
/// op is not a colour correction).
fn pack_hsl(payload: &serde_json::Value) -> Result<Vec<f32>, String> {
    let get = |key: &str| -> Result<f32, String> {
        Ok(payload
            .get(key)
            .and_then(|v| v.as_f64())
            .ok_or(format!("hsl_qualifier payload missing `{key}`"))? as f32)
    };
    let enabled = payload
        .get("enabled")
        .and_then(|v| v.as_bool())
        .ok_or("hsl_qualifier payload missing `enabled`")?;
    let mut v = vec![if enabled { 1.0 } else { 0.0 }];
    for key in [
        "hue_center",
        "hue_width",
        "hue_softness",
        "sat_center",
        "sat_width",
        "sat_softness",
        "luma_center",
        "luma_width",
        "luma_softness",
        "hue_shift",
        "sat_boost",
        "exposure_shift",
    ] {
        v.push(get(key)?);
    }
    let invert = payload
        .get("invert")
        .and_then(|v| v.as_bool())
        .ok_or("hsl_qualifier payload missing `invert`")?;
    v.push(if invert { 1.0 } else { 0.0 });
    Ok(v)
}

fn pack_warper(points: &[saturn_core::project::WarperPoint]) -> Vec<f32> {
    let mut v = vec![points.len() as f32];
    for p in points {
        v.extend_from_slice(&[p.hue as f32, p.sat as f32, p.delta_hue as f32, p.delta_sat as f32]);
    }
    v
}

/// Film grain words: [strength, weave_amplitude_uv, _pad, _pad]. Halation is
/// resolved by the pre-pass, not the per-pixel chain.
fn pack_film(p: &saturn_core::project::FilmPrintPayload) -> Vec<f32> {
    vec![p.grain as f32, (p.weave_mm / 36.0) as f32, 0.0, 0.0]
}

// --- Blob packing (single storage buffer, all-u32 words) --------------------
//
// Layout (documented word order; f32 values are stored bit-exact):
//
//   header    8 words: [node_count, frame, weave_uv_x, weave_uv_y,
//                       width, height, halation_strength, reserved]
//   node i    8 words: [kind, enabled, data_off, data_len, win_off,
//                       win_count, reserved, reserved]
//   window j 16 words: [shape, inverted, opacity, softness, center.xy,
//                       half_size.xy, pivot.xy, scale, rotation_deg,
//                       pts_off, pts_count, reserved, reserved]
//   payload section: concatenated node payload words (f32 bits);
//   polygon section: polygon point pairs (x, y as f32 bits), referenced by
//   window `pts_off`/`pts_count`. All offsets are absolute word indices.

const HEADER_WORDS: usize = 8;
const NODE_RECORD_WORDS: usize = 8;
const WINDOW_RECORD_WORDS: usize = 16;

/// Packs the chain into the single u32-word blob consumed by the WGSL
/// program. Weave arrives pre-computed (see
/// `crate::film_emulation::gate_weave_offset`); halation strength is applied
/// by the shader against the pre-pass texture.
pub fn pack_blob(
    nodes: &[EvalNode],
    film: &FilmPlan,
    frame: u32,
    weave_uv: [f64; 2],
    width: u32,
    height: u32,
) -> Result<Vec<u32>, ColorNodesError> {
    let window_total: usize = nodes.iter().map(|n| n.windows.len()).sum();
    let payload_total: usize = nodes.iter().map(|n| n.data.len()).sum();
    let poly_total: usize = nodes
        .iter()
        .flat_map(|n| n.windows.iter())
        .map(|w| w.polygon.len() * 2)
        .sum();
    let bound = HEADER_WORDS
        + nodes.len() * NODE_RECORD_WORDS
        + payload_total
        + window_total * WINDOW_RECORD_WORDS
        + poly_total;
    if bound > MAX_BLOB_WORDS {
        return Err(ColorNodesError::BlobTooLarge(bound, MAX_BLOB_WORDS));
    }

    // Fixed-size regions first so absolute offsets are known up front.
    let header = [
        nodes.len() as u32,
        frame,
        (weave_uv[0] as f32).to_bits(),
        (weave_uv[1] as f32).to_bits(),
        width,
        height,
        film.halation.to_bits(),
        0,
    ];
    let records_base = HEADER_WORDS;
    let payload_base = records_base + nodes.len() * NODE_RECORD_WORDS;
    let windows_base = payload_base + payload_total;
    let poly_base = windows_base + window_total * WINDOW_RECORD_WORDS;

    let mut blob = vec![0u32; bound];
    blob[..HEADER_WORDS].copy_from_slice(&header);

    let mut payload_cursor = payload_base;
    let mut window_cursor = windows_base;
    let mut poly_cursor = poly_base;
    for (i, node) in nodes.iter().enumerate() {
        let rec = records_base + i * NODE_RECORD_WORDS;
        blob[rec] = node.kind;
        blob[rec + 1] = u32::from(node.enabled);
        blob[rec + 2] = payload_cursor as u32;
        blob[rec + 3] = node.data.len() as u32;
        blob[rec + 4] = window_cursor as u32;
        blob[rec + 5] = node.windows.len() as u32;
        for value in &node.data {
            blob[payload_cursor] = value.to_bits();
            payload_cursor += 1;
        }
        for window in &node.windows {
            blob[window_cursor] = window.shape as u32;
            blob[window_cursor + 1] = u32::from(window.inverted);
            blob[window_cursor + 2] = window.opacity.to_bits();
            blob[window_cursor + 3] = window.softness.to_bits();
            blob[window_cursor + 4] = window.center[0].to_bits();
            blob[window_cursor + 5] = window.center[1].to_bits();
            blob[window_cursor + 6] = window.half_size[0].to_bits();
            blob[window_cursor + 7] = window.half_size[1].to_bits();
            blob[window_cursor + 8] = window.pivot[0].to_bits();
            blob[window_cursor + 9] = window.pivot[1].to_bits();
            blob[window_cursor + 10] = window.scale.to_bits();
            blob[window_cursor + 11] = window.rotation_deg.to_bits();
            if window.polygon.is_empty() {
                blob[window_cursor + 12] = 0;
                blob[window_cursor + 13] = 0;
            } else {
                blob[window_cursor + 12] = poly_cursor as u32;
                blob[window_cursor + 13] = window.polygon.len() as u32;
                for point in &window.polygon {
                    blob[poly_cursor] = point[0].to_bits();
                    blob[poly_cursor + 1] = point[1].to_bits();
                    poly_cursor += 2;
                }
            }
            // Words 14, 15 reserved (already zero).
            window_cursor += WINDOW_RECORD_WORDS;
        }
    }
    debug_assert_eq!(payload_cursor, windows_base);
    debug_assert_eq!(window_cursor, poly_base);
    debug_assert_eq!(poly_cursor, bound);
    Ok(blob)
}