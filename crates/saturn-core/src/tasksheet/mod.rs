// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet v1 core module (TS0/TS1).
//!
//! Provides the structured work order data models, state machine transitions,
//! Capture Event Track (CET v0.1) parsing/generation, and lossless migration
//! to/from `ProjectBrief`.

pub mod cet;
pub mod cet_analysis;
pub mod cet_synthetic;
pub mod delivery;
pub mod delivery_report;
pub mod delivery_verify;
pub mod recipes;
pub mod review;
pub mod state_machine;
pub mod types;

pub use cet_analysis::{
    CET_DEFAULT_MAX_BYTES, CET_UNTRUSTED_FIELDS, CetAnnotatedEvent, CetChapterProposal,
    CetFocusSpan, CetIdleRange, CetProposalOptions, CetProposalReport, CetQueryParams,
    CetQueryResult, CetValidationReport, CetZoomProposal, derive_cet_proposals,
    derive_cet_proposals_from_jsonl, query_cet_events, validate_cet_jsonl,
};
pub use cet_synthetic::{CetScenario, SyntheticCetOptions, generate_synthetic_cet};
pub use recipes::{
    PodcastClipCandidate, PodcastClipRankParams, PodcastRankReport, PodcastReviewFlag,
    WebinarAnalysisReport, WebinarAnalyzeParams, WebinarChapter, WebinarFlaggedHit,
    WebinarHighlight, WebinarTopicCoverage, analyze_webinar_recap, find_word_boundary,
    rank_podcast_clips,
};

pub use cet::{
    parse_cet_jsonl, CetCapture, CetClock, CetDisplay, CetEvent, CetEventType, CetGenerator,
    CetHeader, CetParseError, CetPrivacy, CetRecorder, CetTrust, CetVideo, CET_SPEC_VERSION,
};
pub use delivery::{
    assemble_delivery_package, build_attributed_changelog, build_delivery_provenance,
    compute_blake3_hex, compute_sha256_hex, format_changelog_markdown,
    inspect_delivery_package_integrity, sanitize_file_stem, verify_delivery_package,
    AttributedChangeLog, ClipAttributionEntry, DeliveryBuildOutput, DeliveryExportInput,
    DeliveryManifest, DeliveryPackageOptions, DeliveryProvenance, DeliveryQcSummary,
    ExportSourceSpec, PackageVerificationResult, PackagedFileEntry, ProvenanceMediaAsset,
    DELIVERY_MANIFEST_SCHEMA_VERSION,
};
pub use review::{
    capture_timeline_snapshot, compute_round_diff, ChangedTickRange, ClipDiffEntry, ClipDiffKind,
    RoundClipSnapshot, RoundCueSnapshot, RoundDiffReport, RoundGraphicSnapshot,
    RoundMarkerSnapshot, RoundTimelineSnapshot, RoundTrackSnapshot,
};
pub use state_machine::{apply_transition, can_transition, TaskSheetError};
pub use types::{
    DurationConstraint, InputRole, QuestionOption, QuestionStatus, ReviewNote, ReviewNoteStatus,
    ReviewRound, SheetAcceptance, SheetBudget, SheetDeliverable, SheetHandoff, SheetInput,
    SheetMustNot, SheetPlan, SheetStatus, SheetStep, SheetStepStatus, SheetStyle, SheetTransition,
    TaskSheet, TaskSheetQuestion, TrustLevel, TASKSHEET_SCHEMA_VERSION,
};

use crate::brief::{BriefSource, ProjectBrief};

impl TaskSheet {
    /// Lossless conversion from a `ProjectBrief` to a single-deliverable `TaskSheet` (TS1).
    pub fn from_brief(brief: &ProjectBrief) -> Self {
        let title = if brief.goal.trim().is_empty() {
            "Default Project Sheet".to_string()
        } else {
            brief.goal.clone()
        };

        let duration = brief.target_duration_ticks.map(|ticks| DurationConstraint {
            target_s: ticks as f64 / crate::time::TICKS_PER_SECOND as f64,
            tolerance_s: brief
                .duration_tolerance_ticks
                .map(|tol| tol as f64 / crate::time::TICKS_PER_SECOND as f64),
        });

        let must_include = brief
            .must_keep_clip_ids
            .iter()
            .map(|id| format!("clip:{id}"))
            .collect();

        let deliverable = SheetDeliverable {
            id: "deliv_main".into(),
            name: brief
                .platform
                .as_deref()
                .unwrap_or("Main Deliverable")
                .into(),
            preset: brief.platform.as_deref().unwrap_or("web-1080p").into(),
            duration,
            loudness_lufs: brief.loudness_target_lufs,
            true_peak_dbtp: Some(-1.0),
            captions: Some(false),
            must_include,
        };

        let style = if !brief.style_notes.trim().is_empty() {
            Some(SheetStyle {
                house_style: None,
                pacing: None,
                music: None,
                notes: Some(brief.style_notes.clone()),
            })
        } else {
            None
        };

        let must_not = if !brief.banned_content.is_empty() {
            Some(SheetMustNot {
                words: Vec::new(),
                people: Vec::new(),
                content: brief.banned_content.clone(),
            })
        } else {
            None
        };

        let status = if brief.is_empty() {
            SheetStatus::Draft
        } else {
            SheetStatus::Ready
        };

        Self {
            schema: TASKSHEET_SCHEMA_VERSION.into(),
            id: "sheet_main".into(),
            title,
            requester: "human:brief_owner".into(),
            inputs: Vec::new(),
            deliverables: vec![deliverable],
            style,
            must_not,
            acceptance: Some(SheetAcceptance {
                auto: vec![
                    "audio.integrated_loudness".into(),
                    "audio.silence_dead_air".into(),
                ],
                human: vec!["tone_matches_brand".into()],
            }),
            budget: None,
            escalate_when: vec!["destructive_action".into()],
            handoff: Some(SheetHandoff {
                exports: Some(true),
                project_file: Some(true),
                interchange: vec!["fcpxml".into()],
                qc_report: Some(true),
                change_log: Some(true),
            }),
            status,
            plan: None,
            questions: Vec::new(),
            rounds: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Project single-deliverable view back to `ProjectBrief` for backward compatibility.
    pub fn to_brief(&self) -> ProjectBrief {
        let first_deliv = self.deliverables.first();

        let target_duration_ticks = first_deliv
            .and_then(|d| d.duration)
            .map(|dur| (dur.target_s.max(0.0) * crate::time::TICKS_PER_SECOND as f64).round() as u64);

        let duration_tolerance_ticks = first_deliv
            .and_then(|d| d.duration)
            .and_then(|dur| dur.tolerance_s)
            .map(|tol| (tol.max(0.0) * crate::time::TICKS_PER_SECOND as f64).round() as u64);

        let mut must_keep_clip_ids = Vec::new();
        if let Some(deliv) = first_deliv {
            for item in &deliv.must_include {
                if let Some(rest) = item.strip_prefix("clip:") {
                    if let Ok(id) = rest.parse::<u64>() {
                        must_keep_clip_ids.push(id);
                    }
                }
            }
        }

        let banned_content = self
            .must_not
            .as_ref()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let style_notes = self
            .style
            .as_ref()
            .and_then(|s| s.notes.clone())
            .unwrap_or_default();

        let loudness_target_lufs = first_deliv.and_then(|d| d.loudness_lufs);
        let platform = first_deliv.map(|d| d.preset.clone());

        ProjectBrief {
            goal: self.title.clone(),
            platform,
            target_duration_ticks,
            duration_tolerance_ticks,
            must_keep_clip_ids,
            banned_content,
            style_notes,
            loudness_target_lufs,
            updated_at_unix_ms: 0,
            updated_by: crate::actor::ActorId::Human,
            source: BriefSource::UserPrompt,
        }
    }
}
