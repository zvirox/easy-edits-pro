// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Change journal, attribution records, and selective undo conflict rules (AF2).
//!
//! Every committed mutation appends an immutable `ChangeRecord` with actor attribution,
//! branch revision, and affected entity IDs. Selective undo validates that no subsequent
//! change depends on the affected entities before permitting rollback (plan §2.2 & §5).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::actor::ActorId;

/// An immutable record of one committed engine change or transaction step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub change_id: u64,
    pub transaction_id: u64,
    pub actor: ActorId,
    pub session_id: String,
    pub timestamp_unix_ms: u64,
    pub branch: String,
    pub base_revision: u64,
    pub result_revision: u64,
    pub command_id: String,
    #[serde(default)]
    pub affected_entity_ids: Vec<u64>,
    #[serde(default)]
    pub read_dependencies: Vec<u64>,
    #[serde(default)]
    pub write_footprint: Vec<u64>,
    #[serde(default)]
    pub inverse_state: Option<Value>,
    /// Edit rationale provided by the actor (AF4, max 200 chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Declared confidence level of the edit (AF4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<crate::brief::ConfidenceLevel>,
    /// If this change is a human correction of an earlier change, links to that change_id (AF4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction_of: Option<u64>,
}

fn default_next_id() -> u64 {
    1
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Project-wide change journal recording history and provenance.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangeJournal {
    #[serde(default)]
    pub records: Vec<ChangeRecord>,
    #[serde(default = "default_next_id")]
    pub next_change_id: u64,
    #[serde(default = "default_next_id")]
    pub next_transaction_id: u64,
    #[serde(default)]
    pub head_revision: u64,
}

impl ChangeJournal {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_change_id: 1,
            next_transaction_id: 1,
            head_revision: 1,
        }
    }

    pub fn new_transaction_id(&mut self) -> u64 {
        let id = self.next_transaction_id;
        self.next_transaction_id += 1;
        id
    }

    pub fn record_change(
        &mut self,
        transaction_id: u64,
        actor: ActorId,
        session_id: impl Into<String>,
        branch: impl Into<String>,
        command_id: impl Into<String>,
        affected_entities: Vec<u64>,
        inverse_state: Option<Value>,
    ) -> u64 {
        self.record_change_with_metadata(
            transaction_id,
            actor,
            session_id,
            branch,
            command_id,
            affected_entities,
            inverse_state,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_change_with_metadata(
        &mut self,
        transaction_id: u64,
        actor: ActorId,
        session_id: impl Into<String>,
        branch: impl Into<String>,
        command_id: impl Into<String>,
        affected_entities: Vec<u64>,
        inverse_state: Option<Value>,
        rationale: Option<String>,
        confidence: Option<crate::brief::ConfidenceLevel>,
        correction_of: Option<u64>,
    ) -> u64 {
        let change_id = self.next_change_id;
        self.next_change_id += 1;
        let base_revision = self.head_revision;
        self.head_revision += 1;
        let result_revision = self.head_revision;

        let branch_name = branch.into();
        let write_footprint = affected_entities.clone();
        let read_dependencies = affected_entities.clone();

        self.records.push(ChangeRecord {
            change_id,
            transaction_id,
            actor,
            session_id: session_id.into(),
            timestamp_unix_ms: now_epoch_ms(),
            branch: branch_name,
            base_revision,
            result_revision,
            command_id: command_id.into(),
            affected_entity_ids: affected_entities,
            read_dependencies,
            write_footprint,
            inverse_state,
            rationale,
            confidence,
            correction_of,
        });

        change_id
    }

    /// Evaluates whether selective undo for `change_id` is safe under the conflict rule.
    ///
    /// An inverse is allowed only when no surviving later change on the same branch
    /// reads or depends on the effects being removed (plan §5).
    pub fn check_undo_allowed(&self, change_id: u64) -> Result<&ChangeRecord, String> {
        let target_idx = self
            .records
            .iter()
            .position(|r| r.change_id == change_id)
            .ok_or_else(|| format!("Change {change_id} not found in history journal"))?;

        let target = &self.records[target_idx];
        if target.inverse_state.is_none() {
            return Err(format!(
                "Change {change_id} ('{}') cannot be undone: no inverse state recorded",
                target.command_id
            ));
        }

        // Check for intervening dependent changes on the same branch
        for later in &self.records[target_idx + 1..] {
            if later.branch != target.branch {
                continue;
            }

            // Check if later change touched any entities affected by target change
            for entity_id in &target.affected_entity_ids {
                if later.affected_entity_ids.contains(entity_id)
                    || later.read_dependencies.contains(entity_id)
                    || later.write_footprint.contains(entity_id)
                {
                    return Err(format!(
                        "Conflict: cannot undo change {change_id} ('{}' by {}); subsequent change {} ('{}' by {}) depends on affected entity {entity_id}.",
                        target.command_id, target.actor, later.change_id, later.command_id, later.actor
                    ));
                }
            }
        }

        Ok(target)
    }

    pub fn find_record(&self, change_id: u64) -> Option<&ChangeRecord> {
        self.records.iter().find(|r| r.change_id == change_id)
    }

    pub fn records_for_actor<'a>(&'a self, actor: &'a ActorId) -> impl Iterator<Item = &'a ChangeRecord> {
        self.records.iter().filter(move |r| &r.actor == actor)
    }
}
