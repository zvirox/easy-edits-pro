// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use gstreamer::{self as gst, prelude::*};
use saturn_core::audio_edit::{audio_clip, snapshot_digest};
use saturn_core::{
    CommandRequest, EditorEngine, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick,
};
use saturn_render::{beat_grid::prepare, frame_extractor::decode_frame_rgb};
use serde_json::json;
use std::path::{Path, PathBuf};

fn f(n: i64) -> Tick {
    Tick(n * (TICKS_PER_SECOND / 30))
}
fn video(dir: &Path, pattern: &str) -> PathBuf {
    let path = dir.join(format!("{pattern}.mkv"));
    let pipeline=gst::parse::launch(&format!(
        "videotestsrc pattern={pattern} num-buffers=360 ! video/x-raw,width=160,height=90,framerate=30/1 ! vp8enc deadline=1 ! matroskamux timecodescale=1 ! filesink location={path:?}"
    )).unwrap().downcast::<gst::Pipeline>().unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    let message = pipeline.bus().unwrap().timed_pop_filtered(
        gst::ClockTime::from_seconds(30),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    pipeline.set_state(gst::State::Null).unwrap();
    assert!(matches!(message.unwrap().view(), gst::MessageView::Eos(_)));
    path
}
fn rgb(path: &Path, frame: i64) -> [f64; 3] {
    let (_, _, data) = decode_frame_rgb(path, f(frame)).unwrap();
    let mut sum = [0.0; 3];
    for pixel in data.chunks_exact(3) {
        for i in 0..3 {
            sum[i] += pixel[i] as f64;
        }
    }
    sum.map(|v| v / (data.len() / 3) as f64)
}

#[test]
fn measured_music_rolls_rendered_cuts_keeps_edges_and_preserves_music() {
    gst::init().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut d = ProjectDocument::new("Beat-grid native evidence");
    d.project.settings.width = 160;
    d.project.settings.height = 90;
    for (id, pattern) in [(3, "red"), (4, "blue"), (5, "green")] {
        d.project.media.push(MediaAsset {
            id,
            path: video(dir.path(), pattern),
            kind: MediaKind::Video,
            duration: Some(f(360)),
            ..Default::default()
        });
    }
    let mut samples = vec![0.0; 16000 * 13];
    // Source offset is deliberately off the 10 ms analysis hop grid.
    for at in (4801 + 8000..samples.len() - 480).step_by(8000) {
        for i in 0..480 {
            samples[at + i] = 0.5 * (1.0 - i as f32 / 480.0);
        }
    }
    let music = dir.path().join("music.wav");
    saturn_render::audio_edit_files::write_wav(
        &mut std::fs::File::create(&music).unwrap(),
        &samples,
        1,
        16000,
    )
    .unwrap();
    d.project.media.push(MediaAsset {
        id: 6,
        path: music,
        kind: MediaKind::Audio,
        duration: Some(f(390)),
        audio_channels: Some(1),
        ..Default::default()
    });
    for (id, start, end) in [(3, 30, 82), (4, 82, 139), (5, 139, 210)] {
        let mut c = audio_clip(id, f(start), f(end - start));
        c.source_in = f(30);
        d.project.sequence.tracks[0].clips.push(c);
    }
    let mut c = audio_clip(6, f(0), f(360));
    c.source_in = Tick(4801 * (TICKS_PER_SECOND / 16000));
    d.project.sequence.tracks[1].clips = vec![c];
    d.project.next_id = 7;
    let args =
        json!({"music_track_id":2,"video_track_id":1,"subdivision":"quarter","strength":1.0});
    let edit = prepare(&d, &args).unwrap();
    assert!((edit.telemetry["tempo_bpm"].as_f64().unwrap() - 120.0).abs() < 0.01);
    assert_eq!(
        edit.command.params,
        prepare(&d, &args).unwrap().command.params
    );
    let before = snapshot_digest(&d).unwrap();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(d.clone(), None).unwrap();
    let applied = engine.execute(edit.command).unwrap();
    assert_eq!(applied["changed_cuts"], 2);
    assert_eq!(applied["cuts"][0]["tick"], json!(f(75)));
    assert_eq!(applied["cuts"][1]["tick"], json!(f(135)));
    assert_eq!(
        serde_json::to_value(&engine.document().project.sequence.tracks[1]).unwrap(),
        serde_json::to_value(&d.project.sequence.tracks[1]).unwrap()
    );
    let old = dir.path().join("before.mp4");
    let new = dir.path().join("after.mp4");
    saturn_render::render_project(&d, &old, |_| {}).unwrap();
    saturn_render::render_project(engine.document(), &new, |_| {}).unwrap();
    for (frame, channel) in [(30, 0), (74, 0), (75, 2), (134, 2), (135, 1), (209, 1)] {
        let pixel = rgb(&new, frame);
        assert!(pixel[channel] > 200.0, "{frame}: {pixel:?}");
        assert!(
            pixel
                .iter()
                .enumerate()
                .all(|(i, v)| i == channel || *v < 25.0)
        );
    }
    for frame in [29, 210] {
        assert!(rgb(&new, frame).iter().all(|v| *v < 15.0));
    }
    assert!(
        rgb(&old, 75)[0] > 200.0 && rgb(&old, 135)[2] > 200.0,
        "negative control: original cuts differ"
    );
    let a = saturn_render::audio_pcm::decode_audio_pcm(&old, f(0), Some(f(360))).unwrap();
    let b = saturn_render::audio_pcm::decode_audio_pcm(&new, f(0), Some(f(360))).unwrap();
    assert_eq!(a.samples.len(), b.samples.len());
    let mse = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(a, b)| (*a as f64 - *b as f64).powi(2))
        .sum::<f64>()
        / a.samples.len() as f64;
    assert!(mse < 1e-9, "music changed: {mse}");
    assert!(
        b.samples.iter().any(|v| v.abs() > 0.3),
        "rendered music must be audible"
    );
    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(snapshot_digest(engine.document()).unwrap(), before);

    let silence = dir.path().join("silence.wav");
    saturn_render::audio_edit_files::write_wav(
        &mut std::fs::File::create(&silence).unwrap(),
        &vec![0.0; 16000 * 13],
        1,
        16000,
    )
    .unwrap();
    d.project.media[3].path = silence;
    assert!(prepare(&d, &args).unwrap_err().contains("pulse train"));
    d.project.media[3].path = dir.path().join("missing.wav");
    assert!(prepare(&d, &args).unwrap_err().contains("missing"));
}
