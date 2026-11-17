// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Control server state, router, and UDS runner (AF9).

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use parking_lot::Mutex;
use saturn_commands::context::{AppContext, AppEvent};
use tokio::sync::broadcast;

use crate::audit::{AuditLogger, DEFAULT_MAX_AUDIT_ENTRIES, SharedAuditLogger};
use crate::auth::AuthManager;
use crate::handlers;
use crate::runner::{RunnerConfig, RunnerService};
use crate::session::SessionManager;
use crate::uds::UdsServer;

const EVENT_BUFFER: usize = 1024;

/// Configuration for the control server and daemon.
#[derive(Clone, Debug)]
pub struct DaemonConfig {
    pub master_token: Option<String>,
    pub rate_limit_per_minute: u32,
    pub session_time_budget_secs: Option<u64>,
    pub project_root: Option<PathBuf>,
    pub audit_log_path: Option<PathBuf>,
    pub max_audit_entries: usize,
    pub max_concurrent_workers: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            master_token: None,
            rate_limit_per_minute: 120,
            session_time_budget_secs: Some(3600),
            project_root: None,
            audit_log_path: None,
            max_audit_entries: DEFAULT_MAX_AUDIT_ENTRIES,
            max_concurrent_workers: 2,
        }
    }
}

/// Owns the application context, authentication, sessions, and audit logging.
pub struct ControlServer {
    pub app: Arc<AppContext>,
    pub events: broadcast::Sender<AppEvent>,
    pub broadcasts: broadcast::Sender<serde_json::Value>,
    pub auth: Arc<AuthManager>,
    pub sessions: Arc<SessionManager>,
    pub audit: SharedAuditLogger,
    pub project_root: Option<PathBuf>,
    pub runner: Arc<RunnerService>,
}

impl ControlServer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self::with_config(app, DaemonConfig::default())
    }

    pub fn with_config(app: Arc<AppContext>, config: DaemonConfig) -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let sender = events.clone();

        app.events.subscribe(move |event| {
            let _ = sender.send(event);
        });

        let (broadcasts, _) = broadcast::channel(EVENT_BUFFER);

        let auth = Arc::new(AuthManager::new(config.master_token));
        let sessions = Arc::new(SessionManager::new(
            config.rate_limit_per_minute,
            config.session_time_budget_secs,
            config.project_root.clone(),
        ));
        let audit_logger = AuditLogger::new(config.audit_log_path.clone(), config.max_audit_entries)
            .expect("initialize audit logger");
        let audit = Arc::new(Mutex::new(audit_logger));

        let runner_config = RunnerConfig {
            max_concurrent_workers: config.max_concurrent_workers,
            ..Default::default()
        };
        let runner = Arc::new(RunnerService::new(runner_config, Arc::clone(&sessions)));

        Self {
            app,
            events,
            broadcasts,
            auth,
            sessions,
            audit,
            project_root: config.project_root,
            runner,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }

    pub fn broadcast(&self, message: serde_json::Value) {
        let _ = self.broadcasts.send(message);
    }

    pub fn subscribe_broadcasts(&self) -> broadcast::Receiver<serde_json::Value> {
        self.broadcasts.subscribe()
    }

    pub fn app(&self) -> &Arc<AppContext> {
        &self.app
    }

    pub fn router(&self) -> Router {
        let state = Arc::new(Self {
            app: Arc::clone(&self.app),
            events: self.events.clone(),
            broadcasts: self.broadcasts.clone(),
            auth: Arc::clone(&self.auth),
            sessions: Arc::clone(&self.sessions),
            audit: Arc::clone(&self.audit),
            project_root: self.project_root.clone(),
            runner: Arc::clone(&self.runner),
        });
        let base_router = Router::new()
            .route("/health", get(handlers::health))
            .route("/auth/token", post(handlers::issue_token))
            .route("/project/open", post(handlers::project_open_handler))
            .route("/project/close", post(handlers::project_close_handler))
            .route("/project/status", get(handlers::project_status_handler))
            .route("/project/save", post(handlers::project_save_handler))
            .route("/session/stop", post(handlers::session_stop_handler))
            .route("/session/resume", post(handlers::session_resume_handler))
            .route("/session/status", get(handlers::session_status_handler))
            .route("/commands", get(handlers::list_commands))
            .route("/commands/:id/execute", post(handlers::execute_command))
            .route("/ws", get(handlers::websocket));

        crate::api::mount_api_routes(base_router)
            .layer(axum::middleware::from_fn(crate::guard::origin_guard))
            .with_state(state)
    }

    pub fn create_uds_server(&self, socket_path: impl Into<PathBuf>) -> UdsServer {
        UdsServer::new(
            socket_path,
            self.auth.clone(),
            self.sessions.clone(),
            self.audit.clone(),
            self.app.clone(),
            self.project_root.clone(),
        )
    }

    pub async fn run_uds(&self, socket_path: impl Into<PathBuf>) -> std::io::Result<()> {
        let uds = self.create_uds_server(socket_path);
        let listener = uds.bind().await?;
        uds.run(listener).await;
        Ok(())
    }
}
