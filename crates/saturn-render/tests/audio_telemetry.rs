// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 1.3 evidence: `audio_read_telemetry` end-to-end through the
//! shared payload builder — media mode (WAV decode) and sequence mode
//! (rendered sequence mix analyzed). The reference signal is a 1 kHz stereo
//! sine calibrated through the BS.1770-4 K-weighting filter's *actual*
//! steady-state response at 48 kHz (the ITU reference is defined by that
//! filter), so the media-mode assertion exercises the analyzer, not filter
//! lore. Skips (with a note) when GStreamer decode/encode plugins or the
//! environment lack pieces; run laptop-native for evidence.

use saturn_core::{CommandRequest, EditorEngine};
#[allow(unused_imports)]
use saturn_core::Tick;
use saturn_render::preview_tools::audio_telemetry_payload;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn gstreamer_gate() -> MutexGuard<'static, ()> {
    static GATE: OnceLock<Mutex<()>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// BS.1770-4 K-weighting magnitude at 1 kHz / 48 kHz, from the exact
/// published coefficient set (mirrors `loudness.rs`, test-side).
fn k_weight_gain_at_1khz_48k() -> f64 {
    let w = 2.0 * std::f64::consts::PI * 1_000.0 / 48_000.0;
    let evaluate = |b0: f64, b1: f64, b2: f64, a1: f64, a2: f64| -> f64 {
        let (zr, zi) = (w.cos(), -w.sin());
        let (z2r, z2i) = ((zr * zr - zi * zi), (2.0 * zr * zi));
        let num_re = b0 + b1 * zr + b2 * z2r;
        let num_im = b1 * zi + b2 * z2i;
        let den_re = 1.0 + a1 * zr + a2 * z2r;
        let den_im = a1 * zi + a2 * z2i;
        ((num_re * num_re + num_im * num_im) / (den_re * den_re + den_im * den_im)).sqrt()
    };
    // Stage 1 high shelf + stage 2 RLB high-pass, exact 48 kHz coefficients.
    let shelf = evaluate(
        1.53512485958697,
        -2.69169618940638,
        1.19839281085285,
        -1.69065929318241,
        0.73248077421585,
    );
    let rlb = evaluate(1.0, -2.0, 1.0, -1.99004745483398, 0.99007225036621);
    shelf * rlb
}

/// Amplitude of the 1 kHz stereo sine whose metered loudness is −23 LUFS.
fn reference_amplitude() -> f64 {
    let target = 10.0_f64.powf((-23.0 + 0.691) / 20.0); // A·|H| for −23 LUFS
    target / k_weight_gain_at_1khz_48k()
}

/// Write a 16-bit PCM stereo WAV at 48 kHz with `frames` frames of the
/// calibrated reference sine. Returns the path (in the OS temp dir).
fn write_reference_wav(frames: usize) -> PathBuf {
    let amplitude = reference_amplitude();
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f64 / 48_000.0;
        let v = amplitude * (2.0 * std::f64::consts::PI * 1_000.0 * t).sin();
        let q = (v * 32_767.0).round() as i16;
        pcm.extend_from_slice(&q.to_le_bytes());
        pcm.extend_from_slice(&q.to_le_bytes());
    }
    let mut bytes = Vec::with_capacity(44 + pcm.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&((36 + pcm.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&2_u16.to_le_bytes()); // stereo
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&(48_000_u32 * 4).to_le_bytes()); // byte rate
    bytes.extend_from_slice(&4_u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16_u16.to_le_bytes()); // bits
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&pcm);

    let path = std::env::temp_dir().join(format!(
        "eep-audio-telemetry-ref-{}s.wav",
        std::process::id()
    ));
    std::fs::write(&path, bytes).expect("write the reference WAV");
    path
}

fn can_decode_wav() -> bool {
    let _ = gstreamer::init();
    gstreamer::ElementFactory::find("uridecodebin").is_some()
        && gstreamer::ElementFactory::find("wavparse").is_some()
}

fn can_encode_mp4_audio() -> bool {
    let _ = gstreamer::init();
    gstreamer::ElementFactory::find("avenc_aac").is_some()
        || gstreamer::ElementFactory::find("fdkaacenc").is_some()
}

fn engine_with_media(path: &std::path::Path) -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", serde_json::json!({"name": "Audio"})))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            serde_json::json!({
                "name": path.file_name().unwrap().to_string_lossy(),
                "path": path.to_string_lossy(),
                "kind": "audio"
            }),
        ))
        .unwrap();
    engine
}

#[test]
fn media_mode_reference_signal_reads_minus_23_lufs() {
    let _gate = gstreamer_gate();
    if !can_decode_wav() {
        eprintln!("skipping: no GStreamer WAV decode plugins");
        return;
    }
    let wav = write_reference_wav(48_000 * 2); // 2 s
    let engine = engine_with_media(&wav);
    let media_id = engine.document().project.media[0].id;

    let payload = audio_telemetry_payload(
        engine.document(),
        &serde_json::json!({"media_id": media_id}),
    )
    .expect("media-mode telemetry analyzes the reference signal");
    std::fs::remove_file(&wav).ok();

    let report = &payload["report"];
    assert_eq!(payload["source"], "media file");
    let integrated = report["loudness"]["integratedLufs"]
        .as_f64()
        .expect("integrated loudness present");
    assert!(
        (integrated - (-23.0)).abs() <= 0.1,
        "media-mode exit criterion: integrated {integrated} LUFS not within ±0.1 of −23.0"
    );
    // Octave spectrum: 1 kHz band dominates; distant bands far down.
    let bands: Vec<f64> = report["spectralBandsDb"]
        .as_array()
        .expect("spectral bands present")
        .iter()
        .map(|v| v.as_f64().unwrap_or(f64::MIN))
        .collect();
    assert_eq!(bands.len(), 10);
    assert_eq!(
        bands.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i),
        Some(5),
        "1 kHz sine peaks in the 1 kHz band: {bands:?}"
    );
    // Mono-coherent stereo reads +1.
    let phase = report["phaseCorrelation"].as_f64().expect("phase present");
    assert!(phase > 0.99, "identical channels read mono-coherent: {phase}");
    // Honesty marker: no invented aggregate score.
    assert_eq!(payload["notes"]["aggregateScore"], serde_json::Value::Null);
}

#[test]
fn sequence_mode_renders_and_analyzes_the_mix() {
    let _gate = gstreamer_gate();
    if !can_decode_wav() || !can_encode_mp4_audio() {
        eprintln!("skipping: no GStreamer WAV decode or AAC encode plugins");
        return;
    }
    let wav = write_reference_wav(48_000 * 3); // 3 s
    let mut engine = engine_with_media(&wav);
    let media_id = engine.document().project.media[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_track",
            serde_json::json!({"kind": "audio"}),
        ))
        .unwrap();
    let audio_track_id = engine
        .document()
        .project
        .sequence
        .tracks
        .iter()
        .find(|track| matches!(track.kind, saturn_core::TrackKind::Audio))
        .unwrap()
        .id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            serde_json::json!({"media_id": media_id, "track_id": audio_track_id}),
        ))
        .unwrap();

    let tps = saturn_core::time::TICKS_PER_SECOND;
    let payload = audio_telemetry_payload(
        engine.document(),
        &serde_json::json!({"start_tick": 0, "end_tick": tps}), // first second
    )
    .expect("sequence-mode telemetry renders + analyzes the mix");
    std::fs::remove_file(&wav).ok();

    assert_eq!(payload["source"], "sequence mix");
    let report = &payload["report"];
    assert_eq!(report["startTick"], 0);
    assert_eq!(report["endTick"], tps);
    // The mix rides through the snippet's audio encoder, so the loudness
    // assertion is generous here; the exact ±0.1 LU check lives in the
    // media-mode test above and the calibrated unit test.
    let integrated = report["loudness"]["integratedLufs"]
        .as_f64()
        .expect("integrated loudness measured");
    assert!(
        (integrated - (-23.0)).abs() <= 1.0,
        "sequence mix loudness {integrated} LUFS far off the reference signal"
    );
    // Analysis scratch (the snippet) must not linger.
    let previews = saturn_render::frame_extractor::previews_dir().unwrap();
    let stray = previews.join(format!("audio-telemetry-0-{tps}.mp4"));
    assert!(!stray.exists(), "temp snippet must be removed: {stray:?}");
}

#[test]
fn audio_telemetry_param_errors_are_typed() {
    let engine = EditorEngine::default();
    // Neither mode selected.
    let error =
        audio_telemetry_payload(engine.document(), &serde_json::json!({})).unwrap_err();
    assert!(
        error.contains("media_id, or start_tick+end_tick"),
        "unexpected: {error}"
    );
    // Unknown media id.
    let error = audio_telemetry_payload(
        engine.document(),
        &serde_json::json!({"media_id": 4242}),
    )
    .unwrap_err();
    assert!(error.contains("was not found"), "unexpected: {error}");
    // Track scoping honestly rejected instead of silently ignored.
    let error = audio_telemetry_payload(
        engine.document(),
        &serde_json::json!({"start_tick": 0, "end_tick": 1, "track_id": 7}),
    )
    .unwrap_err();
    assert!(error.contains("track-scoped"), "unexpected: {error}");
    // Unknown standard.
    let error = audio_telemetry_payload(
        engine.document(),
        &serde_json::json!({"media_id": 1, "standard": "bogus"}),
    )
    .unwrap_err();
    assert!(error.contains("Unknown loudness standard"), "unexpected: {error}");
}
