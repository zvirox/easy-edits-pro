// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Cold-process coverage: decoding must not race GES initialization or leave
//! the shared runtime only partially initialized for the next render.
use saturn_core::{MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick};
use saturn_render::{audio_edit_driver, audio_edit_files, decode_audio_pcm, frame_extractor};
use std::{
    path::Path,
    process::Command,
    sync::Barrier,
    time::{Duration, Instant},
};

const CHILD_MODE: &str = "EEP_RUNTIME_INIT_TEST_CHILD";

#[test]
fn cold_media_entry_points_complete_with_a_full_runtime() {
    if let Ok(mode) = std::env::var(CHILD_MODE) {
        exercise(&mode);
        return;
    }
    // Each order needs a fresh process: a warmed OnceLock masks the defect.
    // Kill and reap a wedged child so a native deadlock fails this test in CI.
    for mode in ["frame-first", "audio-first", "hardware-first", "parallel"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cold_media_entry_points_complete_with_a_full_runtime",
                "--nocapture",
            ])
            .env(CHILD_MODE, mode)
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "cold-start mode {mode}: {status}");
                break;
            }
            if started.elapsed() > Duration::from_secs(60) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("cold-start mode {mode} exceeded 60 seconds");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn exercise(mode: &str) {
    // Fixture creation deliberately avoids GStreamer so every entry is cold.
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    let samples: Vec<f32> = (0..96_000)
        .map(|n| (n as f32 * std::f32::consts::TAU * 440.0 / 48_000.0).sin() * 0.1)
        .collect();
    audio_edit_files::write_wav(
        &mut std::fs::File::create(&wav).unwrap(),
        &samples,
        1,
        48_000,
    )
    .unwrap();
    let image = dir.path().join("frame.png");
    let mut encoder = png::Encoder::new(std::fs::File::create(&image).unwrap(), 16, 16);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[90; 16 * 16 * 3])
        .unwrap();

    let mut doc = ProjectDocument::new("Cold runtime");
    doc.project.media.push(MediaAsset {
        id: 4,
        name: "tone.wav".into(),
        path: wav.clone(),
        kind: MediaKind::Audio,
        duration: Some(Tick(2 * TICKS_PER_SECOND)),
        audio_channels: Some(1),
        ..Default::default()
    });
    doc.project.sequence.tracks[1]
        .clips
        .push(saturn_core::audio_edit::audio_clip(
            4,
            Tick(0),
            Tick(2 * TICKS_PER_SECOND),
        ));
    doc.project.next_id = 5;
    let project = dir.path().join("project.saturn");
    doc.save_atomic(&project).unwrap();

    match mode {
        "frame-first" => check_frame(&image),
        "audio-first" => check_audio(&wav),
        "hardware-first" | "parallel" => {}
        _ => panic!("unexpected mode {mode}"),
    }
    if mode != "parallel" {
        saturn_render::detect_hardware_encoders();
        check_elements();
    }

    let barrier = Barrier::new(4);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            barrier.wait();
            check_audio(&wav);
        });
        scope.spawn(|| {
            barrier.wait();
            check_frame(&image);
        });
        scope.spawn(|| {
            barrier.wait();
            saturn_render::detect_hardware_encoders();
            check_elements();
        });
        barrier.wait();
        let edit = audio_edit_driver::prepare(
            &doc,
            Some(&project),
            "normalize_master",
            &serde_json::json!({"standard": "streaming", "true_peak_dbtp": -1.0}),
        )
        .unwrap();
        let lufs = edit.telemetry["measurement"]["integratedLufs"]
            .as_f64()
            .unwrap();
        assert!((lufs + 14.0).abs() < 0.1, "rendered master: {lufs} LUFS");
    });
}

fn check_audio(path: &Path) {
    let audio = decode_audio_pcm(path, Tick(0), Some(Tick(TICKS_PER_SECOND))).unwrap();
    assert_eq!(audio.sample_rate, 48_000);
    assert!(!audio.samples.is_empty());
    assert!(audio.samples.iter().any(|sample| sample.abs() > 0.05));
}

fn check_frame(path: &Path) {
    let (width, height, rgb) = frame_extractor::decode_frame_rgb(path, Tick(0)).unwrap();
    assert_eq!((width, height), (16, 16));
    assert_eq!(rgb, vec![90; 16 * 16 * 3]);
}

fn check_elements() {
    for name in [
        saturn_render::graphics_element::ELEMENT_NAME,
        saturn_render::automation_element::ELEMENT_NAME,
        saturn_render::keyframe_element::ELEMENT_NAME,
        saturn_render::plugin_element::ELEMENT_NAME,
    ] {
        assert!(
            gstreamer::ElementFactory::find(name).is_some(),
            "missing {name}"
        );
    }
}
