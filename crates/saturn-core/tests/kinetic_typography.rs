// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 6.2 — kinetic typography command.

use saturn_core::audio_edit::audio_clip;
use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Kinetic");
    let mut clip = audio_clip(1, Tick(0), Tick(200));
    clip.id = 1;
    document.project.sequence.tracks[0].clips.push(clip);
    document
}

#[test]
fn kinetic_command_persists_words_and_rejects_bad_entries() {
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).expect("load");
    let result = engine
        .execute(CommandRequest::new(
            "graphic.kinetic_typography",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "words": [
                    {"text": "Hello", "start_ticks": 0, "end_ticks": 100},
                    {"text": "world", "start_ticks": 100, "end_ticks": 200}
                ],
                "faces": [{"x": 0.1, "y": 0.1, "width": 0.2, "height": 0.2}]
            }),
        ))
        .expect("configure");
    assert_eq!(result["word_count"], 2);
    assert_eq!(result["face_count"], 1);
    assert_eq!(engine.document().vfx_kinetic_typography.len(), 1);

    let rejected = engine.execute(CommandRequest::new(
        "graphic.kinetic_typography",
        json!({
            "track_id": 1,
            "clip_index": 0,
            "words": [{"text": "bad", "start_ticks": 50, "end_ticks": 10}]
        }),
    ));
    assert!(rejected.is_err());
}
