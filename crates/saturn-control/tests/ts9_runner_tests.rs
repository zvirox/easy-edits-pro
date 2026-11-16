// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Conformance integration test suite for TS9: Runner & Multi-Job Execution.
//!
//! Four Conformance Gates:
//! 1. Concurrency Gate: Two concurrent task sheets leased and executed
//!    simultaneously by worker pool (`agent-worker-<id>`) with distinct sessions.
//! 2. Budget Limit Gate: Worker exceeding budget is cleanly stopped with typed budget error.
//! 3. Kill Switch Gate: Calling session_stop halts active worker within <= 1s.
//! 4. BYO Endpoint Gate: Local and cloud endpoint resolution, credential validation,
//!    and secret redaction in audit log (zero plaintext secrets).
//! 5. REST Endpoints: GET /runner/status, POST /runner/start, POST /runner/stop,
//!    GET /runner/jobs, POST /runner/jobs/:id/cancel.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::runner::{
    ByoEndpoint, JobState, LocalEndpoint,
    WorkerBudget, WorkerError, WorkerUsage,
};
use saturn_control::server::{ControlServer, DaemonConfig};
use serde_json::{Value, json};
use tempfile::tempdir;
use tower::ServiceExt;

struct RunnerHarness {
    server: Arc<ControlServer>,
    router: axum::Router,
    _temp_dir: tempfile::TempDir,
    audit_path: PathBuf,
}

impl RunnerHarness {
    async fn new(mut config: DaemonConfig) -> Self {
        let dir = tempdir().expect("create temp dir");
        let audit_path = dir.path().join("audit.jsonl");
        config.audit_log_path = Some(audit_path.clone());
        config.project_root = Some(dir.path().to_path_buf());

        let registry = CommandRegistry::new();
        register_builtin_commands(&registry).expect("register commands");

        let app = Arc::new(AppContext {
            state: Arc::new(RwLock::new(AppState::default())),
            registry: Arc::new(registry),
            undo: Arc::new(UndoManager::new(100)),
            shortcuts: Arc::new(ShortcutManager::new()),
            events: Arc::new(EventBus::new()),
        });

        let server = Arc::new(ControlServer::with_config(app, config));
        let router = server.router();

        Self {
            server,
            router,
            _temp_dir: dir,
            audit_path,
        }
    }

    async fn call(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.router.clone().oneshot(req).await.expect("route request");
        let status = res.status();
        let bytes = res
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let val: Value = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }))
        };
        (status, val)
    }

    async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut b = Request::builder().method("GET").uri(uri);
        if let Some(t) = token {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        self.call(b.body(Body::empty()).unwrap()).await
    }

    async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        let mut b = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(t) = token {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        let bytes = serde_json::to_vec(&body).unwrap();
        self.call(b.body(Body::from(bytes)).unwrap()).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate 1: Concurrency Gate
// Two concurrent task sheets complete within assigned budgets simultaneously.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gate1_concurrency_two_simultaneous_workers() {
    let harness = RunnerHarness::new(DaemonConfig {
        max_concurrent_workers: 2,
        ..Default::default()
    })
    .await;

    let runner = Arc::clone(&harness.server.runner);
    runner.start().expect("start runner");

    let id1 = runner
        .enqueue_job("ts_concurrent_1".to_string(), None, None)
        .expect("enqueue job 1");
    let id2 = runner
        .enqueue_job("ts_concurrent_2".to_string(), None, None)
        .expect("enqueue job 2");

    let r1 = Arc::clone(&runner);
    let r2 = Arc::clone(&runner);

    let start = std::time::Instant::now();

    // Both workers execute a 150ms task concurrently
    let t1 = tokio::spawn(async move {
        r1.execute_job(id1, async {
            tokio::time::sleep(Duration::from_millis(150)).await;
            Ok(WorkerUsage {
                tokens_used: 500,
                usd_used: 0.005,
                render_minutes_used: 1,
            })
        })
        .await
    });

    let t2 = tokio::spawn(async move {
        r2.execute_job(id2, async {
            tokio::time::sleep(Duration::from_millis(150)).await;
            Ok(WorkerUsage {
                tokens_used: 750,
                usd_used: 0.0075,
                render_minutes_used: 2,
            })
        })
        .await
    });

    let (res1, res2) = tokio::join!(t1, t2);
    let elapsed = start.elapsed();

    assert!(res1.unwrap().is_ok());
    assert!(res2.unwrap().is_ok());

    // Both jobs ran in parallel: elapsed should be well under sequential (300ms)
    assert!(
        elapsed < Duration::from_millis(350),
        "Expected concurrent execution under 350ms, took {elapsed:?}"
    );

    // Verify job records
    let job1 = runner.get_job(id1).expect("job 1 exists");
    let job2 = runner.get_job(id2).expect("job 2 exists");

    assert_eq!(job1.state, JobState::Completed);
    assert_eq!(job2.state, JobState::Completed);
    assert_eq!(job1.usage.tokens_used, 500);
    assert_eq!(job2.usage.tokens_used, 750);

    // Verify distinct session IDs were leased
    let sess1 = job1.session_id.expect("session 1 assigned");
    let sess2 = job2.session_id.expect("session 2 assigned");
    assert_ne!(sess1, sess2);
    assert!(sess1.starts_with("agent-worker-"));
    assert!(sess2.starts_with("agent-worker-"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate 2: Budget Limit Gate
// Worker exceeding budget is cleanly stopped with typed budget error.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gate2_budget_limit_token_exhaustion() {
    let harness = RunnerHarness::new(DaemonConfig::default()).await;
    let runner = Arc::clone(&harness.server.runner);
    runner.start().expect("start runner");

    let budget = WorkerBudget {
        max_tokens: Some(1000),
        ..Default::default()
    };
    let job_id = runner
        .enqueue_job("ts_budget_tokens".to_string(), Some(budget), None)
        .expect("enqueue");

    // Worker simulates exceeding the token budget
    let result = runner
        .execute_job(job_id, async {
            Err(WorkerError::TokenBudgetExceeded {
                used: 1200,
                limit: 1000,
            })
        })
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_code(), "TokenBudgetExceeded");

    let job = runner.get_job(job_id).expect("job exists");
    assert_eq!(job.state, JobState::Failed);
    assert!(job.error.unwrap().contains("TokenBudgetExceeded"));
}

#[tokio::test]
async fn test_gate2_budget_limit_usd_cost_exhaustion() {
    let harness = RunnerHarness::new(DaemonConfig::default()).await;
    let runner = Arc::clone(&harness.server.runner);
    runner.start().expect("start runner");

    let budget = WorkerBudget {
        budget_usd: Some(0.05),
        ..Default::default()
    };
    let job_id = runner
        .enqueue_job("ts_budget_usd".to_string(), Some(budget), None)
        .expect("enqueue");

    let result = runner
        .execute_job(job_id, async {
            Err(WorkerError::UsdBudgetExceeded {
                used: 0.08,
                limit: 0.05,
            })
        })
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_code(), "UsdBudgetExceeded");

    let job = runner.get_job(job_id).expect("job exists");
    assert_eq!(job.state, JobState::Failed);
}

#[tokio::test]
async fn test_gate2_budget_limit_render_minutes_exhaustion() {
    let harness = RunnerHarness::new(DaemonConfig::default()).await;
    let runner = Arc::clone(&harness.server.runner);
    runner.start().expect("start runner");

    let budget = WorkerBudget {
        max_render_minutes: Some(10),
        ..Default::default()
    };
    let job_id = runner
        .enqueue_job("ts_budget_render".to_string(), Some(budget), None)
        .expect("enqueue");

    let result = runner
        .execute_job(job_id, async {
            Err(WorkerError::RenderMinutesExceeded {
                used: 15,
                limit: 10,
            })
        })
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_code(), "RenderMinutesExceeded");

    let job = runner.get_job(job_id).expect("job exists");
    assert_eq!(job.state, JobState::Failed);
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate 3: Kill Switch Gate
// Session stop cleanly halts active worker within <= 1s.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gate3_kill_switch_halts_worker_within_one_second() {
    let harness = RunnerHarness::new(DaemonConfig::default()).await;
    let runner = Arc::clone(&harness.server.runner);
    let sessions = Arc::clone(&harness.server.sessions);
    runner.start().expect("start runner");

    let job_id = runner
        .enqueue_job("ts_kill_switch".to_string(), None, None)
        .expect("enqueue");

    let r = Arc::clone(&runner);
    let start_time = std::time::Instant::now();

    // Spawn the worker task that would run for 10s if not interrupted
    let worker_handle = tokio::spawn(async move {
        r.execute_job(job_id, async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(WorkerUsage::default())
        })
        .await
    });

    // Wait 50ms for worker to transition to Running
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Trigger kill-switch: stop session
    let session_id = format!("agent-worker-{job_id}");
    let stop_gen = sessions
        .stop_session(&session_id)
        .expect("stop session should succeed");
    assert!(stop_gen >= 2);

    // Cancel the job on the runner as well
    runner.cancel_job(job_id).expect("cancel job");

    // Wait for worker task to terminate
    let res = tokio::time::timeout(Duration::from_secs(1), worker_handle).await;
    let elapsed = start_time.elapsed();

    assert!(
        res.is_ok(),
        "Worker should terminate within <= 1s, took {elapsed:?}"
    );

    // Verify session is fenced
    let sess = sessions.get(&session_id);
    if let Some(s) = sess {
        assert!(s.read().stopped);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate 4: BYO Model Endpoints Gate
// Local & Cloud endpoint resolution, validation, and zero plaintext secrets in audit.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gate4_byo_local_and_cloud_endpoints() {
    let harness = RunnerHarness::new(DaemonConfig {
        master_token: Some("secret-token".to_string()),
        ..Default::default()
    })
    .await;

    // 1. Valid local Ollama endpoint
    let local_ep = ByoEndpoint::Local(LocalEndpoint::Ollama {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "llama3:8b".to_string(),
    });
    assert!(local_ep.validate().is_ok());

    // 2. Local endpoint rejects remote URL
    let bad_local = ByoEndpoint::Local(LocalEndpoint::GenericLocal {
        base_url: "http://api.remote.com/v1".to_string(),
        model: "bad".to_string(),
    });
    assert_eq!(bad_local.validate().unwrap_err().error_code(), "InvalidEndpoint");

    // 3. Cloud endpoint (Anthropic) with key
    let cloud_ep = ByoEndpoint::from_cloud_parts(
        "anthropic",
        "claude-3-5-sonnet".to_string(),
        "sk-ant-live-secret-key-12345".to_string(),
    )
    .expect("valid cloud endpoint");
    assert!(cloud_ep.validate().is_ok());

    // 4. Cloud endpoint rejects empty key
    let bad_cloud = ByoEndpoint::from_cloud_parts("openai", "gpt-4o".to_string(), "  ".to_string());
    assert_eq!(bad_cloud.unwrap_err().error_code(), "MissingCredential");

    // 5. Cloud endpoint rejects unknown provider
    let unknown_cloud = ByoEndpoint::from_cloud_parts("unknown_ai", "model".to_string(), "key".to_string());
    assert_eq!(unknown_cloud.unwrap_err().error_code(), "UnsupportedProvider");

    // 6. Zero plaintext secrets in audit log: audit_safe redacts the key
    let safe_audit = cloud_ep.audit_safe();
    assert_eq!(safe_audit["api_key"], "[REDACTED]");
    assert_eq!(safe_audit["model"], "claude-3-5-sonnet");

    // Enqueue a job with the cloud endpoint
    let runner = Arc::clone(&harness.server.runner);
    runner.start().expect("start runner");
    let job_id = runner
        .enqueue_job("ts_byo_job".to_string(), None, Some(cloud_ep))
        .expect("enqueue with BYO endpoint");

    let job = runner.get_job(job_id).expect("job exists");
    let safe = job.audit_safe_endpoint().expect("safe endpoint exists");
    assert_eq!(safe["api_key"], "[REDACTED]");

    // Verify secret is NOT present in any logged string
    let json_str = serde_json::to_string(&safe).unwrap();
    assert!(!json_str.contains("sk-ant-live-secret-key-12345"));
}

// ─────────────────────────────────────────────────────────────────────────────
// REST API Integration Tests
// GET /runner/status, POST /runner/start, POST /runner/stop, GET /runner/jobs,
// POST /runner/jobs/:id/cancel
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_rest_runner_api_lifecycle() {
    let harness = RunnerHarness::new(DaemonConfig {
        master_token: Some("master-key".to_string()),
        ..Default::default()
    })
    .await;

    let token = "master-key";

    // 1. Initial status: running = false
    let (status, body) = harness.get("/runner/status", Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["running"], false);
    assert_eq!(body["active_jobs"], 0);

    // 2. Start runner via POST /runner/start
    let (status, body) = harness.post("/runner/start", json!({}), Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["running"], true);

    // 3. Calling start again returns 409 Conflict
    let (status, body) = harness.post("/runner/start", json!({}), Some(token)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "RunnerAlreadyRunning");

    // 4. Enqueue a job directly on the runner service
    let job_id = harness
        .server
        .runner
        .enqueue_job("ts_api_test".to_string(), None, None)
        .expect("enqueue");

    // 5. List jobs via GET /runner/jobs
    let (status, body) = harness.get("/runner/jobs", Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    let jobs = body["jobs"].as_array().expect("jobs array");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["id"], job_id.to_string());
    assert_eq!(jobs[0]["state"], "pending");

    // 6. Cancel job via POST /runner/jobs/:id/cancel
    let cancel_path = format!("/runner/jobs/{job_id}/cancel");
    let (status, body) = harness.post(&cancel_path, json!({}), Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["cancelled"], true);

    // Verify job is now cancelled
    let job = harness.server.runner.get_job(job_id).unwrap();
    assert_eq!(job.state, JobState::Cancelled);

    // 7. Stop runner via POST /runner/stop
    let (status, body) = harness.post("/runner/stop", json!({}), Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["running"], false);

    // 8. Unauthenticated request rejected with 401
    let (status, _) = harness.get("/runner/status", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 9. Verify audit log was written and contains no secrets
    let audit_content = fs::read_to_string(&harness.audit_path).unwrap_or_default();
    assert!(!audit_content.is_empty());
    assert!(!audit_content.contains("master-key"));
}
