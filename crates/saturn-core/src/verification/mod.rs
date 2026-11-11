// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Measured self-check subsystem (`verify.run`, AF3).
//!
//! Provides bounded, evidence-based verification of timelines against delivery presets
//! and human-owned project briefs without invented aggregate quality scores.

pub mod audio_video;
pub mod captions;
pub mod project;
pub mod provenance;
pub mod types;

pub use types::*;

use crate::audio_edit::snapshot_digest;
use crate::project::ProjectDocument;
use crate::time::Tick;

/// Runs all configured verification checks on a project document.
pub fn run_verification(
    document: &ProjectDocument,
    params: &VerifyParams,
) -> Result<VerifyReport, String> {
    let preset = VerifyPreset::from_name_or_default(params.preset.as_deref());
    let sequence_duration = document.project.sequence.duration();

    let start_tick = Tick(params.start_tick.unwrap_or(0) as i64);
    let end_tick = Tick(
        params
            .end_tick
            .unwrap_or(sequence_duration.0.max(0) as u64) as i64,
    );

    let analyzed_range = AnalyzedRange {
        start_tick: start_tick.0.max(0) as u64,
        end_tick: end_tick.0.max(0) as u64,
        duration_ticks: (end_tick.0.saturating_sub(start_tick.0)).max(0) as u64,
    };

    let filter_active = !params.checks.is_empty();
    let should_run = |check_id: &str| -> bool {
        if !filter_active {
            return true;
        }
        params.checks.iter().any(|c| c == check_id)
    };

    let mut checks = Vec::new();

    // 1. Project target duration
    if should_run("project.target_duration") {
        checks.push(project::verify_target_duration(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 2. Project resolution and aspect ratio
    if should_run("project.resolution_aspect") {
        checks.push(project::verify_resolution_aspect(document, &preset));
    }

    // 3. Project timeline silence / dead air
    if should_run("project.timeline_silence") {
        checks.push(project::verify_timeline_silence(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 4. Caption reading speed (CPS)
    if should_run("captions.reading_speed") {
        checks.push(captions::verify_reading_speed(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 5. Caption timecode overlap
    if should_run("captions.timecode_overlap") {
        checks.push(captions::verify_timecode_overlap(
            document,
            start_tick,
            end_tick,
        ));
    }

    // 6. Caption safe area
    if should_run("captions.safe_area") {
        checks.push(captions::verify_safe_area(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 7. Provenance: unreviewed low-confidence edits
    if should_run("provenance.unreviewed_low_confidence") {
        checks.push(provenance::verify_unreviewed_low_confidence(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 8. Provenance: offline or unresolved media
    if should_run("provenance.offline_media") {
        checks.push(provenance::verify_offline_media(
            document,
            &preset,
            start_tick,
            end_tick,
        ));
    }

    // 9. Audio: integrated loudness
    if should_run("audio.loudness") {
        checks.push(audio_video::verify_loudness(
            document,
            &preset,
            params.audio_metrics.as_ref(),
        ));
    }

    // 10. Audio: true peak ceiling
    if should_run("audio.true_peak") {
        checks.push(audio_video::verify_true_peak(
            document,
            &preset,
            params.audio_metrics.as_ref(),
        ));
    }

    // 11. Video: highlight clipping
    if should_run("video.clipped_highlights") {
        checks.push(audio_video::verify_clipped_highlights(
            document,
            &preset,
            params.video_metrics.as_ref(),
        ));
    }

    // 12. Video: crushed blacks
    if should_run("video.crushed_blacks") {
        checks.push(audio_video::verify_crushed_blacks(
            document,
            &preset,
            params.video_metrics.as_ref(),
        ));
    }

    // 13. Video: black and frozen frames
    if should_run("video.black_and_frozen_frames") {
        checks.push(audio_video::verify_black_and_frozen_frames(
            document,
            &preset,
            params.video_metrics.as_ref(),
            start_tick,
            end_tick,
        ));
    }

    let summary = VerifySummary {
        total_checks: checks.len(),
        passed: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Measured && c.verdict == CheckVerdict::Pass)
            .count(),
        warned: checks
            .iter()
            .filter(|c| c.verdict == CheckVerdict::Warn)
            .count(),
        failed: checks
            .iter()
            .filter(|c| c.verdict == CheckVerdict::Fail)
            .count(),
        unavailable: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Unavailable)
            .count(),
    };

    let snapshot_id = snapshot_digest(document).unwrap_or_else(|_| "unknown_snapshot".into());

    Ok(VerifyReport {
        snapshot_id,
        sequence_id: document.project.sequence.id,
        preset: preset.name,
        analyzed_range,
        checks,
        summary,
        notes: VerifyNotes::default(),
    })
}
