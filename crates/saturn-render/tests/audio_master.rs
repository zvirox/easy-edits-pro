// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_audio_dsp::true_peak::measure_peak;
use saturn_render::{
    audio_master::normalize,
    loudness::{EbuR128Meter, LoudnessStandard},
};
use std::f64::consts::PI;

#[test]
fn streaming_master_meets_loudness_and_peak_with_transients() {
    let pcm: Vec<_> = (0..96_000)
        .flat_map(|i| {
            let base = (2.0 * PI * 1000.0 * i as f64 / 48_000.0).sin() * 0.03;
            // High-frequency intersample peaks, too hot for a gain-only adjustment.
            let transient = if (30_000..30_048).contains(&i) {
                (PI * i as f64 / 2.0 + PI / 4.0).sin() * 0.8
            } else {
                0.0
            };
            [(base + transient) as f32, (base + transient) as f32]
        })
        .collect();
    let (result, telemetry) =
        normalize(&pcm, 48_000, 2, LoudnessStandard::Streaming, -1.0).unwrap();
    let mut meter = EbuR128Meter::new(48_000, 2, LoudnessStandard::Streaming);
    meter.process_interleaved(&result);
    assert!(
        (meter.integrated_lufs_exact() + 14.0).abs() <= 0.1,
        "{telemetry}"
    );
    assert!(
        20.0 * measure_peak(&result, 2).unwrap().log10() <= -1.0,
        "{telemetry}"
    );
    let mut before = EbuR128Meter::new(48_000, 2, LoudnessStandard::Streaming);
    before.process_interleaved(&pcm);
    let gain_only_peak =
        20.0 * measure_peak(&pcm, 2).unwrap().log10() - 14.0 - before.integrated_lufs_exact();
    assert!(
        gain_only_peak > 0.0,
        "Fixture must require limiting: {gain_only_peak}"
    );
    assert_eq!(result.len(), pcm.len());
    eprintln!("Limited transient master: {telemetry}");
    check_independent_meter(&result);
}

fn check_independent_meter(pcm: &[f32]) {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("normalized.wav");
    saturn_render::audio_edit_files::write_wav(
        &mut std::fs::File::create(&wav).unwrap(),
        pcm,
        2,
        48_000,
    )
    .unwrap();
    // Pin the known container: FFmpeg's MPEG-TS probe can falsely match the
    // repetitive PCM bytes of this synthetic sine. Its WAV demuxer validates
    // the real header; decoding and metering remain independent of our code.
    let output = match std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-nostdin", "-f", "wav", "-i"])
        .arg(&wav)
        .args(["-af", "ebur128=peak=true", "-f", "null", "-"])
        .output()
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("SKIP independent FFmpeg meter: ffmpeg is not installed");
            return;
        }
        Err(e) => panic!("Could not run independent meter: {e}"),
    };
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{text}");
    let summary = text.rsplit("Summary:").next().unwrap();
    let number = |label| {
        summary
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix(label)
                    .and_then(|v| v.split_whitespace().next())
                    .and_then(|v| v.parse::<f64>().ok())
            })
            .unwrap()
    };
    let lufs = number("I:");
    let peak = number("Peak:");
    eprintln!("Independent FFmpeg meter: {lufs} LUFS, {peak} dBTP");
    assert!((lufs + 14.0).abs() <= 0.1, "{summary}");
    assert!(peak <= -1.0, "{summary}");
}

#[test]
fn normalized_sine_matches_independent_bs1770_response() {
    let pcm: Vec<_> = (0..96_000)
        .flat_map(|i| {
            let x = (2.0 * PI * 1000.0 * i as f64 / 48_000.0).sin() as f32 * 0.03;
            [x, x]
        })
        .collect();
    for standard in [
        LoudnessStandard::Streaming,
        LoudnessStandard::EbuR128,
        LoudnessStandard::AtscA85,
    ] {
        let (result, _) = normalize(&pcm, 48_000, 2, standard, -1.0).unwrap();
        // Steady-state K-weighting gain at 1 kHz from published coefficients.
        let response = 1.083636;
        let sum_squares = result[48_000..]
            .iter()
            .map(|&v| (v as f64).powi(2))
            .sum::<f64>();
        let lufs = -0.691
            + 10.0
                * (sum_squares / ((result.len() - 48_000) / 2) as f64 * response * response)
                    .log10();
        assert!((lufs - standard.target_lufs()).abs() < 0.1, "{lufs}");
    }
    assert!(
        normalize(
            &vec![0.0; 48_000],
            48_000,
            1,
            LoudnessStandard::Streaming,
            -1.0
        )
        .is_err()
    );
    assert!(
        normalize(
            &[f32::NAN; 48_000],
            48_000,
            1,
            LoudnessStandard::Streaming,
            -1.0
        )
        .is_err()
    );
}
