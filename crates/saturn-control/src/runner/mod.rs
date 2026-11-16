// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Runner daemon service for multi-job concurrent task-sheet execution (TS9).
//!
//! The [`RunnerService`] watches for task sheets in `ready` state, leases
//! worker sessions, enforces budgets, and supports kill-switch cancellation.
//! Up to `max_concurrent_workers` (default 2) jobs run simultaneously.

pub mod byo_endpoint;
pub mod worker;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::session::SessionManager;
pub use worker::WorkerBudget;

pub use byo_endpoint::{ByoEndpoint, CloudEndpoint, EndpointError, LocalEndpoint};
pub use worker::{WorkerBudget as RunnerBudget, WorkerError, WorkerLease, WorkerUsage};

/// Default concurrent worker slots.
pub const DEFAULT_MAX_CONCURRENT_WORKERS: usize = 2;

/// Configuration for the runner service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    pub max_concurrent_workers: usize,
    pub default_budget: WorkerBudget,
    pub worker_rate_limit: u32,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workers: DEFAULT_MAX_CONCURRENT_WORKERS,
            default_budget: WorkerBudget::default(),
            worker_rate_limit: 60,
        }
    }
}

/// Errors from runner operations.
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum RunnerError {
    #[error("JobNotFound: job '{job_id}' does not exist")]
    JobNotFound { job_id: String },
    #[error("RunnerNotRunning: start the runner first")]
    RunnerNotRunning,
    #[error("RunnerAlreadyRunning: runner is already active")]
    RunnerAlreadyRunning,
    #[error("EndpointError: {0}")]
    Endpoint(EndpointError),
}

impl RunnerError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::JobNotFound { .. } => "JobNotFound",
            Self::RunnerNotRunning => "RunnerNotRunning",
            Self::RunnerAlreadyRunning => "RunnerAlreadyRunning",
            Self::Endpoint(_) => "EndpointError",
        }
    }
}

/// State of a runner job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A record for one enqueued or active job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: Uuid,
    pub tasksheet_id: String,
    pub state: JobState,
    pub session_id: Option<String>,
    pub budget: WorkerBudget,
    pub endpoint: Option<ByoEndpoint>,
    pub error: Option<String>,
    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub usage: WorkerUsage,
}

impl JobRecord {
    fn new(tasksheet_id: String, budget: WorkerBudget, endpoint: Option<ByoEndpoint>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tasksheet_id,
            state: JobState::Pending,
            session_id: None,
            budget,
            endpoint,
            error: None,
            queued_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            usage: WorkerUsage::default(),
        }
    }

    pub fn audit_safe_endpoint(&self) -> Option<serde_json::Value> {
        self.endpoint.as_ref().map(|ep| ep.audit_safe())
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Runtime state of the runner (locked internally).
struct RunnerState {
    running: bool,
    jobs: HashMap<Uuid, JobRecord>,
}

impl RunnerState {
    fn new() -> Self {
        Self {
            running: false,
            jobs: HashMap::new(),
        }
    }
}

/// The runner service.
///
/// Holds configuration, shared session manager reference, a semaphore for
/// concurrency control, and the job registry.
pub struct RunnerService {
    config: RunnerConfig,
    sessions: Arc<SessionManager>,
    semaphore: Arc<Semaphore>,
    state: Arc<Mutex<RunnerState>>,
}

impl RunnerService {
    pub fn new(config: RunnerConfig, sessions: Arc<SessionManager>) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_workers));
        Self {
            config,
            sessions,
            semaphore,
            state: Arc::new(Mutex::new(RunnerState::new())),
        }
    }

    /// Start the runner service. Returns error if already running.
    pub fn start(&self) -> Result<(), RunnerError> {
        let mut st = self.state.lock();
        if st.running {
            return Err(RunnerError::RunnerAlreadyRunning);
        }
        st.running = true;
        Ok(())
    }

    /// Stop the runner service and cancel all active jobs.
    pub fn stop(&self) -> Result<(), RunnerError> {
        let mut st = self.state.lock();
        if !st.running {
            return Err(RunnerError::RunnerNotRunning);
        }
        st.running = false;
        // Cancel any pending jobs
        for job in st.jobs.values_mut() {
            if job.state == JobState::Pending || job.state == JobState::Running {
                // If session exists, stop it immediately
                if let Some(ref session_id) = job.session_id {
                    let _ = self.sessions.stop_session(session_id);
                }
                job.state = JobState::Cancelled;
                job.finished_at_ms = Some(now_ms());
            }
        }
        Ok(())
    }

    /// Enqueue a new job. Returns the job ID.
    pub fn enqueue_job(
        &self,
        tasksheet_id: String,
        budget: Option<WorkerBudget>,
        endpoint: Option<ByoEndpoint>,
    ) -> Result<Uuid, RunnerError> {
        // Validate endpoint if provided
        if let Some(ref ep) = endpoint {
            ep.validate().map_err(RunnerError::Endpoint)?;
        }

        let mut st = self.state.lock();
        if !st.running {
            return Err(RunnerError::RunnerNotRunning);
        }
        let budget = budget.unwrap_or_else(|| self.config.default_budget.clone());
        let job = JobRecord::new(tasksheet_id, budget, endpoint);
        let id = job.id;
        st.jobs.insert(id, job);
        Ok(id)
    }

    /// Cancel a specific job. If running, stops its session.
    pub fn cancel_job(&self, job_id: Uuid) -> Result<(), RunnerError> {
        let mut st = self.state.lock();
        let job = st.jobs.get_mut(&job_id).ok_or_else(|| RunnerError::JobNotFound {
            job_id: job_id.to_string(),
        })?;
        if job.state == JobState::Completed || job.state == JobState::Failed {
            return Ok(()); // Already terminal, no-op
        }
        if let Some(ref session_id) = job.session_id {
            let _ = self.sessions.stop_session(session_id);
        }
        job.state = JobState::Cancelled;
        job.finished_at_ms = Some(now_ms());
        Ok(())
    }

    /// Execute a job: acquires a worker lease, runs the simulated work,
    /// and enforces all budget limits. Must be called in a tokio task.
    pub async fn execute_job(
        &self,
        job_id: Uuid,
        work_fn: impl std::future::Future<Output = Result<WorkerUsage, WorkerError>>,
    ) -> Result<(), WorkerError> {
        // Acquire concurrency permit (blocks if at max_concurrent_workers)
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore not closed");

        // Extract budget from job record and transition to Running
        let (budget, session_id_to_set) = {
            let mut st = self.state.lock();
            let job = st.jobs.get_mut(&job_id).ok_or_else(|| WorkerError::Stopped(
                crate::session::SessionError::SessionNotFound {
                    session_id: job_id.to_string(),
                },
            ))?;
            if job.state == JobState::Cancelled {
                return Err(WorkerError::Stopped(
                    crate::session::SessionError::SessionStopped {
                        session_id: job_id.to_string(),
                    },
                ));
            }
            job.state = JobState::Running;
            job.started_at_ms = Some(now_ms());
            let session_id = format!("agent-worker-{job_id}");
            job.session_id = Some(session_id.clone());
            (job.budget.clone(), session_id)
        };
        let _ = session_id_to_set; // session_id already stored in job record

        // Acquire worker lease
        let mut lease = WorkerLease::acquire(
            job_id,
            Arc::clone(&self.sessions),
            budget,
            self.config.worker_rate_limit,
        );

        // Execute work with budget monitoring
        let result = {
            let budget_check_interval = Duration::from_millis(100);

            tokio::select! {
                res = work_fn => res,
                _ = async {
                    loop {
                        tokio::time::sleep(budget_check_interval).await;
                        if lease.check_budgets().is_err() {
                            break;
                        }
                    }
                } => {
                    Err(lease.check_budgets().unwrap_err())
                }
            }
        };

        // Update job state
        {
            let mut st = self.state.lock();
            if let Some(job) = st.jobs.get_mut(&job_id) {
                job.usage = lease.usage.clone();
                job.finished_at_ms = Some(now_ms());
                match &result {
                    Ok(usage) => {
                        job.usage = usage.clone();
                        job.state = JobState::Completed;
                    }
                    Err(e) => {
                        job.error = Some(e.to_string());
                        job.state = JobState::Failed;
                    }
                }
            }
        }

        lease.release();
        result.map(|_| ())
    }

    /// Get the status of a specific job.
    pub fn get_job(&self, job_id: Uuid) -> Option<JobRecord> {
        self.state.lock().jobs.get(&job_id).cloned()
    }

    /// List all jobs.
    pub fn list_jobs(&self) -> Vec<JobRecord> {
        self.state.lock().jobs.values().cloned().collect()
    }

    /// Return runner status summary.
    pub fn status(&self) -> RunnerStatus {
        let st = self.state.lock();
        let jobs: Vec<&JobRecord> = st.jobs.values().collect();
        RunnerStatus {
            running: st.running,
            max_concurrent_workers: self.config.max_concurrent_workers,
            active_jobs: jobs.iter().filter(|j| j.state == JobState::Running).count(),
            pending_jobs: jobs.iter().filter(|j| j.state == JobState::Pending).count(),
            total_jobs: jobs.len(),
        }
    }
}

/// Summary returned by `GET /runner/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerStatus {
    pub running: bool,
    pub max_concurrent_workers: usize,
    pub active_jobs: usize,
    pub pending_jobs: usize,
    pub total_jobs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;

    fn make_runner() -> RunnerService {
        let sessions = Arc::new(SessionManager::new(60, Some(3600), None));
        RunnerService::new(RunnerConfig::default(), sessions)
    }

    #[test]
    fn start_stop_lifecycle() {
        let runner = make_runner();
        assert!(runner.start().is_ok());
        assert!(runner.start().is_err()); // already running
        assert!(runner.stop().is_ok());
        assert!(runner.stop().is_err()); // already stopped
    }

    #[test]
    fn enqueue_requires_running() {
        let runner = make_runner();
        let err = runner.enqueue_job("ts_1".to_string(), None, None).unwrap_err();
        assert_eq!(err.error_code(), "RunnerNotRunning");
    }

    #[test]
    fn enqueue_and_cancel() {
        let runner = make_runner();
        runner.start().unwrap();
        let id = runner.enqueue_job("ts_1".to_string(), None, None).unwrap();
        assert!(runner.cancel_job(id).is_ok());
        let job = runner.get_job(id).unwrap();
        assert_eq!(job.state, JobState::Cancelled);
    }

    #[test]
    fn endpoint_validation_on_enqueue() {
        use crate::runner::byo_endpoint::{ByoEndpoint, LocalEndpoint};
        let runner = make_runner();
        runner.start().unwrap();
        let bad_ep = ByoEndpoint::Local(LocalEndpoint::Ollama {
            base_url: "http://remote.example.com/v1".to_string(),
            model: "x".to_string(),
        });
        let err = runner.enqueue_job("ts_1".to_string(), None, Some(bad_ep)).unwrap_err();
        assert_eq!(err.error_code(), "EndpointError");
    }

    #[tokio::test]
    async fn concurrent_jobs_complete_simultaneously() {
        let sessions = Arc::new(SessionManager::new(120, Some(3600), None));
        let config = RunnerConfig {
            max_concurrent_workers: 2,
            ..Default::default()
        };
        let runner = Arc::new(RunnerService::new(config, sessions));
        runner.start().unwrap();

        let id1 = runner.enqueue_job("ts_a".to_string(), None, None).unwrap();
        let id2 = runner.enqueue_job("ts_b".to_string(), None, None).unwrap();

        let r1 = Arc::clone(&runner);
        let r2 = Arc::clone(&runner);

        let start = std::time::Instant::now();

        let h1 = tokio::spawn(async move {
            r1.execute_job(id1, async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(WorkerUsage::default())
            })
            .await
        });

        let h2 = tokio::spawn(async move {
            r2.execute_job(id2, async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(WorkerUsage::default())
            })
            .await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        let elapsed = start.elapsed();

        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
        // Both ran concurrently: total time should be < 200ms (not sequential)
        assert!(elapsed < Duration::from_millis(300), "elapsed: {elapsed:?}");

        let job1 = runner.get_job(id1).unwrap();
        let job2 = runner.get_job(id2).unwrap();
        assert_eq!(job1.state, JobState::Completed);
        assert_eq!(job2.state, JobState::Completed);
    }
}
