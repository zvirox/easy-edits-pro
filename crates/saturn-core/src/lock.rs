// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Durable actor-owned clip and region locks (AF2).
//!
//! Locks live in a project-wide registry outside undoable branch snapshots.
//! Clip locks protect entities across forks; region locks protect timeline intervals.
//! Disconnect, Stop, and reopen never release locks silently (plan §2.2 and §6).

use serde::{Deserialize, Serialize};

use crate::actor::ActorId;
use crate::time::Tick;

/// The protected target of an actor lock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LockScope {
    Clip {
        clip_id: u64,
    },
    Region {
        sequence_id: u64,
        track_id: u64,
        start_tick: Tick,
        end_tick: Tick,
    },
}

/// Attribution and connection state of the lock owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockOwner {
    pub actor: ActorId,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default = "default_true")]
    pub is_connected: bool,
}

fn default_true() -> bool {
    true
}

/// A durable lock protecting a clip entity or timeline region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRecord {
    pub id: u64,
    pub scope: LockScope,
    pub owner: LockOwner,
    #[serde(default)]
    pub note: String,
    pub created_at_unix_ms: u64,
    pub revision: u64,
}

/// Project-wide lock registry persisted in `ProjectDocument`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRegistry {
    #[serde(default)]
    pub locks: Vec<LockRecord>,
    #[serde(default = "default_next_lock_id")]
    pub next_id: u64,
    #[serde(default)]
    pub revision: u64,
}

fn default_next_lock_id() -> u64 {
    1
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl LockRegistry {
    pub fn new() -> Self {
        Self {
            locks: Vec::new(),
            next_id: 1,
            revision: 0,
        }
    }

    pub fn lock_clip(
        &mut self,
        actor: ActorId,
        session_id: Option<String>,
        clip_id: u64,
        note: impl Into<String>,
    ) -> Result<u64, String> {
        if let Some(existing) = self.locks.iter().find(|l| match &l.scope {
            LockScope::Clip { clip_id: id } => *id == clip_id,
            _ => false,
        }) {
            if existing.owner.actor != actor {
                return Err(format!(
                    "Conflict: clip {clip_id} is already locked by {}",
                    existing.owner.actor
                ));
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.revision += 1;
        self.locks.push(LockRecord {
            id,
            scope: LockScope::Clip { clip_id },
            owner: LockOwner {
                actor,
                session_id,
                is_connected: true,
            },
            note: note.into(),
            created_at_unix_ms: now_epoch_ms(),
            revision: 1,
        });
        Ok(id)
    }

    pub fn lock_region(
        &mut self,
        actor: ActorId,
        session_id: Option<String>,
        sequence_id: u64,
        track_id: u64,
        start_tick: Tick,
        end_tick: Tick,
        note: impl Into<String>,
    ) -> Result<u64, String> {
        if start_tick >= end_tick {
            return Err("Lock region start_tick must be less than end_tick".into());
        }

        for existing in &self.locks {
            if let LockScope::Region {
                sequence_id: s_id,
                track_id: t_id,
                start_tick: s_tick,
                end_tick: e_tick,
            } = &existing.scope
            {
                if *s_id == sequence_id && *t_id == track_id {
                    let overlaps = start_tick < *e_tick && end_tick > *s_tick;
                    if overlaps && existing.owner.actor != actor {
                        return Err(format!(
                            "Conflict: region overlaps existing lock {} owned by {}",
                            existing.id, existing.owner.actor
                        ));
                    }
                }
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.revision += 1;
        self.locks.push(LockRecord {
            id,
            scope: LockScope::Region {
                sequence_id,
                track_id,
                start_tick,
                end_tick,
            },
            owner: LockOwner {
                actor,
                session_id,
                is_connected: true,
            },
            note: note.into(),
            created_at_unix_ms: now_epoch_ms(),
            revision: 1,
        });
        Ok(id)
    }

    pub fn release_lock(&mut self, lock_id: u64, caller: &ActorId) -> Result<(), String> {
        let index = self
            .locks
            .iter()
            .position(|l| l.id == lock_id)
            .ok_or_else(|| format!("Lock {lock_id} does not exist"))?;

        if !caller.is_human() && self.locks[index].owner.actor != *caller {
            return Err(format!(
                "PermissionDenied: lock {lock_id} is owned by {} and cannot be released by {caller}",
                self.locks[index].owner.actor
            ));
        }

        self.locks.remove(index);
        self.revision += 1;
        Ok(())
    }

    pub fn reclaim_lock(
        &mut self,
        lock_id: u64,
        caller: &ActorId,
        new_owner: Option<ActorId>,
    ) -> Result<u64, String> {
        if !caller.is_human() {
            return Err("PermissionDenied: only human authority can reclaim locks".into());
        }

        let lock = self
            .locks
            .iter_mut()
            .find(|l| l.id == lock_id)
            .ok_or_else(|| format!("Lock {lock_id} does not exist"))?;

        lock.owner.actor = new_owner.unwrap_or(ActorId::Human);
        lock.owner.is_connected = true;
        lock.revision += 1;
        self.revision += 1;
        Ok(lock.revision)
    }

    pub fn check_clip_mutation(&self, clip_id: u64, caller: &ActorId) -> Result<(), String> {
        for lock in &self.locks {
            if let LockScope::Clip { clip_id: id } = lock.scope {
                if id == clip_id && lock.owner.actor != *caller {
                    return Err(format!(
                        "Locked: clip {clip_id} is locked by {}. Mutation refused.",
                        lock.owner.actor
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn check_region_mutation(
        &self,
        sequence_id: u64,
        track_id: u64,
        start: Tick,
        end: Tick,
        caller: &ActorId,
    ) -> Result<(), String> {
        for lock in &self.locks {
            if let LockScope::Region {
                sequence_id: s_id,
                track_id: t_id,
                start_tick,
                end_tick,
            } = lock.scope
            {
                if s_id == sequence_id && t_id == track_id {
                    let overlaps = start < end_tick && end > start_tick;
                    if overlaps && lock.owner.actor != *caller {
                        return Err(format!(
                            "Locked: track {track_id} region [{}..{}) is locked by {}. Mutation refused.",
                            start_tick.0, end_tick.0, lock.owner.actor
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn mark_disconnected(&mut self, session_id: &str) {
        for lock in &mut self.locks {
            if lock.owner.session_id.as_deref() == Some(session_id) {
                lock.owner.is_connected = false;
            }
        }
    }
}
