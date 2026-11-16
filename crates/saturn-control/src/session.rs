// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Daemon session management, isolation, AF2 Stop, and disconnect cleanup (AF9).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use saturn_core::actor::{ActorContext, ActorId};
use saturn_core::catalogue::policy::SessionPolicy;
use saturn_core::engine::EditorEngine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::budget::{BudgetError, RateLimiter, TimeBudget};

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum SessionError {
    #[error("SessionStopped: session '{session_id}' has been stopped. Subsequent calls are rejected until resumed by human authority.")]
    SessionStopped { session_id: String },
    #[error("SessionNotFound: session '{session_id}' does not exist")]
    SessionNotFound { session_id: String },
    #[error("PermissionDenied: {message}")]
    PermissionDenied { message: String },
}

impl SessionError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::SessionStopped { .. } => "SessionStopped",
            Self::SessionNotFound { .. } => "SessionNotFound",
            Self::PermissionDenied { .. } => "PermissionDenied",
        }
    }
}

pub struct DaemonSession {
    pub id: String,
    pub actor: ActorId,
    pub engine: Option<EditorEngine>,
    pub rate_limiter: RateLimiter,
    pub time_budget: TimeBudget,
    pub stopped: bool,
    pub generation: u64,
    pub locks: HashSet<u64>,
    pub active_transaction: bool,
    pub project_root: Option<PathBuf>,
    pub policy: SessionPolicy,
    pub created_at_unix_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl DaemonSession {
    pub fn new(
        id: impl Into<String>,
        actor_name: &str,
        project_root: Option<PathBuf>,
        rate_limit: u32,
        time_budget_secs: Option<u64>,
    ) -> Self {
        let session_id = id.into();
        let actor = if actor_name.contains("human") || actor_name == "admin" || actor_name == "daemon-admin" {
            ActorId::human()
        } else {
            ActorId::agent(actor_name)
        };
        Self {
            id: session_id,
            actor,
            engine: None,
            rate_limiter: RateLimiter::new(rate_limit),
            time_budget: TimeBudget::new(time_budget_secs),
            stopped: false,
            generation: 1,
            locks: HashSet::new(),
            active_transaction: false,
            project_root,
            policy: SessionPolicy::default(),
            created_at_unix_ms: now_ms(),
        }
    }

    pub fn check_state(&mut self) -> Result<(), SessionError> {
        if self.stopped {
            return Err(SessionError::SessionStopped {
                session_id: self.id.clone(),
            });
        }
        Ok(())
    }

    pub fn check_budgets(&mut self) -> Result<(), BudgetError> {
        self.time_budget.check()?;
        self.rate_limiter.check()?;
        Ok(())
    }

    pub fn stop(&mut self) -> u64 {
        self.stopped = true;
        self.generation += 1;
        if let Some(ref mut engine) = self.engine {
            engine.stop_session(&self.id);
        }
        self.generation
    }

    pub fn resume(&mut self, caller: &ActorId) -> Result<u64, SessionError> {
        if !caller.is_human() {
            return Err(SessionError::PermissionDenied {
                message: "only human authority can resume stopped sessions".into(),
            });
        }
        self.stopped = false;
        self.generation += 1;
        if let Some(ref mut engine) = self.engine {
            let _ = engine.resume_session(&self.id, caller);
        }
        Ok(self.generation)
    }

    pub fn actor_context(&self) -> ActorContext {
        ActorContext {
            actor: self.actor.clone(),
            session_id: self.id.clone(),
            branch: saturn_core::branch::MAIN_BRANCH.to_string(),
            generation: self.generation,
        }
    }

    pub fn cleanup(&mut self) {
        if let Some(ref mut engine) = self.engine {
            // 1. Release active locks owned by this session
            for lock_id in self.locks.drain() {
                let _ = engine.document_mut().locks.release_lock(lock_id, &self.actor);
            }
            // 2. Rollback active uncommitted transaction/gesture
            if self.active_transaction {
                engine.end_gesture();
                self.active_transaction = false;
            }
        } else {
            self.locks.clear();
            self.active_transaction = false;
        }
    }
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<DaemonSession>>>>>,
    default_rate_limit: u32,
    default_time_budget_secs: Option<u64>,
    default_project_root: Option<PathBuf>,
}

impl SessionManager {
    pub fn new(
        default_rate_limit: u32,
        default_time_budget_secs: Option<u64>,
        default_project_root: Option<PathBuf>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_rate_limit,
            default_time_budget_secs,
            default_project_root,
        }
    }

    pub fn get_or_create(
        &self,
        session_id: &str,
        actor_name: &str,
    ) -> Arc<RwLock<DaemonSession>> {
        let mut map = self.sessions.write();
        map.entry(session_id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(DaemonSession::new(
                    session_id,
                    actor_name,
                    self.default_project_root.clone(),
                    self.default_rate_limit,
                    self.default_time_budget_secs,
                )))
            })
            .clone()
    }

    pub fn get(&self, session_id: &str) -> Option<Arc<RwLock<DaemonSession>>> {
        self.sessions.read().get(session_id).cloned()
    }

    pub fn stop_session(&self, session_id: &str) -> Result<u64, SessionError> {
        let session = self.get(session_id).ok_or_else(|| SessionError::SessionNotFound {
            session_id: session_id.to_string(),
        })?;
        let new_gen = session.write().stop();
        Ok(new_gen)
    }

    pub fn resume_session(&self, session_id: &str, caller: &ActorId) -> Result<u64, SessionError> {
        let session = self.get(session_id).ok_or_else(|| SessionError::SessionNotFound {
            session_id: session_id.to_string(),
        })?;
        session.write().resume(caller)
    }

    pub fn disconnect_cleanup(&self, session_id: &str) {
        if let Some(session_arc) = self.sessions.write().remove(session_id) {
            session_arc.write().cleanup();
        }
    }

    pub fn count(&self) -> usize {
        self.sessions.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_stop_advances_generation_and_fences() {
        let mut session = DaemonSession::new("test-1", "agent-1", None, 60, Some(3600));
        assert!(session.check_state().is_ok());

        let new_gen = session.stop();
        assert_eq!(new_gen, 2);
        assert!(session.stopped);

        let err = session.check_state().unwrap_err();
        assert_eq!(err.error_code(), "SessionStopped");

        // Non-human caller cannot resume
        let agent = ActorId::agent("another-agent");
        assert!(session.resume(&agent).is_err());

        // Human caller resumes
        let human = ActorId::human();
        let resumed_gen = session.resume(&human).unwrap();
        assert_eq!(resumed_gen, 3);
        assert!(!session.stopped);
        assert!(session.check_state().is_ok());
    }

    #[test]
    fn session_manager_lifecycle_and_cleanup() {
        let manager = SessionManager::new(60, Some(3600), None);
        let s1 = manager.get_or_create("s1", "agent-1");
        assert_eq!(manager.count(), 1);

        s1.write().locks.insert(100);
        manager.disconnect_cleanup("s1");
        assert_eq!(manager.count(), 0);
    }
}
