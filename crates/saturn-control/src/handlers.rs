//! HTTP and WebSocket handlers for the control server.

use std::sync::Arc;

use axum::{
    Json,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use saturn_commands::{CommandId, CommandSpec};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::server::ControlServer;

/// Optional request body: `{"params": {...}}` or a bare parameter object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ExecuteRequest {
    Wrapped { params: Value },
    Bare(Value),
}

impl ExecuteRequest {
    fn into_params(self) -> Value {
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
}

/// `GET /health` — liveness probe with the number of executable commands.
pub async fn health(State(server): State<Arc<ControlServer>>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "commands": server.app().registry.all_specs().len(),
    }))
}

/// `GET /commands` — every registered command spec, for client introspection.
pub async fn list_commands(State(server): State<Arc<ControlServer>>) -> Json<Vec<CommandSpec>> {
    Json(server.app().registry.all_specs())
}

/// `POST /commands/:id/execute` — run one command and return its JSON result.
pub async fn execute_command(
    State(server): State<Arc<ControlServer>>,
    Path(id): Path<String>,
    body: Option<Json<ExecuteRequest>>,
) -> Response {
    let command_id = CommandId::new(id);

    if server.app().registry.get_spec(&command_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("command not found: {command_id}"),
            }),
        )
            .into_response();
    }

    let params = body
        .map(|Json(request)| request.into_params())
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| json!({}));

    debug!(%command_id, "executing control command");

    match server
        .app()
        .registry
        .execute(&command_id, params, server.app())
        .await
    {
        Ok(value) => (
            StatusCode::OK,
            Json(CommandResponse {
                success: true,
                command_id: command_id.as_str().to_string(),
                value: Some(value),
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                success: false,
                error: error.to_string(),
            }),
        )
            .into_response(),
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
                // A slow client missed events; keep streaming from the head.
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "websocket client lagged");
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
