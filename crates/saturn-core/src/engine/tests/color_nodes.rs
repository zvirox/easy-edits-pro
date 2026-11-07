// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 3.2 colour-node command tests: chain mutation, strict payload
//! validation, power-window rules, and honest refusals.

use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use crate::project::MediaKind;
use serde_json::json;

fn engine_with_clip() -> (EditorEngine, u64) {
    let mut engine = EditorEngine::default();
    let response = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mov", "path": "/a.mov", "kind": MediaKind::Video}),
        ))
        .unwrap();
    let media_id = response["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id, "track_id": 1, "timeline_start": 0}),
        ))
        .unwrap();
    (engine, media_id)
}

fn clip<'a>(engine: &'a EditorEngine) -> &'a crate::project::Clip {
    &engine.document().project.sequence.tracks[0].clips[0]
}

#[test]
fn node_add_appends_a_neutral_chain_node() {
    let (mut engine, _) = engine_with_clip();
    let response = engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "primary_wheels"}),
        ))
        .unwrap();
    let node_id = response["nodeId"].as_u64().unwrap();
    let nodes = &clip(&engine).color_nodes;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, node_id);
    assert_eq!(nodes[0].kind, crate::project::ColorNodeKind::PrimaryWheels);
    // The default payload must be neutral, not a hidden correction.
    assert_eq!(nodes[0].payload["exposure_stops"], 0.0);
    assert_eq!(nodes[0].payload["gamma"], 0.0);
}

#[test]
fn node_add_rejects_dag_branching_with_typed_error() {
    let (mut engine, _) = engine_with_clip();
    let error = engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "hdr_zones", "inputs": [7]}),
        ))
        .unwrap_err();
    assert!(
        error.to_string().contains("branching is not supported"),
        "{error}"
    );
    assert!(clip(&engine).color_nodes.is_empty());
}

#[test]
fn node_update_stores_strict_validated_payload() {
    let (mut engine, _) = engine_with_clip();
    let node_id = engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "primary_wheels"}),
        ))
        .unwrap()["nodeId"]
        .as_u64()
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "payload": {"exposure_stops": 3.0}}),
        ))
        .unwrap();
    assert_eq!(clip(&engine).color_nodes[0].payload["exposure_stops"], 3.0);

    // Out of range → typed error, payload untouched.
    let error = engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "payload": {"exposure_stops": 99.0}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("outside"), "{error}");
    assert_eq!(clip(&engine).color_nodes[0].payload["exposure_stops"], 3.0);

    // Unknown field → typed error (strict wire).
    let error = engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "payload": {"secret_gain": 1.0}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("unknown field"), "{error}");

    // Unknown node → typed error.
    let error = engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": 424242,
                   "payload": {}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn node_update_validates_against_the_node_kind() {
    let (mut engine, _) = engine_with_clip();
    let lut_id = engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "lut_3d"}),
        ))
        .unwrap()["nodeId"]
        .as_u64()
        .unwrap();
    // A valid wheels payload is NOT a valid lut payload.
    let error = engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": lut_id,
                   "payload": {"exposure_stops": 1.0}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("unknown field"), "{error}");

    // A wrongly-sized LUT grid is rejected with the expected byte count.
    let error = engine
        .execute(CommandRequest::new(
            "color.node_update",
            json!({"track_id": 1, "clip_index": 0, "node_id": lut_id,
                   "payload": {"size": 2, "data_b64": "AAAAAAAA"}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("decodes to"), "{error}");
}

#[test]
fn power_window_attaches_and_rejects_auto_track() {
    let (mut engine, _) = engine_with_clip();
    let node_id = engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "primary_wheels"}),
        ))
        .unwrap()["nodeId"]
        .as_u64()
        .unwrap();

    // auto_track must refuse loudly — never fake a static-only success.
    let error = engine
        .execute(CommandRequest::new(
            "color.add_power_window",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "mask": {"shape": {"ellipse": {"center": [0.5, 0.5], "half_size": [0.2, 0.3]}}},
                   "auto_track": true}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("auto_track"), "{error}");
    assert!(clip(&engine).color_nodes[0].windows.is_empty());

    let response = engine
        .execute(CommandRequest::new(
            "color.add_power_window",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "mask": {"shape": {"ellipse": {"center": [0.5, 0.5], "half_size": [0.2, 0.3]}},
                            "softness": 0.1, "opacity": 0.8}}),
        ))
        .unwrap();
    assert!(response["windowId"].as_u64().is_some());
    let windows = &clip(&engine).color_nodes[0].windows;
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].opacity, 0.8);

    // Invalid geometry → typed error, nothing stored.
    let error = engine
        .execute(CommandRequest::new(
            "color.add_power_window",
            json!({"track_id": 1, "clip_index": 0, "node_id": node_id,
                   "mask": {"shape": {"polygon": {"points": [[0.1, 0.1], [0.9, 0.9]]}}}}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("at least 3"), "{error}");
    assert_eq!(clip(&engine).color_nodes[0].windows.len(), 1);
}

#[test]
fn emulate_film_print_creates_then_updates_one_node() {
    let (mut engine, _) = engine_with_clip();
    let first = engine
        .execute(CommandRequest::new(
            "color.emulate_film_print",
            json!({"track_id": 1, "clip_index": 0, "stock": "generic",
                   "halation": 0.4, "grain": 0.3, "weave": 0.5}),
        ))
        .unwrap();
    let node_id = first["nodeId"].as_u64().unwrap();
    assert_eq!(clip(&engine).color_nodes.len(), 1);
    assert_eq!(clip(&engine).color_nodes[0].payload["weave_mm"], 0.5);

    // A second call updates the same node (one film node per clip).
    let second = engine
        .execute(CommandRequest::new(
            "color.emulate_film_print",
            json!({"track_id": 1, "clip_index": 0, "stock": "generic",
                   "halation": 0.9, "grain": 0.1, "weave": 0.0}),
        ))
        .unwrap();
    assert_eq!(second["nodeId"].as_u64().unwrap(), node_id);
    assert_eq!(clip(&engine).color_nodes.len(), 1);
    assert_eq!(clip(&engine).color_nodes[0].payload["halation"], 0.9);
}

#[test]
fn emulate_film_print_refuses_unshipped_stocks_and_bad_ranges() {
    let (mut engine, _) = engine_with_clip();
    let error = engine
        .execute(CommandRequest::new(
            "color.emulate_film_print",
            json!({"track_id": 1, "clip_index": 0, "stock": "kodak_2383",
                   "halation": 0.4, "grain": 0.3, "weave": 0.5}),
        ))
        .unwrap_err();
    assert!(
        error.to_string().contains("not available"),
        "named stocks without measured data must refuse: {error}"
    );
    let error = engine
        .execute(CommandRequest::new(
            "color.emulate_film_print",
            json!({"track_id": 1, "clip_index": 0, "stock": "generic",
                   "halation": 4.0, "grain": 0.3, "weave": 0.5}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("halation"), "{error}");
    assert!(clip(&engine).color_nodes.is_empty());
}

#[test]
fn color_nodes_persist_through_project_round_trip() {
    let (mut engine, _) = engine_with_clip();
    engine
        .execute(CommandRequest::new(
            "color.node_add",
            json!({"track_id": 1, "clip_index": 0, "kind": "rgb_curves"}),
        ))
        .unwrap();
    let document = &engine.document();
    let text = serde_json::to_string(document).unwrap();
    let back: crate::project::ProjectDocument = serde_json::from_str(&text).unwrap();
    assert_eq!(
        back.project.sequence.tracks[0].clips[0].color_nodes.len(),
        1,
        "older schema versions load because the field is serde(default)"
    );
}

#[test]
fn match_shot_appends_a_provenance_tagged_node() {
    let (mut engine, _) = engine_with_clip();
    // A second media asset + clip acts as the reference hero shot.
    let response = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "b.mov", "path": "/b.mov", "kind": crate::project::MediaKind::Video}),
        ))
        .unwrap();
    let ref_media = response["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": ref_media, "track_id": 1, "timeline_start": 0}),
        ))
        .unwrap();
    let applied = engine
        .execute(CommandRequest::new(
            "color.match_shot_to_reference",
            json!({
                "src_track_id": 1, "src_clip_index": 0,
                "ref_track_id": 1, "ref_clip_index": 1,
                "wheels": {"exposure_stops": 0.5,
                           "gain": {"red": 0.05, "green": 0.0, "blue": -0.05, "luma": 0.0}},
            }),
        ))
        .unwrap();
    assert_eq!(applied["origin"], "shot-match");
    assert_eq!(applied["created"], true);
    assert_eq!(applied["reference"]["clipIndex"], 1);
    let node_id = applied["nodeId"].as_u64().unwrap();
    {
        let node = &clip(&engine).color_nodes[0];
        assert_eq!(node.id, node_id);
        assert_eq!(node.origin.as_deref(), Some("shot-match"));
        assert_eq!(node.kind, crate::project::ColorNodeKind::PrimaryWheels);
        assert_eq!(
            node.payload["exposure_stops"], 0.5,
            "the applied payload is the canonical validated one"
        );
    }

    // Re-running updates the machine node instead of stacking a second.
    let applied = engine
        .execute(CommandRequest::new(
            "color.match_shot_to_reference",
            json!({
                "src_track_id": 1, "src_clip_index": 0,
                "ref_track_id": 1, "ref_clip_index": 1,
                "wheels": {"exposure_stops": 1.0},
            }),
        ))
        .unwrap();
    assert_eq!(applied["created"], false);
    assert_eq!(applied["nodeId"], json!(node_id));
    assert_eq!(clip(&engine).color_nodes.len(), 1);
    assert_eq!(clip(&engine).color_nodes[0].payload["exposure_stops"], 1.0);
}

#[test]
fn match_shot_rejects_bad_payloads_and_missing_anchors() {
    let (mut engine, _) = engine_with_clip();
    let error = engine
        .execute(CommandRequest::new(
            "color.match_shot_to_reference",
            json!({
                "src_track_id": 1, "src_clip_index": 0,
                "ref_track_id": 1, "ref_clip_index": 4,
                "wheels": {"exposure_stops": 0.5},
            }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("Reference clip"), "{error}");
    assert!(clip(&engine).color_nodes.is_empty(), "no node on a failed match");

    let error = engine
        .execute(CommandRequest::new(
            "color.match_shot_to_reference",
            json!({
                "src_track_id": 1, "src_clip_index": 0,
                "ref_track_id": 1, "ref_clip_index": 0,
                "wheels": {"exposure_stops": 999.0},
            }),
        ))
        .unwrap_err();
    assert!(
        error.to_string().contains("999 outside [-10, 10]"),
        "{error}"
    );
    assert!(
        clip(&engine).color_nodes.is_empty(),
        "an unvalidatable payload never becomes a node"
    );
}
