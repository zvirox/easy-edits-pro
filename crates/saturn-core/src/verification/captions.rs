// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Caption verification checks: reading speed, timecode overlaps, and safe area (AF3).

use serde_json::json;

use crate::project::ProjectDocument;
use crate::time::Tick;
use super::types::{CheckEvidence, CheckStatus, CheckVerdict, TickRange, VerifyPreset};

/// Verifies caption reading speed (CPS) against preset limits.
pub fn verify_reading_speed(
    document: &ProjectDocument,
    preset: &VerifyPreset,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let mut offending = Vec::new();
    let mut max_observed_cps = 0.0f64;
    let mut evaluated_count = 0usize;

    for sub in document.project.sequence.caption_tracks.iter().flat_map(|t| &t.cues) {
        if sub.end_ticks.0 <= start_tick.0 || sub.start_ticks.0 >= end_tick.0 {
            continue;
        }
        let duration_ms = sub.end_ticks.as_millis().saturating_sub(sub.start_ticks.as_millis());
        if duration_ms == 0 {
            continue;
        }
        evaluated_count += 1;
        let duration_secs = duration_ms as f64 / 1000.0;
        let char_count = sub.text.chars().count();
        let cps = char_count as f64 / duration_secs;
        if cps > max_observed_cps {
            max_observed_cps = cps;
        }

        if cps > preset.max_caption_cps {
            let snippet = if sub.text.len() > 30 {
                format!("{}...", &sub.text[..sub.text.char_indices().map(|(i, _)| i).nth(30).unwrap_or(sub.text.len())])
            } else {
                sub.text.clone()
            };
            offending.push(TickRange {
                start_tick: sub.start_ticks.0.max(0) as u64,
                end_tick: sub.end_ticks.0.max(0) as u64,
                clip_id: Some(sub.id),
                detail: format!(
                    "Caption #{} exceeds reading speed: {:.1} CPS (limit: {:.1} CPS): {:?}",
                    sub.id, cps, preset.max_caption_cps, snippet
                ),
            });
        }
    }

    if evaluated_count == 0 {
        return CheckEvidence {
            check_id: "captions.reading_speed".into(),
            name: "Caption Reading Speed".into(),
            verdict: CheckVerdict::Pass,
            status: CheckStatus::Measured,
            measured: Some(json!(0.0)),
            threshold: Some(json!({"max_cps": preset.max_caption_cps})),
            message: "No caption cues present in analyzed range.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 100.0,
        };
    }

    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Fail
    };

    let message = if offending.is_empty() {
        format!(
            "All {} caption cues within reading speed limit (max observed: {:.1} CPS, limit: {:.1} CPS).",
            evaluated_count, max_observed_cps, preset.max_caption_cps
        )
    } else {
        format!(
            "{} caption cue(s) exceed maximum reading speed of {:.1} CPS (highest: {:.1} CPS).",
            offending.len(), preset.max_caption_cps, max_observed_cps
        )
    };

    CheckEvidence {
        check_id: "captions.reading_speed".into(),
        name: "Caption Reading Speed".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!(max_observed_cps)),
        threshold: Some(json!({"max_cps": preset.max_caption_cps})),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies that captions do not overlap in time.
pub fn verify_timecode_overlap(
    document: &ProjectDocument,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let mut relevant: Vec<_> = document
        .project
        .sequence
        .caption_tracks
        .iter()
        .flat_map(|t| &t.cues)
        .filter(|s| s.end_ticks.0 > start_tick.0 && s.start_ticks.0 < end_tick.0)
        .collect();

    relevant.sort_by_key(|s| s.start_ticks.0);

    let mut offending = Vec::new();
    for i in 1..relevant.len() {
        let prev = relevant[i - 1];
        let curr = relevant[i];

        if curr.start_ticks.0 < prev.end_ticks.0 {
            offending.push(TickRange {
                start_tick: curr.start_ticks.0.max(0) as u64,
                end_tick: prev.end_ticks.0.max(0) as u64,
                clip_id: Some(curr.id),
                detail: format!(
                    "Caption #{} (starts at {}) overlaps with preceding Caption #{} (ends at {}) by {} ms",
                    curr.id,
                    curr.start_ticks.as_millis(),
                    prev.id,
                    prev.end_ticks.as_millis(),
                    (prev.end_ticks.as_millis()).saturating_sub(curr.start_ticks.as_millis())
                ),
            });
        }
    }

    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Fail
    };

    let message = if offending.is_empty() {
        format!("No overlapping caption cues detected (analyzed {} cues).", relevant.len())
    } else {
        format!("Detected {} caption timecode overlap(s).", offending.len())
    };

    CheckEvidence {
        check_id: "captions.timecode_overlap".into(),
        name: "Caption Timecode Overlap".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!(offending.len())),
        threshold: Some(json!({"max_overlaps": 0})),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies caption safe area layout (line limits and line length bounds).
pub fn verify_safe_area(
    document: &ProjectDocument,
    preset: &VerifyPreset,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let max_lines_allowed = 2usize;
    let max_line_chars = if preset.aspect_ratio == "9:16" { 26usize } else { 42usize };

    let mut offending = Vec::new();
    let mut analyzed_count = 0usize;

    for sub in document.project.sequence.caption_tracks.iter().flat_map(|t| &t.cues) {
        if sub.end_ticks.0 <= start_tick.0 || sub.start_ticks.0 >= end_tick.0 {
            continue;
        }
        analyzed_count += 1;
        let lines: Vec<&str> = sub.text.lines().collect();

        if lines.len() > max_lines_allowed {
            offending.push(TickRange {
                start_tick: sub.start_ticks.0.max(0) as u64,
                end_tick: sub.end_ticks.0.max(0) as u64,
                clip_id: Some(sub.id),
                detail: format!(
                    "Caption #{} has {} lines (exceeds standard {}-line limit for safe area)",
                    sub.id, lines.len(), max_lines_allowed
                ),
            });
        } else if let Some(long_line) = lines.iter().find(|l| l.chars().count() > max_line_chars) {
            offending.push(TickRange {
                start_tick: sub.start_ticks.0.max(0) as u64,
                end_tick: sub.end_ticks.0.max(0) as u64,
                clip_id: Some(sub.id),
                detail: format!(
                    "Caption #{} line exceeds {} character safe margin limit (length: {}): {:?}",
                    sub.id, max_line_chars, long_line.chars().count(), long_line
                ),
            });
        }
    }

    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Warn
    };

    let message = if offending.is_empty() {
        format!(
            "All {} caption cues conform to safe area line/character boundaries for {}.",
            analyzed_count, preset.name
        )
    } else {
        format!(
            "{} caption cue(s) exceed safe area line or character limits for {}.",
            offending.len(), preset.name
        )
    };

    CheckEvidence {
        check_id: "captions.safe_area".into(),
        name: "Caption Safe Area".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!(offending.len())),
        threshold: Some(json!({
            "max_lines": max_lines_allowed,
            "max_line_chars": max_line_chars,
            "margin_pct": preset.title_safe_margin_pct
        })),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}
