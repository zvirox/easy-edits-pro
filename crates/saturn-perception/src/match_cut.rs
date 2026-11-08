// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Measured match-cut features. Focal position and attack matching are labelled
//! heuristics, not gaze prediction, subject identity or perceptual quality scores.
use crate::shot_detect::{SampledFrame, mean_block_displacement};

fn luma(frame: &SampledFrame) -> Result<Vec<f64>, String> {
    let size = (frame.width as usize)
        .checked_mul(frame.height as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or("Frame size overflow")?;
    if frame.width < 2 || frame.height < 2 || size != frame.rgb.len() || size > 640 * 640 * 3 {
        return Err("Match-cut features need bounded, tightly packed RGB frames".into());
    }
    Ok(frame
        .rgb
        .chunks_exact(3)
        .map(|p| (0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64) / 255.0)
        .collect())
}

/// Squared contrast about frame mean, normalized screen coordinates. Uniform
/// frames have no focal measurement; they never acquire a synthetic center.
pub fn focal_position(frame: &SampledFrame) -> Result<Option<[f64; 2]>, String> {
    let values = luma(frame)?;
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let (mut mass, mut x, mut y) = (0.0, 0.0, 0.0);
    for (i, value) in values.iter().enumerate() {
        let w = (value - mean).powi(2);
        mass += w;
        x += w * (i % frame.width as usize) as f64 / (frame.width - 1) as f64;
        y += w * (i / frame.width as usize) as f64 / (frame.height - 1) as f64;
    }
    Ok((mass / values.len() as f64 > 0.0001).then(|| [x / mass, y / mass]))
}

/// Mean block displacement within one shot, in frame-width/height units per
/// project frame. Static/textureless shots have no match-on-action measurement.
pub fn motion_vector(a: &SampledFrame, b: &SampledFrame) -> Result<Option<[f64; 2]>, String> {
    if focal_position(a)?.is_none() || focal_position(b)?.is_none() {
        return Ok(None);
    }
    Ok(mean_block_displacement(a, b)
        .filter(|v| v[0].hypot(v[1]) >= 0.1)
        .map(|v| [v[0] / a.width as f64, v[1] / a.height as f64]))
}

pub fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[derive(Clone, Debug)]
pub struct Attack {
    pub sample: usize,
    pub strength: f64,
    pub envelope: [f64; 8],
}

/// Positive 5 ms RMS rises, with an absolute -60 dBFS floor and 15% of peak
/// peak RMS threshold for rises. A 40 ms post-attack envelope describes each candidate.
pub fn attacks(pcm: &[f32], rate: u32) -> Result<Vec<Attack>, String> {
    if rate != 16_000 || pcm.len() > 5 * rate as usize || pcm.iter().any(|v| !v.is_finite()) {
        return Err(
            "Transient matching needs at most five seconds of finite 16 kHz mono PCM".into(),
        );
    }
    let hop = rate as usize / 200;
    let rms: Vec<f64> = pcm
        .chunks_exact(hop)
        .map(|c| (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / hop as f64).sqrt())
        .collect();
    let peak = rms.iter().copied().fold(0.0_f64, f64::max);
    if peak < 0.001 || rms.len() < 10 {
        return Ok(Vec::new());
    }
    let mut result: Vec<Attack> = Vec::new();
    for i in 1..rms.len() - 1 {
        let rise = rms[i] - rms[i - 1];
        if rise < peak * 0.15 || rise <= (rms[i + 1] - rms[i]) {
            continue;
        }
        // Refine the selected energy block to the first sample crossing a local
        // amplitude threshold. Search one preceding block for split-block attacks.
        let start = (i - 1) * hop;
        let threshold = (rms[i - 1] * 3.0).max(rms[i] * 0.2);
        let Some(offset) = pcm[start..(i + 1) * hop]
            .iter()
            .position(|v| (*v as f64).abs() > threshold)
        else {
            continue;
        };
        let sample = start + offset;
        if sample + 8 * hop > pcm.len() {
            continue;
        }
        if result
            .last()
            .is_some_and(|a| sample.saturating_sub(a.sample) < 8 * hop)
        {
            continue;
        }
        let mut envelope = [0.0; 8];
        for (j, value) in envelope.iter_mut().enumerate() {
            *value = (pcm[sample + j * hop..sample + (j + 1) * hop]
                .iter()
                .map(|v| (*v as f64).powi(2))
                .sum::<f64>()
                / hop as f64)
                .sqrt();
        }
        result.push(Attack {
            sample,
            strength: rise,
            envelope,
        });
    }
    Ok(result)
}

/// Cosine mismatch of post-attack RMS envelopes (gain independent), 0 = same
/// shape. This cannot identify an instrument or guarantee inaudible edits.
pub fn attack_distance(a: &Attack, b: &Attack) -> f64 {
    let dot = a
        .envelope
        .iter()
        .zip(b.envelope)
        .map(|(a, b)| a * b)
        .sum::<f64>();
    let norm = |v: &[f64; 8]| v.iter().map(|v| v * v).sum::<f64>().sqrt();
    1.0 - (dot / (norm(&a.envelope) * norm(&b.envelope)).max(1e-12)).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "match_cut_tests.rs"]
mod tests;
