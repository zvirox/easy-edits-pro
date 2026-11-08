// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Deterministic spectral room-tone synthesis from a caller-selected noise bed.
//! The caller must provide speech-free audio; there is no speech-removal model.
use crate::fft::{Complex, fft};
use std::f64::consts::PI;

pub fn synthesize(
    profile: &[f32],
    frames: usize,
    fade_frames: usize,
    seed: u64,
) -> Result<Vec<f32>, String> {
    const N: usize = 1024;
    if profile.len() < N
        || profile.len() > 192_000 * 10
        || frames == 0
        || frames > 192_000 * 120
        || profile.iter().any(|v| !v.is_finite())
        || fade_frames > frames / 2
    {
        return Err("Room tone needs finite mono noise (1024 frames..10 seconds at 192 kHz) and a bounded fade".into());
    }
    let mean = profile.iter().map(|&v| v as f64).sum::<f64>() / profile.len() as f64;
    let rms = (profile
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / profile.len() as f64)
        .sqrt();
    if rms < 1e-8 {
        return Err("Room-tone profile is silent".into());
    }
    let mut power = vec![0.0; N / 2 + 1];
    let mut count = 0;
    for chunk in profile.chunks_exact(N) {
        let mut bins: Vec<_> = chunk
            .iter()
            .enumerate()
            .map(|(i, &x)| Complex {
                re: (x as f64 - mean) * (0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos()),
                im: 0.0,
            })
            .collect();
        fft(&mut bins, false);
        for (sum, bin) in power.iter_mut().zip(&bins) {
            *sum += bin.norm().powi(2);
        }
        count += 1;
    }
    for p in &mut power {
        *p = (*p / count as f64).sqrt();
    }
    let mut random = seed.max(1);
    let mut output = vec![0.0_f64; frames + N];
    for start in (0..frames + N / 2).step_by(N / 2) {
        let mut bins = vec![Complex::default(); N];
        for k in 1..N / 2 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let angle = 2.0 * PI * (random >> 11) as f64 / (1_u64 << 53) as f64;
            bins[k] = Complex {
                re: power[k] * angle.cos(),
                im: power[k] * angle.sin(),
            };
            bins[N - k] = Complex {
                re: bins[k].re,
                im: -bins[k].im,
            };
        }
        fft(&mut bins, true);
        for (i, bin) in bins.iter().enumerate() {
            if let Some(out) = output.get_mut(start + i) {
                *out += bin.re * (0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos());
            }
        }
    }
    let output = &output[N / 2..N / 2 + frames];
    let output_rms = (output.iter().map(|v| v * v).sum::<f64>() / frames as f64).sqrt();
    if output_rms < 1e-12 {
        return Err("Room-tone profile has no usable non-DC energy".into());
    }
    Ok(output
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let edge = i.min(frames - 1 - i);
            let fade = if fade_frames == 0 {
                1.0
            } else {
                ((edge as f64 / fade_frames as f64).min(1.0) * PI / 2.0).sin()
            };
            (v * rms / output_rms * fade) as f32
        })
        .collect())
}
