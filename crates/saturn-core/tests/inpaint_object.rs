// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 6.1 — object removal command.

use saturn_core::audio_edit::audio_clip;
use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Inpaint");
    let mut clip = audio_clip(1, Tick(0), Tick(100));
    clip.id = 1;
    document.project.sequence.tracks[0].clips.push(clip);
    document
}

#[test]
fn inpaint_command_persists_settings_and_rejects_bad_regions() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load");
    let result = engine
        .execute(CommandRequest::new(
            "vfx.inpaint_object_removal",
            json!({"track_id": 1, "clip_index": 0, "region": [0.2, 0.2, 0.4, 0.4], "search_frames": 4}),
        ))
        .expect("configure");
    assert_eq!(result["clip_id"], 1);
    assert_eq!(engine.document().vfx_inpaints.len(), 1);

    let rejected = engine.execute(CommandRequest::new(
        "vfx.inpaint_object_removal",
        json!({"track_id": 1, "clip_index": 0, "region": [0.9, 0.9, 0.4, 0.4], "search_frames": 4}),
    ));
    assert!(rejected.is_err());
}
