// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 3.2 colour-node commands:
//!
//! - `color.node_add` — appends a node to the clip's ordered colour chain;
//! - `color.node_update` — strictly validates and stores a node payload;
//! - `color.add_power_window` — attaches a static geometric window to a node
//!   (AI-tracked windows are a later packet and fail with a typed error);
//! - `color.emulate_film_print` — creates/updates the clip's film print node
//!   from operator parameters.

use super::*;

/// Immutable lookup used to validate a command before any checkpoint.
fn node_kind_in(
    document: &ProjectDocument,
    track_index: usize,
    clip_index: usize,
    node_id: u64,
) -> Result<ColorNodeKind, DispatchError> {
    let clip = document
        .project
        .sequence
        .tracks
        .get(track_index)
        .and_then(|track| track.clips.get(clip_index));
    let Some(clip) = clip else {
        return Err(DispatchError::InvalidParameters("Timeline clip does not exist".into()));
    };
    clip.color_nodes
        .iter()
        .find(|node| node.id == node_id)
        .map(|node| node.kind)
        .ok_or_else(|| {
            DispatchError::InvalidParameters(format!(
                "color node {node_id} does not exist on this clip"
            ))
        })
}

impl EditorEngine {
    pub(super) fn dispatch_color_nodes(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "color.node_add" => Some((|| {
                let params: AddColorNodeParams = parse_params(params)?;
                if params.inputs.as_ref().is_some_and(|i| !i.is_empty()) {
                    // Honest limitation: the colour DAG is a linear chain in
                    // this packet; socket branching is not silently ignored.
                    return Err(DispatchError::InvalidParameters(
                        "colour DAG branching is not supported yet: nodes execute in insertion order; omit \"inputs\"".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index]
                    .clips
                    .get(params.clip_index)
                    .is_none()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                let node_id = self.document.project.next_id;
                let node = ColorNode {
                    id: node_id,
                    kind: params.kind,
                    enabled: true,
                    payload: default_payload(params.kind),
                    windows: Vec::new(),
                    origin: None,
                };
                self.checkpoint();
                self.document.project.next_id = node_id.saturating_add(1);
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .color_nodes
                    .push(node);
                Ok(json!({ "nodeId": node_id, "kind": params.kind.as_str() }))
            })()),
            "color.node_update" => Some((|| {
                let params: UpdateColorNodeParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let kind = node_kind_in(&self.document, track_index, params.clip_index, params.node_id)?;
                // Validate against the discovered kind before checkpointing;
                // the payload must pass the same contract the GPU/CPU eval
                // relies on for its bounds.
                let canonical =
                    validate_payload(kind, &params.payload).map_err(DispatchError::InvalidParameters)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let node = clip
                    .color_nodes
                    .iter_mut()
                    .find(|node| node.id == params.node_id)
                    .expect("node existed in the pre-checkpoint lookup");
                node.payload = canonical;
                Ok(json!({ "nodeId": node.id, "kind": node.kind.as_str() }))
            })()),
            "color.add_power_window" => Some((|| {
                let params: AddPowerWindowParams = parse_params(params)?;
                if params.auto_track == Some(true) {
                    // Typed refusal, not a fake static success: AI mask
                    // segmentation/tracking is a later packet.
                    return Err(DispatchError::InvalidParameters(
                        "auto_track is not available yet: AI power-window tracking lands with the AI segmentation packet; use a static window".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                node_kind_in(&self.document, track_index, params.clip_index, params.node_id)?;
                let wire: PowerWindowWire = serde_json::from_value(params.mask)
                    .map_err(|e| DispatchError::InvalidParameters(format!("mask: {e}")))?;
                let mut window = validate_window(&wire)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let node = clip
                    .color_nodes
                    .iter_mut()
                    .find(|node| node.id == params.node_id)
                    .expect("node existed in the pre-checkpoint lookup");
                let window_id = self.document.project.next_id;
                self.document.project.next_id = window_id.saturating_add(1);
                window.id = window_id;
                node.windows.push(window);
                Ok(json!({ "windowId": window_id }))
            })()),
            "color.emulate_film_print" => Some((|| {
                let params: EmulateFilmPrintParams = parse_params(params)?;
                if !SUPPORTED_PRINT_STOCKS.contains(&params.stock.as_str()) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "stock \"{}\" is not available: this packet ships only the parameterised {SUPPORTED_PRINT_STOCKS:?} response; measured print-stock LUTs are a later packet",
                        params.stock
                    )));
                }
                for (name, v, (lo, hi)) in [
                    ("halation", params.halation, (0.0, 1.0)),
                    ("grain", params.grain, (0.0, 1.0)),
                    ("weave", params.weave, (0.0, 2.0)),
                ] {
                    if !v.is_finite() || !(lo..=hi).contains(&v) {
                        return Err(DispatchError::InvalidParameters(format!(
                            "{name} must be in [{lo}, {hi}], got {v}"
                        )));
                    }
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index]
                    .clips
                    .get(params.clip_index)
                    .is_none()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let payload = json!({
                    "stock": params.stock,
                    "halation": params.halation,
                    "grain": params.grain,
                    "weave_mm": params.weave,
                });
                let response = json!({
                    "stock": params.stock,
                    "halation": params.halation,
                    "grain": params.grain,
                    "weaveMm": params.weave,
                });
                if let Some(node) = clip
                    .color_nodes
                    .iter_mut()
                    .find(|node| node.kind == ColorNodeKind::FilmPrintEmulation)
                {
                    node.payload = payload;
                    let node_id = node.id;
                    let mut merged = response;
                    merged["nodeId"] = json!(node_id);
                    Ok(merged)
                } else {
                    let node_id = self.document.project.next_id;
                    self.document.project.next_id = node_id.saturating_add(1);
                    clip.color_nodes.push(ColorNode {
                        id: node_id,
                        kind: ColorNodeKind::FilmPrintEmulation,
                        enabled: true,
                        payload,
                        windows: Vec::new(),
                        origin: None,
                    });
                    let mut merged = response;
                    merged["nodeId"] = json!(node_id);
                    Ok(merged)
                }
            })()),
            "color.match_shot_to_reference" => Some((|| {
                let params: MatchShotToReferenceParams = parse_params(params)?;
                // The node lands on the matched (src) clip and records the
                // reference as provenance; both anchors must exist so the
                // response never points at shots that do not.
                let src_index = find_track_index(&self.document, params.src_track_id)?;
                if self.document.project.sequence.tracks[src_index]
                    .clips
                    .get(params.src_clip_index)
                    .is_none()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                let ref_index = find_track_index(&self.document, params.ref_track_id)?;
                if self.document.project.sequence.tracks[ref_index]
                    .clips
                    .get(params.ref_clip_index)
                    .is_none()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Reference clip does not exist".into(),
                    ));
                }
                // The fitted payload passes the exact validation
                // `color.node_update` enforces before anything is stored.
                let canonical = validate_payload(ColorNodeKind::PrimaryWheels, &params.wheels)
                    .map_err(DispatchError::InvalidParameters)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[src_index].clips
                    [params.src_clip_index];
                let existing = clip
                    .color_nodes
                    .iter_mut()
                    .find(|node| {
                        node.kind == ColorNodeKind::PrimaryWheels
                            && node.origin.as_deref() == Some(SHOT_MATCH_ORIGIN)
                    })
                    .map(|node| node.id);
                let (node_id, created) = if let Some(id) = existing {
                    let node = clip
                        .color_nodes
                        .iter_mut()
                        .find(|node| node.id == id)
                        .expect("shot-match node existed in the pre-checkpoint lookup");
                    node.payload = canonical.clone();
                    (id, false)
                } else {
                    let node_id = self.document.project.next_id;
                    self.document.project.next_id = node_id.saturating_add(1);
                    clip.color_nodes.push(ColorNode {
                        id: node_id,
                        kind: ColorNodeKind::PrimaryWheels,
                        enabled: true,
                        payload: canonical.clone(),
                        windows: Vec::new(),
                        origin: Some(SHOT_MATCH_ORIGIN.into()),
                    });
                    (node_id, true)
                };
                Ok(json!({
                    "nodeId": node_id,
                    "origin": SHOT_MATCH_ORIGIN,
                    "created": created,
                    "applied": canonical,
                    "reference": {
                        "trackId": params.ref_track_id,
                        "clipIndex": params.ref_clip_index,
                    },
                    "lockSkinTones": params.lock_skin_tones.unwrap_or(true),
                }))
            })()),
            _ => None,
        }
    }
}
