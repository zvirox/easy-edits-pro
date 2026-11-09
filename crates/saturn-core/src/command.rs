// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Command execution request and status types.
//!
//! The catalogue itself lives in [`crate::catalogue`] and is re-exported here
//! so existing `saturn_core::command::*` imports keep working.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::catalogue::*;

/// A request to run one catalogued command.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandRequest {
    pub id: String,
    #[serde(default)]
    pub params: Value,
    /// Optional edit rationale (AF4, max 200 chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Optional declared confidence level (AF4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<crate::brief::ConfidenceLevel>,
}

impl CommandRequest {
    pub fn new(id: impl Into<String>, params: Value) -> Self {
        Self {
            id: id.into(),
            params,
            rationale: None,
            confidence: None,
        }
    }

    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    pub fn with_confidence(mut self, confidence: crate::brief::ConfidenceLevel) -> Self {
        self.confidence = Some(confidence);
        self
    }
}

/// Whether a command can run right now, and why not when it cannot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandStatus {
    pub enabled: bool,
    pub reason: Option<String>,
}

/// Build a [`CommandStatus`]; `reason` is kept only when disabled.
pub fn status(enabled: bool, reason: impl Into<String>) -> CommandStatus {
    CommandStatus {
        enabled,
        reason: (!enabled).then(|| reason.into()),
    }
}
