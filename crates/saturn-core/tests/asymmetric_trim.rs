// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_core::asymmetric_trim::{AsymmetricTrimParams, plan};
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use serde_json::json;

fn t(n: f64) -> Tick {
    Tick((n * TICKS_PER_SECOND as f64).round() as i64)
}
fn fixture() -> ProjectDocument {
    let mut doc = ProjectDocument::new("J/L edit");
    for id in [3, 4] {
        doc.project.media.push(MediaAsset {
            id,
            kind: MediaKind::Video,
            path: format!("/source-{id}.mkv").into(),
            duration: Some(t(10.0)),
            audio_channels: Some(2),
            ..Default::default()
        });
    }
    let mut left = audio_clip(3, t(2.0), t(2.0));
    left.source_in = t(1.0);
    let mut right = audio_clip(4, t(4.0), t(2.0));
    right.source_in = t(3.0);
    doc.project.sequence.tracks[0].clips = vec![left, right];
    doc.project.next_id = 5;
    doc
}
fn params(audio: f64) -> AsymmetricTrimParams {
    AsymmetricTrimParams {
        track_id: 1,
        clip_index: 1,
        video_delta: t(0.25),
        audio_delta: t(audio),
        fade_ticks: t(0.2),
    }
}

#[test]
fn j_and_l_cuts_preserve_source_sync_outer_edges_and_picture_state() {
    for audio in [-0.5, 0.5] {
        let mut doc = fixture();
        doc.project.sequence.tracks[0].clips[0].color.exposure = 0.5;
        let result = plan(&doc, &params(audio)).unwrap();
        assert_eq!(result.video_cut, t(4.25));
        assert_eq!(result.audio_cut, t(4.0 + audio));
        assert_eq!(result.left_picture.timeline_start, t(2.0));
        assert_eq!(result.left_picture.duration, t(2.25));
        assert_eq!(result.right_picture.source_in, t(3.25));
        assert_eq!(result.right_picture.duration, t(1.75));
        assert_eq!(result.left_picture.color.exposure, 0.5);
        assert!(!result.left_picture.audio_enabled && !result.right_picture.audio_enabled);
        assert!(result.left_picture.enabled && result.right_picture.enabled);
        assert_eq!(result.right_audio.source_in, t(3.0 + audio - 0.1));
        assert_eq!(result.left_audio.duration, t(2.0 + audio + 0.1));
        assert_eq!(result.duration, t(4.0));
    }
    // Timeline adjacency, not vector ordering, selects the predecessor.
    let mut doc = fixture();
    doc.project.sequence.tracks[0].clips.swap(0, 1);
    let mut p = params(-0.5);
    p.clip_index = 0;
    assert_eq!(plan(&doc, &p).unwrap().left_index, 1);
}

#[test]
fn rejects_locks_unsupported_state_missing_handles_ambiguity_and_overflow() {
    let edits: Vec<Box<dyn Fn(&mut ProjectDocument)>> = vec![
        Box::new(|d| d.project.sequence.tracks[0].locked = true),
        Box::new(|d| {
            d.project.sequence.tracks[0].group_id = Some(9);
            d.project.sequence.track_groups =
                serde_json::from_value(json!([{"id":9,"name":"Locked","locked":true}])).unwrap();
        }),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].source_in = t(0.0)),
        Box::new(|d| d.project.media[0].duration = Some(t(3.0))),
        Box::new(|d| d.project.media[1].audio_channels = Some(0)),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].speed = 2.0),
        Box::new(|d| d.project.sequence.tracks[0].clips[0].audio_enabled = false),
        Box::new(|d| d.project.sequence.tracks[0].clips[0].group_id = Some(7)),
        Box::new(|d| d.project.sequence.tracks[0].clips[0].enabled = false),
        Box::new(|d| d.project.sequence.tracks[0].pan = 0.5),
        Box::new(|d| d.project.sequence.tracks[0].clips[0].duration = t(1.9)),
        Box::new(|d| {
            let c = d.project.sequence.tracks[0].clips[0].clone();
            d.project.sequence.tracks[0].clips.push(c);
        }),
        Box::new(|d| {
            d.project.sequence.tracks[0]
                .clips
                .push(audio_clip(3, t(3.0), t(2.0)))
        }),
    ];
    for edit in edits {
        let mut doc = fixture();
        edit(&mut doc);
        assert!(plan(&doc, &params(-0.5)).is_err());
    }
    for (video, audio, fade) in [
        (i64::MAX, 0, 0),
        (0, i64::MIN, 0),
        (0, 0, -1),
        (t(2.0).0, 0, 0),
        (0, t(-2.0).0, 0),
        (0, 0, t(6.0).0),
    ] {
        let mut p = params(0.0);
        p.video_delta = Tick(video);
        p.audio_delta = Tick(audio);
        p.fade_ticks = Tick(fade);
        assert!(plan(&fixture(), &p).is_err());
    }
}

#[test]
fn atomic_history_stale_rejection_and_schema_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("edit.wav");
    std::fs::write(&wav, b"core payload fixture").unwrap();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(fixture(), None).unwrap();
    let before = snapshot_digest(engine.document()).unwrap();
    let mut prepared = serde_json::to_value(params(-0.5)).unwrap();
    prepared["expected_digest"] = json!(before);
    prepared["audio"] = json!({"path":wav,"duration":t(4.0),"channels":2});
    let mut invalid = prepared.clone();
    invalid["audio"]["duration"] = json!(t(1.0));
    assert!(
        engine
            .execute(CommandRequest::new("timeline.asymmetric_trim", invalid))
            .is_err()
    );
    assert_eq!(snapshot_digest(engine.document()).unwrap(), before);
    assert!(
        engine
            .execute(CommandRequest::new("edit.undo", json!({})))
            .is_err()
    );
    engine
        .execute(CommandRequest::new(
            "timeline.asymmetric_trim",
            prepared.clone(),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks.len(), 3);
    let after = snapshot_digest(engine.document()).unwrap();
    assert!(
        engine
            .execute(CommandRequest::new("timeline.asymmetric_trim", prepared))
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), before);
    engine
        .execute(CommandRequest::new("edit.redo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    let path = dir.path().join("edit.saturn");
    engine
        .execute(CommandRequest::new("file.save", json!({"path":path})))
        .unwrap();
    engine
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);

    let mut legacy = serde_json::to_value(fixture()).unwrap();
    legacy["schema_version"] = json!(15);
    for c in legacy["project"]["sequence"]["tracks"][0]["clips"]
        .as_array_mut()
        .unwrap()
    {
        c.as_object_mut().unwrap().remove("audio_enabled");
    }
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    engine
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    assert_eq!(engine.document().schema_version, 16);
    assert!(
        engine.document().project.sequence.tracks[0]
            .clips
            .iter()
            .all(|c| c.audio_enabled)
    );
}
