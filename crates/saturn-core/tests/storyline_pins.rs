// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 5.1 — magnetic storyline pin command and ripple propagation.

use saturn_core::audio_edit::audio_clip;
use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Storyline pins");
    let mut parent = audio_clip(1, Tick(100), Tick(200));
    parent.id = 1;
    let mut child = audio_clip(2, Tick(140), Tick(60));
    child.id = 2;
    document.project.sequence.tracks[0].clips.push(parent);
    if document.project.sequence.tracks.len() < 2 {
        document.project.sequence.tracks.push(
            serde_json::from_value(json!({
                "id": 2, "kind": "video", "name": "V2", "clips": [], "transitions": []
            }))
            .expect("track fixture"),
        );
    }
    document.project.sequence.tracks[1].clips.push(child);
    document
}

#[test]
fn pin_command_persists_the_link_and_ripple_moves_the_child() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load fixture");
    let result = engine
        .execute(CommandRequest::new(
            "timeline.pin_clip_to_storyline",
            json!({
                "child_track_id": 2,
                "child_clip_index": 0,
                "parent_track_id": 1,
                "parent_clip_index": 0,
            }),
        ))
        .expect("pin command");
    assert_eq!(result["offset_ticks"], 40);
    assert_eq!(engine.document().storyline_pins.len(), 1);

    // A ripple delete in the parent track shifts the parent; the pinned child
    // follows by the same delta.
    let result = engine
        .execute(CommandRequest::new(
            "timeline.ripple_delete_ranges",
            json!({"ranges": [[0, 25]]}),
        ))
        .expect("ripple delete");
    assert_eq!(result["deleted_ranges_count"], 1);
    assert_eq!(
        engine.document().project.sequence.tracks[1].clips[0].timeline_start,
        Tick(115)
    );
}

#[test]
fn pin_command_rejects_an_invalid_child() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load fixture");
    let result = engine.execute(CommandRequest::new(
        "timeline.pin_clip_to_storyline",
        json!({
            "child_track_id": 2,
            "child_clip_index": 9,
            "parent_track_id": 1,
            "parent_clip_index": 0,
        }),
    ));
    assert!(result.is_err());
}
