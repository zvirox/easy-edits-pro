// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::beat_grid::{
    BeatGridParams, MeasuredBeatGrid, Subdivision, plan, quantized_clips,
};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use serde_json::json;

fn f(n: i64) -> Tick {
    Tick(n * (TICKS_PER_SECOND / 30))
}
fn fixture(boundaries: &[i64]) -> ProjectDocument {
    let mut d = ProjectDocument::new("Beat grid");
    for (id, kind) in [(3, MediaKind::Video), (4, MediaKind::Audio)] {
        d.project.media.push(MediaAsset {
            id,
            path: format!("/source-{id}").into(),
            kind,
            duration: Some(f(1800)),
            audio_channels: Some(2),
            ..Default::default()
        });
    }
    d.project.sequence.tracks[0].clips = boundaries
        .windows(2)
        .map(|pair| {
            let mut c = audio_clip(3, f(pair[0]), f(pair[1] - pair[0]));
            c.source_in = f(300 + pair[0]);
            c
        })
        .collect();
    d.project.sequence.tracks[1].clips = vec![audio_clip(4, f(0), f(360))];
    d.project.next_id = 5;
    d
}
fn params(subdivision: Subdivision, strength: f64) -> BeatGridParams {
    BeatGridParams {
        video_track_id: 1,
        music_track_id: 2,
        subdivision,
        strength,
        downbeat_tick: (subdivision != Subdivision::Quarter).then_some(f(15)),
    }
}
fn grid() -> MeasuredBeatGrid {
    MeasuredBeatGrid {
        origin: f(15),
        period: f(15),
        beats: 16,
    }
}
fn request(d: &ProjectDocument, p: &BeatGridParams, grid: &MeasuredBeatGrid) -> CommandRequest {
    let mut params = serde_json::to_value(p).unwrap();
    params["grid"] = json!(grid);
    params["expected_digest"] = json!(snapshot_digest(d).unwrap());
    CommandRequest::new("timeline.beat_grid_quantize", params)
}

#[test]
fn all_subdivisions_partial_strength_and_original_source_mapping() {
    let d = fixture(&[30, 82, 139, 210]);
    for (subdivision, expected) in [
        (Subdivision::Quarter, [75, 135]),
        (Subdivision::Half, [75, 135]),
        (Subdivision::Bar, [75, 135]),
    ] {
        for (strength, cuts) in [(1.0, expected), (0.5, [78, 137]), (0.0, [82, 139])] {
            let (_, clips, shifts) =
                quantized_clips(&d, &params(subdivision, strength), &grid()).unwrap();
            assert_eq!(shifts, vec![(f(82), f(cuts[0])), (f(139), f(cuts[1]))]);
            assert_eq!(clips[0].timeline_start, f(30));
            assert_eq!(clips[2].timeline_start.0 + clips[2].duration.0, f(210).0);
            for (old, new) in d.project.sequence.tracks[0].clips.iter().zip(&clips) {
                assert_eq!(
                    old.source_in.0 - old.timeline_start.0,
                    new.source_in.0 - new.timeline_start.0
                );
            }
        }
    }
    // A supplied downbeat sets bar/half phase, including grid beats before it.
    let d = fixture(&[30, 100, 161, 240]);
    let expected = [
        (Subdivision::Quarter, [105, 165]),
        (Subdivision::Half, [105, 165]),
        (Subdivision::Bar, [75, 135]),
    ];
    for (subdivision, cuts) in expected {
        let (_, _, shifts) = quantized_clips(&d, &params(subdivision, 1.0), &grid()).unwrap();
        assert_eq!(shifts, vec![(f(100), f(cuts[0])), (f(161), f(cuts[1]))]);
    }
    let mut p = params(Subdivision::Bar, 1.0);
    p.downbeat_tick = Some(f(105));
    let (_, _, shifts) = quantized_clips(&d, &p, &grid()).unwrap();
    assert_eq!(shifts, vec![(f(100), f(105)), (f(161), f(165))]);
}

#[test]
fn ties_prefer_earlier_grid_and_ntsc_rounding_stays_on_frames() {
    let d = fixture(&[30, 90, 210]);
    let (_, _, cuts) = quantized_clips(&d, &params(Subdivision::Half, 1.0), &grid()).unwrap();
    assert_eq!(cuts, vec![(f(90), f(75))]);
    let mut d = fixture(&[30, 82, 139, 210]);
    d.project.settings.frame_rate = saturn_core::FrameRate::new(30000, 1001).unwrap();
    let frame = d.project.settings.frame_rate.frame_ticks().unwrap();
    for c in &mut d.project.sequence.tracks[0].clips {
        c.timeline_start = Tick(c.timeline_start.0 / f(1).0 * frame);
        c.duration = Tick(c.duration.0 / f(1).0 * frame);
    }
    let (_, _, cuts) = quantized_clips(&d, &params(Subdivision::Quarter, 1.0), &grid()).unwrap();
    assert_eq!(cuts[0].1, Tick(75 * frame));
    assert_eq!(cuts[1].1, Tick(135 * frame));
}

#[test]
fn atomic_roll_history_noop_stale_and_persistence() {
    let d = fixture(&[30, 82, 139, 210]);
    let mut engine = EditorEngine::default();
    engine.restore_recovery(d.clone(), None).unwrap();
    let p = params(Subdivision::Quarter, 1.0);
    let edit = request(&d, &p, &grid());
    let noop = request(&d, &params(Subdivision::Quarter, 0.0), &grid());
    assert_eq!(engine.execute(noop.clone()).unwrap()["changed_cuts"], 0);
    assert!(
        engine
            .execute(CommandRequest::new("edit.undo", json!({})))
            .is_err()
    );
    assert_eq!(engine.execute(edit.clone()).unwrap()["changed_cuts"], 2);
    let after = snapshot_digest(engine.document()).unwrap();
    assert!(
        engine
            .execute(edit)
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(
        snapshot_digest(engine.document()).unwrap(),
        snapshot_digest(&d).unwrap()
    );
    engine.execute(noop).unwrap();
    engine
        .execute(CommandRequest::new("edit.redo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("beat.saturn");
    engine
        .execute(CommandRequest::new("file.save", json!({"path":path})))
        .unwrap();
    engine
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
}

#[test]
fn collisions_late_handle_failures_and_forged_grids_reject_whole_edit() {
    let p = params(Subdivision::Quarter, 1.0);
    let mut handles = fixture(&[30, 82, 139, 210]);
    handles.project.sequence.tracks[0].clips[2].source_in = Tick(0);
    for d in [fixture(&[30, 81, 82, 210]), handles] {
        let mut engine = EditorEngine::default();
        engine.restore_recovery(d.clone(), None).unwrap();
        assert!(engine.execute(request(&d, &p, &grid())).is_err());
        assert_eq!(
            snapshot_digest(engine.document()).unwrap(),
            snapshot_digest(&d).unwrap()
        );
        assert!(
            engine
                .execute(CommandRequest::new("edit.undo", json!({})))
                .is_err()
        );
    }
    let d = fixture(&[30, 82, 139, 210]);
    for grid in [
        MeasuredBeatGrid {
            origin: f(0),
            period: f(15),
            beats: 5,
        },
        MeasuredBeatGrid {
            origin: f(0),
            period: Tick(0),
            beats: 20,
        },
        MeasuredBeatGrid {
            origin: f(150),
            period: f(15),
            beats: 6,
        },
        MeasuredBeatGrid {
            origin: Tick(i64::MAX),
            period: f(15),
            beats: 20,
        },
    ] {
        assert!(quantized_clips(&d, &p, &grid).is_err());
    }
    let mut p = params(Subdivision::Bar, 1.0);
    p.downbeat_tick = None;
    assert!(plan(&d, &p).is_err());
    p.downbeat_tick = Some(f(20));
    assert!(quantized_clips(&d, &p, &grid()).is_err());
}

#[test]
fn unsupported_tracks_clips_and_parameters_are_rejected() {
    let mutations: Vec<Box<dyn Fn(&mut ProjectDocument)>> = vec![
        Box::new(|d| d.project.sequence.tracks[0].locked = true),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].speed = 2.0),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].audio_enabled = false),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].timeline_start = f(83)),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].timeline_start = Tick(f(82).0 + 1)),
        Box::new(|d| d.project.sequence.tracks[0].clips[1].group_id = Some(9)),
        Box::new(|d| d.project.sequence.tracks[1].clips[0].duration = f(100)),
        Box::new(|d| d.project.media[1].audio_channels = Some(6)),
        Box::new(|d| d.project.media[0].duration = None),
        Box::new(|d| {
            d.project.sequence.tracks[0].group_id = Some(9);
            d.project.sequence.track_groups =
                serde_json::from_value(json!([{"id":9,"name":"Locked","locked":true}])).unwrap();
        }),
    ];
    for mutate in mutations {
        let mut d = fixture(&[30, 82, 139, 210]);
        mutate(&mut d);
        assert!(plan(&d, &params(Subdivision::Quarter, 1.0)).is_err());
    }
    for strength in [-0.1, 1.1, f64::NAN] {
        assert!(
            plan(
                &fixture(&[30, 82, 139, 210]),
                &params(Subdivision::Quarter, strength)
            )
            .is_err()
        );
    }
}
