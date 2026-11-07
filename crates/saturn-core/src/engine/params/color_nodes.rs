// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Strict wire structs for the Packet 3.2 colour-node commands. These are
//! deliberately separate from the persisted schema (which stays permissive
//! for forward compatibility): the command channel rejects unknown fields
//! and out-of-range values instead of silently ignoring or clamping them.

use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct AddColorNodeParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) kind: ColorNodeKind,
    #[serde(default)]
    pub(in crate::engine) inputs: Option<Vec<u64>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateColorNodeParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) node_id: u64,
    pub(in crate::engine) payload: serde_json::Value,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddPowerWindowParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) node_id: u64,
    pub(in crate::engine) mask: serde_json::Value,
    #[serde(default)]
    pub(in crate::engine) auto_track: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct EmulateFilmPrintParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) stock: String,
    pub(in crate::engine) halation: f64,
    pub(in crate::engine) grain: f64,
    pub(in crate::engine) weave: f64,
}

/// `color.match_shot_to_reference`. The engine is decode-free by design:
/// the caller that owns frame decoding (`saturn-render::shot_match_driver`
/// through the bridge) measures both frames and fits the correction, then
/// hands the fitted payload here for validation, checkpointing and storage.
/// `src_*` names the clip that receives the match node; `ref_*` is recorded
/// provenance for where the anchor came from (both must exist).
#[derive(Deserialize)]
pub(in crate::engine) struct MatchShotToReferenceParams {
    pub(in crate::engine) src_track_id: u64,
    pub(in crate::engine) src_clip_index: usize,
    pub(in crate::engine) ref_track_id: u64,
    pub(in crate::engine) ref_clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) lock_skin_tones: Option<bool>,
    pub(in crate::engine) wheels: serde_json::Value,
}

/// Validated power-window payload as sent by `color.add_power_window` (the
/// engine assigns the id itself). Wire shape mirrors `PowerWindow` minus id.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(in crate::engine) struct PowerWindowWire {
    pub(in crate::engine) enabled: bool,
    pub(in crate::engine) shape: WindowShape,
    pub(in crate::engine) softness: f64,
    pub(in crate::engine) inverted: bool,
    pub(in crate::engine) opacity: f64,
    pub(in crate::engine) transform: Option<WindowTransform>,
}

impl Default for PowerWindowWire {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: WindowShape::Rect { center: [0.5, 0.5], half_size: [0.25, 0.25] },
            softness: 0.0,
            inverted: false,
            opacity: 1.0,
            transform: None,
        }
    }
}

/// Validates the window wire payload; returns the schema [`PowerWindow`] to
/// store once the engine has assigned an id.
pub(in crate::engine) fn validate_window(
    wire: &PowerWindowWire,
) -> Result<PowerWindow, DispatchError> {
    let range = |v: f64, lo: f64, hi: f64, name: &str| -> Result<(), DispatchError> {
        if v.is_finite() && v >= lo && v <= hi {
            Ok(())
        } else {
            Err(DispatchError::InvalidParameters(format!(
                "{name} must be in [{lo}, {hi}], got {v}"
            )))
        }
    };
    range(wire.softness, 0.0, 0.5, "softness")?;
    range(wire.opacity, 0.0, 1.0, "opacity")?;
    match &wire.shape {
        WindowShape::Rect { center, half_size }
        | WindowShape::Ellipse { center, half_size } => {
            range(center[0], 0.0, 1.0, "center.x")?;
            range(center[1], 0.0, 1.0, "center.y")?;
            range(half_size[0], 0.001, 1.0, "half_size.x")?;
            range(half_size[1], 0.001, 1.0, "half_size.y")?;
        }
        WindowShape::Polygon { points } => {
            if points.len() < 3 {
                return Err(DispatchError::InvalidParameters(
                    "polygon windows need at least 3 points".into(),
                ));
            }
            for (i, p) in points.iter().enumerate() {
                range(p[0], 0.0, 1.0, &format!("points[{i}].x"))?;
                range(p[1], 0.0, 1.0, &format!("points[{i}].y"))?;
            }
        }
    }
    if let Some(t) = &wire.transform {
        range(t.pos_x, 0.0, 1.0, "transform.pos_x")?;
        range(t.pos_y, 0.0, 1.0, "transform.pos_y")?;
        range(t.scale, 0.01, 100.0, "transform.scale")?;
        range(t.rotation_deg, -360.0, 360.0, "transform.rotation_deg")?;
    }
    Ok(PowerWindow {
        id: 0, // assigned by the caller
        enabled: wire.enabled,
        shape: wire.shape.clone(),
        softness: wire.softness,
        inverted: wire.inverted,
        opacity: wire.opacity,
        transform: wire.transform.unwrap_or_default(),
    })
}
