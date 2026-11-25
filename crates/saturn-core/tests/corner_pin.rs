// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 6.1 — planar corner pin command.

use saturn_core::audio_edit::audio_clip;
use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Corner pin");
    let mut clip = audio_clip(1, Tick(0), Tick(100));
    clip.id = 1;
    document.project.sequence.tracks[0].clips.push(clip);
    document
}

#[test]
fn corner_pin_command_persists_and_rejects_bad_quads() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load");
    let corners = [[0.1, 0.1], [0.9, 0.2], [0.8, 0.9], [0.2, 0.8]];
    let result = engine
        .execute(CommandRequest::new(
            "vfx.planar_corner_pin",
            json!({"track_id": 1, "clip_index": 0, "corners": corners, "motion_blur": true}),
        ))
        .expect("corner pin");
    assert_eq!(result["clip_id"], 1);
    assert_eq!(engine.document().vfx_corner_pins.len(), 1);
    assert_eq!(engine.document().vfx_corner_pins[0].corners, corners);

    let bad = engine.execute(CommandRequest::new(
        "vfx.planar_corner_pin",
        json!({"track_id": 1, "clip_index": 0, "corners": [[0.0,0.0],[1.0,0.0],[1.0,0.0],[0.0,0.0]]}),
    ));
    assert!(bad.is_err());
}
