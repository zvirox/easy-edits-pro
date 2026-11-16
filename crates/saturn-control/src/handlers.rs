// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! HTTP and WebSocket handlers for the control server and daemon (AF9).

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use saturn_commands::{CommandId, CommandSpec};
use saturn_core::actor::ActorId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tracing::{debug, warn};
use crate::project::{project_close, project_open, project_save, project_status};
use crate::server::ControlServer;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ExecuteRequest {
    Wrapped { params: Value },
    Bare(Value),
}

impl ExecuteRequest {
    pub fn into_params(self) -> Value {
        match self {
            ExecuteRequest::Wrapped { params } => params,
            ExecuteRequest::Bare(value) => value,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub command_id: String,
    pub value: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

fn extract_auth(headers: &HeaderMap, server: &ControlServer) -> Result<(String, String), (StatusCode, Json<ErrorResponse>)> {
    let header_val = headers
        .get("authorization")
        .or_else(|| headers.get("x-auth-token"))
        .and_then(|v| v.to_str().ok());

    match server.auth.validate_token(header_val) {
        Ok(claims) => Ok((claims.session_id, claims.actor)),
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )),
    }
}

/// `GET /health` — liveness probe with command specs and active daemon sessions.
pub async fn health(State(server): State<Arc<ControlServer>>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "commands": server.app().registry.all_specs().len(),
        "active_sessions": server.sessions.count(),
    }))
}

/// `POST /auth/token` — issue a new session token.
pub async fn issue_token(
    State(server): State<Arc<ControlServer>>,
    body: Option<Json<Value>>,
) -> impl IntoResponse {
    let payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let session_id = payload
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("default-session");
    let actor = payload
        .get("actor")
        .and_then(Value::as_str)
        .unwrap_or("agent");

    let token = server.auth.issue_session_token(session_id, actor);
    let _session = server.sessions.get_or_create(session_id, actor);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "token": token,
            "session_id": session_id,
            "actor": actor,
        })),
    )
}

/// `POST /project/open` — open project headless in session.
pub async fn project_open_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let (session_id, actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let session_arc = server.sessions.get_or_create(&session_id, &actor);
    let mut session = session_arc.write();

    if let Err(e) = session.check_state() {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response();
    }

    let payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let path_opt = payload.get("path").and_then(Value::as_str);

    match project_open(&mut session, path_opt, server.project_root.as_deref()) {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response(),
    }
}

/// `POST /project/close` — close active project.
pub async fn project_close_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let (session_id, actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let session_arc = server.sessions.get_or_create(&session_id, &actor);
    let mut session = session_arc.write();

    match project_close(&mut session) {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response(),
    }
}

/// `GET /project/status` — inspect headless project document state.
pub async fn project_status_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let (session_id, actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let session_arc = server.sessions.get_or_create(&session_id, &actor);
    let session = session_arc.read();
    (StatusCode::OK, Json(project_status(&session))).into_response()
}

/// `POST /project/save` — save project document to path.
pub async fn project_save_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let (session_id, actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let session_arc = server.sessions.get_or_create(&session_id, &actor);
    let mut session = session_arc.write();

    let payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let path_opt = payload.get("path").and_then(Value::as_str);

    match project_save(&mut session, path_opt, server.project_root.as_deref()) {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response(),
    }
}

/// `POST /session/stop` — AF2 Stop halts active agent session immediately.
pub async fn session_stop_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let (session_id, _actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    match server.sessions.stop_session(&session_id) {
        Ok(generation) => {
            server.broadcast(json!({
                "type": "session_stopped",
                "session_id": session_id,
                "generation": generation,
            }));
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "stopped": true,
                    "session_id": session_id,
                    "generation": generation,
                })),
            )
            .into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response(),
    }
}

/// `POST /session/resume` — AF2 Resume resumes stopped session under human authority.
pub async fn session_resume_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let (session_id, _actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let caller_str = payload
        .get("caller")
        .and_then(Value::as_str)
        .unwrap_or("human");
    let caller = if caller_str.contains("human") || caller_str == "admin" {
        ActorId::human()
    } else {
        ActorId::agent(caller_str)
    };

    match server.sessions.resume_session(&session_id, &caller) {
        Ok(generation) => {
            server.broadcast(json!({
                "type": "session_resumed",
                "session_id": session_id,
                "generation": generation,
            }));
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "resumed": true,
                    "session_id": session_id,
                    "generation": generation,
                })),
            )
            .into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            }),
        )
            .into_response(),
    }
}

/// `GET /session/status` — inspect session status, generation, and stopped flag.
pub async fn session_status_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let (session_id, actor) = match extract_auth(&headers, &server) {
        Ok(s) => s,
        Err(err) => return err.into_response(),
    };

    let session_arc = server.sessions.get_or_create(&session_id, &actor);
    let session = session_arc.read();

    (
        StatusCode::OK,
        Json(json!({
            "session_id": session.id,
            "actor": session.actor,
            "stopped": session.stopped,
            "generation": session.generation,
            "locks": session.locks.iter().collect::<Vec<_>>(),
            "created_at_unix_ms": session.created_at_unix_ms,
        })),
    )
        .into_response()
}

/// `GET /commands` — every registered command spec, for client introspection.
pub async fn list_commands(State(server): State<Arc<ControlServer>>) -> Json<Vec<CommandSpec>> {
    Json(server.app().registry.all_specs())
}

/// `POST /commands/:id/execute` — run one command and return its JSON result.
pub async fn execute_command(
    State(server): State<Arc<ControlServer>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<ExecuteRequest>>,
) -> Response {
    let command_id = CommandId::new(id);

    if server.app().registry.get_spec(&command_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("command not found: {command_id}"),
                code: Some("CommandNotFound".into()),
            }),
        )
            .into_response();
    }

    let params = body
        .map(|Json(request)| request.into_params())
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| json!({}));

    // If auth header is provided, enforce session state, rate limit, and budget
    let auth_info = headers
        .get("authorization")
        .or_else(|| headers.get("x-auth-token"))
        .and_then(|v| v.to_str().ok());

    let (session_id, actor) = if let Some(raw_token) = auth_info {
        match server.auth.validate_token(Some(raw_token)) {
            Ok(claims) => {
                let session_arc = server.sessions.get_or_create(&claims.session_id, &claims.actor);
                let mut session = session_arc.write();

                if let Err(e) = session.check_state() {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            success: false,
                            error: e.to_string(),
                            code: Some(e.error_code().to_string()),
                        }),
                    )
                        .into_response();
                }

                if let Err(e) = session.check_budgets() {
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        Json(ErrorResponse {
                            success: false,
                            error: e.to_string(),
                            code: Some(e.error_code().to_string()),
                        }),
                    )
                        .into_response();
                }

                (claims.session_id, claims.actor)
            }
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        success: false,
                        error: e.to_string(),
                        code: Some(e.error_code().to_string()),
                    }),
                )
                    .into_response();
            }
        }
    } else {
        ("anonymous".to_string(), "anonymous".to_string())
    };

    debug!(%command_id, "executing control command");
    let start_time = Instant::now();

    match server
        .app()
        .registry
        .execute(&command_id, params.clone(), server.app())
        .await
    {
        Ok(value) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut logger = server.audit.lock();
            let _ = logger.record(
                &session_id,
                &actor,
                command_id.as_str(),
                &params,
                "edit",
                duration_ms,
                "ok",
            );

            (
                StatusCode::OK,
                Json(CommandResponse {
                    success: true,
                    command_id: command_id.as_str().to_string(),
                    value: Some(value),
                }),
            )
                .into_response()
        }
        Err(error) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut logger = server.audit.lock();
            let _ = logger.record(
                &session_id,
                &actor,
                command_id.as_str(),
                &params,
                "edit",
                duration_ms,
                "error",
            );

            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse {
                    success: false,
                    error: error.to_string(),
                    code: Some("ExecutionError".into()),
                }),
            )
                .into_response()
        }
    }
}

/// `GET /ws` — upgrade to a WebSocket that streams registry events.
pub async fn websocket(
    State(server): State<Arc<ControlServer>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| stream_events(socket, server))
}

async fn stream_events(mut socket: WebSocket, server: Arc<ControlServer>) {
    let mut events = server.subscribe();
    let mut broadcasts = server.subscribe_broadcasts();
    let hello = json!({
        "type": "connected",
        "commands": server.app().registry.all_specs().len(),
    });

    if socket.send(Message::Text(hello.to_string())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    let payload = json!({ "type": "event", "event": event });
                    if socket.send(Message::Text(payload.to_string())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "websocket client lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            broadcast_msg = broadcasts.recv() => match broadcast_msg {
                Ok(msg) => {
                    if socket.send(Message::Text(msg.to_string())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "websocket broadcast lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(error)) => {
                    debug!(%error, "websocket receive failed");
                    break;
                }
                Some(Ok(_)) => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_accepts_wrapped_params() {
        let request: ExecuteRequest = serde_json::from_value(json!({"params": {"a": 1}})).unwrap();
        assert_eq!(request.into_params(), json!({"a": 1}));
    }

    #[test]
    fn request_accepts_bare_params() {
        let request: ExecuteRequest = serde_json::from_value(json!({"a": 1})).unwrap();
        assert_eq!(request.into_params(), json!({"a": 1}));
    }
}
