//! Control Server for External Automation in Easy Edits Pro
//!
//! This binary provides a WebSocket/HTTP control server that exposes
//! the command registry for external automation, scripting, and integration.

use std::sync::Arc;
use axum::{
    extract::{State, WebSocketUpgrade, Path},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use saturn_commands::{
    CommandRegistry, CommandSpec, CommandId, CommandResult,
    context::{AppContext, AppState, ExecutionContext},
    undo::UndoManager,
    shortcuts::ShortcutManager,
    register_builtin_commands,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use uuid::Uuid;
use anyhow::Result;

mod server;
mod handlers;

use server::ControlServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Saturn Control Server");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8080;
    let mut host = "127.0.0.1".to_string();

    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--control" | "-c" => {
                if let Some(p) = args.get(i + 1) {
                    port = p.parse().unwrap_or(8080);
                }
            }
            "--host" | "-h" => {
                if let Some(h) = args.get(i + 1) {
                    host = h.clone();
                }
            }
            _ => {}
        }
    }

    // Create application context
    let registry = std::sync::Arc::new(saturn_commands::CommandRegistry::new());
    saturn_commands::register_builtin_commands(&registry)?;

    let app_context = std::sync::Arc::new(saturn_commands::context::AppContext {
        state: std::sync::Arc::new(tokio::sync::RwLock::new(saturn_commands::context::AppState::default())),
        registry: std::sync::Arc::clone(&saturn_commands::CommandRegistry::new()),
        undo: std::sync::Arc::new(saturn_commands::UndoManager::new(1000)),
        shortcuts: std::sync::Arc::new(saturn_commands::ShortcutManager::new()),
        events: std::sync::Arc::new(saturn_commands::context::EventBus::new()),
    });

    // Create event broadcaster for real-time updates
    let (event_tx, _event_rx) = broadcast::channel::<AppEvent>(1000);

    // Create control server
    let server = ControlServer::new(
        saturn_commands::CommandRegistry::new(),
        event_tx,
    );

    // Build router
    let app = create_router(server);

    // Start server
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Control Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use uuid::Uuid;
use anyhow::Result;
use std::sync::Arc;
use saturn_commands::{
    CommandRegistry, CommandSpec, CommandId, CommandResult,
    context::{AppContext, AppState, AppEvent, ExecutionContext},
    undo::UndoManager,
    shortcuts::ShortcutManager,
};

/// Application event for broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    CommandExecuted { command_id: String, success: bool, result: Value },
    PlayheadMoved { position: u64 },
    SelectionChanged { clips: Vec<String>, tracks: Vec<String> },
    ProjectChanged { project_id: String },
    TimelineChanged { sequence_id: String },
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    /// Execute a command
    Execute { id: u64, command: String, params: Value },
    /// Subscribe to events
    Subscribe { events: Vec<String> },
    /// Unsubscribe from events
    Unsubscribe { events: Vec<String> },
    /// Ping
    Ping { id: u64 },
    /// Pong response
    Pong { id: u64 },
    /// Command execution result
    Result { id: u64, success: bool, result: Value, error: Option<String> },
    /// Event notification
    Event { event: AppEvent },
    /// Error
    Error { id: u64, message: String },
}

/// HTTP request for command execution
#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    command: String,
    params: Option<Value>,
}

/// HTTP response for command execution
#[derive(Debug, Serialize)]
struct ExecuteResponse {
    success: bool,
    result: Option<Value>,
    error: Option<String>,
    execution_id: String,
}

/// Application state for the control server
#[derive(Clone)]
struct ControlServerState {
    registry: Arc<CommandRegistry>,
    event_tx: broadcast::Sender<AppEvent>,
}

impl ControlServerState {
    fn new(registry: CommandRegistry) -> Self {
        let (event_tx, _event_rx) = broadcast::channel(1000);
        Self {
            registry: Arc::new(registry),
            event_tx,
        }
    }

    async fn execute_command(&self, command_id: &str, params: Value) -> ExecuteResponse {
        let command_id = CommandId::new(command_id);
        let registry = self.registry.clone();

        let handler = match registry.get(&CommandId::new(command_id)) {
            Some(h) => h,
            None => {
                return ExecuteResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Command not found: {}", command_id)),
                    execution_id: Uuid::new_v4().to_string(),
                };
            }
        };

        let spec = registry.get_spec(&CommandId::new(command_id))
            .unwrap_or_else(|| CommandSpec::new(command_id, command_id));

        // Create a minimal app context for execution
        let app_context = saturn_commands::context::AppContext {
            state: Arc::new(tokio::sync::RwLock::new(saturn_commands::context::AppState::default())),
            registry: Arc::new(saturn_commands::CommandRegistry::new()),
            undo: Arc::new(saturn_commands::UndoManager::new(100)),
            shortcuts: Arc::new(saturn_commands::ShortcutManager::new()),
            events: Arc::new(saturn_commands::context::EventBus::new()),
        };

        let exec_ctx = ExecutionContext {
            app: &app_context,
            command: &spec,
            params,
            execution_id: Uuid::new_v4(),
            start_time: std::time::Instant::now(),
        };

        match handler.execute(exec_ctx).await {
            Ok(result) => ExecuteResponse {
                success: true,
                result: Some(result),
                error: None,
                execution_id: exec_ctx.execution_id.to_string(),
            },
            Err(e) => ExecuteResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                execution_id: exec_ctx.execution_id.to_string(),
            },
        }
    }
}

/// Create the HTTP/WebSocket router
fn create_router(state: ControlServerState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/commands", get(list_commands))
        .route("/api/commands/:id", get(get_command))
        .route("/api/execute", post(execute_command))
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "saturn-control" }))
}

async fn list_commands(State(state): State<ControlServerState>) -> impl IntoResponse {
    let specs = state.registry.all_specs();
    let commands: Vec<Value> = specs.iter().map(|spec| json!({
        "id": spec.id.to_string(),
        "label": spec.label,
        "category": format!("{:?}", spec.category),
        "mutating": spec.mutating,
        "shortcut": spec.shortcut,
        "help": spec.help,
    })).collect();

    Json(json!({ "commands": commands }))
}

async fn get_command(
    State(state): State<ControlServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let spec = state.registry.get_spec(&CommandId::new(id));
    match spec {
        Some(spec) => Json(json!({
            "id": spec.id.to_string(),
            "label": spec.label,
            "category": format!("{:?}", spec.category),
            "mutating": spec.mutating,
            "shortcut": spec.shortcut,
            "help": spec.help,
            "params": spec.params.iter().map(|p| json!({
                "name": p.name,
                "label": p.label,
                "required": p.required,
                "description": p.description,
            })).collect::<Vec<_>>(),
        })).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, Json(json!({ "error": "Command not found" }))).into_response(),
    }
}

async fn execute_command(
    State(state): State<ControlServerState>,
    Json(req): Json<ExecuteRequest>,
) -> impl IntoResponse {
    let response = state.execute_command(&req.command, req.params.unwrap_or_default()).await;
    Json(response)
}

async fn undo(State(state): State<ControlServerState>) -> impl IntoResponse {
    // TODO: Implement undo via command registry
    Json(json!({ "success": false, "error": "Not implemented" }))
}

async fn redo(State(state): State<ControlServerState>) -> impl IntoResponse {
    Json(json!({ "success": false, "error": "Not implemented" }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ControlServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: ControlServerState) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.event_tx.subscribe();
    let mut pending_requests: std::collections::HashMap<u64, tokio::sync::oneshot::Sender<ExecuteResponse>> = std::collections::HashMap::new();
    let mut next_id = 0u64;

    // Spawn event forwarding task
    let mut sender_clone = sender.clone();
    let mut event_rx = state.event_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = WsMessage::Event { event };
            if let Ok(text) = serde_json::to_string(&msg) {
                let _ = sender_clone.send(axum::extract::ws::Message::Text(text.into())).await;
            }
        }
    });

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Execute { id, command, params } => {
                            let response = state.execute_command(&command, params).await;
                            let msg = WsMessage::Result {
                                id,
                                success: response.success,
                                result: response.result.unwrap_or_default(),
                                error: response.error,
                            };
                            if let Ok(text) = serde_json::to_string(&msg) {
                                let _ = sender.send(axum::extract::ws::Message::Text(text.into())).await;
                            }
                        }
                        WsMessage::Subscribe { events: _ } => {
                            // TODO: Implement event filtering
                        }
                        WsMessage::Unsubscribe { events: _ } => {
                            // TODO: Implement event filtering
                        }
                        WsMessage::Ping { id } => {
                            let msg = WsMessage::Pong { id };
                            if let Ok(text) = serde_json::to_string(&msg) {
                                let _ = sender.send(axum::extract::ws::Message::Text(text.into())).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(axum::extract::ws::Message::Binary(_)) => {}
            Ok(axum::extract::ws::Message::Close(_)) => break,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    Execute { id: u64, command: String, params: Value },
    Subscribe { events: Vec<String> },
    Unsubscribe { events: Vec<String> },
    Ping { id: u64 },
    Pong { id: u64 },
    Result { id: u64, success: bool, result: Value, error: Option<String> },
    Event { event: AppEvent },
    Error { id: u64, message: String },
}