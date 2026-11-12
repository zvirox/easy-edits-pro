// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Delivery manifest schemas, attributed change logs, and provenance metadata (TS5).

use serde::{Deserialize, Serialize};

use crate::actor::ActorContext;
use crate::brief::ConfidenceLevel;
use crate::journal::{ChangeJournal, ChangeRecord};
use crate::project::ProjectDocument;
use crate::provenance::{OriginTracker, PROVENANCE_SCHEMA_VERSION};
use crate::verification::VerifyParams;

use super::types::{ReviewRound, SheetDeliverable, SheetInput, SheetTransition};

/// Schema identifier for TS5 delivery manifests.
pub const DELIVERY_MANIFEST_SCHEMA_VERSION: &str = "delivery/1";

pub(crate) const ARTIFACT_SUBDIRS: &[&str] = &[
    "project",
    "exports",
    "interchange",
    "qc",
    "changelog",
    "provenance",
];

/// Caller-supplied export artifact metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryExportInput {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deliverable_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Accepts either a plain file path string or a detailed [`DeliveryExportInput`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExportSourceSpec {
    Path(String),
    Detailed(DeliveryExportInput),
}

impl ExportSourceSpec {
    pub fn into_input(self) -> DeliveryExportInput {
        match self {
            Self::Path(path) => DeliveryExportInput {
                path,
                deliverable_id: None,
                label: None,
            },
            Self::Detailed(input) => input,
        }
    }
}

/// Configuration for building a TS5 delivery bundle.
#[derive(Clone, Debug)]
pub struct DeliveryPackageOptions {
    pub package_dir: std::path::PathBuf,
    pub exports: Vec<DeliveryExportInput>,
    pub interchange_formats: Option<Vec<String>>,
    pub include_project_file: Option<bool>,
    pub include_exports: Option<bool>,
    pub include_qc_report: Option<bool>,
    pub include_change_log: Option<bool>,
    pub include_provenance: bool,
    pub verify_params: Option<VerifyParams>,
    pub require_rendered_exports: bool,
    pub timestamp_unix_ms: u64,
}

/// Per-file cryptographic digest entry in `manifest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagedFileEntry {
    pub relative_path: String,
    pub role: String,
    pub format: String,
    pub size_bytes: u64,
    pub blake3: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deliverable_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Summary of `verify.run` embedded in the delivery manifest (no synthetic score).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryQcSummary {
    pub preset: String,
    pub total_checks: usize,
    pub passed: usize,
    pub warned: usize,
    pub failed: usize,
    pub unavailable: usize,
}

/// Delivery manifest written to `manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeliveryManifest {
    pub schema: String,
    pub created_at_unix_ms: u64,
    pub created_by: String,
    pub session_id: String,
    pub project_name: String,
    pub project_format: String,
    pub project_schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_sheet_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_sheet_status: Option<String>,
    pub hash_algorithms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qc_summary: Option<DeliveryQcSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deliverables: Vec<SheetDeliverable>,
    pub staged_exports_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interchange_formats: Vec<String>,
    pub changelog_entries_count: usize,
    pub review_rounds_count: usize,
    pub files: Vec<PackagedFileEntry>,
}

/// Per-clip AI rationale and human correction record included in change log & provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipAttributionEntry {
    pub track_id: u64,
    pub track_name: String,
    pub clip_id: u64,
    pub media_id: u64,
    pub timeline_start_ticks: i64,
    pub duration_ticks: i64,
    pub confidence: ConfidenceLevel,
    pub rationale: String,
    pub actor: String,
    pub timestamp_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_correction_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrected_by: Option<String>,
}

/// Attributed change log written to `changelog/changelog.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributedChangeLog {
    pub schema: String,
    pub generated_at_unix_ms: u64,
    pub project_name: String,
    pub head_revision: u64,
    pub total_changes: usize,
    pub ai_changes: usize,
    pub human_changes: usize,
    pub records: Vec<ChangeRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<SheetTransition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_rounds: Vec<ReviewRound>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_notes: Vec<ClipAttributionEntry>,
}

/// Asset summary included inside `provenance/provenance.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceMediaAsset {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub offline: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_path: Option<String>,
}

/// Provenance metadata bundle written to `provenance/provenance.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryProvenance {
    pub schema_version: String,
    pub generated_at_unix_ms: u64,
    pub packaged_by: String,
    pub session_id: String,
    pub project_name: String,
    pub brief_source: crate::brief::BriefSource,
    pub brief_updated_by: crate::actor::ActorId,
    pub brief_updated_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_sheet_inputs: Vec<SheetInput>,
    pub media_assets: Vec<ProvenanceMediaAsset>,
    pub untrusted_origin_count: usize,
    pub untrusted_origins: Vec<crate::provenance::UntrustedText>,
    pub ai_change_count: usize,
    pub human_change_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_attribution: Vec<ClipAttributionEntry>,
    pub hash_algorithms: Vec<String>,
}

/// Result of verifying a delivery package's checksums and manifest consistency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageVerificationResult {
    pub valid: bool,
    pub package_dir: String,
    pub manifest_schema: String,
    pub checked_files: usize,
    pub blake3_verified: bool,
    pub sha256_verified: bool,
    pub manifest_checksum_verified: bool,
    pub checksum_file_blake3_present: bool,
    pub checksum_file_sha256_present: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Full output returned after assembling and verifying a delivery package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryBuildOutput {
    pub package_dir: String,
    pub manifest_path: String,
    pub checksums_blake3_path: String,
    pub checksums_sha256_path: String,
    pub manifest: DeliveryManifest,
    pub verification: PackageVerificationResult,
}

pub(crate) fn extract_clip_attribution(document: &ProjectDocument) -> Vec<ClipAttributionEntry> {
    let mut entries = Vec::new();
    for track in &document.project.sequence.tracks {
        for clip in &track.clips {
            if let Some(ref note) = clip.agent_note {
                entries.push(ClipAttributionEntry {
                    track_id: track.id,
                    track_name: track.name.clone(),
                    clip_id: clip.id,
                    media_id: clip.media_id,
                    timeline_start_ticks: clip.timeline_start.0,
                    duration_ticks: clip.duration.0,
                    confidence: note.confidence,
                    rationale: note.rationale.clone(),
                    actor: note.actor.to_string(),
                    timestamp_unix_ms: note.created_at_unix_ms,
                    human_correction_note: note.correction.as_ref().map(|c| c.note.clone()),
                    corrected_by: note.correction.as_ref().map(|c| c.corrected_by.to_string()),
                });
            }
        }
    }
    entries
}

/// Builds an attributed change log from journal history, review rounds, and clip notes.
pub fn build_attributed_changelog(
    document: &ProjectDocument,
    journal: &ChangeJournal,
    generated_at_unix_ms: u64,
) -> AttributedChangeLog {
    let ai_changes = journal
        .records
        .iter()
        .filter(|r| !r.actor.is_human())
        .count();
    let human_changes = journal.records.len().saturating_sub(ai_changes);
    let (transitions, review_rounds) = match &document.project.task_sheet {
        Some(sheet) => (sheet.transitions.clone(), sheet.rounds.clone()),
        None => (Vec::new(), Vec::new()),
    };

    AttributedChangeLog {
        schema: "delivery-changelog/1".to_string(),
        generated_at_unix_ms,
        project_name: document.project.name.clone(),
        head_revision: journal.head_revision,
        total_changes: journal.records.len(),
        ai_changes,
        human_changes,
        records: journal.records.clone(),
        transitions,
        review_rounds,
        clip_notes: extract_clip_attribution(document),
    }
}

/// Renders a human-readable Markdown summary for `changelog/CHANGELOG.md`.
pub fn format_changelog_markdown(changelog: &AttributedChangeLog) -> String {
    let mut md = format!(
        "# Delivery Change Log — {}\n\n- Head revision: `{}`\n- Committed changes: `{}` (Human: `{}`, Agent: `{}`)\n- Lifecycle transitions: `{}`\n- Review rounds: `{}`\n",
        changelog.project_name,
        changelog.head_revision,
        changelog.total_changes,
        changelog.human_changes,
        changelog.ai_changes,
        changelog.transitions.len(),
        changelog.review_rounds.len()
    );

    if !changelog.records.is_empty() {
        md.push_str("\n## Change Journal\n\n");
        for rec in &changelog.records {
            let rationale = rec.rationale.as_deref().unwrap_or("-");
            let conf = rec
                .confidence
                .map(|c| format!("{c:?}").to_lowercase())
                .unwrap_or_else(|| "-".to_string());
            md.push_str(&format!(
                "- `#{} (rev {})` `{}` by `{}` (branch `{}`, confidence `{}`) — {}\n",
                rec.change_id,
                rec.result_revision,
                rec.command_id,
                rec.actor,
                rec.branch,
                conf,
                rationale
            ));
        }
    }

    if !changelog.review_rounds.is_empty() {
        md.push_str("\n## Review Rounds\n\n");
        for round in &changelog.review_rounds {
            let label = round.label.as_deref().unwrap_or("-");
            let outcome = round.outcome.as_deref().unwrap_or("open");
            md.push_str(&format!(
                "- Round `v{}` (`{label}`): outcome `{outcome}`, `{}` note(s)\n",
                round.round_number,
                round.notes.len()
            ));
            for note in &round.notes {
                let res = note.resolution.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "  - `{}` [{:?}] by `{}`: {} (resolution: {})\n",
                    note.id, note.status, note.author, note.text, res
                ));
            }
        }
    }

    md
}

/// Builds the provenance metadata document for `provenance/provenance.json`.
pub fn build_delivery_provenance(
    document: &ProjectDocument,
    journal: &ChangeJournal,
    actor: &ActorContext,
    generated_at_unix_ms: u64,
) -> DeliveryProvenance {
    let tracker = OriginTracker::from_document(document);
    let untrusted_origins = tracker.recorded_items().to_vec();
    let ai_change_count = journal
        .records
        .iter()
        .filter(|r| !r.actor.is_human())
        .count();
    let human_change_count = journal.records.len().saturating_sub(ai_change_count);

    let media_assets = document
        .project
        .media
        .iter()
        .map(|m| ProvenanceMediaAsset {
            id: m.id,
            name: m.name.clone(),
            path: m.path.to_string_lossy().into_owned(),
            kind: format!("{:?}", m.kind).to_lowercase(),
            offline: m.is_missing,
            proxy_path: m
                .proxy_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        })
        .collect();

    let task_sheet_inputs = document
        .project
        .task_sheet
        .as_ref()
        .map(|s| s.inputs.clone())
        .unwrap_or_default();

    DeliveryProvenance {
        schema_version: PROVENANCE_SCHEMA_VERSION.to_string(),
        generated_at_unix_ms,
        packaged_by: actor.actor.to_string(),
        session_id: actor.session_id.clone(),
        project_name: document.project.name.clone(),
        brief_source: document.project.brief.source,
        brief_updated_by: document.project.brief.updated_by.clone(),
        brief_updated_at_unix_ms: document.project.brief.updated_at_unix_ms,
        task_sheet_inputs,
        media_assets,
        untrusted_origin_count: untrusted_origins.len(),
        untrusted_origins,
        ai_change_count,
        human_change_count,
        clip_attribution: extract_clip_attribution(document),
        hash_algorithms: vec!["blake3".to_string(), "sha256".to_string()],
    }
}
