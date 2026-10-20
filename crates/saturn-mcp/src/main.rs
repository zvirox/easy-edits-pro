//! MCP Server for AI-assisted editing in Easy Edits Pro
//!
//! This binary provides an MCP (Model Context Protocol) server that exposes
//! the command registry to AI assistants for intelligent editing assistance.

use std::sync::Arc;
use saturn_commands::{
    CommandRegistry, CommandHandler, CommandHandlerError, CommandSpec,
    context::{AppContext, AppState, EventBus},
    undo::UndoManager,
    shortcuts::ShortcutManager,
    register_builtin_commands,
};
use rmcp::{
    ServerHandler,
    model::{CallToolRequestParam, CallToolResult, ListToolsResult, Tool},
    service::ServiceExt,
    transport::stdio,
};
use tracing::{info, error};
use uuid::Uuid;
use anyhow::Result;

mod handlers;
mod tools;

use handlers::McpServerHandler;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Saturn MCP Server");

    // Create application context
    let registry = Arc::new(CommandRegistry::new());
    register_builtin_commands(&registry)?;

    let app_context = Arc::new(AppContext {
        state: Arc::new(tokio::sync::RwLock::new(AppState::default())),
        registry: registry.clone(),
        undo: Arc::new(saturn_commands::UndoManager::new(1000)),
        shortcuts: Arc::new(ShortcutManager::new()),
        events: Arc::new(EventBus::new()),
    });

    // Create MCP server handler
    let handler = McpServerHandler::new(app_context);

    // Start MCP server over stdio
    let service = handler.serve(stdio()).await?;

    info!("MCP Server running on stdio");

    // Wait for shutdown
    tokio::signal::ctrl_c().await?;
    info!("Shutting down MCP Server");

    Ok(())
}