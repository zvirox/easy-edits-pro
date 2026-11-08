// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use gstreamer::{self as gst, prelude::*};
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use saturn_perception::{SampledFrame, match_cut::focal_position};
use saturn_render::{frame_extractor::decode_frame_rgb, match_cut::prepare};
use serde_json::json;
use std::path::{Path, PathBuf};

fn f(n: i64) -> Tick {
    Tick(n * (TICKS_PER_SECOND / 30))
}
fn video(dir: &Path, name: &str, x: impl Fn(usize) -> usize) -> PathBuf {
    let path = dir.join(format!("{name}.mkv"));
    let pipeline=gst::parse::launch(&format!(
        "appsrc name=frames format=time caps=video/x-raw,format=RGB,width=160,height=90,framerate=30/1 ! videoconvert ! vp8enc deadline=1 min-quantizer=0 max-quantizer=0 ! matroskamux timecodescale=1 ! filesink location={path:?}"
    )).unwrap().downcast::<gst::Pipeline>().unwrap();
    let src = pipeline
        .by_name("frames")
        .unwrap()
        .downcast::<gstreamer_app::AppSrc>()
        .unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    for i in 0..90 {
        let mut rgb = vec![20u8; 160 * 90 * 3];
        for y in 30..50 {
            for xx in x(i)..x(i) + 15 {
                rgb[(y * 160 + xx) * 3..(y * 160 + xx) * 3 + 3].fill(230);
            }
        }
        let mut buffer = gst::Buffer::from_mut_slice(rgb);
        let b = buffer.get_mut().unwrap();
        b.set_pts(gst::ClockTime::from_nseconds(
            f(i as i64).as_nanoseconds().unwrap(),
        ));
        b.set_duration(gst::ClockTime::from_nseconds(
            f(1).as_nanoseconds().unwrap(),
        ));
        src.push_buffer(buffer).unwrap();
    }
    src.end_of_stream().unwrap();
    let message = pipeline.bus().unwrap().timed_pop_filtered(
        gst::ClockTime::from_seconds(30),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    pipeline.set_state(gst::State::Null).unwrap();
    assert!(matches!(message.unwrap().view(), gst::MessageView::Eos(_)));
    path
}
fn doc(left: PathBuf, right: PathBuf, kind: MediaKind) -> ProjectDocument {
    let mut d = ProjectDocument::new("Match-cut native evidence");
    d.project.settings.width = 160;
    d.project.settings.height = 90;
    if kind == MediaKind::Audio {
        d.project.sequence.tracks[0].kind = saturn_core::TrackKind::Audio;
    }
    for (id, path) in [(3, left), (4, right)] {
        d.project.media.push(MediaAsset {
            id,
            path,
            kind: kind.clone(),
            duration: Some(f(90)),
            audio_channels: Some(1),
            ..Default::default()
        });
    }
    let left = audio_clip(3, f(0), f(30));
    let mut right = audio_clip(4, f(30), f(30));
    right.source_in = f(30);
    d.project.sequence.tracks[0].clips = vec![left, right];
    d.project.next_id = 5;
    d
}
fn focal(path: &Path, tick: Tick) -> [f64; 2] {
    let (width, height, rgb) = decode_frame_rgb(path, tick).unwrap();
    focal_position(&SampledFrame {
        tick,
        width,
        height,
        rgb,
    })
    .unwrap()
    .unwrap()
}
fn apply(d: &ProjectDocument, mode: &str) -> EditorEngine {
    let args = json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":mode,"search_frames":8});
    let edit = prepare(d, &args).unwrap();
    let repeated = prepare(d, &args).unwrap();
    assert_eq!(
        edit.command.params, repeated.command.params,
        "analysis must be deterministic"
    );
    assert_eq!(
        edit.command.params["matched_source_in"],
        json!(f(36)),
        "{}",
        edit.telemetry
    );
    if mode != "transient" {
        assert!(
            edit.telemetry["distance_after"].as_f64().unwrap()
                < edit.telemetry["distance_before"].as_f64().unwrap()
        );
    }
    let mut engine = EditorEngine::default();
    engine.restore_recovery(d.clone(), None).unwrap();
    engine.execute(edit.command).unwrap();
    let mut expected = d.clone();
    expected.project.sequence.tracks[0].clips[1].source_in = f(36);
    assert_eq!(
        snapshot_digest(engine.document()).unwrap(),
        snapshot_digest(&expected).unwrap()
    );
    engine
}

#[test]
fn moving_sources_seek_deterministically_and_visual_alignment_changes_export() {
    gst::init().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let incoming = video(dir.path(), "incoming", |i| 24 + i);
    let outgoing = video(dir.path(), "outgoing", |_| 60);
    // Backwards/repeated seeks and off-grid positions must resolve the requested
    // frame, independent of decoder speed. A Playing/drop sink races this test.
    for n in [36, 5, 60, 36, 0, 89, 5] {
        let a = focal(&incoming, f(n));
        let b = focal(&incoming, Tick(f(n).0 + f(1).0 / 3));
        assert!((a[0] - b[0]).abs() < 0.002, "frame {n}: {a:?} vs {b:?}");
        let expected = (24.0 + n as f64 + 7.0) / 159.0;
        assert!(
            (a[0] - expected).abs() < 0.012,
            "frame {n}: {a:?} vs {expected}"
        );
    }
    let d = doc(outgoing, incoming, MediaKind::Video);
    let engine = apply(&d, "eye_trace");
    let before = dir.path().join("before.mp4");
    let after = dir.path().join("after.mp4");
    saturn_render::render_project(&d, &before, |_| {}).unwrap();
    saturn_render::render_project(engine.document(), &after, |_| {}).unwrap();
    let reference = focal(&after, f(29))[0];
    assert!((focal(&after, f(30))[0] - reference).abs() < 0.01);
    assert!((focal(&before, f(30))[0] - reference).abs() > 0.025);

    let left = video(dir.path(), "motion-left", |i| 44 + (i % 5) * 4);
    let right = video(dir.path(), "motion-right", |i| match i {
        0..=32 => 140 - i * 4,
        33 => 50,
        34 => 51,
        35 => 52,
        36 => 60,
        37 => 64,
        38 => 65,
        39 => 66,
        _ => 90 - i,
    });
    let d = doc(left, right, MediaKind::Video);
    let engine = apply(&d, "motion");
    let output = dir.path().join("motion.mp4");
    saturn_render::render_project(engine.document(), &output, |_| {}).unwrap();
    let outgoing = focal(&output, f(29))[0] - focal(&output, f(28))[0];
    let incoming = focal(&output, f(31))[0] - focal(&output, f(30))[0];
    assert!(outgoing > 0.015 && (outgoing - incoming).abs() < 0.008);
    let flat = video(dir.path(), "static", |_| 30);
    let d = doc(flat.clone(), flat, MediaKind::Video);
    assert!(
        prepare(
            &d,
            &json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":"motion"})
        )
        .unwrap_err()
        .contains("no measurable")
    );
    let tie = prepare(
        &d,
        &json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":"eye_trace"}),
    )
    .unwrap();
    assert_eq!(
        tie.command.params["matched_source_in"],
        json!(f(30)),
        "ties prefer zero slip"
    );
    let mut missing = d;
    missing.project.media[1].path = dir.path().join("missing.mkv");
    assert!(
        prepare(
            &missing,
            &json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":"eye_trace"})
        )
        .unwrap_err()
        .contains("missing")
    );
}

#[test]
fn real_transients_align_to_cut_and_silence_errors_without_state_changes() {
    gst::init().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let wav = |name: &str, onset: usize| {
        let path = dir.path().join(name);
        let mut pcm = vec![0.0; 48000 * 3];
        for n in 0..1920 {
            pcm[onset + n] = 0.6 * (1.0 - n as f32 / 1920.0);
        }
        // A closer, differently shaped onset must lose to the matching attack.
        if name == "right.wav" {
            pcm[50_400..50_880].fill(0.6);
        }
        saturn_render::audio_edit_files::write_wav(
            &mut std::fs::File::create(&path).unwrap(),
            &pcm,
            1,
            48000,
        )
        .unwrap();
        path
    };
    let left = wav("left.wav", 43200);
    let right = wav("right.wav", 57600);
    let d = doc(left, right, MediaKind::Audio);
    let engine = apply(&d, "transient");
    let before = dir.path().join("before.mp4");
    let after = dir.path().join("after.mp4");
    saturn_render::render_project(&d, &before, |_| {}).unwrap();
    saturn_render::render_project(engine.document(), &after, |_| {}).unwrap();
    let energy = |path: &Path| {
        let pcm = saturn_render::audio_pcm::decode_audio_pcm(path, f(30), Some(f(31))).unwrap();
        pcm.samples.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / pcm.samples.len() as f64
    };
    assert!(energy(&before) < 1e-6 && energy(&after) > 0.03);
    let silence = dir.path().join("silence.wav");
    saturn_render::audio_edit_files::write_wav(
        &mut std::fs::File::create(&silence).unwrap(),
        &vec![0.0; 48000 * 3],
        1,
        48000,
    )
    .unwrap();
    let mut silent = d.clone();
    silent.project.media[0].path = silence;
    let before = snapshot_digest(&silent).unwrap();
    assert!(
        prepare(
            &silent,
            &json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":"transient"})
        )
        .unwrap_err()
        .contains("No outgoing transient")
    );
    assert_eq!(snapshot_digest(&silent).unwrap(), before);
    // Core remains undoable after measured application.
    let mut engine = engine;
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(
        snapshot_digest(engine.document()).unwrap(),
        snapshot_digest(&d).unwrap()
    );
}
