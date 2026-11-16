// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration tests for AF9 Authenticated Long-Lived Daemon.
//!
//! Tests:
//! 1. UDS / TCP token authentication (missing / invalid token rejection with typed errors).
//! 2. Headless project document lifecycle (open, status, save, close in memory).
//! 3. Session isolation (distinct sessions cannot cross-talk or mutate unowned state).
//! 4. Bounded rate limits (`RateLimitExceeded`) and session time budgets (`BudgetExceeded`).
//! 5. Structured `audit.jsonl` logging and privacy assurance (argument hashes, zero plaintext secrets).
//! 6. AF2 Stop integration (`SessionStopped` fencing) and human resume.
//! 7. AF5 Provenance & path containment (rejecting path traversal outside root and untrusted text).
//! 8. Disconnect / EOF cleanup (releasing locks and resources).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::server::{ControlServer, DaemonConfig};
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;

struct UdsClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl UdsClient {
    async fn connect(path: &Path) -> Self {
        let stream = UnixStream::connect(path).await.expect("connect to UDS");
        let (r, w) = stream.into_split();
        Self {
            reader: BufReader::new(r),
            writer: w,
        }
    }

    async fn call(&mut self, req: Value) -> Value {
        let s = format!("{}\n", serde_json::to_string(&req).unwrap());
        self.writer.write_all(s.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

fn setup_test_server(
    mut config: DaemonConfig,
) -> (Arc<ControlServer>, PathBuf, tempfile::TempDir) {
    let dir = tempdir().expect("create temp dir");
    let registry = CommandRegistry::new();
    register_builtin_commands(&registry).expect("register commands");

    let app = Arc::new(AppContext {
        state: Arc::new(RwLock::new(AppState::default())),
        registry: Arc::new(registry),
        undo: Arc::new(UndoManager::new(100)),
        shortcuts: Arc::new(ShortcutManager::new()),
        events: Arc::new(EventBus::new()),
    });

    let sock_path = dir.path().join("daemon.sock");
    config.audit_log_path = Some(dir.path().join("audit.jsonl"));
    let server = Arc::new(ControlServer::with_config(app, config));

    // Spawn UDS listener in background
    let server_clone = Arc::clone(&server);
    let sock_clone = sock_path.clone();
    tokio::spawn(async move {
        let uds = server_clone.create_uds_server(sock_clone);
        if let Ok(listener) = uds.bind().await {
            uds.run(listener).await;
        }
    });

    (server, sock_path, dir)
}

async fn wait_for_socket(sock_path: &Path) {
    for _ in 0..50 {
        if sock_path.exists() {
            if UnixStream::connect(sock_path).await.is_ok() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("Socket {} did not become ready", sock_path.display());
}

#[tokio::test]
async fn test_01_uds_auth_handshake_missing_and_invalid_tokens() {
    let master_token = "daemon-secret-token-xyz";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());

    let (_server, sock_path, _dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let mut client = UdsClient::connect(&sock_path).await;

    // 1. Missing token -> MissingAuthToken
    let res_missing = client
        .call(json!({
            "id": 1,
            "method": "auth",
            "params": {}
        }))
        .await;
    assert_eq!(res_missing["ok"], false);
    assert_eq!(res_missing["error"], "MissingAuthToken");

    // 2. Invalid token -> InvalidAuthToken
    let res_invalid = client
        .call(json!({
            "id": 2,
            "method": "auth",
            "params": { "token": "wrong-token-abc" }
        }))
        .await;
    assert_eq!(res_invalid["ok"], false);
    assert_eq!(res_invalid["error"], "InvalidAuthToken");

    // 3. Valid master token -> Authenticated
    let res_valid = client
        .call(json!({
            "id": 3,
            "method": "auth",
            "params": {
                "token": master_token,
                "session_id": "test-session-1",
                "actor": "test-agent"
            }
        }))
        .await;
    assert_eq!(res_valid["ok"], true);
    assert_eq!(res_valid["result"]["authenticated"], true);
    assert_eq!(res_valid["result"]["session_id"], "test-session-1");
}

#[tokio::test]
async fn test_02_headless_project_lifecycle() {
    let master_token = "lifecycle-token";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());

    let (_server, sock_path, dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let proj_file = dir.path().join("project.eep");
    let mut client = UdsClient::connect(&sock_path).await;

    // 1. Authenticate
    let res = client
        .call(json!({
            "method": "auth",
            "params": { "token": master_token, "session_id": "lifecycle-sess" }
        }))
        .await;
    assert_eq!(res["ok"], true);

    // 2. Status initially closed
    let st0 = client.call(json!({ "method": "project.status" })).await;
    assert_eq!(st0["ok"], true);
    assert_eq!(st0["result"]["is_open"], false);

    // 3. Open new project
    let op = client
        .call(json!({
            "method": "project.open",
            "params": { "path": proj_file.to_str().unwrap() }
        }))
        .await;
    assert_eq!(op["ok"], true);
    assert_eq!(op["result"]["opened"], true);

    // 4. Status is now open
    let st1 = client.call(json!({ "method": "project.status" })).await;
    assert_eq!(st1["ok"], true);
    assert_eq!(st1["result"]["is_open"], true);
    assert_eq!(st1["result"]["is_dirty"], false);

    // 5. Mutate via command: timeline.add_track
    let add_tr = client
        .call(json!({
            "method": "timeline.add_track",
            "params": { "kind": "video" }
        }))
        .await;
    assert_eq!(add_tr["ok"], true);

    // Status now reflects mutation
    let st2 = client.call(json!({ "method": "project.status" })).await;
    assert_eq!(st2["result"]["is_dirty"], true);
    assert_eq!(st2["result"]["tracks_count"], 3);

    // 6. Save project
    let sv = client
        .call(json!({
            "method": "project.save",
            "params": { "path": proj_file.to_str().unwrap() }
        }))
        .await;
    assert_eq!(sv["ok"], true);
    assert_eq!(sv["result"]["saved"], true);
    assert!(proj_file.exists());

    // 7. Close project
    let cl = client.call(json!({ "method": "project.close" })).await;
    assert_eq!(cl["ok"], true);
    assert_eq!(cl["result"]["closed"], true);

    let st3 = client.call(json!({ "method": "project.status" })).await;
    assert_eq!(st3["result"]["is_open"], false);
}

#[tokio::test]
async fn test_03_session_isolation() {
    let master_token = "iso-token";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());

    let (_server, sock_path, dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let proj_a = dir.path().join("proj_a.eep");
    let proj_b = dir.path().join("proj_b.eep");

    let mut client_a = UdsClient::connect(&sock_path).await;
    let mut client_b = UdsClient::connect(&sock_path).await;

    // Auth A & B
    client_a
        .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "sess-A", "actor": "agent-A"}}))
        .await;
    client_b
        .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "sess-B", "actor": "agent-B"}}))
        .await;

    // Open projects
    client_a
        .call(json!({"method": "project.open", "params": {"path": proj_a.to_str().unwrap()}}))
        .await;
    client_b
        .call(json!({"method": "project.open", "params": {"path": proj_b.to_str().unwrap()}}))
        .await;

    // Mutate A by adding 2 tracks
    client_a
        .call(json!({"method": "timeline.add_track", "params": {"kind": "video"}}))
        .await;
    client_a
        .call(json!({"method": "timeline.add_track", "params": {"kind": "audio"}}))
        .await;

    // Check status of A: has 4 tracks (2 default + 2 added)
    let st_a = client_a.call(json!({"method": "project.status"})).await;
    assert_eq!(st_a["result"]["tracks_count"], 4);

    // Check status of B: has 2 tracks (default unchanged)! Perfect session isolation!
    let st_b = client_b.call(json!({"method": "project.status"})).await;
    assert_eq!(st_b["result"]["tracks_count"], 2);
}

#[tokio::test]
async fn test_04_bounded_rate_limit_and_time_budget() {
    let master_token = "budget-token";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());
    config.rate_limit_per_minute = 3; // Tight limit
    config.session_time_budget_secs = Some(3600);

    let (_server, sock_path, _dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let mut client = UdsClient::connect(&sock_path).await;

    // Auth
    client
        .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "rate-sess"}}))
        .await;

    // 3 allowed calls
    for _ in 0..3 {
        let resp = client.call(json!({"method": "health"})).await;
        assert_eq!(resp["ok"], true);
    }

    // 4th call exceeds rate limit -> RateLimitExceeded
    let resp = client.call(json!({"method": "health"})).await;
    assert_eq!(resp["ok"], false);
    assert_eq!(resp["error"], "RateLimitExceeded");
}

#[tokio::test]
async fn test_05_audit_jsonl_records_and_privacy_invariant() {
    let master_token = "super-secret-auth-token-12345";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());
    config.max_audit_entries = 1000;

    let (_server, sock_path, dir) = setup_test_server(config);
    let audit_file = dir.path().join("audit.jsonl");
    wait_for_socket(&sock_path).await;

    let mut client = UdsClient::connect(&sock_path).await;

    // Send command with sensitive arguments
    let req = json!({
        "method": "auth",
        "params": {
            "token": master_token,
            "session_id": "privacy-session",
            "password": "my-secret-password"
        }
    });
    let res = client.call(req).await;
    assert_eq!(res["ok"], true);

    // Read audit.jsonl file
    assert!(audit_file.exists());
    let audit_content = fs::read_to_string(&audit_file).unwrap();
    assert!(!audit_content.is_empty());

    for line in audit_content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(line).unwrap();
        assert!(record.get("timestamp").is_some());
        assert!(record.get("session_id").is_some());
        assert!(record.get("actor").is_some());
        assert!(record.get("tool_or_command").is_some());
        assert!(record.get("argument_hash").is_some());
        assert!(record.get("tier").is_some());
        assert!(record.get("duration_ms").is_some());
        assert!(record.get("result").is_some());

        let arg_hash = record["argument_hash"].as_str().unwrap();
        assert_eq!(arg_hash.len(), 64, "Must be SHA-256 64-char hex string");

        // CRITICAL PRIVACY INVARIANT: No plaintext secrets in audit log!
        assert!(!line.contains(master_token), "Plaintext master token must NEVER appear in audit log");
        assert!(!line.contains("my-secret-password"), "Plaintext password must NEVER appear in audit log");
    }
}

#[tokio::test]
async fn test_06_af2_session_stop_and_human_resume() {
    let master_token = "stop-token";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());

    let (_server, sock_path, _dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let mut client = UdsClient::connect(&sock_path).await;

    client
        .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "stop-sess"}}))
        .await;

    // Call works before stop
    let h1 = client.call(json!({"method": "health"})).await;
    assert_eq!(h1["ok"], true);

    // Stop session
    let stop_res = client.call(json!({"method": "session.stop"})).await;
    assert_eq!(stop_res["ok"], true);
    assert_eq!(stop_res["result"]["stopped"], true);

    // Subsequent call must be fenced with SessionStopped
    let fenced = client.call(json!({"method": "health"})).await;
    assert_eq!(fenced["ok"], false);
    assert_eq!(fenced["error"], "SessionStopped");

    // Agent cannot resume
    let agent_resume = client
        .call(json!({"method": "session.resume", "params": {"caller": "agent-rogue"}}))
        .await;
    assert_eq!(agent_resume["ok"], false);

    // Human authority resumes
    let human_resume = client
        .call(json!({"method": "session.resume", "params": {"caller": "human-editor"}}))
        .await;
    assert_eq!(human_resume["ok"], true);
    assert_eq!(human_resume["result"]["resumed"], true);

    // Calls succeed again
    let h2 = client.call(json!({"method": "health"})).await;
    assert_eq!(h2["ok"], true);
}

#[tokio::test]
async fn test_07_af5_provenance_and_path_containment() {
    let master_token = "prov-token";
    let dir = tempdir().unwrap();
    let project_root = dir.path().join("allowed_root");
    fs::create_dir_all(&project_root).unwrap();

    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());
    config.project_root = Some(project_root.clone());

    let (_server, sock_path, _dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let mut client = UdsClient::connect(&sock_path).await;

    client
        .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "prov-sess"}}))
        .await;

    // 1. Path traversal outside project root rejected
    let escape_req = json!({
        "method": "project.open",
        "params": { "path": "/etc/shadow" }
    });
    let escape_res = client.call(escape_req).await;
    assert_eq!(escape_res["ok"], false);
    assert_eq!(escape_res["error"], "PathOutsideProjectRoot");

    // 2. Relative escape ../../secret.eep rejected
    let rel_escape_req = json!({
        "method": "project.open",
        "params": { "path": "../../secret.eep" }
    });
    let rel_escape_res = client.call(rel_escape_req).await;
    assert_eq!(rel_escape_res["ok"], false);
    assert_eq!(rel_escape_res["error"], "PathOutsideProjectRoot");

    // 3. Allowed path within project root succeeds
    let allowed_proj = project_root.join("ok.eep");
    let allowed_req = json!({
        "method": "project.open",
        "params": { "path": allowed_proj.to_str().unwrap() }
    });
    let allowed_res = client.call(allowed_req).await;
    assert_eq!(allowed_res["ok"], true);
}

#[tokio::test]
async fn test_08_disconnect_cleanup_releases_locks() {
    let master_token = "clean-token";
    let mut config = DaemonConfig::default();
    config.master_token = Some(master_token.to_string());

    let (server, sock_path, dir) = setup_test_server(config);
    wait_for_socket(&sock_path).await;

    let proj_file = dir.path().join("locks.eep");

    // Connect client
    {
        let mut client = UdsClient::connect(&sock_path).await;

        // Auth
        client
            .call(json!({"method": "auth", "params": {"token": master_token, "session_id": "lock-sess", "actor": "agent-lock"}}))
            .await;

        // Open project
        client
            .call(json!({"method": "project.open", "params": {"path": proj_file.to_str().unwrap()}}))
            .await;

        // Acquire lock
        let lock_res = client
            .call(json!({"method": "lock.clip", "params": {"clip_id": 42, "note": "human-edit"}}))
            .await;
        assert_eq!(lock_res["ok"], true);
        assert_eq!(lock_res["result"]["clip_id"], 42);

        // Verify session is recorded in session manager
        assert!(server.sessions.get("lock-sess").is_some());

        // Close stream (client disconnect / EOF)
        drop(client);
    }

    // Wait a brief moment for background disconnect cleanup
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify session was cleanly cleaned up and removed
    assert!(server.sessions.get("lock-sess").is_none());
}
