// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Regression tests for the 2026-11-16 control-daemon security audit.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::server::{ControlServer, DaemonConfig};
use serde_json::{Value, json};
use tower::ServiceExt;

const MASTER: &str = "audit-master-token-0123456789";

fn router(master: Option<&str>, root: Option<&std::path::Path>) -> axum::Router {
    let registry = CommandRegistry::new();
    register_builtin_commands(&registry).unwrap();
    let app = Arc::new(AppContext {
        state: Arc::new(RwLock::new(AppState::default())),
        registry: Arc::new(registry),
        undo: Arc::new(UndoManager::new(100)),
        shortcuts: Arc::new(ShortcutManager::new()),
        events: Arc::new(EventBus::new()),
    });
    let config = DaemonConfig {
        master_token: master.map(str::to_string),
        project_root: root.map(std::path::Path::to_path_buf),
        ..Default::default()
    };
    ControlServer::with_config(app, config).router()
}

async fn send(r: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = r.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

fn post(uri: &str, token: Option<&str>, body: Value) -> Request<Body> {
    let mut b = Request::post(uri).header("content-type", "application/json");
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn execute_without_token_is_rejected() {
    let r = router(Some(MASTER), None);
    let id = {
        let (_, list) = send(&r, Request::get("/commands").header("authorization", format!("Bearer {MASTER}")).body(Body::empty()).unwrap()).await;
        list[0]["id"].as_str().unwrap().to_string()
    };
    let (status, body) = send(&r, post(&format!("/commands/{id}/execute"), None, json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "MissingAuthToken");
    let (status, _) = send(&r, post(&format!("/commands/{id}/execute"), Some("wrong"), json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn command_list_requires_token_and_health_leaks_nothing() {
    let r = router(Some(MASTER), None);
    let (status, _) = send(&r, Request::get("/commands").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = send(&r, Request::get("/health").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("commands").is_none() && body.get("active_sessions").is_none());
}

#[tokio::test]
async fn token_issuance_requires_the_master_token() {
    let r = router(Some(MASTER), None);
    let (s, b) = send(&r, post("/auth/token", None, json!({"session_id":"s","actor":"human"}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "anonymous mint must fail: {b}");
    let (s, _) = send(&r, post("/auth/token", Some("not-the-master"), json!({"session_id":"s","actor":"human"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, b) = send(&r, post("/auth/token", Some(MASTER), json!({"session_id":"s1","actor":"agent"}))).await;
    assert_eq!(s, StatusCode::OK);
    let session_token = b["token"].as_str().unwrap().to_string();
    // A session token must not be able to mint more tokens (no privilege escalation / hijack).
    let (s, _) = send(&r, post("/auth/token", Some(&session_token), json!({"session_id":"victim","actor":"human"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn token_issuance_disabled_without_master_and_rejects_reserved_identities() {
    let open = router(None, None);
    let (s, b) = send(&open, post("/auth/token", None, json!({}))).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{b}");
    let r = router(Some(MASTER), None);
    for body in [
        json!({"session_id":"master-session","actor":"agent"}),
        json!({"session_id":"s","actor":"daemon-admin"}),
        json!({"session_id":"bad id","actor":"agent"}),
    ] {
        let (s, _) = send(&r, post("/auth/token", Some(MASTER), body)).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn cross_origin_requests_are_denied() {
    let r = router(Some(MASTER), None);
    let req = Request::get("/health")
        .header("host", "127.0.0.1:8080")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let (s, b) = send(&r, req).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["code"], "CrossOriginDenied");
    let same = Request::get("/health")
        .header("host", "127.0.0.1:8080")
        .header("origin", "http://127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&r, same).await.0, StatusCode::OK);
}

#[tokio::test]
async fn project_save_cannot_escape_root_via_symlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    let r = router(Some(MASTER), Some(root.path()));
    let (_, b) = send(&r, post("/auth/token", Some(MASTER), json!({"session_id":"p","actor":"agent"}))).await;
    let t = b["token"].as_str().unwrap().to_string();
    assert_eq!(send(&r, post("/project/open", Some(&t), json!({}))).await.0, StatusCode::OK);
    let (s, _) = send(&r, post("/project/save", Some(&t), json!({"path":"link/pwn.eep"}))).await;
    assert_ne!(s, StatusCode::OK);
    assert!(!outside.path().join("pwn.eep").exists(), "file must not be written outside the root");
}
