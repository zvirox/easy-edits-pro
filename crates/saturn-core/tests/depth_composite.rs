// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 6.1 — depth compositor command.

use saturn_core::audio_edit::audio_clip;
use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Depth");
    let mut clip = audio_clip(1, Tick(0), Tick(100));
    clip.id = 1;
    document.project.sequence.tracks[0].clips.push(clip);
    document
}

#[test]
fn depth_composite_command_persists_settings_and_rejects_bad_radii() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load");
    let result = engine
        .execute(CommandRequest::new(
            "vfx.depth_map_compositor",
            json!({"track_id": 1, "clip_index": 0, "mode": "dof_bokeh", "threshold": 0.6, "blur_radius": 8}),
        ))
        .expect("configure");
    assert_eq!(result["clip_id"], 1);
    assert_eq!(engine.document().vfx_depth_composites.len(), 1);
    assert_eq!(engine.document().vfx_depth_composites[0].blur_radius, 8);

    let rejected = engine.execute(CommandRequest::new(
        "vfx.depth_map_compositor",
        json!({"track_id": 1, "clip_index": 0, "mode": "z_threshold", "threshold": 0.5, "blur_radius": 64}),
    ));
    assert!(rejected.is_err());
}
