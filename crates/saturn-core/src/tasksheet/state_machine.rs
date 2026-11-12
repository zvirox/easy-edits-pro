// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet state machine and transition validator (TS0/TS1).
//!
//! Enforces the lifecycle state machine:
//! `draft → ready → planning → awaiting_plan_approval → running ⇄ needs_input → in_review → revision_requested → running … → approved → delivered`,
//! plus `failed`, `cancelled`, `stopped`.
//! Every transition records actor, time, and reason.

use super::types::{SheetStatus, SheetTransition, TaskSheet};
use std::fmt;

/// Errors arising during task sheet state transitions or validations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskSheetError {
    InvalidTransition {
        from: SheetStatus,
        to: SheetStatus,
        reason: String,
    },
    ValidationError(String),
    Unauthorized(String),
    PlanApprovalRequired(String),
}

impl fmt::Display for TaskSheetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to, reason } => {
                write!(
                    f,
                    "Invalid state transition from `{}` to `{}`: {}",
                    from.as_str(),
                    to.as_str(),
                    reason
                )
            }
            Self::ValidationError(msg) => write!(f, "Task sheet validation error: {msg}"),
            Self::Unauthorized(msg) => write!(f, "Unauthorized task sheet action: {msg}"),
            Self::PlanApprovalRequired(msg) => write!(f, "Plan approval required: {msg}"),
        }
    }
}

impl std::error::Error for TaskSheetError {}

/// Check if a state transition is structurally permitted.
pub fn can_transition(from: SheetStatus, to: SheetStatus) -> bool {
    if from == to {
        return false;
    }
    match from {
        SheetStatus::Draft => matches!(to, SheetStatus::Ready | SheetStatus::Cancelled),
        SheetStatus::Ready => matches!(
            to,
            SheetStatus::Planning
                | SheetStatus::Running
                | SheetStatus::NeedsInput
                | SheetStatus::Cancelled
        ),
        SheetStatus::Planning => matches!(
            to,
            SheetStatus::AwaitingPlanApproval
                | SheetStatus::Running
                | SheetStatus::NeedsInput
                | SheetStatus::Cancelled
                | SheetStatus::Stopped
                | SheetStatus::Failed
        ),
        SheetStatus::AwaitingPlanApproval => matches!(
            to,
            SheetStatus::Running
                | SheetStatus::Planning
                | SheetStatus::Cancelled
                | SheetStatus::Stopped
        ),
        SheetStatus::Running => matches!(
            to,
            SheetStatus::NeedsInput
                | SheetStatus::InReview
                | SheetStatus::Stopped
                | SheetStatus::Cancelled
                | SheetStatus::Failed
        ),
        SheetStatus::NeedsInput => matches!(
            to,
            SheetStatus::Running
                | SheetStatus::Planning
                | SheetStatus::Stopped
                | SheetStatus::Cancelled
        ),
        SheetStatus::InReview => matches!(
            to,
            SheetStatus::RevisionRequested
                | SheetStatus::Approved
                | SheetStatus::Stopped
                | SheetStatus::Cancelled
        ),
        SheetStatus::RevisionRequested => matches!(
            to,
            SheetStatus::Running
                | SheetStatus::InReview
                | SheetStatus::Stopped
                | SheetStatus::Cancelled
        ),
        SheetStatus::Approved => matches!(to, SheetStatus::Delivered | SheetStatus::Cancelled),
        SheetStatus::Stopped => matches!(
            to,
            SheetStatus::Running
                | SheetStatus::Ready
                | SheetStatus::Planning
                | SheetStatus::InReview
                | SheetStatus::Cancelled
        ),
        SheetStatus::Delivered | SheetStatus::Cancelled | SheetStatus::Failed => false,
    }
}

/// Validate and apply a state transition on a TaskSheet, appending a `SheetTransition` record.
pub fn apply_transition(
    sheet: &mut TaskSheet,
    to: SheetStatus,
    actor: impl Into<String>,
    timestamp_unix_ms: u64,
    reason: Option<String>,
) -> Result<(), TaskSheetError> {
    let from = sheet.status;
    let actor_str = actor.into();

    if !can_transition(from, to) {
        return Err(TaskSheetError::InvalidTransition {
            from,
            to,
            reason: format!(
                "Cannot transition from `{}` to `{}` according to lifecycle rules",
                from.as_str(),
                to.as_str()
            ),
        });
    }

    // Additional semantic checks for specific target states
    if to == SheetStatus::Ready {
        sheet.validate().map_err(TaskSheetError::ValidationError)?;
    }

    if to == SheetStatus::Running && from == SheetStatus::AwaitingPlanApproval {
        // Human authorization required for plan approval
        if !actor_str.starts_with("human:") {
            return Err(TaskSheetError::Unauthorized(
                "Plan approval requires human actor authorization".into(),
            ));
        }
    }

    // Record the transition
    sheet.transitions.push(SheetTransition {
        from,
        to,
        actor: actor_str,
        timestamp_unix_ms,
        reason,
    });

    sheet.status = to;
    Ok(())
}
