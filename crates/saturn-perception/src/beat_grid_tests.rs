// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

fn pulse(pcm: &mut [f32], at: usize, gain: f32) {
    for i in 0..480 {
        pcm[at + i] += gain * (1.0 - i as f32 / 480.0);
    }
}

#[test]
fn measured_period_and_phase_are_gain_independent_and_do_not_drift() {
    for period in [4800, 8000, 11520, 16000] {
        let mut pcm = vec![0.0; RATE * 20];
        for at in (3200..pcm.len() - 480).step_by(period) {
            pulse(&mut pcm, at, 0.5);
        }
        let grid = estimate(&pcm, RATE as u32).unwrap();
        assert!(
            (grid.period_samples - period as f64).abs() < 2.0,
            "{grid:?}"
        );
        assert!((grid.origin_sample - 3200.0).abs() < 2.0, "{grid:?}");
        assert!(grid.onset_support > 0.99 && grid.beat_occupancy > 0.99);
        for v in &mut pcm {
            *v *= 0.1;
        }
        let quiet = estimate(&pcm, RATE as u32).unwrap();
        assert!((quiet.period_samples - grid.period_samples).abs() < 0.01);
    }
    let mut pcm = vec![0.0; RATE * 120];
    for (i, at) in (3200..pcm.len() - 480).step_by(7313).enumerate() {
        pulse(&mut pcm, at + (i % 3) * 8, 0.5);
    }
    let grid = estimate(&pcm, RATE as u32).unwrap();
    assert!((grid.period_samples - 7313.0).abs() < 0.1, "{grid:?}");
    assert!(grid.rms_timing_error_ms < 1.0);
}

#[test]
fn sparse_noise_silence_and_unsteady_attacks_error_instead_of_inventing_a_grid() {
    assert!(estimate(&vec![0.0; RATE * 8], RATE as u32).is_err());
    assert!(estimate(&vec![0.5; RATE * 8], RATE as u32).is_err());
    assert!(estimate(&vec![f32::NAN; RATE * 8], RATE as u32).is_err());
    let mut pcm = vec![0.0; RATE * 10];
    for at in [
        4000, 11113, 29007, 34103, 55231, 77109, 94013, 111109, 120987, 140567,
    ] {
        pulse(&mut pcm, at, 0.5);
    }
    assert!(estimate(&pcm, RATE as u32).is_err());
    let mut pcm = vec![0.0; RATE * 10];
    for at in [3200, 11200, 19200, 119200, 127200, 135200, 143200] {
        pulse(&mut pcm, at, 0.5);
    }
    assert!(estimate(&pcm, RATE as u32).is_err());
}
