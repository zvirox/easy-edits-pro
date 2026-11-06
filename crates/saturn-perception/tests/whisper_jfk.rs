// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! End-to-end transcription check against the canonical whisper.cpp sample
//! (`jfk.wav`, "And so my fellow Americans…", 16 kHz mono, 11 s).
//!
//! Gated: it downloads the `tiny.en` weights (~77 MB, MIT, SHA-256-pinned)
//! on first run, so `cargo test` in constrained CI skips it unless
//! `SATURN_PERCEPTION_E2E=1` or the model is already cached in
//! `SATURN_MODELS_DIR`.
//!
//! This is the empirical t0/t1 verification too: the file is 11 s long, so
//! if whisper.cpp's centisecond convention holds, segment end ticks stay at
//! or below 11 s worth of ticks (a millisecond convention would overshoot).

use saturn_core::time::TICKS_PER_SECOND;
use saturn_perception::{transcribe_pcm, TranscribeOptions, WhisperModel, WHISPER_SAMPLE_RATE};

fn e2e_enabled() -> bool {
    std::env::var("SATURN_PERCEPTION_E2E").is_ok_and(|v| v != "0")
        || std::env::var("SATURN_MODELS_DIR").is_ok()
}

#[test]
fn jfk_sample_transcribes_with_measured_word_ticks() {
    if !e2e_enabled() {
        eprintln!("skipping: set SATURN_PERCEPTION_E2E=1 to run the Whisper end-to-end test");
        return;
    }
    let fixture = match std::env::var("SATURN_PERCEPTION_JFK_WAV") {
        Ok(path) => std::path::PathBuf::from(path),
        Err(_) => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/assets/jfk.wav"),
    };
    let Ok(decoded) = saturn_render_stub(&fixture) else {
        eprintln!("skipping: fixture not found at {} (download instructions in the checklist)", fixture.display());
        return;
    };

    let transcript = transcribe_pcm(
        &decoded,
        WhisperModel::TinyEn,
        &TranscribeOptions {
            language: Some("en".to_owned()),
            detect_fillers: true,
            prompt: None,
        },
    )
    .expect("transcription of the canonical sample succeeds");

    assert!(!transcript.segments.is_empty(), "silence would fail: got no segments");
    assert!(
        transcript.audio_seconds > 10.0 && transcript.audio_seconds < 12.0,
        "fixture is 11 s, got {}",
        transcript.audio_seconds
    );

    // Semantic check directly on real inference output.
    let all_text: String = transcript
        .segments
        .iter()
        .map(|s| s.text.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let normalized: String = all_text
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    assert!(
        normalized.contains("fellow americans"),
        "expected \"fellow americans\" in the transcript, got: {all_text}"
    );

    // Tick sanity: monotonic, within the audio span (centisecond units —
    // an ms-based interpretation would exceed 11 s of ticks), positive
    // confidences only (no invented values).
    let mut last_end = 0_i64;
    for segment in &transcript.segments {
        assert!(segment.start_ticks.0 >= last_end, "segments overlap/retreat");
        assert!(
            segment.end_ticks.0 <= 11 * TICKS_PER_SECOND + TICKS_PER_SECOND / 2,
            "segment end {} exceeds the 11 s audio span — timestamp unit assumption broken",
            segment.end_ticks.0
        );
        assert!(segment.confidence > 0.0 && segment.confidence <= 1.0);
        let mut last_word_end = segment.start_ticks.0;
        for word in &segment.words {
            assert!(word.start_ticks.0 >= last_word_end.saturating_sub(TICKS_PER_SECOND / 100));
            assert!(word.confidence > 0.0 && word.confidence <= 1.0);
            last_word_end = word.end_ticks.0;
        }
        last_end = segment.end_ticks.0;
    }
    // Sample-rate contract: callers decode to exactly this rate.
    assert_eq!(WHISPER_SAMPLE_RATE, 16_000);
}

/// Minimal 16-bit PCM WAV reader for the fixture (mono 16 kHz — the exact
/// Whisper input format; keeps the test independent of GStreamer).
fn saturn_render_stub(path: &std::path::Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let find = |tag: &[u8; 4]| -> Option<usize> {
        bytes.windows(4).position(|w| w == tag).map(|i| i + 8)
    };
    let fmt = find(b"fmt ").ok_or("fixture wav has no fmt chunk")?;
    let channels = u16::from_le_bytes([bytes[fmt + 2], bytes[fmt + 3]]) as usize;
    let sample_rate = u32::from_le_bytes([
        bytes[fmt + 4],
        bytes[fmt + 5],
        bytes[fmt + 6],
        bytes[fmt + 7],
    ]);
    let bits = u16::from_le_bytes([bytes[fmt + 14], bytes[fmt + 15]]) as usize;
    assert_eq!((sample_rate, channels, bits), (16_000, 1, 16), "fixture format");
    let data = find(b"data").ok_or("fixture wav has no data chunk")?;
    let mut samples = Vec::with_capacity((bytes.len() - data) / 2);
    for chunk in bytes[data..].chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0);
    }
    Ok(samples)
}
