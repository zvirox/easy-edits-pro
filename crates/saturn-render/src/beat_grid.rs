// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode the bounded music reference off-lock, estimate pulses, prepare one roll batch.
use saturn_core::{CommandRequest, ProjectDocument, TICKS_PER_SECOND, Tick};
use saturn_core::{
    audio_edit::snapshot_digest,
    beat_grid::{BeatGridParams, MeasuredBeatGrid, plan, quantized_clips},
};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct PreparedBeatGrid {
    pub command: CommandRequest,
    pub telemetry: Value,
}

pub fn prepare(doc: &ProjectDocument, params: &Value) -> Result<PreparedBeatGrid, String> {
    let p: BeatGridParams = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let plan = plan(doc, &p)?;
    let sample_ticks = TICKS_PER_SECOND / 16_000;
    let start = plan
        .music_clip
        .source_in
        .checked_add(plan.analysis_start.0 - plan.music_clip.timeline_start.0)
        .ok_or("Music source overflow")?;
    // Start at the next real 16 kHz sample and report that exact timeline origin.
    let start = Tick(
        start
            .0
            .checked_add(sample_ticks - 1)
            .ok_or("Music source overflow")?
            / sample_ticks
            * sample_ticks,
    );
    let timeline_start = plan
        .music_clip
        .timeline_start
        .checked_add(start.0 - plan.music_clip.source_in.0)
        .ok_or("Music timeline overflow")?;
    let end = start
        .checked_add(plan.analysis_end.0 - timeline_start.0)
        .ok_or("Music source overflow")?;
    let decoded = crate::audio_pcm::decode_audio_pcm_constrained(
        &plan.music_media.path,
        start,
        Some(end),
        Some(16_000),
        Some(1),
    )?;
    let expected = ((end.0 - start.0) / sample_ticks) as usize;
    if decoded.samples.len() + 1 < expected {
        return Err("Music source ends inside the analysis range; relink or trim it".into());
    }
    // Decoder budgets round up to whole samples; do not analyze past the range.
    let samples = &decoded.samples[..decoded.samples.len().min(expected)];
    let estimate = saturn_perception::beat_grid::estimate(samples, 16_000)?;
    let grid = MeasuredBeatGrid {
        origin: timeline_start
            .checked_add((estimate.origin_sample * sample_ticks as f64).round() as i64)
            .ok_or("Beat origin overflow")?,
        period: Tick((estimate.period_samples * sample_ticks as f64).round() as i64),
        beats: estimate.beats,
    };
    // Fail infeasible edits before returning a prepared command; core repeats
    // this check against its live snapshot before the single checkpoint.
    quantized_clips(doc, &p, &grid)?;
    let telemetry = json!({"method":"RMS-rise autocorrelation and weighted constant-pulse fit; heuristic",
        "tempo_bpm":60.0*16_000.0/estimate.period_samples,"grid":grid,"measurements":estimate,
        "analysis_start_tick":timeline_start,"analysis_end_tick":plan.analysis_end,
        "downbeat_tick":p.downbeat_tick,"meter":if p.subdivision==saturn_core::beat_grid::Subdivision::Quarter { Value::Null } else { json!("caller-supplied downbeat; 4/4 assumed") },
        "limitation":"raw first audio stream mixed to mono; no meter recognition; half/double-tempo ambiguity and syncopation can defeat this energy heuristic"});
    let mut params = serde_json::to_value(&p).map_err(|e| e.to_string())?;
    params["expected_digest"] = json!(snapshot_digest(doc)?);
    params["grid"] = json!(grid);
    Ok(PreparedBeatGrid {
        command: CommandRequest::new("timeline.beat_grid_quantize", params),
        telemetry,
    })
}
