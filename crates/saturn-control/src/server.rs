//! Control server state and routing.

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use saturn_commands::context::{AppContext, AppEvent};
use tokio::sync::broadcast;

use crate::handlers;

/// Broadcast capacity for control-socket events. Slow subscribers drop the
/// oldest events rather than blocking command execution.
const EVENT_BUFFER: usize = 1024;

/// Owns the application context and the event stream exposed over WebSocket.
pub struct ControlServer {
    app: Arc<AppContext>,
    events: broadcast::Sender<AppEvent>,
}

impl ControlServer {
    pub fn new(app: Arc<AppContext>) -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let sender = events.clone();

        // Bridge the registry's synchronous event bus into the async broadcast
        // channel that WebSocket clients subscribe to.
        app.events.subscribe(move |event| {
            let _ = sender.send(event);
        });

        Self { app, events }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }

    pub fn app(&self) -> &Arc<AppContext> {
        &self.app
    }

    pub fn router(self) -> Router {
        let state = Arc::new(self);
        Router::new()
            .route("/health", get(handlers::health))
            .route("/commands", get(handlers::list_commands))
            .route("/commands/:id/execute", post(handlers::execute_command))
            .route("/ws", get(handlers::websocket))
            .with_state(state)
    }
}
