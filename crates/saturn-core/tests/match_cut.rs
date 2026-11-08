// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::match_cut::{MatchCutMode, MatchCutParams, plan, validate_choice};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use serde_json::json;

fn t(n: i64) -> Tick {
    Tick(n * TICKS_PER_SECOND)
}
fn fixture() -> ProjectDocument {
    let mut d = ProjectDocument::new("Match cut");
    for id in [3, 4] {
        d.project.media.push(MediaAsset {
            id,
            path: format!("/source-{id}.mkv").into(),
            kind: MediaKind::Video,
            duration: Some(t(10)),
            audio_channels: Some(2),
            ..Default::default()
        });
    }
    let left = audio_clip(3, t(0), t(1));
    let mut right = audio_clip(4, t(1), t(1));
    right.source_in = t(1);
    d.project.sequence.tracks[0].clips = vec![left, right];
    d.project.next_id = 5;
    d
}
fn params(mode: MatchCutMode) -> MatchCutParams {
    MatchCutParams {
        track_id: 1,
        left_clip_index: 0,
        right_clip_index: 1,
        mode,
        search_frames: 12,
    }
}
fn request(d: &ProjectDocument, mode: MatchCutMode, source: Tick) -> CommandRequest {
    let mut p = serde_json::to_value(params(mode)).unwrap();
    p["expected_digest"] = json!(snapshot_digest(d).unwrap());
    p["matched_source_in"] = json!(source);
    CommandRequest::new("edit.match_cut_align", p)
}

#[test]
fn bounded_handles_frame_steps_audio_only_and_unordered_pair() {
    let mut d = fixture();
    d.project.sequence.tracks[0].clips[1].source_in = Tick(1);
    let p = params(MatchCutMode::EyeTrace);
    let result = plan(&d, &p).unwrap();
    assert_eq!(result.min_source_in, Tick(0));
    assert!(validate_choice(&result, p.mode, Tick(0)).is_err());
    assert!(validate_choice(&result, p.mode, Tick(1 + result.frame_ticks)).is_ok());
    d.project.sequence.tracks[0].clips[1].source_in = t(9);
    assert_eq!(plan(&d, &p).unwrap().max_source_in, t(9));
    d.project.sequence.tracks[0].clips.swap(0, 1);
    let mut p = p;
    p.left_clip_index = 1;
    p.right_clip_index = 0;
    assert!(plan(&d, &p).is_ok());
    d.project.sequence.tracks[0].kind = saturn_core::TrackKind::Audio;
    for m in &mut d.project.media {
        m.kind = MediaKind::Audio;
    }
    assert!(plan(&d, &p).is_err());
    p.mode = MatchCutMode::Transient;
    assert!(plan(&d, &p).is_ok());
}

#[test]
fn unsupported_state_and_bad_ranges_are_rejected_before_editing() {
    let mutations: Vec<Box<dyn Fn(&mut ProjectDocument)>> = vec![
        Box::new(|d| d.project.sequence.tracks[0].locked = true),
        Box::new(|d| {
            d.project.sequence.tracks[0].group_id = Some(9);
            d.project.sequence.track_groups =
                serde_json::from_value(json!([{"id":9,"name":"Locked","locked":true}])).unwrap();
        }),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].speed = 2.0),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].audio_enabled = false),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].enabled = false),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].group_id = Some(9)),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].source_in = t(10)),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].timeline_start = t(2)),
        Box::new(|d| d.project.media[0].duration = None),
        Box::new(|d| {
            let c = d.project.sequence.tracks[0].clips[0].clone();
            d.project.sequence.tracks[0].clips.push(c);
        }),
    ];
    for mutate in mutations {
        let mut d = fixture();
        mutate(&mut d);
        assert!(plan(&d, &params(MatchCutMode::EyeTrace)).is_err());
    }
    for n in [0, 31, u32::MAX] {
        let mut p = params(MatchCutMode::Motion);
        p.search_frames = n;
        assert!(plan(&fixture(), &p).is_err());
    }
    let mut d = fixture();
    d.project.media[0].audio_channels = None;
    assert!(plan(&d, &params(MatchCutMode::Transient)).is_err());
}

#[test]
fn atomic_slip_history_noop_stale_and_persistence() {
    for mode in [
        MatchCutMode::EyeTrace,
        MatchCutMode::Motion,
        MatchCutMode::Transient,
    ] {
        let mut engine = EditorEngine::default();
        let d = fixture();
        engine.restore_recovery(d.clone(), None).unwrap();
        let before = snapshot_digest(&d).unwrap();
        let source = Tick(t(1).0 + d.project.settings.frame_rate.frame_ticks().unwrap() * 3);
        let prepared = request(&d, mode, source);
        let mut invalid = prepared.clone();
        invalid.params["matched_source_in"] = json!(t(20));
        assert!(engine.execute(invalid).is_err());
        let noop = request(&d, mode, t(1));
        assert_eq!(engine.execute(noop.clone()).unwrap()["changed"], false);
        assert!(
            engine
                .execute(CommandRequest::new("edit.undo", json!({})))
                .is_err()
        );
        assert_eq!(engine.execute(prepared.clone()).unwrap()["changed"], true);
        let mut expected = d.clone();
        expected.project.sequence.tracks[0].clips[1].source_in = source;
        let after = snapshot_digest(&expected).unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
        assert!(
            engine
                .execute(prepared)
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        engine
            .execute(CommandRequest::new("edit.undo", json!({})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), before);
        engine.execute(noop).unwrap(); // no-op must preserve redo
        engine
            .execute(CommandRequest::new("edit.redo", json!({})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cut.saturn");
        engine
            .execute(CommandRequest::new("file.save", json!({"path":path})))
            .unwrap();
        engine
            .execute(CommandRequest::new("file.open", json!({"path":path})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    }
}
