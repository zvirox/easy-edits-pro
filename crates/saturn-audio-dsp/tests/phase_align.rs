// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_audio_dsp::phase_align::{estimate_delay, shift_interleaved};
use std::f64::consts::PI;

// Analytic multi-frequency signal: fractional delay is generated independently
// of the correction kernel, so an interpolation bug cannot cancel itself out.
fn signal(delay: f64, invert: bool) -> Vec<f32> {
    (0..32_768)
        .map(|n| {
            let t = (n as f64 - delay) / 48_000.0;
            (1..=96)
                .map(|k| {
                    // Irregular spacing avoids the artificial repeated delay peaks
                    // of an equally spaced frequency comb.
                    let f = 170.0 + ((k * 7919) % 7901) as f64;
                    (2.0 * PI * f * t + (k * k) as f64 * 0.31).sin() * 0.008
                })
                .sum::<f64>() as f32
                * if invert { -1.0 } else { 1.0 }
        })
        .collect()
}

#[test]
fn boom_lav_1_7_ms_delay_removes_comb_filtering() {
    let reference = signal(0.0, false);
    let target = signal(81.6, false); // Exactly 1.7 ms at 48 kHz.
    let report = estimate_delay(&reference, &target, 48_000).unwrap();
    assert!((report.lag_samples - 81.6).abs() < 0.02, "{report:?}");
    let fixed =
        shift_interleaved(&target, 1, report.lag_samples, report.inverted_polarity).unwrap();
    let rms =
        |v: &[f32]| (v.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
    let before: Vec<_> = reference[512..30_000]
        .iter()
        .zip(&target[512..30_000])
        .map(|(a, b)| a - b)
        .collect();
    let after: Vec<_> = reference[512..30_000]
        .iter()
        .zip(&fixed[512..30_000])
        .map(|(a, b)| a - b)
        .collect();
    let improvement_db = 20.0 * (rms(&after) / rms(&before)).log10();
    assert!(
        improvement_db < -45.0,
        "residual improvement {improvement_db} dB; {report:?}"
    );
    assert!(report.correlation > 0.999);
}

#[test]
fn earlier_inverted_target_is_corrected_without_channel_crosstalk() {
    let reference = signal(0.0, false);
    let target = signal(-37.25, true);
    let report = estimate_delay(&reference, &target, 48_000).unwrap();
    assert!((report.lag_samples + 37.25).abs() < 0.02, "{report:?}");
    assert!(report.inverted_polarity);
    let stereo: Vec<_> = target.iter().flat_map(|x| [*x, 0.0]).collect();
    let fixed = shift_interleaved(&stereo, 2, report.lag_samples, true).unwrap();
    assert!(fixed.chunks_exact(2).all(|f| f[1] == 0.0));
    assert!((fixed[10_000 * 2] - reference[10_000]).abs() < 0.001);
}

#[test]
fn silence_invalid_and_ambiguous_inputs_fail() {
    assert!(estimate_delay(&vec![0.0; 4096], &vec![0.0; 4096], 48_000).is_err());
    assert!(estimate_delay(&[1.0], &[1.0], 48_000).is_err());
    assert!(shift_interleaved(&[f32::NAN], 1, 0.0, false).is_err());
    let tone: Vec<_> = (0..16_384)
        .map(|n| (2.0 * PI * n as f64 / 48.0).sin() as f32)
        .collect();
    assert!(estimate_delay(&tone, &tone, 48_000).is_err());
}

#[test]
fn integer_shift_has_zero_padding_not_wraparound() {
    let output = shift_interleaved(&[1.0, 2.0, 3.0, 4.0], 1, 2.0, false).unwrap();
    for (a, b) in output.iter().zip([3.0, 4.0, 0.0, 0.0]) {
        assert!((a - b).abs() < 1e-6);
    }
}
