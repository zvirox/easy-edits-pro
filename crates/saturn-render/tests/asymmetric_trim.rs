// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Real AV -> split edit -> save/history -> picture and sound export evidence.
use gstreamer::{self as gst, prelude::*};
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use saturn_render::{
    audio_edit_driver::prepare, audio_edit_files::write_wav, audio_pcm::decode_audio_pcm,
};
use serde_json::json;
use std::path::{Path, PathBuf};

fn t(seconds: f64) -> Tick {
    Tick((seconds * TICKS_PER_SECOND as f64).round() as i64)
}

fn source(dir: &Path, color: &str, channel: usize) -> PathBuf {
    let wav = dir.join(format!("{color}.wav"));
    let samples: Vec<f32> = (0..4 * 48_000)
        .flat_map(|n| {
            let value = (std::f64::consts::TAU * 480.0 * n as f64 / 48_000.0).sin() as f32 * 0.2;
            if channel == 0 {
                [value, 0.0]
            } else {
                [0.0, value]
            }
        })
        .collect();
    write_wav(
        &mut std::fs::File::create(&wav).unwrap(),
        &samples,
        2,
        48_000,
    )
    .unwrap();
    let path = dir.join(format!("{color}.mkv"));
    let pipeline = gst::parse::launch(&format!(
        "matroskamux name=mux ! filesink location={:?} \
         videotestsrc pattern={color} num-buffers=120 ! video/x-raw,width=160,height=90,framerate=30/1 \
         ! vp8enc deadline=1 ! queue ! mux. \
         filesrc location={:?} ! wavparse ! audioconvert ! audio/x-raw,format=F32LE ! queue ! mux.", path, wav
    )).unwrap().downcast::<gst::Pipeline>().unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    let message = pipeline.bus().unwrap().timed_pop_filtered(
        gst::ClockTime::from_seconds(30),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    pipeline.set_state(gst::State::Null).unwrap();
    let message = message.expect("fixture generation timed out");
    assert!(
        matches!(message.view(), gst::MessageView::Eos(_)),
        "fixture: {message:?}"
    );
    path
}

fn pcm(path: &Path, duration: Tick) -> Vec<f32> {
    let decoded = decode_audio_pcm(path, Tick(0), Some(duration)).unwrap();
    assert_eq!(decoded.sample_rate, 48_000);
    assert_eq!(decoded.channels, 2);
    decoded.samples
}
fn residual(a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len());
    assert!(!a.is_empty());
    10.0 * (a
        .iter()
        .zip(b)
        .map(|(a, b)| (*a as f64 - *b as f64).powi(2))
        .sum::<f64>()
        / a.len() as f64)
        .log10()
}

#[test]
fn real_split_edits_render_the_new_picture_cut_and_only_the_crossfaded_sound() {
    gst::init().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let red = source(dir.path(), "red", 0);
    let blue = source(dir.path(), "blue", 1);
    let mut doc = ProjectDocument::new("Split-edit native evidence");
    doc.project.settings.width = 160;
    doc.project.settings.height = 90;
    for (id, path) in [(3, red), (4, blue)] {
        doc.project.media.push(MediaAsset {
            id,
            path,
            kind: MediaKind::Video,
            duration: Some(t(4.0)),
            audio_channels: Some(2),
            ..Default::default()
        });
    }
    let mut left = audio_clip(3, t(0.0), t(1.0));
    left.source_in = t(1.0);
    let mut right = audio_clip(4, t(1.0), t(1.0));
    right.source_in = t(1.0);
    doc.project.sequence.tracks[0].clips = vec![left, right];
    doc.project.next_id = 5;
    let path = dir.path().join("project.saturn");
    doc.save_atomic(&path).unwrap();

    for audio_delta in [-0.25, 0.25] {
        let mut engine = EditorEngine::default();
        engine
            .restore_recovery(doc.clone(), Some(path.clone()))
            .unwrap();
        let args = json!({"operation":"asymmetric_trim", "track_id":1, "clip_index":1,
            "video_delta":t(0.2), "audio_delta":t(audio_delta), "fade_ticks":t(0.2)});
        let mut edit = prepare(engine.document(), Some(&path), "asymmetric_trim", &args).unwrap();
        let derived: PathBuf =
            serde_json::from_value(edit.command.params["audio"]["path"].clone()).unwrap();
        let before = snapshot_digest(engine.document()).unwrap();
        engine.execute(edit.command.clone()).unwrap();
        edit.commit();
        drop(edit);
        let after = snapshot_digest(engine.document()).unwrap();
        engine
            .execute(CommandRequest::new("edit.undo", json!({})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), before);
        engine
            .execute(CommandRequest::new("edit.redo", json!({})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
        engine
            .execute(CommandRequest::new("file.save", json!({"path":path})))
            .unwrap();
        engine
            .execute(CommandRequest::new("file.open", json!({"path":path})))
            .unwrap();
        assert_eq!(snapshot_digest(engine.document()).unwrap(), after);
        assert!(derived.is_file());

        // Orthogonal input channels: the fade preserves total power sample by
        // sample, and its channel weights move at the sound cut, not the picture cut.
        let mixed = pcm(&derived, t(2.0));
        assert_eq!(mixed.len(), 2 * 48_000 * 2);
        let center = ((1.0 + audio_delta) * 48_000.0) as usize;
        for n in (center - 4000..center + 4000).step_by(31) {
            let sine = (std::f64::consts::TAU * 480.0 * n as f64 / 48_000.0).sin() * 0.2;
            let power = mixed[2 * n].powi(2) as f64 + mixed[2 * n + 1].powi(2) as f64;
            assert!(
                (power - sine.powi(2)).abs() < 1e-5,
                "fade power at {n}: {power} vs {}",
                sine.powi(2)
            );
        }
        assert!(
            mixed[2 * (center - 6000) + 1..2 * (center - 5900)]
                .iter()
                .step_by(2)
                .all(|v| v.abs() < 1e-6)
        );
        assert!(
            mixed[2 * (center + 6000)..2 * (center + 6100)]
                .iter()
                .step_by(2)
                .all(|v| v.abs() < 1e-6)
        );

        let exported = dir.path().join(format!("split-{audio_delta}.mp4"));
        saturn_render::render_project(engine.document(), &exported, |_| {}).unwrap();
        let (w, h, frame) =
            saturn_render::frame_extractor::decode_frame_rgb(&exported, t(1.1)).unwrap();
        let center_pixel = ((h / 2 * w + w / 2) * 3) as usize;
        assert!(
            frame[center_pixel] > frame[center_pixel + 2] + 100,
            "picture before new cut must remain red"
        );
        let (_, _, frame) =
            saturn_render::frame_extractor::decode_frame_rgb(&exported, t(1.3)).unwrap();
        assert!(
            frame[center_pixel + 2] > frame[center_pixel] + 100,
            "picture after new cut must be blue"
        );

        // Render the same bounce with picture sources removed. Any accidentally
        // retained source audio makes this comparison fail (including near the cut).
        let mut control = engine.document().clone();
        control.project.sequence.tracks[0].clips.clear();
        let control_path = dir.path().join(format!("control-{audio_delta}.mp4"));
        saturn_render::render_project(&control, &control_path, |_| {}).unwrap();
        let output = pcm(&exported, t(2.0));
        let control_pcm = pcm(&control_path, t(2.0));
        let db = residual(&output, &control_pcm);
        assert!(db < -90.0, "source-audio suppression residual {db} dBFS");
        let energy = output.iter().map(|v| v.powi(2) as f64).sum::<f64>() / output.len() as f64;
        assert!(energy > 0.005, "export must be audible");

        if audio_delta < 0.0 {
            let mut doubled = engine.document().clone();
            for clip in &mut doubled.project.sequence.tracks[0].clips {
                clip.audio_enabled = true;
            }
            let doubled_path = dir.path().join("negative-control.mp4");
            saturn_render::render_project(&doubled, &doubled_path, |_| {}).unwrap();
            assert!(residual(&pcm(&doubled_path, t(2.0)), &control_pcm) > -30.0);
        }
    }

    // Fractional-sample boundaries must not invent an extra tail-handle
    // requirement when the incoming clip already ends exactly at source EOF.
    let mut bounded = doc.clone();
    bounded.project.media[1].duration = Some(t(2.0));
    let fractional = json!({"track_id":1,"clip_index":1,"video_delta":0,
        "audio_delta":t(-0.25000001),"fade_ticks":t(0.04000001)});
    drop(prepare(&bounded, Some(&path), "asymmetric_trim", &fractional).unwrap());

    // Stale preparation and failed decoding keep the document/history intact and
    // clean up the uncommitted WAV. No empty success is reported.
    let mut engine = EditorEngine::default();
    engine
        .restore_recovery(doc.clone(), Some(path.clone()))
        .unwrap();
    let args = json!({"track_id":1,"clip_index":1,"video_delta":0,"audio_delta":t(-0.25)});
    let edit = prepare(engine.document(), Some(&path), "asymmetric_trim", &args).unwrap();
    let abandoned: PathBuf =
        serde_json::from_value(edit.command.params["audio"]["path"].clone()).unwrap();
    engine
        .execute(CommandRequest::new(
            "track.set_gain_db",
            json!({"track_id":1,"gain_db":-3}),
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
    doc.project.media[0].path = dir.path().join("missing.mkv");
    assert!(prepare(&doc, Some(&path), "asymmetric_trim", &args).is_err());
}
