// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Offline dialogue-track energy keyed ducking. This is an energy detector,
//! not speech recognition: music on the designated dialogue track also keys it.
use saturn_core::project::AutomationPoint;
use saturn_core::{TICKS_PER_SECOND, Tick};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct DuckingOptions {
    pub duck_db: f64,
    pub lookahead_ms: u32,
    pub hold_ms: u32,
    pub release_ms: u32,
    pub threshold_dbfs: f64,
}

impl Default for DuckingOptions {
    fn default() -> Self {
        Self {
            duck_db: -12.0,
            lookahead_ms: 40,
            hold_ms: 150,
            release_ms: 300,
            threshold_dbfs: -38.0,
        }
    }
}

pub fn envelope(
    samples: &[f32],
    rate: u32,
    options: DuckingOptions,
) -> Result<Vec<AutomationPoint>, String> {
    if rate < 8000
        || rate > 192000
        || samples.is_empty()
        || samples.len() > rate as usize * 120
        || samples.iter().any(|v| !v.is_finite())
        || !options.duck_db.is_finite()
        || !(-60.0..=0.0).contains(&options.duck_db)
        || !options.threshold_dbfs.is_finite()
        || !(-80.0..=0.0).contains(&options.threshold_dbfs)
        || options.lookahead_ms > 1000
        || options.hold_ms > 5000
        || !(10..=5000).contains(&options.release_ms)
    {
        return Err("Invalid ducking settings or PCM (maximum duration 120 seconds)".into());
    }
    let window = rate as usize / 100;
    let threshold = 10_f64.powf(options.threshold_dbfs / 10.0);
    let active: Vec<_> = samples
        .chunks(window)
        .enumerate()
        .filter_map(|(i, chunk)| {
            let energy =
                chunk.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / chunk.len() as f64;
            (energy >= threshold).then_some((i * window, ((i + 1) * window).min(samples.len())))
        })
        .collect();
    let lookahead = rate as usize * options.lookahead_ms as usize / 1000;
    let hold = rate as usize * options.hold_ms as usize / 1000;
    let release = rate as usize * options.release_ms as usize / 1000;
    let attack = (rate as usize / 50).min(lookahead);
    // Merge activity+hold regions before ramp generation to avoid pumping on
    // adjacent detector windows and brief pauses in a spoken phrase.
    let mut regions: Vec<(usize, usize)> = Vec::new();
    for (start, end) in active {
        let start = start.saturating_sub(lookahead);
        let end = (end + hold).min(samples.len());
        if let Some(last) = regions.last_mut() {
            if start <= last.1 + release {
                last.1 = last.1.max(end);
                continue;
            }
        }
        regions.push((start, end));
    }
    let mut points = Vec::<AutomationPoint>::new();
    let mut push = |sample: usize, value: f64| {
        let time = Tick((sample as i128 * TICKS_PER_SECOND as i128 / rate as i128) as i64);
        if let Some(last) = points.last_mut() {
            if last.time == time {
                last.value = value;
                return;
            }
        }
        points.push(AutomationPoint { time, value });
    };
    push(0, 0.0);
    for (start, end) in regions {
        // Two adjacent samples represent a zero-lookahead step; overwriting
        // the start point alone would ramp down across the preceding silence.
        push(
            if attack == 0 {
                start.saturating_sub(1)
            } else {
                start
            },
            0.0,
        );
        push((start + attack).min(end), options.duck_db);
        push(end, options.duck_db);
        let release_end = (end + release).min(samples.len());
        let remaining = 1.0 - (release_end - end) as f64 / release as f64;
        push(release_end, options.duck_db * remaining.max(0.0));
    }
    if points.last().is_some_and(|p| {
        p.time.0 < (samples.len() as i128 * TICKS_PER_SECOND as i128 / rate as i128) as i64
    }) {
        let value = points.last().unwrap().value;
        points.push(AutomationPoint {
            time: Tick((samples.len() as i128 * TICKS_PER_SECOND as i128 / rate as i128) as i64),
            value,
        });
    }
    Ok(points)
}

/// Stereo-linked sample-peak lookahead limiter. The caller measures the
/// reconstructed true peak afterwards and applies any residual correction.
pub fn limit_lookahead(
    samples: &mut [f32],
    channels: usize,
    rate: u32,
    ceiling: f64,
) -> Result<(), String> {
    if channels == 0
        || channels > 8
        || samples.len() % channels != 0
        || rate < 8000
        || rate > 192000
        || !ceiling.is_finite()
        || !(0.0..=1.0).contains(&ceiling)
        || ceiling == 0.0
        || samples.iter().any(|v| !v.is_finite())
    {
        return Err("Invalid limiter PCM or ceiling".into());
    }
    let frames = samples.len() / channels;
    let peaks: Vec<f64> = samples
        .chunks_exact(channels)
        .map(|f| f.iter().map(|v| (*v as f64).abs()).fold(0.0, f64::max))
        .collect();
    let lookahead = rate as usize / 200;
    let release = (-1.0 / (rate as f64 * 0.1)).exp();
    let mut queue = std::collections::VecDeque::<usize>::new();
    let mut next = 0;
    let mut gain = 1.0_f64;
    for i in 0..frames {
        while next < (i + lookahead + 1).min(frames) {
            while queue.back().is_some_and(|&j| peaks[j] <= peaks[next]) {
                queue.pop_back();
            }
            queue.push_back(next);
            next += 1;
        }
        while queue.front().is_some_and(|&j| j < i) {
            queue.pop_front();
        }
        let target = (ceiling / peaks[*queue.front().unwrap()].max(1e-12)).min(1.0);
        gain = if target < gain {
            target
        } else {
            release * gain + (1.0 - release) * target
        };
        for v in &mut samples[i * channels..(i + 1) * channels] {
            *v *= gain as f32;
        }
    }
    Ok(())
}
