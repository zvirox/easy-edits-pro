// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MCP stdio entry point for Easy Edits Pro's editor engine.

use anyhow::{Context, Result};
use rmcp::{ServiceExt, transport::stdio};
use saturn_core::{
    CommandRequest, EditorEngine,
    catalogue::policy::{SessionMode, SessionPolicy},
};

mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut project_path = None;
    let mut bridge = false;
    let mut profile = None;
    let mut mode = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bridge" => bridge = true,
            "--project" => {
                project_path = Some(args.next().context("--project requires a project path")?);
            }
            "--profile" => {
                profile = Some(args.next().context("--profile requires a profile name")?);
            }
            "--mode" => {
                mode = Some(args.next().context("--mode requires a mode name (autopilot, coedit, review, manual)")?);
            }
            "--help" | "-h" => {
                println!(
                    "saturn-mcp [--project <path> | --bridge] [--profile <name>] [--mode <mode>]\nRuns the Easy Edits Pro MCP server over stdio. --bridge connects to the running desktop app."
                );
                return Ok(());
            }
            unknown => anyhow::bail!("unknown argument: {unknown}"),
        }
    }

    if bridge && project_path.is_some() {
        anyhow::bail!("--bridge and --project cannot be used together");
    }

    let mut policy = match mode.as_deref() {
        Some("review") => SessionPolicy::review(),
        Some("manual") => SessionPolicy::manual(),
        Some(m) => {
            let session_mode = SessionMode::parse(m).context("invalid session mode")?;
            SessionPolicy {
                mode: session_mode,
                ..SessionPolicy::default()
            }
        }
        None => SessionPolicy::default(),
    };
    if let Some(prof) = profile {
        policy.active_profiles.clear();
        policy
            .load_profile(&prof)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    let server = if bridge {
        server::EditorMcp::bridge_with_policy(policy)
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
        server::EditorMcp::new_with_policy(engine, policy)
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
