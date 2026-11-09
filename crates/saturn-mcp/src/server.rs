// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MCP tools backed by the same command engine used by the desktop app.
//!
//! The tool catalogue lives in `server_tools`; its tests in `server_tests`.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use rmcp::{
    model::{
        Annotated, CallToolRequestParams, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, RawContent, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    ErrorData,
};
use saturn_core::{
    ActorContext, CommandRequest, EditorEngine, command_catalogue,
    catalogue::policy::SessionPolicy,
};
use saturn_core::JobManager;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(test)]
#[path = "server_tests.rs"]
mod server_tests;
#[path = "server_tools.rs"]
mod server_tools;
#[path = "server_analysis.rs"]
mod server_analysis;
#[path = "server_render.rs"]
mod server_render;

#[cfg(unix)]
use tokio::net::UnixStream;

#[derive(Clone)]
pub struct EditorMcp {
    backend: Backend,
    jobs: Arc<JobManager>,
    policy: Arc<tokio::sync::RwLock<SessionPolicy>>,
    pub context: Arc<tokio::sync::RwLock<ActorContext>>,
}


#[derive(Clone)]
enum Backend {
    Headless(Arc<Mutex<EditorEngine>>),
    Bridge(PathBuf),
}

impl EditorMcp {
    pub fn new(engine: EditorEngine) -> Self {
        Self {
            backend: Backend::Headless(Arc::new(Mutex::new(engine))),
            jobs: Arc::new(JobManager::new(2)),
            policy: Arc::new(tokio::sync::RwLock::new(SessionPolicy::default())),
            context: Arc::new(tokio::sync::RwLock::new(ActorContext::agent("saturn-mcp-agent", "mcp-session-1"))),
        }
    }

    pub fn new_with_policy(engine: EditorEngine, policy: SessionPolicy) -> Self {
        Self {
            backend: Backend::Headless(Arc::new(Mutex::new(engine))),
            jobs: Arc::new(JobManager::new(2)),
            policy: Arc::new(tokio::sync::RwLock::new(policy)),
            context: Arc::new(tokio::sync::RwLock::new(ActorContext::agent("saturn-mcp-agent", "mcp-session-1"))),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_context(engine: EditorEngine, policy: SessionPolicy, context: ActorContext) -> Self {
        Self {
            backend: Backend::Headless(Arc::new(Mutex::new(engine))),
            jobs: Arc::new(JobManager::new(2)),
            policy: Arc::new(tokio::sync::RwLock::new(policy)),
            context: Arc::new(tokio::sync::RwLock::new(context)),
        }
    }

    #[allow(dead_code)]
    pub fn bridge() -> Self {
        Self {
            backend: Backend::Bridge(saturn_core::local_control::mcp_socket_path()),
            jobs: Arc::new(JobManager::new(1)),
            policy: Arc::new(tokio::sync::RwLock::new(SessionPolicy::default())),
            context: Arc::new(tokio::sync::RwLock::new(ActorContext::agent("saturn-mcp-agent", "mcp-session-1"))),
        }
    }

    pub fn bridge_with_policy(policy: SessionPolicy) -> Self {
        Self {
            backend: Backend::Bridge(saturn_core::local_control::mcp_socket_path()),
            jobs: Arc::new(JobManager::new(1)),
            policy: Arc::new(tokio::sync::RwLock::new(policy)),
            context: Arc::new(tokio::sync::RwLock::new(ActorContext::agent("saturn-mcp-agent", "mcp-session-1"))),
        }
    }

    pub fn bridge_with_path(path: PathBuf) -> Self {
        Self {
            backend: Backend::Bridge(path),
            jobs: Arc::new(JobManager::new(1)),
            policy: Arc::new(tokio::sync::RwLock::new(SessionPolicy::default())),
            context: Arc::new(tokio::sync::RwLock::new(ActorContext::agent("saturn-mcp-agent", "mcp-session-1"))),
        }
    }

    #[allow(dead_code)]
    pub fn policy(&self) -> Arc<tokio::sync::RwLock<SessionPolicy>> {
        self.policy.clone()
    }

    pub async fn active_tools(&self) -> Vec<rmcp::model::Tool> {
        let policy = self.policy.read().await;
        server_tools::tool_list(&policy.active_profiles)
    }

    async fn bridge_call(&self, method: &str, params: Value) -> Result<Value, String> {
        #[cfg(unix)]
        {
            let Backend::Bridge(path) = &self.backend else {
                return Err("MCP server is in headless mode".to_owned());
            };
            let mut stream = UnixStream::connect(path).await.map_err(|error| {
            format!("Could not connect to Easy Edits Pro at {}: {error}. Open the desktop app and retry.", path.display())
        })?;
            let request = json!({"method": method, "params": params});
            stream
                .write_all(format!("{request}\n").as_bytes())
                .await
                .map_err(|error| format!("Could not write to the app bridge: {error}"))?;
            stream
                .shutdown()
                .await
                .map_err(|error| format!("Could not finish the app bridge request: {error}"))?;
            let mut response = Vec::new();
            stream
                .take(1_048_577)
                .read_to_end(&mut response)
                .await
                .map_err(|error| format!("Could not read from the app bridge: {error}"))?;
            if response.len() > 1_048_576 {
                return Err("App bridge response exceeds 1 MiB".to_owned());
            }
            let response: Value = serde_json::from_slice(&response)
                .map_err(|error| format!("App bridge returned invalid JSON: {error}"))?;
            if let Some(error) = response.get("error").and_then(Value::as_str) {
                return Err(error.to_owned());
            }
            response
                .get("result")
                .cloned()
                .ok_or_else(|| "App bridge response did not contain a result".to_owned())
        }
        #[cfg(not(unix))]
        {
            let _ = (method, params);
            Err(
                "The live-app MCP bridge is currently supported on Unix desktop systems."
                    .to_owned(),
            )
        }
    }

    async fn execute(&self, id: String, params: Value) -> Result<Value, String> {
        self.execute_with_provenance(id, params, None, None).await
    }

    async fn execute_with_provenance(
        &self,
        id: String,
        params: Value,
        rationale: Option<String>,
        confidence: Option<saturn_core::brief::ConfidenceLevel>,
    ) -> Result<Value, String> {
        let ctx = self.context.read().await.clone();
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call(
                    "command_run",
                    json!({
                        "id": id,
                        "params": params,
                        "rationale": rationale,
                        "confidence": confidence,
                        "context": ctx
                    }),
                )
                .await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let mut req = CommandRequest::new(id, params);
            if let Some(r) = rationale {
                req = req.with_rationale(r);
            }
            if let Some(c) = confidence {
                req = req.with_confidence(c);
            }
            engine
                .execute_with_context(req, &ctx)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| format!("editor command task failed: {error}"))?
    }

    async fn command_catalogue(
        &self,
        filter: Option<String>,
        enabled_only: bool,
    ) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call(
                    "command_list",
                    json!({"filter": filter, "enabled_only": enabled_only}),
                )
                .await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let needle = filter.unwrap_or_default().to_lowercase();
            let commands = command_catalogue()
                .iter()
                .filter_map(|spec| {
                    let status = engine.command_status(spec.id).ok()?;
                    let matches_filter = needle.is_empty()
                        || spec.id.to_lowercase().contains(&needle)
                        || spec.label.to_lowercase().contains(&needle);
                    (matches_filter && (!enabled_only || status.enabled)).then(|| {
                        json!({
                            "id": spec.id,
                            "label": spec.label,
                            "menu": spec.menu,
                            "shortcut": spec.shortcut,
                            "mutating": spec.mutating,
                            "enabled": status.enabled,
                            "reason": status.reason,
                            "tier": spec.tier.as_str(),
                            "tiers": spec.tiers.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
                            "profiles": spec.profiles,
                        })
                    })
                })
                .collect::<Vec<_>>();
            Ok(Value::Array(commands))
        })
        .await
        .map_err(|error| format!("command catalogue task failed: {error}"))?
    }

    async fn document(&self) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("doc_inspect", json!({})).await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let document = serde_json::to_value(engine.document()).map_err(|e| e.to_string())?;
            Ok(json!({
                "project": document,
                "path": engine.current_path().map(|p| p.to_string_lossy().to_string()),
                "is_dirty": engine.is_dirty(),
            }))
        })
        .await
        .map_err(|error| format!("project inspection task failed: {error}"))?
    }


    async fn call(&self, request: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        let tool_name = request.name.as_ref();
        if tool_name == "tools.list_profiles" || tool_name == "tools_list_profiles" {
            return self.list_profiles().await;
        }
        if tool_name == "tools.load_profile" || tool_name == "tools_load_profile" {
            return self.load_profile(&arguments).await;
        }

        if let Some(spec) = server_tools::tool_spec(tool_name) {
            let policy = self.policy.read().await;
            if let Err(err) = policy.check_tool(tool_name, spec.tiers, spec.profiles) {
                return Err(ErrorData::invalid_params(err.to_string(), None));
            }
        }

        let result = match request.name.as_ref() {
            "command_list" => {
                self.command_catalogue(
                    arguments
                        .get("filter")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    arguments
                        .get("enabled_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )
                .await
            }
            "command_run" => {
                let id = arguments.get("id").and_then(Value::as_str).ok_or_else(|| {
                    ErrorData::invalid_params("command_run requires string `id`", None)
                })?;
                let spec = saturn_core::command_catalogue()
                    .iter()
                    .find(|cmd| cmd.id == id)
                    .ok_or_else(|| {
                        ErrorData::invalid_params(format!("unknown command id: {id}"), None)
                    })?;
                {
                    let policy = self.policy.read().await;
                    if let Err(err) = policy.check_command(spec) {
                        return Err(ErrorData::invalid_params(err.to_string(), None));
                    }
                }
                let rationale = arguments
                    .get("rationale")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let confidence = arguments
                    .get("confidence")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                self.execute_with_provenance(
                    id.to_owned(),
                    arguments
                        .get("params")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                    rationale,
                    confidence,
                )
                .await
            }
            "command_batch" => {
                let steps = arguments
                    .get("steps")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        ErrorData::invalid_params("command_batch requires `steps` array", None)
                    })?;
                let stop_on_error = arguments
                    .get("stop_on_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let atomic = arguments
                    .get("atomic")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if atomic {
                    for step in steps {
                        let Some(id) = step.get("id").and_then(Value::as_str) else {
                            return Err(ErrorData::invalid_params("step requires string `id`", None));
                        };
                        let spec = saturn_core::command_catalogue()
                            .iter()
                            .find(|cmd| cmd.id == id)
                            .ok_or_else(|| ErrorData::invalid_params(format!("unknown command id: {id}"), None))?;
                        let policy = self.policy.read().await;
                        if let Err(err) = policy.check_command(spec) {
                            return Err(ErrorData::invalid_params(format!("Policy violation on step '{id}': {err}"), None));
                        }
                    }

                    let ctx = self.context.read().await.clone();
                    if matches!(&self.backend, Backend::Bridge(_)) {
                        self.bridge_call("command_batch", arguments).await
                    } else {
                        let Backend::Headless(engine) = &self.backend else { unreachable!() };
                        let engine = engine.clone();
                        let cmd_steps = steps
                            .iter()
                            .map(|step| {
                                let id = step.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                                let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
                                let mut req = CommandRequest::new(id, params);
                                if let Some(r) = step.get("rationale").and_then(Value::as_str) {
                                    req = req.with_rationale(r.to_string());
                                }
                                if let Some(c) = step.get("confidence").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                                    req = req.with_confidence(c);
                                }
                                req
                            })
                            .collect::<Vec<_>>();

                        tokio::task::spawn_blocking(move || {
                            let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                            engine.execute_transaction(cmd_steps, &ctx).map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| format!("transaction task failed: {e}"))
                        .and_then(|res| res)
                    }
                } else {
                    let mut completed = 0usize;
                    let mut failed = 0usize;
                    let mut results = Vec::new();
                    for step in steps {
                        let Some(id) = step.get("id").and_then(Value::as_str) else {
                            failed += 1;
                            results.push(json!({"ok": false, "error": "step requires string `id`"}));
                            if stop_on_error {
                                break;
                            }
                            continue;
                        };
                        let spec = saturn_core::command_catalogue()
                            .iter()
                            .find(|cmd| cmd.id == id);
                        let policy_err = match spec {
                            Some(spec) => {
                                let policy = self.policy.read().await;
                                policy.check_command(spec).err()
                            }
                            None => Some(saturn_core::catalogue::policy::PolicyError::InvalidProfile(
                                format!("unknown command id: {id}"),
                            )),
                        };
                        if let Some(err) = policy_err {
                            failed += 1;
                            results.push(json!({"ok": false, "error": err.to_string()}));
                            if stop_on_error {
                                break;
                            }
                            continue;
                        }
                        let rationale = step.get("rationale").and_then(Value::as_str).map(str::to_string);
                        let confidence = step.get("confidence").and_then(|v| serde_json::from_value(v.clone()).ok());
                        match self
                            .execute_with_provenance(
                                id.to_owned(),
                                step.get("params").cloned().unwrap_or_else(|| json!({})),
                                rationale,
                                confidence,
                            )
                            .await
                        {
                            Ok(value) => {
                                completed += 1;
                                results.push(json!({"ok": true, "result": value}));
                            }
                            Err(error) => {
                                failed += 1;
                                results.push(json!({"ok": false, "error": error}));
                                if stop_on_error {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(json!({"completed": completed, "failed": failed, "results": results}))
                }
            }
            "session.stop" | "session_stop" => {
                let session_id = arguments
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(String::from);
                let ctx = self.context.read().await.clone();
                let target_session = session_id.unwrap_or(ctx.session_id);
                if matches!(&self.backend, Backend::Bridge(_)) {
                    self.bridge_call("session.stop", json!({"session_id": target_session})).await
                } else {
                    let Backend::Headless(engine) = &self.backend else { unreachable!() };
                    let engine = engine.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                        let generation = engine.stop_session(&target_session);
                        Ok(json!({
                            "session_id": target_session,
                            "stopped": true,
                            "generation": generation,
                        }))
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|res| res)
                }
            }
            "session.resume" | "session_resume" => {
                let target_session = match arguments.get("session_id").and_then(Value::as_str) {
                    Some(s) => s,
                    None => return Ok(CallToolResult::error(vec![Annotated::new(
                        RawContent::text("session.resume requires `session_id`"),
                        None,
                    )])),
                };
                let ctx = self.context.read().await.clone();
                let caller = match arguments.get("caller").and_then(Value::as_str) {
                    Some("human") => saturn_core::ActorId::Human,
                    _ => ctx.actor.clone(),
                };
                if matches!(&self.backend, Backend::Bridge(_)) {
                    self.bridge_call("session.resume", json!({"session_id": target_session, "caller": caller})).await
                } else {
                    let Backend::Headless(engine) = &self.backend else { unreachable!() };
                    let engine = engine.clone();
                    let target_str = target_session.to_string();
                    tokio::task::spawn_blocking(move || {
                        let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                        let generation = engine.resume_session(&target_str, &caller).map_err(|e| e.to_string())?;
                        Ok(json!({
                            "session_id": target_str,
                            "resumed": true,
                            "generation": generation,
                        }))
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|res| res)
                }
            }
            "session.status" | "session_status" => {
                let ctx = self.context.read().await.clone();
                let target_session = arguments
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or(ctx.session_id);
                if matches!(&self.backend, Backend::Bridge(_)) {
                    self.bridge_call("session.status", json!({"session_id": target_session})).await
                } else {
                    let Backend::Headless(engine) = &self.backend else { unreachable!() };
                    let engine = engine.clone();
                    tokio::task::spawn_blocking(move || {
                        let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                        let is_stopped = engine.is_session_stopped(&target_session);
                        let session = engine.sessions.get(&target_session);
                        Ok(json!({
                            "session_id": target_session,
                            "is_stopped": is_stopped,
                            "generation": session.map(|s| s.generation).unwrap_or(1),
                            "actor": session.map(|s| s.actor.clone()).unwrap_or(ctx.actor),
                            "active_branch": session.map(|s| s.active_branch.clone()).unwrap_or_else(|| saturn_core::branch::MAIN_BRANCH.to_string()),
                            "playhead": session.map(|s| s.playhead.0).unwrap_or(0),
                            "locks": engine.document().locks.locks.iter().filter(|l| l.owner.session_id.as_deref() == Some(&target_session)).collect::<Vec<_>>(),
                        }))
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|res| res)
                }
            }
            "history.undo_change" | "history_undo_change" => {
                let change_id = match arguments.get("change_id").and_then(Value::as_u64) {
                    Some(id) => id,
                    None => return Ok(CallToolResult::error(vec![Annotated::new(
                        RawContent::text("history.undo_change requires integer `change_id`"),
                        None,
                    )])),
                };
                let ctx = self.context.read().await.clone();
                if matches!(&self.backend, Backend::Bridge(_)) {
                    self.bridge_call("history.undo_change", json!({"change_id": change_id, "context": ctx})).await
                } else {
                    let Backend::Headless(engine) = &self.backend else { unreachable!() };
                    let engine = engine.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                        engine.undo_change(change_id, &ctx).map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|res| res)
                }
            }
            "doc_inspect" => self.document().await,
            "sequence_inspect" => self.document().await.and_then(|value| {
                value
                    .pointer("/project/project/sequence")
                    .cloned()
                    .ok_or_else(|| "active sequence not found".to_owned())
            }),
            "media_search" => self.media_search(&arguments).await,
            "media_preview" => self.media_preview(&arguments).await,
            "vision_extract_frame_sheet" => self.vision_extract_frame_sheet(&arguments).await,
            "render_preview_clip_proxy" => self.render_preview_clip_proxy(&arguments).await,
            "color_read_scopes" => self.color_read_scopes(&arguments).await,
            "audio_process" => self.audio_process(&arguments).await,
            "edit_match_cut_align" => self.edit_match_cut_align(&arguments).await,
            "timeline_beat_grid_quantize" => self.timeline_beat_grid_quantize(&arguments).await,
            "color_match_shot_to_reference" => {
                self.color_match_shot_to_reference(&arguments).await
            }
            "audio_read_telemetry" => self.audio_read_telemetry(&arguments).await,
            "media_semantic_query" => self.media_semantic_query(&arguments).await,
            "edit_evaluate_cut_continuity" => self.edit_evaluate_cut_continuity(&arguments).await,
            "render_start" => self.start_render(&arguments).await,
            "render_status" => self.render_job_status(&arguments).await,
            "render_cancel" => self.cancel_render(&arguments).await,
            "render_project" => self.render_project(&arguments).await,
            _ => {
                return Err(ErrorData::invalid_params(
                    format!("unknown tool: {}", request.name),
                    None,
                ));
            }
        };
        Ok(match result {
            Ok(value) => {
                // In-band image convention (Packet 1.1): a payload may carry
                // an `image_block` (base64 JPEG/PNG). The MCP response gets a
                // real image content block; the text block keeps the JSON
                // without the base64 blob so the payload is not duplicated.
                let content = match value.get("image_block") {
                    Some(block) => {
                        let data = block.get("data").and_then(Value::as_str).unwrap_or("");
                        let mime_type = block
                            .get("mime_type")
                            .and_then(Value::as_str)
                            .unwrap_or("image/jpeg");
                        if data.is_empty() {
                            vec![Annotated::new(RawContent::text(continue_text_only(&value)), None)]
                        } else {
                            let slim = continue_text_only(&value);
                            vec![
                                Annotated::new(RawContent::text(slim), None),
                                Annotated::new(RawContent::image(data, mime_type), None),
                            ]
                        }
                    }
                    None => vec![Annotated::new(RawContent::text(
                        serde_json::to_string_pretty(&value)
                            .unwrap_or_else(|_| value.to_string()),
                    ), None)],
                };
                CallToolResult::success(content)
            }
            Err(error) => {
                CallToolResult::error(vec![Annotated::new(RawContent::text(error), None)])
            }
        })
    }

    async fn media_search(&self, arguments: &Value) -> Result<Value, String> {
        let document = self.document().await?;
        let media = document
            .pointer("/project/project/media")
            .and_then(Value::as_array)
            .ok_or_else(|| "Current project has no media collection".to_owned())?;
        let query = arguments
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        let kind = arguments
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        let limit = arguments
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .clamp(1, 500) as usize;
        Ok(Value::Array(
            media
                .iter()
                .filter(|item| {
                    let name = item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_lowercase();
                    let path = item
                        .pointer("/path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_lowercase();
                    let item_kind = item
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_lowercase();
                    (query.is_empty() || name.contains(&query) || path.contains(&query))
                        && (kind.is_empty() || item_kind == kind)
                })
                .take(limit)
                .cloned()
                .collect(),
        ))
    }

    /// Snapshot of the typed project document. Bridge mode resolves documents on
    /// the desktop-app side, so headless callers only.
    async fn headless_document(&self) -> Result<saturn_core::ProjectDocument, String> {
        let Backend::Headless(engine) = &self.backend else {
            unreachable!("headless_document is only called after the bridge check")
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            Ok(engine.document().clone())
        })
        .await
        .map_err(|error| format!("project snapshot task failed: {error}"))?
    }
}

/// Text form of a payload that also carries an image block: the JSON without
/// the base64 blob (neither `image_block` nor a still-frame `data_base64`) so
/// the image is never duplicated across content blocks.
fn continue_text_only(value: &Value) -> String {
    let mut slim = value.clone();
    if let Some(object) = slim.as_object_mut() {
        object.remove("image_block");
        if let Some(still) = object.get_mut("still_frame").and_then(Value::as_object_mut) {
            still.remove("data_base64");
            still.insert(
                "image".to_owned(),
                json!("attached as an image content block"),
            );
        }
    }
    serde_json::to_string_pretty(&slim).unwrap_or_else(|_| value.to_string())
}

impl rmcp::handler::server::ServerHandler for EditorMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(rmcp::model::ProtocolVersion::default())
            .with_server_info(Implementation::new("easy-edits-pro", env!("CARGO_PKG_VERSION")))
            .with_instructions("Easy Edits Pro MCP server. Use command_list to discover editor engine commands, command_run to apply an edit, command_batch for ordered edits, doc_inspect or sequence_inspect to inspect the project, and render_project to export it. By default the server owns a headless session; start it with --bridge to edit the project in the running desktop app.")
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.active_tools().await))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.call(request).await
    }
}