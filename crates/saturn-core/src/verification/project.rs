// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Project-level verification checks: target duration, resolution/aspect ratio, and timeline silence (AF3).

use serde_json::json;

use crate::project::ProjectDocument;
use crate::time::{Tick, TICKS_PER_SECOND};
use crate::TrackKind;
use super::types::{CheckEvidence, CheckStatus, CheckVerdict, TickRange, VerifyPreset};

/// Verifies sequence total duration against human-owned project brief target duration.
pub fn verify_target_duration(
    document: &ProjectDocument,
    _preset: &VerifyPreset,
    _start_tick: Tick,
    _end_tick: Tick,
) -> CheckEvidence {
    let brief = &document.project.brief;
    let sequence_duration_ticks = document.project.sequence.duration().0.max(0) as u64;

    let Some(target_ticks) = brief.target_duration_ticks else {
        return CheckEvidence {
            check_id: "project.target_duration".into(),
            name: "Project Target Duration".into(),
            verdict: CheckVerdict::Pass,
            status: CheckStatus::Skipped,
            measured: Some(json!({
                "sequence_duration_ticks": sequence_duration_ticks,
                "sequence_duration_seconds": sequence_duration_ticks as f64 / TICKS_PER_SECOND as f64
            })),
            threshold: None,
            message: "No target duration configured in project brief; check skipped.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 100.0,
        };
    };

    let tolerance_ticks = brief.duration_tolerance_ticks.unwrap_or(0);
    let min_allowed = target_ticks.saturating_sub(tolerance_ticks);
    let max_allowed = target_ticks.saturating_add(tolerance_ticks);

    let within_tolerance = sequence_duration_ticks >= min_allowed && sequence_duration_ticks <= max_allowed;
    let diff_ticks = (sequence_duration_ticks as i128 - target_ticks as i128).unsigned_abs() as u64;
    let diff_seconds = diff_ticks as f64 / TICKS_PER_SECOND as f64;

    let (verdict, message, offending_ranges) = if within_tolerance {
        (
            CheckVerdict::Pass,
            format!(
                "Sequence duration ({:.2}s, {} ticks) matches brief target ({:.2}s ±{:.2}s).",
                sequence_duration_ticks as f64 / TICKS_PER_SECOND as f64,
                sequence_duration_ticks,
                target_ticks as f64 / TICKS_PER_SECOND as f64,
                tolerance_ticks as f64 / TICKS_PER_SECOND as f64
            ),
            Vec::new(),
        )
    } else {
        let is_over = sequence_duration_ticks > max_allowed;
        let direction = if is_over { "longer" } else { "shorter" };
        let msg = format!(
            "Sequence duration ({:.2}s) is {} than brief target ({:.2}s ±{:.2}s) by {:.2}s.",
            sequence_duration_ticks as f64 / TICKS_PER_SECOND as f64,
            direction,
            target_ticks as f64 / TICKS_PER_SECOND as f64,
            tolerance_ticks as f64 / TICKS_PER_SECOND as f64,
            diff_seconds
        );
        let offending = vec![TickRange {
            start_tick: min_allowed.min(sequence_duration_ticks),
            end_tick: max_allowed.max(sequence_duration_ticks),
            clip_id: None,
            detail: msg.clone(),
        }];
        (CheckVerdict::Fail, msg, offending)
    };

    CheckEvidence {
        check_id: "project.target_duration".into(),
        name: "Project Target Duration".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({
            "sequence_duration_ticks": sequence_duration_ticks,
            "sequence_duration_seconds": sequence_duration_ticks as f64 / TICKS_PER_SECOND as f64,
            "delta_seconds": if sequence_duration_ticks >= target_ticks { diff_seconds } else { -diff_seconds }
        })),
        threshold: Some(json!({
            "target_duration_ticks": target_ticks,
            "tolerance_ticks": tolerance_ticks,
            "min_allowed_ticks": min_allowed,
            "max_allowed_ticks": max_allowed
        })),
        message,
        offending_ranges,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies sequence resolution and aspect ratio against delivery preset expectations.
pub fn verify_resolution_aspect(
    document: &ProjectDocument,
    preset: &VerifyPreset,
) -> CheckEvidence {
    let settings = &document.project.settings;
    let width = settings.width;
    let height = settings.height;

    let is_exact_dim = width == preset.width && height == preset.height;
    
    // Calculate aspect ratio comparison
    let current_ratio = if height > 0 { width as f64 / height as f64 } else { 0.0 };
    let expected_ratio = if preset.height > 0 { preset.width as f64 / preset.height as f64 } else { 0.0 };
    let ratio_diff = (current_ratio - expected_ratio).abs();
    let aspect_matches = ratio_diff < 0.02;

    let (verdict, message, offending_ranges) = if is_exact_dim {
        (
            CheckVerdict::Pass,
            format!(
                "Project resolution ({}x{}) and aspect ratio ({}) match delivery preset `{}`.",
                width, height, preset.aspect_ratio, preset.name
            ),
            Vec::new(),
        )
    } else if aspect_matches {
        (
            CheckVerdict::Warn,
            format!(
                "Project aspect ratio matches ({}), but resolution is {}x{} instead of preset `{}` default ({}x{}).",
                preset.aspect_ratio, width, height, preset.name, preset.width, preset.height
            ),
            vec![TickRange {
                start_tick: 0,
                end_tick: document.project.sequence.duration().0.max(0) as u64,
                clip_id: None,
                detail: format!("Scaled resolution: {}x{} vs preset target {}x{}", width, height, preset.width, preset.height),
            }],
        )
    } else {
        let msg = format!(
            "Project resolution {}x{} (ratio {:.2}) mismatches delivery preset `{}` target {}x{} (ratio {:.2}, {}).",
            width, height, current_ratio, preset.name, preset.width, preset.height, expected_ratio, preset.aspect_ratio
        );
        let offending = vec![TickRange {
            start_tick: 0,
            end_tick: document.project.sequence.duration().0.max(0) as u64,
            clip_id: None,
            detail: msg.clone(),
        }];
        (CheckVerdict::Fail, msg, offending)
    };

    CheckEvidence {
        check_id: "project.resolution_aspect".into(),
        name: "Resolution and Aspect Ratio".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({
            "width": width,
            "height": height,
            "aspect_ratio": format!("{:.2}", current_ratio)
        })),
        threshold: Some(json!({
            "expected_width": preset.width,
            "expected_height": preset.height,
            "expected_aspect_ratio": preset.aspect_ratio
        })),
        message,
        offending_ranges,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies timeline audio silence/dead-air gaps against preset duration limits.
pub fn verify_timeline_silence(
    document: &ProjectDocument,
    preset: &VerifyPreset,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let max_silence_ticks = (preset.max_silence_duration_seconds * TICKS_PER_SECOND as f64) as u64;
    let range_start = start_tick.0.max(0) as u64;
    let range_end = end_tick.0.max(range_start as i64) as u64;

    if range_end <= range_start {
        return CheckEvidence {
            check_id: "project.timeline_silence".into(),
            name: "Timeline Silence / Dead Air".into(),
            verdict: CheckVerdict::Pass,
            status: CheckStatus::Measured,
            measured: Some(json!({"gaps_count": 0, "max_gap_seconds": 0.0})),
            threshold: Some(json!({"max_silence_seconds": preset.max_silence_duration_seconds})),
            message: "Empty timeline range analyzed.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 100.0,
        };
    }

    // Collect all audio intervals across tracks
    let mut intervals: Vec<(u64, u64)> = Vec::new();
    for track in &document.project.sequence.tracks {
        let is_audio_track = track.kind == TrackKind::Audio;
        for clip in &track.clips {
            if !clip.enabled {
                continue;
            }
            if is_audio_track || clip.audio_enabled {
                let clip_start = clip.timeline_start.0.max(0) as u64;
                let clip_end = clip_start.saturating_add(clip.duration.0.max(0) as u64);
                if clip_end > range_start && clip_start < range_end {
                    intervals.push((clip_start.max(range_start), clip_end.min(range_end)));
                }
            }
        }
    }

    intervals.sort_by_key(|&(s, _)| s);

    // Merge overlapping audio intervals
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        } else {
            merged.push((start, end));
        }
    }

    // Find gaps
    let mut offending = Vec::new();
    let mut current_cursor = range_start;
    let mut max_observed_gap_ticks = 0u64;

    for (audio_start, audio_end) in &merged {
        if *audio_start > current_cursor {
            let gap = audio_start - current_cursor;
            if gap > max_observed_gap_ticks {
                max_observed_gap_ticks = gap;
            }
            if gap > max_silence_ticks {
                let gap_seconds = gap as f64 / TICKS_PER_SECOND as f64;
                offending.push(TickRange {
                    start_tick: current_cursor,
                    end_tick: *audio_start,
                    clip_id: None,
                    detail: format!(
                        "Silence / dead air of {:.2}s ({} ticks) exceeds preset limit of {:.2}s",
                        gap_seconds, gap, preset.max_silence_duration_seconds
                    ),
                });
            }
        }
        current_cursor = current_cursor.max(*audio_end);
    }

    // Check trailing gap
    if range_end > current_cursor {
        let trailing_gap = range_end - current_cursor;
        if trailing_gap > max_observed_gap_ticks {
            max_observed_gap_ticks = trailing_gap;
        }
        if trailing_gap > max_silence_ticks {
            let gap_seconds = trailing_gap as f64 / TICKS_PER_SECOND as f64;
            offending.push(TickRange {
                start_tick: current_cursor,
                end_tick: range_end,
                clip_id: None,
                detail: format!(
                    "Trailing silence of {:.2}s ({} ticks) exceeds preset limit of {:.2}s",
                    gap_seconds, trailing_gap, preset.max_silence_duration_seconds
                ),
            });
        }
    }

    let max_gap_seconds = max_observed_gap_ticks as f64 / TICKS_PER_SECOND as f64;
    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Warn
    };

    let message = if offending.is_empty() {
        format!(
            "No silence gaps exceed preset limit of {:.2}s (max gap observed: {:.2}s).",
            preset.max_silence_duration_seconds, max_gap_seconds
        )
    } else {
        format!(
            "{} silence / dead-air gap(s) exceed maximum threshold of {:.2}s (longest: {:.2}s).",
            offending.len(), preset.max_silence_duration_seconds, max_gap_seconds
        )
    };

    CheckEvidence {
        check_id: "project.timeline_silence".into(),
        name: "Timeline Silence / Dead Air".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({
            "offending_gaps_count": offending.len(),
            "max_gap_seconds": max_gap_seconds,
            "max_gap_ticks": max_observed_gap_ticks
        })),
        threshold: Some(json!({
            "max_silence_duration_seconds": preset.max_silence_duration_seconds,
            "max_silence_ticks": max_silence_ticks
        })),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}
