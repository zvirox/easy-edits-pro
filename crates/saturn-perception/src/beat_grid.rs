// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Constant-pulse estimation from measured RMS rises. This is a percussion/
//! energy heuristic, not meter recognition, downbeat detection or a neural beat tracker.
use serde::Serialize;
use std::collections::BTreeMap;

const RATE: usize = 16_000;
const HOP: usize = 160;

#[derive(Clone, Debug, Serialize)]
pub struct PulseGrid {
    pub origin_sample: f64,
    pub period_samples: f64,
    pub beats: u32,
    pub onset_count: usize,
    pub matched_onsets: usize,
    pub periodicity: f64,
    pub onset_support: f64,
    pub beat_occupancy: f64,
    pub rms_timing_error_ms: f64,
}

#[derive(Clone, Copy)]
struct Onset {
    sample: f64,
    strength: f64,
}

fn onsets(pcm: &[f32]) -> Vec<Onset> {
    let rms: Vec<f64> = pcm
        .chunks_exact(HOP)
        .map(|v| (v.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / HOP as f64).sqrt())
        .collect();
    let rise: Vec<f64> = rms.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();
    let peak = rise.iter().copied().fold(0.0, f64::max);
    if peak < 0.001 {
        return Vec::new();
    }
    let mut result: Vec<Onset> = Vec::new();
    for i in 1..rise.len() - 1 {
        if rise[i] < peak * 0.12 || rise[i] < rise[i - 1] || rise[i] <= rise[i + 1] {
            continue;
        }
        let block = i + 1;
        let start = (block - 1) * HOP;
        let threshold = (rms[block - 1] * 3.0).max(rms[block] * 0.2);
        let Some(offset) = pcm[start..(block + 1) * HOP]
            .iter()
            .position(|v| (*v as f64).abs() > threshold)
        else {
            continue;
        };
        let hit = Onset {
            sample: (start + offset) as f64,
            strength: rise[i],
        };
        if let Some(last) = result.last_mut() {
            if hit.sample - last.sample < (RATE as f64 * 0.08) {
                if hit.strength > last.strength {
                    *last = hit;
                }
                continue;
            }
        }
        result.push(hit);
    }
    result
}

fn assignments(hits: &[Onset], origin: f64, period: f64) -> BTreeMap<i64, Onset> {
    let mut slots: BTreeMap<i64, Onset> = BTreeMap::new();
    let tolerance = (period * 0.06).max(2.0 * HOP as f64);
    for &hit in hits {
        let index = ((hit.sample - origin) / period).round() as i64;
        let distance = (hit.sample - origin - index as f64 * period).abs();
        if distance > tolerance {
            continue;
        }
        if slots
            .get(&index)
            .is_none_or(|old| (old.sample - origin - index as f64 * period).abs() > distance)
        {
            slots.insert(index, hit);
        }
    }
    slots
}

fn fit(slots: &BTreeMap<i64, Onset>) -> Option<(f64, f64)> {
    let weight = slots.values().map(|v| v.strength).sum::<f64>();
    if weight <= 0.0 || slots.len() < 6 {
        return None;
    }
    let x = slots
        .iter()
        .map(|(i, v)| *i as f64 * v.strength)
        .sum::<f64>()
        / weight;
    let y = slots.values().map(|v| v.sample * v.strength).sum::<f64>() / weight;
    let xx = slots
        .iter()
        .map(|(i, v)| (*i as f64 - x).powi(2) * v.strength)
        .sum::<f64>();
    if xx <= 0.0 {
        return None;
    }
    let period = slots
        .iter()
        .map(|(i, v)| (*i as f64 - x) * (v.sample - y) * v.strength)
        .sum::<f64>()
        / xx;
    Some((y - period * x, period))
}

pub fn estimate(pcm: &[f32], rate: u32) -> Result<PulseGrid, String> {
    if rate != RATE as u32
        || !(4 * RATE..=120 * RATE).contains(&pcm.len())
        || pcm.iter().any(|v| !v.is_finite())
    {
        return Err("Beat analysis needs 4 to 120 seconds of finite 16 kHz mono PCM".into());
    }
    let hits = onsets(pcm);
    if hits.len() < 6 {
        return Err("No usable pulse train: provide at least six clear music attacks".into());
    }
    // Autocorrelation of measured, lightly spread attack impulses. Normalize by
    // total energy, not overlap energy, to favor the fundamental over its repeats.
    let mut envelope = vec![0.0; pcm.len() / HOP + 2];
    for hit in &hits {
        let i = (hit.sample / HOP as f64).round() as usize;
        for (offset, weight) in [(0, 0.5), (1, 1.0), (2, 0.5)] {
            envelope[i + offset] += hit.strength * weight;
        }
    }
    let energy = envelope.iter().map(|v| v * v).sum::<f64>();
    let mut lag = 0;
    let mut periodicity = 0.0;
    for candidate in 30..=100 {
        let score = envelope[candidate..]
            .iter()
            .zip(&envelope)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / energy;
        if score > periodicity {
            lag = candidate;
            periodicity = score;
        }
    }
    if periodicity < 0.35 {
        return Err(
            "Music has no stable 60-200 BPM energy pulse; choose a steadier section".into(),
        );
    }
    let coarse = (lag * HOP) as f64;
    // Refine coarse 10 ms lag with measured intervals before phase fitting;
    // otherwise small tempo rounding drifts across a long music section.
    let mut intervals: Vec<f64> = hits
        .windows(2)
        .filter_map(|pair| {
            let distance = pair[1].sample - pair[0].sample;
            let multiple = (distance / coarse).round();
            let period = distance / multiple;
            ((1.0..=3.0).contains(&multiple) && (period - coarse).abs() < coarse * 0.08)
                .then_some(period)
        })
        .collect();
    if intervals.len() < 4 {
        return Err("Too few consistent music intervals; choose a steadier section".into());
    }
    intervals.sort_by(f64::total_cmp);
    let median = intervals[intervals.len() / 2];
    let near: Vec<f64> = intervals
        .into_iter()
        .filter(|v| (v - median).abs() < median * 0.03)
        .collect();
    let period = near.iter().sum::<f64>() / near.len() as f64;
    let mut seeds = hits.clone();
    seeds.sort_by(|a, b| {
        b.strength
            .total_cmp(&a.strength)
            .then_with(|| a.sample.total_cmp(&b.sample))
    });
    let total_strength = hits.iter().map(|v| v.strength).sum::<f64>();
    let mut best: Option<(f64, f64, BTreeMap<i64, Onset>, f64)> = None;
    for seed in seeds.iter().take(32) {
        let initial = assignments(&hits, seed.sample, period);
        let Some((origin, period)) = fit(&initial) else {
            continue;
        };
        // Permit sub-sample floating-point error at the tempo endpoints.
        if !(RATE as f64 * 0.3 - 1e-6..=RATE as f64 + 1e-6).contains(&period) {
            continue;
        }
        let period = period.clamp(RATE as f64 * 0.3, RATE as f64);
        let slots = assignments(&hits, origin, period);
        if slots.len() < 6 {
            continue;
        }
        let support = slots.values().map(|v| v.strength).sum::<f64>() / total_strength;
        if best.as_ref().is_none_or(|(_, _, _, old)| support > *old) {
            best = Some((origin, period, slots, support));
        }
    }
    let Some((origin, period, slots, support)) = best else {
        return Err("No stable pulse fit; choose another music section".into());
    };
    let first = *slots.first_key_value().unwrap().0;
    let last = *slots.last_key_value().unwrap().0;
    let beats = (last - first + 1) as u32;
    let occupancy = slots.len() as f64 / beats as f64;
    let rms = (slots
        .iter()
        .map(|(i, v)| (v.sample - origin - *i as f64 * period).powi(2))
        .sum::<f64>()
        / slots.len() as f64)
        .sqrt();
    let long_gap = slots
        .keys()
        .zip(slots.keys().skip(1))
        .any(|(a, b)| b - a > 3);
    let start = origin + first as f64 * period;
    let end = origin + last as f64 * period;
    if slots.len() < 6
        || support < 0.70
        || occupancy < 0.65
        || rms > period * 0.04
        || long_gap
        || start < 0.0
        || end >= pcm.len() as f64
    {
        return Err(
            "Music pulse fit is sparse or irregular; choose a steadier section with clear attacks"
                .into(),
        );
    }
    Ok(PulseGrid {
        origin_sample: start,
        period_samples: period,
        beats,
        onset_count: hits.len(),
        matched_onsets: slots.len(),
        periodicity,
        onset_support: support,
        beat_occupancy: occupancy,
        rms_timing_error_ms: rms / RATE as f64 * 1000.0,
    })
}

#[cfg(test)]
#[path = "beat_grid_tests.rs"]
mod tests;
