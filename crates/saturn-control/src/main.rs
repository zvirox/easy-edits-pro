//! Control Server for External Automation in Easy Edits Pro
//!
//! Exposes the command registry over HTTP and WebSocket so external tools
//! (scripts, CI, editors, agents) can drive the editor without the GUI.

use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use tracing::info;

use saturn_control::server::ControlServer;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_HOST: &str = "127.0.0.1";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (host, port) = parse_args(std::env::args().skip(1))?;
    let app = Arc::new(build_context()?);

    let server = ControlServer::new(Arc::clone(&app));
    let router = server.router();

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(
        "Saturn control server listening on http://{addr} ({} commands)",
        app.registry.all_specs().len()
    );

    axum::serve(listener, router).await?;
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

fn parse_args(args: impl Iterator<Item = String>) -> Result<(String, u16)> {
    let mut host = DEFAULT_HOST.to_string();
    let mut port = DEFAULT_PORT;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--control" | "--port" | "-c" | "-p" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a port number"))?;
                port = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port: {value}"))?;
            }
            "--host" | "-H" => {
                host = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("{arg} requires a host"))?;
            }
            "--help" | "-h" => {
                println!(
                    "saturn-control [--host {DEFAULT_HOST}] [--port {DEFAULT_PORT}]\n\
                     \n\
                     HTTP endpoints:\n\
                       GET  /health                  server status and command count\n\
                       GET  /commands                every registered command spec\n\
                       POST /commands/:id/execute    execute one command with JSON params\n\
                       GET  /ws                      WebSocket stream of registry events"
                );
                std::process::exit(0);
            }
            other => return Err(anyhow::anyhow!("unknown argument: {other}")),
        }
    }

    Ok((host, port))
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
        assert_eq!(
            parse_args(args(&[])).unwrap(),
            (DEFAULT_HOST.to_string(), DEFAULT_PORT)
        );
    }

    #[test]
    fn parses_control_port_and_host() {
        let parsed = parse_args(args(&["--control", "9091", "--host", "0.0.0.0"])).unwrap();
        assert_eq!(parsed, ("0.0.0.0".to_string(), 9091));
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
