// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Offline GCC-PHAT delay estimation and windowed-sinc sub-sample correction.
//! Positive lag means the target arrives later than the reference. Correction
//! reads target[n + lag], so it advances late audio without delaying the edit.
//! Samples beyond either edge are zero-filled, never wrapped around.

use crate::fft::{Complex, fft};
use serde::Serialize;
use std::f64::consts::PI;

pub const MAX_ANALYSIS_FRAMES: usize = 65_536;
pub const MAX_LAG_SECONDS: f64 = 0.1;
const RADIUS: i64 = 32;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseAlignment {
    pub lag_samples: f64,
    pub lag_seconds: f64,
    pub inverted_polarity: bool,
    /// Measured normalized correlation after correction, not model confidence.
    pub correlation: f64,
    pub analyzed_frames: usize,
}

pub fn estimate_delay(
    reference: &[f32],
    target: &[f32],
    rate: u32,
) -> Result<PhaseAlignment, String> {
    if !(8_000..=192_000).contains(&rate)
        || reference.len() != target.len()
        || reference.len() < 1024
        || reference.len() > MAX_ANALYSIS_FRAMES
        || reference.iter().chain(target).any(|v| !v.is_finite())
    {
        return Err(
            "Phase alignment requires equal finite mono buffers, 1024..65536 frames at 8..192 kHz"
                .into(),
        );
    }
    let frames = reference.len();
    let max_lag = ((rate as f64 * MAX_LAG_SECONDS) as usize).min(frames / 4);
    let n = (frames * 2).next_power_of_two();
    let spectrum = |input: &[f32]| {
        let mean = input.iter().map(|&x| x as f64).sum::<f64>() / frames as f64;
        let mut data = vec![Complex::default(); n];
        for (i, &x) in input.iter().enumerate() {
            let window = 0.5 - 0.5 * (2.0 * PI * i as f64 / (frames - 1) as f64).cos();
            data[i].re = (x as f64 - mean) * window;
        }
        fft(&mut data, false);
        data
    };
    let a = spectrum(reference);
    let b = spectrum(target);
    let mut cross: Vec<_> = a
        .iter()
        .zip(&b)
        .map(|(a, b)| {
            b.mul(Complex {
                re: a.re,
                im: -a.im,
            })
        })
        .collect();
    let max = cross.iter().map(|v| v.norm()).fold(0.0, f64::max);
    if max < 1e-12 {
        return Err("Cannot phase-align silence or constant audio".into());
    }
    // Do not whiten roundoff or bins with negligible joint energy.
    for value in &mut cross {
        let magnitude = value.norm();
        if magnitude < max * 1e-5 {
            *value = Complex::default();
        } else {
            value.re /= magnitude;
            value.im /= magnitude;
        }
    }
    cross[0] = Complex::default();
    let mut correlation = cross.clone();
    fft(&mut correlation, true);
    let at = |lag: i64| correlation[lag.rem_euclid(n as i64) as usize].re;
    let coarse = (-(max_lag as i64)..=max_lag as i64)
        .max_by(|&x, &y| at(x).abs().total_cmp(&at(y).abs()))
        .unwrap();
    if coarse.unsigned_abs() as usize == max_lag {
        return Err(
            "Delay reaches the search boundary; use coarsely synchronized recordings".into(),
        );
    }
    let polarity = if at(coarse) < 0.0 { -1.0 } else { 1.0 };
    // Refine the PHAT peak by evaluating its band-limited Fourier interpolant.
    let score = |lag: f64| -> f64 {
        cross[1..n / 2]
            .iter()
            .enumerate()
            .map(|(k, v)| {
                let angle = 2.0 * PI * (k + 1) as f64 * lag / n as f64;
                (v.re * angle.cos() - v.im * angle.sin()) * polarity
            })
            .sum()
    };
    let (mut low, mut high) = (coarse as f64 - 0.75, coarse as f64 + 0.75);
    for _ in 0..24 {
        let x = low + (high - low) / 3.0;
        let y = high - (high - low) / 3.0;
        if score(x) > score(y) {
            high = y;
        } else {
            low = x;
        }
    }
    let lag = (low + high) / 2.0;
    let corrected = shift_interleaved(target, 1, lag, polarity < 0.0)?;
    let margin = max_lag + RADIUS as usize;
    let (mut dot, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (&x, &y) in reference[margin..frames - margin]
        .iter()
        .zip(&corrected[margin..frames - margin])
    {
        let (x, y) = (x as f64, y as f64);
        dot += x * y;
        aa += x * x;
        bb += y * y;
    }
    let agreement = dot / (aa * bb).sqrt();
    if !agreement.is_finite() || agreement < 0.6 {
        return Err(format!(
            "Recordings do not support reliable phase alignment (measured correlation {agreement:.3})"
        ));
    }
    // A periodic tone has several equally plausible lags. Refuse rather than
    // selecting an arbitrary period and reporting it as microphone delay.
    if (-(max_lag as i64)..=max_lag as i64)
        .any(|i| (i - coarse).abs() > 8 && at(i).abs() > at(coarse).abs() * 0.9)
    {
        return Err("Phase alignment is ambiguous: multiple comparable correlation peaks".into());
    }
    Ok(PhaseAlignment {
        lag_samples: lag,
        lag_seconds: lag / rate as f64,
        inverted_polarity: polarity < 0.0,
        correlation: agreement,
        analyzed_frames: frames,
    })
}

/// Advance (positive) or delay (negative) interleaved audio. The same 65-tap
/// Blackman-windowed sinc is used for every channel, preserving stereo phase.
pub fn shift_interleaved(
    input: &[f32],
    channels: usize,
    advance_samples: f64,
    invert: bool,
) -> Result<Vec<f32>, String> {
    if channels == 0
        || channels > 8
        || input.len() % channels != 0
        || !advance_samples.is_finite()
        || advance_samples.abs() > 19_200.0
        || input.iter().any(|v| !v.is_finite())
    {
        return Err("Invalid fractional audio shift".into());
    }
    let whole = advance_samples.floor() as i64;
    let fraction = advance_samples - whole as f64;
    let mut kernel = Vec::new();
    for k in -RADIUS..=RADIUS {
        let x = k as f64 - fraction;
        let sinc = if x.abs() < 1e-12 {
            1.0
        } else {
            (PI * x).sin() / (PI * x)
        };
        let window = 0.42
            + 0.5 * (PI * x / (RADIUS + 1) as f64).cos()
            + 0.08 * (2.0 * PI * x / (RADIUS + 1) as f64).cos();
        kernel.push(sinc * window);
    }
    let norm = kernel.iter().sum::<f64>() * if invert { -1.0 } else { 1.0 };
    let mut output = vec![0.0; input.len()];
    let frames = input.len() / channels;
    for (frame, dst) in output.chunks_exact_mut(channels).enumerate() {
        for (tap, &weight) in kernel.iter().enumerate() {
            let source = frame as i64 + whole + tap as i64 - RADIUS;
            if source >= 0 && source < frames as i64 {
                for c in 0..channels {
                    dst[c] += (input[source as usize * channels + c] as f64 * weight / norm) as f32;
                }
            }
        }
    }
    Ok(output)
}
