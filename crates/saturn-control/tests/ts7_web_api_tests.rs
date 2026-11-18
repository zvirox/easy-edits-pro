// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Conformance integration tests for TS7: Daemon Web API on saturn-control.
//!
//! Tests:
//! 1. Bearer token authentication enforcement & rejection on protected endpoints (401).
//! 2. Rate limiter & session wall-clock budget enforcement (429).
//! 3. Task Sheet REST lifecycle: create, update, submit, plan submit/approve/reject, report.
//! 4. Question pause/resume lifecycle: ask question (needs_input), answer question (running).
//! 5. Review notes & rounds lifecycle: add note, list, resolve, changes, round open/close.
//! 6. Verification & delivery endpoints (`verify.run`, `delivery.verify`).
//! 7. WebSocket `/ws` real-time broadcasts for all lifecycle transitions & events.
//! 8. Audit log invariant: complete `audit.jsonl` entries, SHA-256 hashes, zero plaintext secrets.
//! 9. AF2 Stop fencing on Web API: stopped session rejects with 403, resumed session recovers.

use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::audit::AuditRecord;
use saturn_control::server::{ControlServer, DaemonConfig};
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::net::TcpListener;
use tower::ServiceExt;

const TEST_MASTER: &str = "test-master-token-0123456789";

struct TestHarness {
    #[allow(dead_code)]
    server: Arc<ControlServer>,
    router: axum::Router,
    _temp_dir: tempfile::TempDir,
    audit_path: PathBuf,
    addr: SocketAddr,
}

impl TestHarness {
    async fn new(mut config: DaemonConfig) -> Self {
        let dir = tempdir().expect("create temp dir");
        let audit_path = dir.path().join("audit.jsonl");
        config.audit_log_path = Some(audit_path.clone());
        config.project_root = Some(dir.path().to_path_buf());
        if config.master_token.is_none() {
            config.master_token = Some(TEST_MASTER.to_string());
        }

        let registry = CommandRegistry::new();
        register_builtin_commands(&registry).expect("register builtin commands");

        let app = Arc::new(AppContext {
            state: Arc::new(RwLock::new(AppState::default())),
            registry: Arc::new(registry),
            undo: Arc::new(UndoManager::new(100)),
            shortcuts: Arc::new(ShortcutManager::new()),
            events: Arc::new(EventBus::new()),
        });

        let server = Arc::new(ControlServer::with_config(app, config));
        let router = server.router();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral TCP listener");
        let addr = listener.local_addr().expect("local addr");

        let router_clone = router.clone();
        tokio::spawn(async move {
            axum::serve(listener, router_clone)
                .await
                .expect("serve axum router");
        });

        Self {
            server,
            router,
            _temp_dir: dir,
            audit_path,
            addr,
        }
    }

    async fn call(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.router.clone().oneshot(req).await.expect("route request");
        let status = res.status();
        let bytes = res
            .into_body()
            .collect()
            .await
            .expect("read body bytes")
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
        let mut builder = Request::builder().method("GET").uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        self.call(builder.body(Body::empty()).unwrap()).await
    }

    async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        self.call(
            builder
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
    }

    async fn issue_token(&self, session_id: &str, actor: &str) -> String {
        let (status, res) = self
            .post(
                "/auth/token",
                json!({ "session_id": session_id, "actor": actor }),
                Some(TEST_MASTER),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        res["token"].as_str().expect("token field").to_string()
    }

    async fn open_project(&self, token: &str) {
        let (status, res) = self.post("/project/open", json!({}), Some(token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res["opened"], true);
    }
}

#[tokio::test]
async fn test_01_bearer_auth_rejection_missing_and_invalid() {
    let harness = TestHarness::new(DaemonConfig::default()).await;

    // 1. Missing auth token
    let protected_gets = ["/tasksheet", "/tasksheet/report", "/tasksheet/questions", "/review/notes", "/review/changes"];
    for route in protected_gets {
        let (status, body) = harness.get(route, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "route {route} must reject missing auth");
        assert_eq!(body["code"], "MissingAuthToken");
        assert_eq!(body["success"], false);
    }

    let protected_posts = [
        ("/tasksheet", json!({})),
        ("/tasksheet/submit", json!({})),
        ("/tasksheet/plan/submit", json!({ "steps": [] })),
        ("/tasksheet/plan/approve", json!({})),
        ("/tasksheet/plan/reject", json!({ "reason": "bad" })),
        ("/tasksheet/questions", json!({})),
        ("/tasksheet/questions/q1/answer", json!({ "option_id": "opt1" })),
        ("/review/notes", json!({ "text": "fix audio" })),
        ("/review/notes/n1/resolve", json!({})),
        ("/review/round/open", json!({})),
        ("/review/round/close", json!({ "outcome": "approved" })),
        ("/verify/run", json!({})),
        ("/delivery/verify", json!({})),
    ];

    for (route, payload) in &protected_posts {
        let (status, body) = harness.post(route, payload.clone(), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "POST {route} must reject missing auth");
        assert_eq!(body["code"], "MissingAuthToken");
    }

    // 2. Invalid auth token
    for (route, payload) in &protected_posts {
        let (status, body) = harness.post(route, payload.clone(), Some("invalid-bearer-token")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "POST {route} must reject invalid auth");
        assert_eq!(body["code"], "InvalidAuthToken");
    }
}

#[tokio::test]
async fn test_02_rate_limiter_and_time_budget_enforcement() {
    let mut config = DaemonConfig::default();
    config.rate_limit_per_minute = 5;
    let harness = TestHarness::new(config).await;

    let token = harness.issue_token("rate-session", "agent").await;
    harness.open_project(&token).await;

    // Up to 5 requests succeed
    for _ in 0..5 {
        let (status, _) = harness.get("/tasksheet", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
    }

    // 6th request is rejected with 429 RateLimitExceeded
    let (status, body) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "RateLimitExceeded");

    // Wall-clock budget expiration
    let mut budget_config = DaemonConfig::default();
    budget_config.session_time_budget_secs = Some(0); // Expires immediately
    let budget_harness = TestHarness::new(budget_config).await;

    let budget_token = budget_harness.issue_token("budget-session", "agent").await;
    budget_harness.open_project(&budget_token).await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    let (status, body) = budget_harness.get("/tasksheet", Some(&budget_token)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "BudgetExceeded");
}

#[tokio::test]
async fn test_03_tasksheet_rest_lifecycle_and_api_v1() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let token = harness.issue_token("ts-session", "human:creator").await;
    harness.open_project(&token).await;

    // 1. Initial GET /tasksheet returns draft default sheet
    let (status, body) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);

    // 2. POST /tasksheet creates/updates sheet with structured deliverables
    let create_payload = json!({
        "id": "sheet_ts7_01",
        "title": "TS7 Web API Deliverable",
        "requester": "human:producer",
        "deliverables": [
            {
                "id": "deliv_1080p",
                "name": "1080p Web Master",
                "preset": "web-1080p"
            }
        ]
    });
    let (status, body) = harness.post("/api/v1/tasksheet", create_payload, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["sheet"]["title"], "TS7 Web API Deliverable");

    // 3. POST /tasksheet/submit transitions to ready
    let (status, body) = harness
        .post("/tasksheet/submit", json!({ "reason": "sheet finalized" }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");

    // 4. POST /tasksheet/plan/submit transitions to awaiting_plan_approval
    let plan_payload = json!({
        "steps": [
            { "id": 1, "title": "Rough cut selection", "status": "pending" },
            { "id": 2, "title": "Loudness normalization", "status": "pending" }
        ]
    });
    let (status, body) = harness
        .post("/api/v1/tasksheet/plan/submit", plan_payload.clone(), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "awaiting_plan_approval");

    // 5. POST /tasksheet/plan/reject transitions back to planning
    let (status, body) = harness
        .post(
            "/tasksheet/plan/reject",
            json!({ "reason": "Need audio sweetening step" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "planning");

    // 6. Resubmit plan and approve
    let (status, _) = harness
        .post("/tasksheet/plan/submit", plan_payload, Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = harness
        .post(
            "/api/v1/tasksheet/plan/approve",
            json!({ "reason": "approved by producer" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "running");

    // 7. Report step progress
    let (status, body) = harness
        .post(
            "/tasksheet/run_report",
            json!({ "completed_step_id": 1, "current_step_id": 2 }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reported"], true);

    // 8. GET /tasksheet/report inspects plan and status
    let (status, body) = harness.get("/api/v1/tasksheet/report", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "running");
    assert_eq!(body["plan"]["steps"][0]["status"], "done");
    assert_eq!(body["plan"]["steps"][1]["status"], "current");
}

#[tokio::test]
async fn test_04_questions_pause_and_resume_lifecycle() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let human_token = harness.issue_token("q-session", "human:producer").await;
    let token = harness.issue_token("q-session", "agent").await;
    harness.open_project(&token).await;

    // Fast-forward sheet to running
    harness.post("/tasksheet/submit", json!({}), Some(&token)).await;
    harness
        .post(
            "/tasksheet/plan/submit",
            json!({ "steps": [{ "id": 1, "title": "Edit", "status": "pending" }] }),
            Some(&token),
        )
        .await;
    harness.post("/tasksheet/plan/approve", json!({}), Some(&human_token)).await;

    // 1. Agent asks structured question -> pauses session into needs_input
    let ask_payload = json!({
        "question_id": "q_intro_music",
        "question": "Which intro music cue should be selected?",
        "options": [
            { "id": "cue_upbeat", "label": "Upbeat Electronic" },
            { "id": "cue_ambient", "label": "Ambient Cinematic" }
        ],
        "default_option_id": "cue_upbeat",
        "deadline_seconds": 3600
    });
    let (status, body) = harness
        .post("/api/v1/tasksheet/questions", ask_payload, Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["asked"], true);
    assert_eq!(body["status"], "needs_input");

    // 2. GET /tasksheet/questions lists open question
    let (status, body) = harness.get("/tasksheet/questions", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let questions = body["questions"].as_array().expect("questions array");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0]["id"], "q_intro_music");

    // 3. Human answers question -> resumes session to running
    let (status, body) = harness
        .post(
            "/api/v1/tasksheet/questions/q_intro_music/answer",
            json!({ "option_id": "cue_ambient" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["answered"], true);
    assert_eq!(body["status"], "running");
    assert_eq!(body["option_id"], "cue_ambient");
}

#[tokio::test]
async fn test_05_review_notes_and_rounds_lifecycle() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let token = harness.issue_token("review-session", "human:reviewer").await;
    harness.open_project(&token).await;

    // 1. Open review round
    let (status, body) = harness
        .post(
            "/api/v1/review/round/open",
            json!({ "label": "Rough Cut Review", "reason": "Reviewing pacing" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["round_opened"], true);
    let round_num = body["round_number"].as_u64().unwrap();

    // 2. Add timecoded review note
    let (status, body) = harness
        .post(
            "/review/notes",
            json!({
                "id": "note_cut_01",
                "text": "Trim 12 frames at cut point",
                "target_tick": 48000,
                "round_number": round_num
            }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["added"], true);

    // 3. List review notes
    let (status, body) = harness.get("/api/v1/review/notes", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let notes = body["notes"].as_array().expect("notes array");
    assert!(notes.iter().any(|n| n["text"].as_str().unwrap().contains("Trim 12 frames")));

    // 4. Resolve review note
    let (status, body) = harness
        .post(
            "/review/notes/note_cut_01/resolve",
            json!({ "resolution": "Trimmed 12 frames on V1" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["resolved"], true);

    // 5. List changes
    let (status, body) = harness.get("/review/changes", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("changes").is_some());

    // 6. Close review round
    let (status, body) = harness
        .post(
            "/review/round/close",
            json!({ "outcome": "approved", "reason": "All notes resolved" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["round_closed"], true);
    assert_eq!(body["outcome"], "approved");
}

#[tokio::test]
async fn test_06_verification_and_delivery_endpoints() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let token = harness.issue_token("verify-session", "agent").await;
    harness.open_project(&token).await;

    // 1. POST /verify/run executes sequence checks
    let (status, body) = harness
        .post(
            "/api/v1/verify/run",
            json!({ "preset": "web-1080p" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verified"], true);
    assert_eq!(body["report"]["preset"], "web-1080p");
    assert!(body["report"]["summary"]["total_checks"].as_u64().unwrap() > 0);

    // 2. POST /delivery/verify path traversal containment check
    let (status, body) = harness
        .post(
            "/delivery/verify",
            json!({ "path": "/etc/passwd" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PathOutsideProjectRoot");
}

#[tokio::test]
async fn test_07_websocket_broadcast_integration() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let ws_url = format!("ws://{}/ws", harness.addr);

    // /ws requires a token (no anonymous event stream).
    {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let anon = tokio_tungstenite::connect_async(ws_url.as_str().into_client_request().unwrap()).await;
        assert!(anon.is_err(), "anonymous websocket upgrade must be refused");
    }
    let ws_token = harness.issue_token("ws-probe", "agent").await;
    let mut ws_req = {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        ws_url.as_str().into_client_request().unwrap()
    };
    ws_req
        .headers_mut()
        .insert("authorization", format!("Bearer {ws_token}").parse().unwrap());
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(ws_req)
        .await
        .expect("connect to /ws");

    // 1. Initial greeting
    let first_msg = ws_stream.next().await.unwrap().unwrap();
    let text = first_msg.to_text().unwrap();
    let first_json: Value = serde_json::from_str(text).unwrap();
    assert_eq!(first_json["type"], "connected");

    let token = harness.issue_token("ws-session", "human:producer").await;
    harness.open_project(&token).await;

    // 2. Create task sheet -> broadcast tasksheet_transition
    let (s, _) = harness
        .post(
            "/tasksheet",
            json!({
                "id": "sheet_ws_01",
                "title": "WS Deliverable",
                "requester": "human:producer",
                "deliverables": [
                    {
                        "id": "deliv_ws",
                        "name": "WS Master",
                        "preset": "web-1080p"
                    }
                ]
            }),
            Some(&token),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "tasksheet_transition");

    // 3. Submit task sheet -> broadcast tasksheet_transition
    let (s, _) = harness.post("/tasksheet/submit", json!({}), Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "tasksheet_transition");
    assert_eq!(json_msg["event"], "tasksheet.submit");
    assert_eq!(json_msg["status"], "ready");

    // 4. Ask question -> broadcast question_asked
    let ask_payload = json!({
        "question_id": "ws_q1",
        "question": "Export with subtitles?",
        "options": [
            { "id": "yes", "label": "Yes" },
            { "id": "no", "label": "No" }
        ],
        "default_option_id": "yes"
    });
    let (s, _) = harness.post("/tasksheet/questions", ask_payload, Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "question_asked");
    assert_eq!(json_msg["question_id"], "ws_q1");

    // 5. Answer question -> broadcast question_answered
    let (s, _) = harness
        .post("/tasksheet/questions/ws_q1/answer", json!({ "option_id": "yes" }), Some(&token))
        .await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "question_answered");
    assert_eq!(json_msg["question_id"], "ws_q1");

    // 6. Add review note -> broadcast review_note_added
    let (s, _) = harness
        .post("/review/notes", json!({ "id": "ws_note_1", "text": "Fix fade" }), Some(&token))
        .await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "review_note_added");

    // 7. Resolve review note -> broadcast review_note_resolved
    let (s, _) = harness
        .post(
            "/review/notes/ws_note_1/resolve",
            json!({ "resolution": "Fade fixed" }),
            Some(&token),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "review_note_resolved");

    // 8. Verify run -> broadcast verification_completed
    let (s, _) = harness.post("/verify/run", json!({}), Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "verification_completed");

    // 9. Session stop & resume -> broadcast session_stopped & session_resumed
    let (s, _) = harness.post("/session/stop", json!({}), Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "session_stopped");

    let (s, _) = harness
        .post("/session/resume", json!({ "caller": "human" }), Some(&token))
        .await;
    assert_eq!(s, StatusCode::OK);
    let msg = ws_stream.next().await.unwrap().unwrap();
    let json_msg: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(json_msg["type"], "session_resumed");

    let _ = ws_stream.close(None).await;
}

#[tokio::test]
async fn test_08_audit_jsonl_completeness_and_privacy() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let token = harness.issue_token("audit-session", "agent").await;
    harness.open_project(&token).await;

    // Call several endpoints
    let (s1, _) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, _) = harness
        .post(
            "/tasksheet",
            json!({
                "id": "sheet_audit",
                "title": "Audit Sheet",
                "requester": "human:user",
                "deliverables": [{ "id": "d1", "name": "Main", "preset": "web-1080p" }]
            }),
            Some(&token),
        )
        .await;
    assert_eq!(s2, StatusCode::OK);
    let (s3, _) = harness
        .post("/verify/run", json!({ "preset": "web-1080p" }), Some(&token))
        .await;
    assert_eq!(s3, StatusCode::OK);

    let audit_content = fs::read_to_string(&harness.audit_path).expect("read audit log");
    assert!(!audit_content.trim().is_empty(), "audit log must not be empty");

    let lines: Vec<&str> = audit_content.lines().collect();
    assert!(lines.len() >= 3, "expected multiple audit entries");

    for line in lines {
        let record: AuditRecord = serde_json::from_str(line).expect("valid AuditRecord JSONL");
        assert!(!record.timestamp.is_empty());
        assert_eq!(record.argument_hash.len(), 64, "SHA-256 hash must be 64 hex characters");
        assert!(record.argument_hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!line.contains(&token), "auth token must NEVER appear in audit log in plaintext");
    }
}

#[tokio::test]
async fn test_09_session_stopped_fencing_on_web_api() {
    let harness = TestHarness::new(DaemonConfig::default()).await;
    let token = harness.issue_token("fence-session", "agent").await;
    harness.open_project(&token).await;

    // 1. Normal call succeeds
    let (status, _) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    // 2. Stop session
    let (status, res) = harness.post("/session/stop", json!({}), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["stopped"], true);

    // 3. Subsequent stateful calls are fenced with 403 SessionStopped
    let (status, body) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "SessionStopped");

    let (status, body) = harness.post("/verify/run", json!({}), Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "SessionStopped");

    // 4. Human authority resumes session
    let (status, res) = harness
        .post("/session/resume", json!({ "caller": "human:supervisor" }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["resumed"], true);

    // 5. Subsequent calls succeed again
    let (status, body) = harness.get("/tasksheet", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}
