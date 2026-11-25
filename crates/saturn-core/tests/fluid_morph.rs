// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 5.1 — bounded motion-compensated morph cut.

use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::{CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, Tick};
use serde_json::json;

fn fixture() -> ProjectDocument {
    let mut document = ProjectDocument::new("Morph cut");
    for id in [1u64, 2] {
        document.project.media.push(MediaAsset {
            id,
            path: format!("/source-{id}").into(),
            kind: MediaKind::Video,
            duration: Some(Tick(600)),
            ..Default::default()
        });
    }
    let mut outgoing = audio_clip(1, Tick(0), Tick(100));
    outgoing.id = 1;
    let mut incoming = audio_clip(2, Tick(100), Tick(100));
    incoming.id = 2;
    document.project.sequence.tracks[0].clips = vec![outgoing, incoming];
    document.project.next_id = 3;
    document
}

fn request(document: &ProjectDocument) -> CommandRequest {
    CommandRequest::new(
        "timeline.fluid_morph_cut",
        json!({
            "track_id": 1,
            "cut_at": 100,
            "duration": 40,
            "measurement": {
                "dx_px": 4.0, "dy_px": 0.0, "confidence": 0.9,
                "ticks_per_px": 2.0, "audio_crossfade_ticks": 20
            },
            "expected_digest": snapshot_digest(document).expect("digest"),
        }),
    )
}

#[test]
fn morph_creates_an_overlapped_crossfade_and_motion_slip() {
    let document = fixture();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(document.clone(), None).expect("load");
    let result = engine.execute(request(&document)).expect("morph cut");
    assert_eq!(result["duration"], 40);
    assert_eq!(result["overlap_each_side"], 20);
    assert_eq!(result["motion_slip_ticks"], 8);
    assert_eq!(result["audio_crossfade_ticks"], 20);
    let track = &engine.document().project.sequence.tracks[0];
    assert_eq!(track.transitions.len(), 1);
    assert_eq!(track.clips[0].duration, Tick(120));
    assert_eq!(track.clips[1].timeline_start, Tick(80));
}

#[test]
fn stale_measurement_is_rejected() {
    let document = fixture();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(document.clone(), None).expect("load");
    let mut request = request(&document);
    request.params["expected_digest"] = json!("stale");
    assert!(engine.execute(request).is_err());
}

#[test]
fn excessive_displacement_is_rejected() {
    let document = fixture();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(document.clone(), None).expect("load");
    let mut request = request(&document);
    request.params["measurement"]["dx_px"] = json!(1000.0);
    assert!(engine.execute(request).is_err());
}
