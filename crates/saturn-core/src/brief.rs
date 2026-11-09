// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Project brief, edit rationale, confidence, and provenance models (AF4).
//!
//! Stores human-owned goals, platform constraints, must-keep lists, and style rules,
//! alongside per-clip agent notes (rationale ≤200 characters, declared confidence,
//! and human correction records preserving original provenance).

use serde::{Deserialize, Serialize};

use crate::actor::ActorId;

/// Maximum character length for edit rationales.
pub const MAX_RATIONALE_LEN: usize = 200;

/// Declared agent uncertainty level for an edit (AF4).
///
/// NOTE: This is declared uncertainty, NOT measured quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::str::FromStr for ConfidenceLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "high" => Ok(ConfidenceLevel::High),
            "medium" => Ok(ConfidenceLevel::Medium),
            "low" => Ok(ConfidenceLevel::Low),
            other => Err(format!(
                "invalid confidence level `{other}`; expected `high`, `medium`, or `low`"
            )),
        }
    }
}

/// A human correction attached to an agent note, preserving provenance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanCorrection {
    pub corrected_by: ActorId,
    pub corrected_at_unix_ms: u64,
    #[serde(default)]
    pub note: String,
}

/// Provenance metadata attached to a timeline clip modified by an agent (AF4).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipAgentNote {
    pub rationale: String,
    pub confidence: ConfidenceLevel,
    pub actor: ActorId,
    pub session_id: String,
    pub created_at_unix_ms: u64,
    pub change_id: u64,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction: Option<HumanCorrection>,
}

fn default_true() -> bool {
    true
}

impl ClipAgentNote {
    pub fn new(
        rationale: impl Into<String>,
        confidence: ConfidenceLevel,
        actor: ActorId,
        session_id: impl Into<String>,
        created_at_unix_ms: u64,
        change_id: u64,
    ) -> Result<Self, String> {
        let rationale = rationale.into();
        if rationale.chars().count() > MAX_RATIONALE_LEN {
            return Err(format!(
                "Rationale exceeds {} characters (length: {})",
                MAX_RATIONALE_LEN,
                rationale.chars().count()
            ));
        }
        Ok(Self {
            rationale,
            confidence,
            actor,
            session_id: session_id.into(),
            created_at_unix_ms,
            change_id,
            active: true,
            correction: None,
        })
    }

    /// Mark as corrected by a human actor, clearing the active agent mark while preserving provenance.
    pub fn record_correction(
        &mut self,
        human: ActorId,
        timestamp_unix_ms: u64,
        note: impl Into<String>,
    ) {
        self.active = false;
        self.correction = Some(HumanCorrection {
            corrected_by: human,
            corrected_at_unix_ms: timestamp_unix_ms,
            note: note.into(),
        });
    }
}

/// Origin source of a project brief or brief excerpt (AF4 / AF5).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefSource {
    #[default]
    UserPrompt,
    ImportedExcerpt,
    Preset,
}

/// Human-owned project brief (AF4).
///
/// Sets guidelines, duration constraints, must-keep clips, banned topics, and loudness targets.
/// Modifications require human authorization (`caller.is_human()`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ProjectBrief {
    #[serde(default)]
    pub goal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_duration_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_tolerance_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_keep_clip_ids: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub banned_content: Vec<String>,
    #[serde(default)]
    pub style_notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness_target_lufs: Option<f64>,
    #[serde(default)]
    pub updated_at_unix_ms: u64,
    #[serde(default)]
    pub updated_by: ActorId,
    #[serde(default)]
    pub source: BriefSource,
}

impl ProjectBrief {
    pub fn is_empty(&self) -> bool {
        self.goal.is_empty()
            && self.platform.is_none()
            && self.target_duration_ticks.is_none()
            && self.must_keep_clip_ids.is_empty()
            && self.banned_content.is_empty()
            && self.style_notes.is_empty()
            && self.loudness_target_lufs.is_none()
    }
}
