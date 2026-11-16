// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Control Server and Headless Daemon for External Automation in Easy Edits Pro (AF9).
//!
//! Exposes the command registry and headless project lifecycle over HTTP, WebSocket,
//! and Unix Domain Socket (UDS) with session token authentication, rate limits,
//! time budgets, bounded JSONL audit logging, AF2 Stop, and disconnect cleanup.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use tracing::info;

use saturn_control::server::{ControlServer, DaemonConfig};

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone)]
struct ServerArgs {
    host: String,
    port: Option<u16>,
    socket_path: Option<PathBuf>,
    auth_token: Option<String>,
    rate_limit: u32,
    time_budget_secs: Option<u64>,
    project_root: Option<PathBuf>,
    audit_file: Option<PathBuf>,
    max_audit_entries: usize,
}

impl Default for ServerArgs {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: Some(DEFAULT_PORT),
            socket_path: None,
            auth_token: None,
            rate_limit: 120,
            time_budget_secs: Some(3600),
            project_root: None,
            audit_file: None,
            max_audit_entries: 2000,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = parse_args(std::env::args().skip(1))?;
    let app = Arc::new(build_context()?);

    let config = DaemonConfig {
        master_token: args.auth_token.clone(),
        rate_limit_per_minute: args.rate_limit,
        session_time_budget_secs: args.time_budget_secs,
        project_root: args.project_root.clone(),
        audit_log_path: args.audit_file.clone(),
        max_audit_entries: args.max_audit_entries,
    };

    let server = Arc::new(ControlServer::with_config(Arc::clone(&app), config));

    let uds_handle = if let Some(ref sock_path) = args.socket_path {
        let server_clone = Arc::clone(&server);
        let path = sock_path.clone();
        Some(tokio::spawn(async move {
            if let Err(e) = server_clone.run_uds(path).await {
                tracing::error!("UDS daemon error: {e}");
            }
        }))
    } else {
        None
    };

    let tcp_handle = if let Some(port) = args.port {
        let router = server.router();
        let addr = format!("{}:{}", args.host, port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!(
            "Saturn control server listening on http://{addr} ({} commands)",
            app.registry.all_specs().len()
        );
        Some(tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("TCP server error: {e}");
            }
        }))
    } else {
        None
    };

    match (uds_handle, tcp_handle) {
        (Some(u), Some(t)) => {
            tokio::select! {
                res = u => { let _ = res; }
                res = t => { let _ = res; }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal; stopping daemon");
                }
            }
        }
        (Some(u), None) => {
            tokio::select! {
                res = u => { let _ = res; }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal; stopping daemon");
                }
            }
        }
        (None, Some(t)) => {
            tokio::select! {
                res = t => { let _ = res; }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal; stopping daemon");
                }
            }
        }
        (None, None) => {}
    }

    Ok(())
}

/// Build the shared application context with every builtin command registered.
fn build_context() -> Result<AppContext> {
    let registry = CommandRegistry::new();
    register_builtin_commands(&registry)?;

    Ok(AppContext {
        state: Arc::new(RwLock::new(AppState::default())),
        registry: Arc::new(registry),
        undo: Arc::new(UndoManager::new(1000)),
        shortcuts: Arc::new(ShortcutManager::new()),
        events: Arc::new(EventBus::new()),
    })
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<ServerArgs> {
    let mut result = ServerArgs::default();
    let mut args = args.peekable();
    let mut port_specified = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--control" | "--port" | "-c" | "-p" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a port number"))?;
                let port = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port: {value}"))?;
                result.port = Some(port);
                port_specified = true;
            }
            "--host" | "-H" => {
                result.host = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a host"))?;
            }
            "--uds" | "--socket" | "-s" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a socket path"))?;
                result.socket_path = Some(PathBuf::from(path));
                if !port_specified {
                    result.port = None;
                }
            }
            "--auth-token" | "--token" => {
                result.auth_token = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} requires a token string"))?,
                );
            }
            "--rate-limit" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a number"))?;
                result.rate_limit = val.parse()?;
            }
            "--time-budget" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires seconds"))?;
                result.time_budget_secs = Some(val.parse()?);
            }
            "--project-root" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a directory path"))?;
                result.project_root = Some(PathBuf::from(val));
            }
            "--audit-file" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a file path"))?;
                result.audit_file = Some(PathBuf::from(val));
            }
            "--help" | "-h" => {
                println!(
                    "saturn-control [--host 127.0.0.1] [--port 8080] [--socket /path/to.sock] [--auth-token <TOKEN>]\n\
                     \n\
                     Options:\n\
                       --host, -H          Bind host (default 127.0.0.1)\n\
                       --port, -p          TCP port (default 8080)\n\
                       --socket, --uds, -s Unix Domain Socket path\n\
                       --auth-token        Master bearer token for daemon authentication\n\
                       --rate-limit        Max requests per minute per session (default 120)\n\
                       --time-budget       Max session lifetime in seconds (default 3600)\n\
                       --project-root      Allowed directory root for filesystem operations\n\
                       --audit-file        Path to audit.jsonl log file\n"
                );
                std::process::exit(0);
            }
            other => return Err(anyhow::anyhow!("unknown argument: {other}")),
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn defaults_when_no_arguments_are_given() {
        let parsed = parse_args(args(&[])).unwrap();
        assert_eq!(parsed.host, DEFAULT_HOST);
        assert_eq!(parsed.port, Some(DEFAULT_PORT));
        assert!(parsed.socket_path.is_none());
    }

    #[test]
    fn parses_control_port_and_host() {
        let parsed = parse_args(args(&["--control", "9091", "--host", "0.0.0.0"])).unwrap();
        assert_eq!(parsed.host, "0.0.0.0");
        assert_eq!(parsed.port, Some(9091));
    }

    #[test]
    fn parses_uds_and_auth_token() {
        let parsed = parse_args(args(&[
            "--uds",
            "/tmp/test.sock",
            "--auth-token",
            "secret123",
            "--rate-limit",
            "50",
        ]))
        .unwrap();
        assert_eq!(parsed.socket_path, Some(PathBuf::from("/tmp/test.sock")));
        assert_eq!(parsed.auth_token, Some("secret123".to_string()));
        assert_eq!(parsed.rate_limit, 50);
        // When only UDS is specified, port is None
        assert_eq!(parsed.port, None);
    }

    #[test]
    fn rejects_invalid_port() {
        assert!(parse_args(args(&["--port", "not-a-port"])).is_err());
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(parse_args(args(&["--wat"])).is_err());
    }

    #[test]
    fn context_registers_builtin_commands() {
        let app = build_context().unwrap();
        let specs = app.registry.all_specs();
        assert!(!specs.is_empty());
        assert!(specs.iter().any(|spec| spec.id.as_str() == "test.echo"));
    }
}
