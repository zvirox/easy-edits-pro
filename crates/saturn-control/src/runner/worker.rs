// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Worker lease management for TS9 runner jobs.
//!
//! Each worker lease corresponds to one task-sheet job. A lease:
//! - creates/acquires a named session via `SessionManager`
//! - enforces wall-clock time, token-cost, and render-minute budgets
//! - supports immediate kill-switch via `SessionManager::stop_session`

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::budget::BudgetError;
use crate::session::{DaemonSession, SessionError, SessionManager};

/// Extended budget for a runner job (superset of session TimeBudget).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerBudget {
    /// Wall-clock time limit in seconds (enforced by existing TimeBudget).
    pub budget_secs: Option<u64>,
    /// Maximum USD cost of LLM tokens (advisory; enforced at poll).
    pub budget_usd: Option<f64>,
    /// Maximum input+output tokens across all calls.
    pub max_tokens: Option<u64>,
    /// Maximum render minutes consumed by this job.
    pub max_render_minutes: Option<u64>,
}

impl Default for WorkerBudget {
    fn default() -> Self {
        Self {
            budget_secs: Some(3600),
            budget_usd: None,
            max_tokens: None,
            max_render_minutes: None,
        }
    }
}

/// Errors from worker lease operations.
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum WorkerError {
    #[error("BudgetExceeded: {0}")]
    Budget(BudgetError),
    #[error("SessionStopped: {0}")]
    Stopped(SessionError),
    #[error("TokenBudgetExceeded: used {used} tokens, limit {limit}")]
    TokenBudgetExceeded { used: u64, limit: u64 },
    #[error("UsdBudgetExceeded: used ${used:.4}, limit ${limit:.4}")]
    UsdBudgetExceeded { used: f64, limit: f64 },
    #[error("RenderMinutesExceeded: used {used} minutes, limit {limit}")]
    RenderMinutesExceeded { used: u64, limit: u64 },
}

impl WorkerError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Budget(_) => "BudgetExceeded",
            Self::Stopped(_) => "SessionStopped",
            Self::TokenBudgetExceeded { .. } => "TokenBudgetExceeded",
            Self::UsdBudgetExceeded { .. } => "UsdBudgetExceeded",
            Self::RenderMinutesExceeded { .. } => "RenderMinutesExceeded",
        }
    }
}

/// Tracks runtime resource consumption for a worker lease.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WorkerUsage {
    pub tokens_used: u64,
    pub usd_used: f64,
    pub render_minutes_used: u64,
}

/// A leased worker slot for one task-sheet job.
///
/// Callers must call [`WorkerLease::check_budgets`] before each unit of work.
/// When the job is done or cancelled call [`WorkerLease::release`].
pub struct WorkerLease {
    pub session_id: String,
    pub job_id: uuid::Uuid,
    pub budget: WorkerBudget,
    pub usage: WorkerUsage,
    pub started_at: Instant,
    sessions: Arc<SessionManager>,
}

impl WorkerLease {
    /// Create a new lease, creating the named session in the manager.
    pub fn acquire(
        job_id: uuid::Uuid,
        sessions: Arc<SessionManager>,
        budget: WorkerBudget,
        rate_limit: u32,
    ) -> Self {
        let session_id = format!("agent-worker-{job_id}");
        // Create the session with the wall-clock budget and rate limit
        {
            let _sess = sessions.get_or_create_with_budget(
                &session_id,
                &session_id,
                rate_limit,
                budget.budget_secs,
            );
        }
        Self {
            session_id,
            job_id,
            budget,
            usage: WorkerUsage::default(),
            started_at: Instant::now(),
            sessions,
        }
    }

    /// Check all budget dimensions. Returns an error if any limit is exceeded.
    pub fn check_budgets(&mut self) -> Result<(), WorkerError> {
        // 1. Session-level time + rate budget
        if let Some(sess_arc) = self.sessions.get(&self.session_id) {
            let mut sess = sess_arc.write();
            sess.check_state().map_err(WorkerError::Stopped)?;
            sess.check_budgets().map_err(WorkerError::Budget)?;
        }

        // 2. Token budget
        if let (Some(limit), used) = (self.budget.max_tokens, self.usage.tokens_used) {
            if used >= limit {
                return Err(WorkerError::TokenBudgetExceeded { used, limit });
            }
        }

        // 3. USD cost budget
        if let Some(limit) = self.budget.budget_usd {
            let used = self.usage.usd_used;
            if used >= limit {
                return Err(WorkerError::UsdBudgetExceeded { used, limit });
            }
        }

        // 4. Render minutes budget
        if let (Some(limit), used) = (self.budget.max_render_minutes, self.usage.render_minutes_used) {
            if used >= limit {
                return Err(WorkerError::RenderMinutesExceeded { used, limit });
            }
        }

        Ok(())
    }

    /// Record token and cost consumption from one model call.
    pub fn record_usage(&mut self, tokens: u64, usd: f64) {
        self.usage.tokens_used += tokens;
        self.usage.usd_used += usd;
    }

    /// Record render time consumed.
    pub fn record_render_minutes(&mut self, minutes: u64) {
        self.usage.render_minutes_used += minutes;
    }

    /// Kill-switch: immediately stops the leased session, fencing further calls.
    /// Returns the new session generation on success.
    pub fn kill_switch(&self) -> Result<u64, SessionError> {
        self.sessions.stop_session(&self.session_id)
    }

    /// Release the lease, cleaning up the session.
    pub fn release(self) {
        self.sessions.disconnect_cleanup(&self.session_id);
    }

    /// Wall-clock elapsed time for this lease.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Extension on `SessionManager` to support custom budget parameters.
/// This is an inherent impl block in the same crate — we add a convenience
/// method so the runner can pass rate_limit + time budget together.
impl SessionManager {
    pub fn get_or_create_with_budget(
        &self,
        session_id: &str,
        actor_name: &str,
        rate_limit: u32,
        budget_secs: Option<u64>,
    ) -> Arc<parking_lot::RwLock<DaemonSession>> {
        // If the session already exists, return it; otherwise create with explicit budget.
        if let Some(existing) = self.get(session_id) {
            return existing;
        }
        // Create via get_or_create (uses default rate_limit and budget from manager).
        // We override immediately below by replacing the session's time_budget.
        let sess_arc = self.get_or_create(session_id, actor_name);
        {
            let mut sess = sess_arc.write();
            sess.rate_limiter = crate::budget::RateLimiter::new(rate_limit);
            sess.time_budget = crate::budget::TimeBudget::new(budget_secs);
        }
        sess_arc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;

    fn make_manager() -> Arc<SessionManager> {
        Arc::new(SessionManager::new(60, Some(3600), None))
    }

    #[test]
    fn acquire_creates_session_with_correct_budget() {
        let sessions = make_manager();
        let job_id = uuid::Uuid::new_v4();
        let budget = WorkerBudget {
            budget_secs: Some(10),
            ..Default::default()
        };
        let mut lease = WorkerLease::acquire(job_id, Arc::clone(&sessions), budget, 60);
        assert!(sessions.get(&lease.session_id).is_some());
        assert!(!lease.check_budgets().is_err()); // within budget
        lease.release();
    }

    #[test]
    fn kill_switch_fences_session() {
        let sessions = make_manager();
        let job_id = uuid::Uuid::new_v4();
        let lease = WorkerLease::acquire(job_id, Arc::clone(&sessions), WorkerBudget::default(), 60);
        let generation = lease.kill_switch().expect("kill switch should succeed");
        assert!(generation >= 2);
        // Session should now be stopped
        let sess_arc = sessions.get(&lease.session_id).unwrap();
        assert!(sess_arc.read().stopped);
        // Cannot release after kill (cleanup still safe)
        sessions.disconnect_cleanup(&lease.session_id);
    }

    #[test]
    fn token_budget_exceeded() {
        let sessions = make_manager();
        let job_id = uuid::Uuid::new_v4();
        let budget = WorkerBudget {
            max_tokens: Some(100),
            ..Default::default()
        };
        let mut lease = WorkerLease::acquire(job_id, Arc::clone(&sessions), budget, 60);
        lease.record_usage(100, 0.0);
        let err = lease.check_budgets().unwrap_err();
        assert_eq!(err.error_code(), "TokenBudgetExceeded");
        lease.release();
    }

    #[test]
    fn usd_budget_exceeded() {
        let sessions = make_manager();
        let job_id = uuid::Uuid::new_v4();
        let budget = WorkerBudget {
            budget_usd: Some(1.0),
            ..Default::default()
        };
        let mut lease = WorkerLease::acquire(job_id, Arc::clone(&sessions), budget, 60);
        lease.record_usage(0, 1.5);
        let err = lease.check_budgets().unwrap_err();
        assert_eq!(err.error_code(), "UsdBudgetExceeded");
        lease.release();
    }

    #[test]
    fn render_minutes_budget_exceeded() {
        let sessions = make_manager();
        let job_id = uuid::Uuid::new_v4();
        let budget = WorkerBudget {
            max_render_minutes: Some(5),
            ..Default::default()
        };
        let mut lease = WorkerLease::acquire(job_id, Arc::clone(&sessions), budget, 60);
        lease.record_render_minutes(5);
        let err = lease.check_budgets().unwrap_err();
        assert_eq!(err.error_code(), "RenderMinutesExceeded");
        lease.release();
    }
}
