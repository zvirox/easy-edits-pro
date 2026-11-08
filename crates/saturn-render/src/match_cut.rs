// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode/analyze outside the engine lock; the core validates and applies one slip.
use saturn_core::audio_edit::snapshot_digest;
use saturn_core::match_cut::{MatchCutMode, MatchCutParams, MatchCutPlan, plan, validate_choice};
use saturn_core::{CommandRequest, ProjectDocument, TICKS_PER_SECOND, Tick};
use saturn_perception::match_cut::{
    attack_distance, attacks, distance, focal_position, motion_vector,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct PreparedMatchCut {
    pub command: CommandRequest,
    pub telemetry: Value,
}

pub fn prepare(doc: &ProjectDocument, params: &Value) -> Result<PreparedMatchCut, String> {
    let p: MatchCutParams = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let plan = plan(doc, &p)?;
    let (source_in, mut telemetry) = match p.mode {
        MatchCutMode::EyeTrace | MatchCutMode::Motion => visual(&plan, &p)?,
        MatchCutMode::Transient => transient(&plan)?,
    };
    validate_choice(&plan, p.mode, source_in)?;
    telemetry["search_min_source_in"] = json!(plan.min_source_in);
    telemetry["search_max_source_in"] = json!(plan.max_source_in);
    telemetry["mode"] = json!(p.mode);
    telemetry["measured_on"] = json!(
        "raw source media; existing grading, processing and other tracks are outside the measurement domain"
    );
    let mut params = serde_json::to_value(&p).map_err(|e| e.to_string())?;
    params["expected_digest"] = json!(snapshot_digest(doc)?);
    params["matched_source_in"] = json!(source_in);
    Ok(PreparedMatchCut {
        command: CommandRequest::new("edit.match_cut_align", params),
        telemetry,
    })
}

fn visual(plan: &MatchCutPlan<'_>, p: &MatchCutParams) -> Result<(Tick, Value), String> {
    // Center samples represent the visible frame interval, avoiding ambiguous
    // shared endpoints in containers with millisecond-rounded timestamps.
    let center = plan.frame_ticks / 2;
    let outgoing = plan
        .left_end
        .checked_sub(plan.frame_ticks - center)
        .ok_or("Outgoing frame overflow")?;
    let reference_ticks = if p.mode == MatchCutMode::Motion {
        vec![
            outgoing
                .checked_sub(plan.frame_ticks)
                .ok_or("Outgoing frame overflow")?,
            outgoing,
        ]
    } else {
        vec![outgoing]
    };
    let reference =
        crate::frame_extractor::match_cut_frames(&plan.left_media.path, &reference_ticks)?;
    let feature = if p.mode == MatchCutMode::Motion {
        motion_vector(&reference[0], &reference[1])?
    } else {
        focal_position(&reference[0])?
    }
    .ok_or("Outgoing shot has no measurable contrast or motion for this mode")?;
    let candidates: Vec<Tick> = (-(p.search_frames as i64)..=p.search_frames as i64)
        .filter_map(|i| plan.right.source_in.checked_add(i * plan.frame_ticks))
        .filter(|t| *t >= plan.min_source_in && *t <= plan.max_source_in)
        .collect();
    let mut ticks: BTreeSet<Tick> = candidates.iter().copied().collect();
    if p.mode == MatchCutMode::Motion {
        for tick in &candidates {
            ticks.insert(
                tick.checked_add(plan.frame_ticks)
                    .ok_or("Incoming frame overflow")?,
            );
        }
    }
    let ticks: Vec<_> = ticks.into_iter().collect();
    let sample_ticks: Vec<Tick> = ticks
        .iter()
        .map(|t| t.checked_add(center).ok_or("Incoming frame overflow"))
        .collect::<Result<_, _>>()?;
    let frames = crate::frame_extractor::match_cut_frames(&plan.right_media.path, &sample_ticks)?;
    let mut best: Option<(Tick, f64, [f64; 2])> = None;
    let mut before = None;
    let mut measured = 0;
    for tick in &candidates {
        let index = ticks
            .binary_search(tick)
            .map_err(|_| "Missing sampled frame")?;
        let candidate = if p.mode == MatchCutMode::Motion {
            let next = ticks
                .binary_search(&Tick(tick.0 + plan.frame_ticks))
                .map_err(|_| "Missing neighbor frame")?;
            motion_vector(&frames[index], &frames[next])?
        } else {
            focal_position(&frames[index])?
        };
        let Some(candidate) = candidate else {
            continue;
        };
        measured += 1;
        let error = distance(feature, candidate);
        if *tick == plan.right.source_in {
            before = Some(error);
        }
        if best.is_none_or(|(old, score, _)| {
            better(
                error,
                tick.0 - plan.right.source_in.0,
                score,
                old.0 - plan.right.source_in.0,
            )
        }) {
            best = Some((*tick, error, candidate));
        }
    }
    let (tick, after, selected) =
        best.ok_or("Incoming search range has no measurable contrast or motion")?;
    Ok((
        tick,
        json!({"method": if p.mode == MatchCutMode::Motion {
        "mean block-displacement vector matching; static shots excluded"
    } else { "squared-contrast centroid heuristic; not measured gaze or subject identity" },
        "reference_feature": feature, "selected_feature": selected,
        "distance_before": before, "distance_after": after,
        "candidate_count": candidates.len(), "measured_candidates": measured,
        "sample_size": [160,90], "frame_step_ticks": plan.frame_ticks,
        "sample_position": "center of each project-frame interval", "motion_search_pixels": 8}),
    ))
}

fn better(score: f64, delta: i64, old_score: f64, old_delta: i64) -> bool {
    score < old_score - 1e-9
        || ((score - old_score).abs() <= 1e-9
            && (delta.unsigned_abs(), delta) < (old_delta.unsigned_abs(), old_delta))
}

fn transient(plan: &MatchCutPlan<'_>) -> Result<(Tick, Value), String> {
    let sample_ticks = TICKS_PER_SECOND / 16_000;
    let reference_start = plan
        .left_end
        .saturating_sub(TICKS_PER_SECOND / 4)
        .max(plan.left.source_in);
    let reference_start = Tick(
        reference_start
            .0
            .checked_add(sample_ticks - 1)
            .ok_or("Audio search overflow")?
            / sample_ticks
            * sample_ticks,
    );
    let incoming_start = plan
        .min_source_in
        .saturating_sub(TICKS_PER_SECOND / 25)
        .max(Tick(0));
    let incoming_start = Tick(incoming_start.0 / sample_ticks * sample_ticks);
    let incoming_end = plan
        .max_source_in
        .checked_add(TICKS_PER_SECOND * 3 / 50)
        .ok_or("Audio search overflow")?;
    let decode = |path: &std::path::Path, start, end: Tick| -> Result<Vec<f32>, String> {
        let decoded = crate::audio_pcm::decode_audio_pcm_constrained(
            path,
            start,
            Some(end),
            Some(16_000),
            Some(1),
        )?;
        let expected = ((end.0 - start.0) as i128 * 16_000 / TICKS_PER_SECOND as i128) as usize;
        if decoded.samples.len() + 1 < expected {
            return Err("Source audio ends inside the match-cut search range".into());
        }
        Ok(decoded.samples)
    };
    let outgoing_pcm = decode(&plan.left_media.path, reference_start, plan.left_end)?;
    let incoming_pcm = decode(&plan.right_media.path, incoming_start, incoming_end)?;
    let references = attacks(&outgoing_pcm, 16_000)?;
    let reference = references
        .iter()
        .max_by(|a, b| a.strength.total_cmp(&b.strength))
        .ok_or("No outgoing transient with a 40 ms attack envelope in the final 250 ms")?;
    let incoming = attacks(&incoming_pcm, 16_000)?;
    let mut best = None;
    let mut nearest = None;
    let mut measured = 0;
    for attack in &incoming {
        let tick = incoming_start
            .checked_add(attack.sample as i64 * (TICKS_PER_SECOND / 16_000))
            .ok_or("Attack timing overflow")?;
        if tick < plan.min_source_in || tick > plan.max_source_in {
            continue;
        }
        measured += 1;
        let delta = tick.0 - plan.right.source_in.0;
        nearest = Some(nearest.map_or(delta.unsigned_abs(), |old: u64| {
            old.min(delta.unsigned_abs())
        }));
        let score = attack_distance(reference, attack);
        if best.is_none_or(|(old, old_score): (Tick, f64)| {
            better(score, delta, old_score, old.0 - plan.right.source_in.0)
        }) {
            best = Some((tick, score));
        }
    }
    let (tick, score) = best.ok_or("No incoming transient inside the available search handles")?;
    Ok((
        tick,
        json!({"method": "positive 5 ms RMS rises; cosine match of 40 ms attack envelopes; selected incoming onset placed at cut",
        "reference_attack_source_tick": reference_start.0 + reference.sample as i64 * (TICKS_PER_SECOND / 16_000),
        "attack_shape_distance": score, "nearest_onset_distance_before_ticks": nearest,
        "selected_onset_distance_after_ticks": 0, "measured_candidates": measured, "sample_rate": 16_000,
        "limitation": "gain-independent envelope heuristic; not instrument recognition or an audio crossfade"}),
    ))
}
