// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Real WAV -> preparation -> atomic edit -> undo/redo -> saved project tests.
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
    audio_edit::{audio_clip, snapshot_digest},
};
use saturn_render::{
    audio_edit_driver::prepare, audio_edit_files::write_wav, audio_pcm::decode_audio_pcm,
};
use serde_json::json;
use std::{
    f64::consts::PI,
    path::{Path, PathBuf},
};

fn tick(seconds: f64) -> Tick {
    Tick((seconds * TICKS_PER_SECOND as f64).round() as i64)
}

fn write_source(dir: &Path, name: &str, lag: f64) -> PathBuf {
    let path = dir.join(name);
    let pcm: Vec<_> = (0..96_000)
        .map(|n| {
            let t = (n as f64 - lag) / 48_000.0;
            (1..=96)
                .map(|k| {
                    let f = 170.0 + ((k * 7919) % 7901) as f64;
                    (2.0 * PI * f * t + (k * k) as f64 * 0.31).sin() * 0.0008
                })
                .sum::<f64>() as f32
        })
        .collect();
    write_wav(&mut std::fs::File::create(&path).unwrap(), &pcm, 1, 48_000).unwrap();
    path
}

fn setup(dir: &Path) -> (EditorEngine, PathBuf) {
    let mut doc = ProjectDocument::new("Audio edits fixture");
    let mut target = doc.project.sequence.tracks[1].clone();
    target.id = 3;
    doc.project.sequence.tracks.push(target);
    for (id, name, lag) in [(4, "reference.wav", 0.0), (5, "target.wav", 81.6)] {
        doc.project.media.push(MediaAsset {
            id,
            name: name.into(),
            path: write_source(dir, name, lag),
            kind: MediaKind::Audio,
            duration: Some(tick(2.0)),
            audio_channels: Some(1),
            ..Default::default()
        });
    }
    doc.project.sequence.tracks[1]
        .clips
        .push(audio_clip(4, Tick(0), tick(2.0)));
    doc.project.sequence.tracks[2]
        .clips
        .push(audio_clip(5, Tick(0), tick(2.0)));
    doc.project.next_id = 6;
    let path = dir.join("project.saturn");
    doc.save_atomic(&path).unwrap();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(doc, Some(path.clone())).unwrap();
    (engine, path)
}

fn history_roundtrip(engine: &mut EditorEngine, before: &str, path: &Path) {
    let after = snapshot_digest(engine.document()).unwrap();
    assert_ne!(before, after);
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), before);
    engine
        .execute(CommandRequest::new("edit.redo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    engine
        .execute(CommandRequest::new("file.save", json!({"path": path})))
        .unwrap();
    engine
        .execute(CommandRequest::new("file.open", json!({"path": path})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
    assert!(
        engine
            .document()
            .project
            .media
            .iter()
            .all(|m| m.path.is_file())
    );
}

#[test]
fn phase_edit_roundtrip_stale_rejection_and_rollback_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, path) = setup(dir.path());
    let args = json!({"ref_track_id": 2, "target_track_id": 3});
    let mut edit = prepare(engine.document(), Some(&path), "phase_align", &args).unwrap();
    assert!((edit.telemetry["alignment"]["lagSamples"].as_f64().unwrap() - 81.6).abs() < 0.03);
    let derived: PathBuf =
        serde_json::from_value(edit.command.params["audio"]["path"].clone()).unwrap();
    let before = snapshot_digest(engine.document()).unwrap();
    engine.execute(edit.command.clone()).unwrap();
    edit.commit();
    drop(edit);
    assert!(derived.is_file());
    let ref_pcm = decode_audio_pcm(
        &engine.document().project.media[0].path,
        Tick(0),
        Some(tick(2.0)),
    )
    .unwrap();
    let corrected = decode_audio_pcm(&derived, Tick(0), Some(tick(2.0))).unwrap();
    let mse = ref_pcm.samples[512..80_000]
        .iter()
        .zip(corrected.samples.chunks_exact(2).skip(512))
        .map(|(a, b)| (*a as f64 - b[0] as f64).powi(2))
        .sum::<f64>()
        / (80_000 - 512) as f64;
    assert!(
        10.0 * mse.log10() < -85.0,
        "Aligned WAV residual {} dBFS",
        10.0 * mse.log10()
    );
    history_roundtrip(&mut engine, &before, &path);

    let edit = prepare(engine.document(), Some(&path), "phase_align", &args).unwrap();
    let abandoned: PathBuf =
        serde_json::from_value(edit.command.params["audio"]["path"].clone()).unwrap();
    engine
        .execute(CommandRequest::new(
            "track.set_gain_db",
            json!({"track_id": 2, "gain_db": -3.0}),
        ))
        .unwrap();
    let changed = snapshot_digest(engine.document()).unwrap();
    assert!(
        engine
            .execute(edit.command.clone())
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
    assert_eq!(snapshot_digest(engine.document()).unwrap(), changed);
    drop(edit);
    assert!(!abandoned.exists());
    // Rejection did not checkpoint: undo restores the preceding gain edit.
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[1].gain_db, 0.0);
    assert!(prepare(engine.document(), None, "phase_align", &args).is_err());
}

#[test]
fn room_tone_and_duck_edits_roundtrip_and_reject_existing_automation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, path) = setup(dir.path());
    let mut doc = engine.document().clone();
    doc.project.sequence.tracks[1].clips = vec![
        audio_clip(4, Tick(0), tick(0.5)),
        audio_clip(4, tick(1.5), tick(0.5)),
    ];
    engine.restore_recovery(doc, Some(path.clone())).unwrap();
    let before = snapshot_digest(engine.document()).unwrap();
    let mut edit = prepare(
        engine.document(),
        Some(&path),
        "room_tone",
        &json!({
            "track_id": 2, "room_tone_media_id": 4, "profile_start_tick": 0,
            "profile_end_tick": tick(0.3), "min_gap_ms": 100, "crossfade_ms": 30
        }),
    )
    .unwrap();
    engine.execute(edit.command.clone()).unwrap();
    edit.commit();
    assert_eq!(engine.document().project.sequence.tracks[1].clips.len(), 3);
    history_roundtrip(&mut engine, &before, &path);
    let before = snapshot_digest(engine.document()).unwrap();
    let args = json!({"dialogue_track_id": 2, "music_track_id": 3, "threshold_dbfs": -55.0});
    let mut edit = prepare(engine.document(), Some(&path), "duck", &args).unwrap();
    engine.execute(edit.command.clone()).unwrap();
    edit.commit();
    assert!(
        engine.document().project.sequence.tracks[2]
            .automation
            .volume_points
            .iter()
            .any(|p| p.value == -12.0)
    );
    assert!(
        prepare(engine.document(), Some(&path), "duck", &args)
            .err()
            .expect("Existing automation must be rejected")
            .contains("empty")
    );
    history_roundtrip(&mut engine, &before, &path);
}

#[test]
fn master_bounce_uses_lossless_ges_mix_and_preserves_history() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, path) = setup(dir.path());
    let before = snapshot_digest(engine.document()).unwrap();
    let mut edit = prepare(
        engine.document(),
        Some(&path),
        "normalize_master",
        &json!({"standard": "streaming", "true_peak_dbtp": -1.0}),
    )
    .unwrap();
    eprintln!("Master telemetry: {}", edit.telemetry);
    let measured = &edit.telemetry["measurement"];
    assert!((measured["integratedLufs"].as_f64().unwrap() + 14.0).abs() < 0.1);
    assert!(measured["truePeakDbtp"].as_f64().unwrap() <= -1.0);
    engine.execute(edit.command.clone()).unwrap();
    edit.commit();
    let tracks = &engine.document().project.sequence.tracks;
    assert!(tracks[..3].iter().all(|t| t.muted && !t.solo));
    assert!(!tracks[3].muted);
    history_roundtrip(&mut engine, &before, &path);
    // A second bounce exercises muted-track exclusion: summing originals again
    // would alter the pre-normalization loudness relative to the saved master.
    let second = prepare(
        engine.document(),
        Some(&path),
        "normalize_master",
        &json!({"standard": "streaming", "true_peak_dbtp": -1.0}),
    )
    .unwrap();
    assert!(
        (second.telemetry["measurement"]["beforeLufs"]
            .as_f64()
            .unwrap()
            + 14.0)
            .abs()
            < 0.1,
        "{}",
        second.telemetry
    );
}

#[test]
fn failed_preparation_removes_earlier_gap_artifacts_and_rejects_symlink_directory() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, path) = setup(dir.path());
    let mut doc = engine.document().clone();
    doc.project.sequence.tracks[1].clips = vec![
        audio_clip(4, Tick(0), tick(0.5)),
        audio_clip(4, tick(1.5), tick(0.5)),
        audio_clip(4, tick(123.0), tick(0.5)),
    ];
    let params = json!({"track_id": 2, "room_tone_media_id": 4, "profile_start_tick": 0,
        "profile_end_tick": tick(0.3), "min_gap_ms": 100, "crossfade_ms": 30});
    assert!(prepare(&doc, Some(&path), "room_tone", &params).is_err());
    let derived = path.with_extension("saturn-audio");
    assert_eq!(std::fs::read_dir(&derived).unwrap().count(), 0);
    #[cfg(unix)]
    {
        std::fs::remove_dir(&derived).unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), &derived).unwrap();
        assert!(prepare(engine.document(), Some(&path), "room_tone", &params).is_err());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}
