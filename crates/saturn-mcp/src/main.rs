//! MCP stdio entry point for Easy Edits Pro's editor engine.

use anyhow::{Context, Result};
use rmcp::{ServiceExt, transport::stdio};
use saturn_core::{CommandRequest, EditorEngine};

mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut project_path = None;
    let mut bridge = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bridge" => bridge = true,
            "--project" => {
                project_path = Some(args.next().context("--project requires a project path")?);
            }
            "--help" | "-h" => {
                println!(
                    "saturn-mcp [--project <path> | --bridge]\nRuns the Easy Edits Pro MCP server over stdio. --bridge connects to the running desktop app."
                );
                return Ok(());
            }
            unknown => anyhow::bail!("unknown argument: {unknown}"),
        }
    }

    if bridge && project_path.is_some() {
        anyhow::bail!("--bridge and --project cannot be used together");
    }

    let server = if bridge {
        server::EditorMcp::bridge()
    } else {
        let mut engine = EditorEngine::default();
        if let Some(path) = project_path {
            engine
                .execute(CommandRequest::new(
                    "file.open",
                    serde_json::json!({ "path": path }),
                ))
                .context("failed to open the requested project")?;
        }
        server::EditorMcp::new(engine)
    };

    let service = server
        .serve(stdio())
        .await
        .context("failed to start MCP stdio service")?;
    service
        .waiting()
        .await
        .context("MCP service stopped with an error")?;
    Ok(())
}
