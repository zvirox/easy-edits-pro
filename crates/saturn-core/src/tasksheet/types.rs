// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet v1 data models (TS0/TS1).
//!
//! Pure, product-agnostic types representing structured post-production work orders,
//! inputs, deliverables, acceptance criteria, budgets, plans, and review rounds.
//! Kept isolated from video/timeline editor types so it can be extracted into a shared
//! agent-contract crate across Easy Edits Pro, Audio Labs Pro, and Screen Studio Pro.

use serde::{Deserialize, Serialize};

pub const TASKSHEET_SCHEMA_VERSION: &str = "tasksheet/1";

/// Operating status of a task sheet in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetStatus {
    Draft,
    Ready,
    Planning,
    AwaitingPlanApproval,
    Running,
    NeedsInput,
    InReview,
    RevisionRequested,
    Approved,
    Delivered,
    Failed,
    Cancelled,
    Stopped,
}

impl SheetStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Ready => "ready",
            Self::Planning => "planning",
            Self::AwaitingPlanApproval => "awaiting_plan_approval",
            Self::Running => "running",
            Self::NeedsInput => "needs_input",
            Self::InReview => "in_review",
            Self::RevisionRequested => "revision_requested",
            Self::Approved => "approved",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Stopped => "stopped",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Delivered | Self::Cancelled | Self::Failed)
    }
}

/// Role of an input asset provided to the task sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRole {
    MainVideo,
    IsolatedAudio,
    MusicTrack,
    BRoll,
    Transcript,
    Captions,
    CaptureEventTrack,
    Other,
}

/// Trust classification of an input asset (AF5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Trusted,
    Untrusted,
}

/// Input media or asset reference bound to a task sheet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetInput {
    pub id: String,
    pub media: String,
    pub role: InputRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<TrustLevel>,
}

/// Duration target and allowed tolerance in seconds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurationConstraint {
    pub target_s: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_s: Option<f64>,
}

/// A single deliverable specification within a task sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetDeliverable {
    pub id: String,
    pub name: String,
    pub preset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness_lufs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_peak_dbtp: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captions: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_include: Vec<String>,
}

/// Style guidelines for deliverables.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pacing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Banned or excluded content rules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetMustNot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<String>,
}

/// Automated and human acceptance criteria.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetAcceptance {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub human: Vec<String>,
}

/// Resource budgets assigned to an agent run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SheetBudget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_clock_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_min: Option<f64>,
}

/// Handoff package requirements upon delivery.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetHandoff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exports: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_file: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interchange: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qc_report: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_log: Option<bool>,
}

/// Step execution status within an agent plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetStepStatus {
    Pending,
    Current,
    Done,
    Skipped,
}

/// An individual step in an agent's proposed plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetStep {
    pub id: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: SheetStepStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_tick: Option<i64>,
}

/// Proposed execution plan submitted by an agent for human approval.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<SheetStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// An individual choice/option in a structured agent question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// Status of an agent question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Open,
    Answered,
    Expired,
}

/// A structured question asked by an agent to pause and request human input (TS3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskSheetQuestion {
    pub id: String,
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub options: Vec<QuestionOption>,
    pub default_option_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked_at_unix_ms: Option<u64>,
    pub status: QuestionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_option_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at_unix_ms: Option<u64>,
}

/// Status of a review note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewNoteStatus {
    Open,
    Resolved,
    Dismissed,
}

/// A timecoded review note created during a review round.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewNote {
    pub id: String,
    pub author: String,
    pub created_at_unix_ms: u64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_tick: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_tick: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_id: Option<u64>,
    pub status: ReviewNoteStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at_unix_ms: Option<u64>,
}

/// A distinct human review cycle (v1, v2…) with timecoded notes and snapshots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRound {
    pub round_number: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub started_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_at_open: Option<super::review::RoundTimelineSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_at_close: Option<super::review::RoundTimelineSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_from_prior: Option<super::review::RoundDiffReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<ReviewNote>,
}

/// Record of a state machine status transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetTransition {
    pub from: SheetStatus,
    pub to: SheetStatus,
    pub actor: String,
    pub timestamp_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Structured task sheet work order for agent-operated post-production (TS0/TS1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskSheet {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub requester: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<SheetInput>,
    pub deliverables: Vec<SheetDeliverable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<SheetStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub must_not: Option<SheetMustNot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<SheetAcceptance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<SheetBudget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escalate_when: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<SheetHandoff>,
    pub status: SheetStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<SheetPlan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<TaskSheetQuestion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rounds: Vec<ReviewRound>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<SheetTransition>,
}

impl TaskSheet {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        requester: impl Into<String>,
    ) -> Self {
        Self {
            schema: TASKSHEET_SCHEMA_VERSION.to_string(),
            id: id.into(),
            title: title.into(),
            requester: requester.into(),
            inputs: Vec::new(),
            deliverables: Vec::new(),
            style: None,
            must_not: None,
            acceptance: None,
            budget: None,
            escalate_when: Vec::new(),
            handoff: None,
            status: SheetStatus::Draft,
            plan: None,
            questions: Vec::new(),
            rounds: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Validate sheet structure and essential constraints.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TASKSHEET_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported task sheet schema `{}`; expected `{}`",
                self.schema, TASKSHEET_SCHEMA_VERSION
            ));
        }
        if self.id.trim().is_empty() {
            return Err("Task sheet id cannot be empty".into());
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!(
                "Task sheet id `{}` must contain only letters, numbers, underscores, or dashes",
                self.id
            ));
        }
        if self.title.trim().is_empty() {
            return Err("Task sheet title cannot be empty".into());
        }
        if self.requester.trim().is_empty() {
            return Err("Task sheet requester cannot be empty".into());
        }
        if !self.requester.starts_with("human:") && !self.requester.starts_with("agent:") {
            return Err(format!(
                "Task sheet requester `{}` must start with `human:` or `agent:`",
                self.requester
            ));
        }
        if self.deliverables.is_empty() {
            return Err("Task sheet must contain at least one deliverable".into());
        }
        for (idx, d) in self.deliverables.iter().enumerate() {
            if d.id.trim().is_empty() {
                return Err(format!("Deliverable at index {idx} has an empty id"));
            }
            if d.name.trim().is_empty() {
                return Err(format!("Deliverable `{}` has an empty name", d.id));
            }
            if d.preset.trim().is_empty() {
                return Err(format!("Deliverable `{}` has an empty preset", d.id));
            }
            if let Some(dur) = d.duration {
                if dur.target_s <= 0.0 || !dur.target_s.is_finite() {
                    return Err(format!(
                        "Deliverable `{}` has invalid target duration",
                        d.id
                    ));
                }
                if let Some(tol) = dur.tolerance_s {
                    if tol < 0.0 || !tol.is_finite() {
                        return Err(format!(
                            "Deliverable `{}` has negative duration tolerance",
                            d.id
                        ));
                    }
                }
            }
        }
        if let Some(b) = self.budget {
            if let Some(w) = b.wall_clock_min {
                if w < 0.0 || !w.is_finite() {
                    return Err("Task sheet budget wall_clock_min must be non-negative".into());
                }
            }
            if let Some(m) = b.model_cost_usd {
                if m < 0.0 || !m.is_finite() {
                    return Err("Task sheet budget model_cost_usd must be non-negative".into());
                }
            }
            if let Some(r) = b.render_min {
                if r < 0.0 || !r.is_finite() {
                    return Err("Task sheet budget render_min must be non-negative".into());
                }
            }
        }
        for q in &self.questions {
            if q.id.trim().is_empty() {
                return Err("Task sheet question id cannot be empty".into());
            }
            if q.question.trim().is_empty() {
                return Err(format!("Question `{}` has empty question text", q.id));
            }
            if q.options.len() < 2 {
                return Err(format!(
                    "Question `{}` must offer at least 2 options (found {})",
                    q.id,
                    q.options.len()
                ));
            }
            if !q.options.iter().any(|opt| opt.id == q.default_option_id) {
                return Err(format!(
                    "Question `{}` default_option_id `{}` does not match any option ID",
                    q.id, q.default_option_id
                ));
            }
            for opt in &q.options {
                if opt.id.trim().is_empty() {
                    return Err(format!(
                        "Question `{}` contains an option with an empty id",
                        q.id
                    ));
                }
                if opt.label.trim().is_empty() {
                    return Err(format!(
                        "Option `{}` in question `{}` has an empty label",
                        opt.id, q.id
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check deadlines on open questions, transitioning expired ones to `QuestionStatus::Expired`.
    /// Returns the IDs of questions that expired.
    pub fn check_deadlines(&mut self, now_unix_ms: u64) -> Vec<String> {
        let mut expired = Vec::new();
        for q in &mut self.questions {
            if q.status == QuestionStatus::Open {
                if let Some(deadline_s) = q.deadline_seconds {
                    let asked = q.asked_at_unix_ms.unwrap_or(0);
                    if now_unix_ms >= asked + (deadline_s * 1000) {
                        q.status = QuestionStatus::Expired;
                        expired.push(q.id.clone());
                    }
                }
            }
        }
        expired
    }

    pub fn has_open_questions(&self) -> bool {
        self.questions
            .iter()
            .any(|q| q.status == QuestionStatus::Open)
    }

    pub fn has_expired_questions(&self) -> bool {
        self.questions
            .iter()
            .any(|q| q.status == QuestionStatus::Expired)
    }
}
