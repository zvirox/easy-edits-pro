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
}

impl CommandRequest {
    pub fn new(id: impl Into<String>, params: Value) -> Self {
        Self {
            id: id.into(),
            params,
        }
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
