// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_audio_dsp::{
    ducking::{DuckingOptions, envelope, limit_lookahead},
    room_tone::synthesize,
};
use saturn_core::{
    TICKS_PER_SECOND, Tick,
    project::{AutomationInterpolation, TrackAutomation},
};

#[test]
fn ducking_anticipates_activity_holds_brief_pauses_and_recovers() {
    let mut pcm = vec![0.0; 48_000 * 3];
    pcm[48_000..57_600].fill(0.1);
    pcm[60_000..67_200].fill(0.1);
    let points = envelope(&pcm, 48_000, DuckingOptions::default()).unwrap();
    assert!(points.windows(2).all(|p| p[0].time < p[1].time));
    let lane = TrackAutomation {
        volume_points: points,
        interpolation: AutomationInterpolation::Linear,
        ..Default::default()
    };
    let at = |ms| {
        lane.evaluate_volume(Tick(TICKS_PER_SECOND * ms / 1000))
            .unwrap()
    };
    assert_eq!(at(950), 0.0);
    assert_eq!(at(980), -12.0);
    assert_eq!(at(1220), -12.0); // brief pause, no pumping
    assert_eq!(at(1550), -12.0); // 150 ms hold after last activity
    assert!((at(1700) + 6.0).abs() < 0.001);
    assert_eq!(at(1850), 0.0);
    assert_eq!(at(2990), 0.0);
    let silence = envelope(&vec![0.0; 48_000], 48_000, DuckingOptions::default()).unwrap();
    assert!(silence.iter().all(|p| p.value == 0.0));
    assert!(envelope(&[f32::NAN; 1024], 48_000, DuckingOptions::default()).is_err());
    assert!(
        envelope(
            &pcm,
            48_000,
            DuckingOptions {
                release_ms: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let step = TrackAutomation {
        volume_points: envelope(
            &pcm,
            48_000,
            DuckingOptions {
                lookahead_ms: 0,
                ..Default::default()
            },
        )
        .unwrap(),
        ..Default::default()
    };
    assert_eq!(
        step.evaluate_volume(Tick(TICKS_PER_SECOND * 999 / 1000)),
        Some(0.0)
    );
    assert_eq!(step.evaluate_volume(Tick(TICKS_PER_SECOND)), Some(-12.0));
}

#[test]
fn limiter_preserves_stereo_ratio_and_anticipates_transients() {
    let mut pcm: Vec<_> = (0..48_000)
        .flat_map(|i| {
            let x = if i == 24_000 { 2.0 } else { 0.1 };
            [x, x * 0.25]
        })
        .collect();
    limit_lookahead(&mut pcm, 2, 48_000, 0.8).unwrap();
    assert!(
        pcm.chunks_exact(2)
            .all(|f| f[0].abs() <= 0.800001 && f[1] == f[0] * 0.25)
    );
    assert!(pcm[23_800 * 2] < 0.05);
    assert!(pcm[47_999 * 2] > 0.099);
}

#[test]
fn room_tone_matches_noise_level_and_spectral_color_with_faded_edges() {
    let mut random = 12345_u64;
    let mut low = 0.0;
    let profile: Vec<_> = (0..48_000)
        .map(|_| {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let white = (random >> 11) as f64 / (1_u64 << 53) as f64 - 0.5;
            low = 0.95 * low + 0.05 * white;
            low as f32
        })
        .collect();
    let tone = synthesize(&profile, 96_000, 2400, 87).unwrap();
    let rms =
        |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    assert!((20.0 * (rms(&tone[2400..93_600]) / rms(&profile)).log10()).abs() < 0.5);
    // First-difference energy independently tests preserved low-frequency bias.
    let roughness = |x: &[f32]| {
        x.windows(2)
            .map(|v| (v[1] as f64 - v[0] as f64).powi(2))
            .sum::<f64>()
            / x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()
    };
    assert!((roughness(&tone) / roughness(&profile) - 1.0).abs() < 0.35);
    assert_eq!(tone[0], 0.0);
    assert_eq!(tone[tone.len() - 1], 0.0);
    assert!(rms(&tone[..200]) < rms(&tone[5000..5200]) * 0.25);
    assert_eq!(tone, synthesize(&profile, 96_000, 2400, 87).unwrap());
    assert_ne!(tone, synthesize(&profile, 96_000, 2400, 88).unwrap());
    assert!(synthesize(&vec![0.0; 1024], 1000, 0, 1).is_err());
    assert!(synthesize(&vec![0.2; 1024], 1000, 0, 1).is_err());
    assert!(synthesize(&profile, 100, 51, 1).is_err());
}
