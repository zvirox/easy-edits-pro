//! Route-level tests for the control server.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::server::ControlServer;
use serde_json::{Value, json};
use tower::ServiceExt;

fn app() -> axum::Router {
    let registry = CommandRegistry::new();
    register_builtin_commands(&registry).unwrap();
    let context = Arc::new(AppContext {
        state: Arc::new(RwLock::new(AppState::default())),
        registry: Arc::new(registry),
        undo: Arc::new(UndoManager::new(100)),
        shortcuts: Arc::new(ShortcutManager::new()),
        events: Arc::new(EventBus::new()),
    });
    ControlServer::new(context).router()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_reports_status_and_command_count() {
    let response = app()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["status"], "ok");
    assert!(body["commands"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn commands_endpoint_lists_registered_specs() {
    let response = app()
        .oneshot(Request::get("/commands").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let ids: Vec<String> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|spec| spec["id"].to_string())
        .collect();
    assert!(ids.iter().any(|id| id.contains("test.echo")));
}

#[tokio::test]
async fn execute_runs_a_command_and_returns_its_value() {
    let response = app()
        .oneshot(
            Request::post("/commands/test.echo/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"params":{"message":"hello"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["command_id"], "test.echo");
    assert_eq!(body["value"]["echo"], "hello");
}

#[tokio::test]
async fn unknown_command_is_a_not_found_error() {
    let response = app()
        .oneshot(
            Request::post("/commands/nope.nothing/execute")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn invalid_params_are_a_client_error() {
    let response = app()
        .oneshot(
            Request::post("/commands/test.echo/execute")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(response).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("message"));
}
