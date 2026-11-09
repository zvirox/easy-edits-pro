// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Actor identity, session context, and per-actor view state (AF2).
//!
//! Rust owns all actor contexts and per-actor views (playhead, selection,
//! targeting, active branch, trim, and gestures). Changing an agent's cursor
//! or selection never moves the human GUI cursor and never dirties the document.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::time::Tick;
use crate::viewport_state::SelectedClipRef;

/// Durable attribution identity. A display name is not proof of identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ActorId {
    #[default]
    Human,
    Agent(String),
}

impl ActorId {
    pub fn human() -> Self {
        Self::Human
    }

    pub fn agent(id: impl Into<String>) -> Self {
        Self::Agent(id.into())
    }

    pub fn is_human(&self) -> bool {
        matches!(self, Self::Human)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Human => "human",
            Self::Agent(id) => id.as_str(),
        }
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => write!(f, "human"),
            Self::Agent(id) => write!(f, "agent:{id}"),
        }
    }
}

/// Execution and attribution context bound to a transport connection or session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContext {
    pub actor: ActorId,
    pub session_id: String,
    pub generation: u64,
    pub branch: String,
}

impl Default for ActorContext {
    fn default() -> Self {
        Self::human()
    }
}

impl ActorContext {
    pub fn human() -> Self {
        Self {
            actor: ActorId::Human,
            session_id: "human-gui".to_string(),
            generation: 1,
            branch: crate::branch::MAIN_BRANCH.to_string(),
        }
    }

    pub fn agent(id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            actor: ActorId::Agent(id.into()),
            session_id: session_id.into(),
            generation: 1,
            branch: crate::branch::MAIN_BRANCH.to_string(),
        }
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    pub fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }
}

/// Per-actor session view state.
///
/// Moving cursor, selection, or targeting updates only this view, leaving
/// the human view and project document unmodified (Packet AF2 contract).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionViewState {
    pub actor: ActorId,
    pub session_id: String,
    pub playhead: Tick,
    pub selected_clips: Vec<SelectedClipRef>,
    pub selected_entity_ids: Vec<u64>,
    pub active_branch: String,
    pub gesture_depth: u32,
    pub gesture_checkpointed: bool,
    pub generation: u64,
    pub is_stopped: bool,
}

impl SessionViewState {
    pub fn new(actor: ActorId, session_id: impl Into<String>) -> Self {
        Self {
            actor,
            session_id: session_id.into(),
            playhead: Tick(0),
            selected_clips: Vec::new(),
            selected_entity_ids: Vec::new(),
            active_branch: crate::branch::MAIN_BRANCH.to_string(),
            gesture_depth: 0,
            gesture_checkpointed: false,
            generation: 1,
            is_stopped: false,
        }
    }

    pub fn human() -> Self {
        Self::new(ActorId::Human, "human-gui")
    }

    pub fn stop(&mut self) -> u64 {
        self.is_stopped = true;
        self.generation += 1;
        self.generation
    }

    pub fn resume(&mut self) -> u64 {
        self.is_stopped = false;
        self.generation
    }
}
