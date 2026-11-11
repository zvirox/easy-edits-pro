// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Provenance and resource verification: unreviewed low-confidence edits and offline media (AF3 / AF4).

use serde_json::json;

use crate::brief::ConfidenceLevel;
use crate::project::ProjectDocument;
use crate::time::Tick;
use crate::MediaKind;
use super::types::{CheckEvidence, CheckStatus, CheckVerdict, TickRange, VerifyPreset};

/// Verifies that no unreviewed low-confidence agent edits remain active without human review.
pub fn verify_unreviewed_low_confidence(
    document: &ProjectDocument,
    _preset: &VerifyPreset,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let mut offending = Vec::new();
    let mut total_notes_evaluated = 0usize;

    for track in &document.project.sequence.tracks {
        for clip in &track.clips {
            let clip_start = clip.timeline_start.0.max(0) as u64;
            let clip_end = clip_start.saturating_add(clip.duration.0.max(0) as u64);
            if clip_end <= start_tick.0.max(0) as u64 || clip_start >= end_tick.0.max(0) as u64 {
                continue;
            }

            if let Some(note) = &clip.agent_note {
                total_notes_evaluated += 1;
                if note.active && note.confidence == ConfidenceLevel::Low {
                    offending.push(TickRange {
                        start_tick: clip_start,
                        end_tick: clip_end,
                        clip_id: Some(clip.id),
                        detail: format!(
                            "Clip #{} has active low-confidence edit by {} (change #{}): {:?}",
                            clip.id, note.actor, note.change_id, note.rationale
                        ),
                    });
                }
            }
        }
    }

    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Warn
    };

    let message = if offending.is_empty() {
        format!(
            "No unreviewed low-confidence edits on timeline (evaluated {} agent note(s)).",
            total_notes_evaluated
        )
    } else {
        format!(
            "Flagged {} unreviewed low-confidence agent edit(s) requiring human review.",
            offending.len()
        )
    };

    CheckEvidence {
        check_id: "provenance.unreviewed_low_confidence".into(),
        name: "Unreviewed Low-Confidence Edits".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({
            "unreviewed_low_confidence_count": offending.len(),
            "total_agent_notes_evaluated": total_notes_evaluated
        })),
        threshold: Some(json!({"max_unreviewed_low_confidence": 0})),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies that all clips resolve to valid media assets and that no referenced media is offline.
pub fn verify_offline_media(
    document: &ProjectDocument,
    _preset: &VerifyPreset,
    start_tick: Tick,
    end_tick: Tick,
) -> CheckEvidence {
    let mut offending = Vec::new();
    let media_pool = &document.project.media;

    // Check each clip on the timeline
    for track in &document.project.sequence.tracks {
        for clip in &track.clips {
            let clip_start = clip.timeline_start.0.max(0) as u64;
            let clip_end = clip_start.saturating_add(clip.duration.0.max(0) as u64);
            if clip_end <= start_tick.0.max(0) as u64 || clip_start >= end_tick.0.max(0) as u64 {
                continue;
            }

            let asset_opt = media_pool.iter().find(|m| m.id == clip.media_id);
            match asset_opt {
                None => {
                    offending.push(TickRange {
                        start_tick: clip_start,
                        end_tick: clip_end,
                        clip_id: Some(clip.id),
                        detail: format!(
                            "Clip #{} references non-existent media_id {}",
                            clip.id, clip.media_id
                        ),
                    });
                }
                Some(asset) => {
                    let is_file_backed = asset.kind != MediaKind::Title && asset.kind != MediaKind::CompoundSequence;
                    let file_missing_on_disk = is_file_backed
                        && !asset.path.as_os_str().is_empty()
                        && !asset.path.exists();

                    if asset.is_missing || file_missing_on_disk {
                        let reason = if asset.is_missing {
                            "marked missing/offline"
                        } else {
                            "source file does not exist on disk"
                        };
                        offending.push(TickRange {
                            start_tick: clip_start,
                            end_tick: clip_end,
                            clip_id: Some(clip.id),
                            detail: format!(
                                "Clip #{} uses offline media #{} ({:?}): {} ({})",
                                clip.id, asset.id, asset.name, reason, asset.path.display()
                            ),
                        });
                    }
                }
            }
        }
    }

    let verdict = if offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Fail
    };

    let message = if offending.is_empty() {
        "All timeline clips resolve to online media assets.".into()
    } else {
        format!(
            "Detected {} offline or unresolved media reference(s) on timeline.",
            offending.len()
        )
    };

    CheckEvidence {
        check_id: "provenance.offline_media".into(),
        name: "Offline Media and Relinks".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({"offline_references_count": offending.len()})),
        threshold: Some(json!({"max_offline_references": 0})),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}
