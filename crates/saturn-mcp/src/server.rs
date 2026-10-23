//! MCP tools backed by the same command engine used by the desktop app.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use rmcp::{
    Error, ServerHandler,
    model::{
        AnnotateAble, CallToolRequestParam, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParam, RawContent, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
};
use saturn_core::{CommandRequest, EditorEngine, command_catalogue};
use saturn_core::{JobId, JobKind, JobManager, JobPriority};
use serde_json::{Map, Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;

#[derive(Clone)]
pub struct EditorMcp {
    backend: Backend,
    jobs: Arc<JobManager>,
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };

    #[tokio::test]
    async fn media_preview_never_returns_a_source_path_placeholder() {
        let mut engine = EditorEngine::default();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.new",
                json!({"name": "MCP preview"}),
            ))
            .unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                json!({"name": "clip.mp4", "path": "/tmp/nonexistent-clip.mp4", "kind": "video"}),
            ))
            .unwrap();
        let media_id = engine.document().project.media[0].id;

        let mcp = EditorMcp::new(engine);
        let result = mcp
            .media_preview(&json!({"media_id": media_id}))
            .await
            .expect("media_preview on a real asset");
        assert!(result.get("preview_path").is_none(), "no source-path placeholder");
        assert_eq!(result["still_frame"]["available"], false);

        // Unknown ids are a typed error, not a fabricated payload.
        let missing = mcp.media_preview(&json!({"media_id": 4242})).await;
        assert!(missing.is_err());
    }

    #[tokio::test]
    async fn bridge_client_reconnects_after_server_restart() {
        let path =
            std::env::temp_dir().join(format!("eep-mcp-reconnect-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let client = EditorMcp {
            backend: Backend::Bridge(path.clone()),
            jobs: Arc::new(JobManager::new(1)),
        };

        for expected in ["first", "second"] {
            let listener = UnixListener::bind(&path).expect("bind bridge socket");
            let response = expected.to_owned();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("accept bridge request");
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                BufReader::new(read)
                    .read_line(&mut line)
                    .await
                    .expect("read request");
                let request: Value = serde_json::from_str(&line).expect("valid request json");
                assert_eq!(request["method"], "lifecycle_test");
                write
                    .write_all(json!({"result": response}).to_string().as_bytes())
                    .await
                    .expect("write response");
                write.write_all(b"\n").await.expect("terminate response");
            });
            let result = client
                .bridge_call("lifecycle_test", json!({}))
                .await
                .expect("bridge reconnect call");
            assert_eq!(result, expected);
            server.await.expect("mock bridge task");
            let _ = std::fs::remove_file(&path);
        }
    }
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
        }
    }

    pub fn bridge() -> Self {
        Self {
            backend: Backend::Bridge(saturn_core::local_control::mcp_socket_path()),
            jobs: Arc::new(JobManager::new(1)),
        }
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
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("command_run", json!({"id": id, "params": params}))
                .await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let mut engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            engine
                .execute(CommandRequest::new(id, params))
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

    fn tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "command_list",
                "Discover commands supported by the editor engine, including project, media, timeline, effects and audio operations.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "filter": { "type": "string", "description": "Optional case-insensitive id or label filter." },
                        "enabled_only": { "type": "boolean", "description": "Only include commands enabled in the current project." }
                    }
                })),
            ),
            Tool::new(
                "command_run",
                "Run an editor command by id with its JSON parameters. Use command_list to discover ids.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Editor command id." },
                        "params": { "type": "object", "description": "Command-specific parameters." }
                    },
                    "required": ["id"]
                })),
            ),
            Tool::new(
                "command_batch",
                "Run editor commands in order. Each successful command is applied to the current project session.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "steps": { "type": "array", "items": { "type": "object", "properties": {
                            "id": { "type": "string" }, "params": { "type": "object" }
                        }, "required": ["id"] } },
                        "stop_on_error": { "type": "boolean", "default": true }
                    },
                    "required": ["steps"]
                })),
            ),
            Tool::new(
                "doc_inspect",
                "Inspect the current project document, including media, settings and active timeline.",
                schema(json!({ "type": "object", "properties": {} })),
            ),
            Tool::new(
                "sequence_inspect",
                "Inspect the active sequence, including its tracks and timeline clips.",
                schema(json!({ "type": "object", "properties": {} })),
            ),
            Tool::new(
                "media_search",
                "Search media already imported in the active project by name, path, and optional media kind.",
                schema(json!({ "type": "object", "properties": {
                    "query": { "type": "string" },
                    "kind": { "type": "string", "enum": ["video", "audio", "image", "title", "compoundsequence", "unknown"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500, "default": 100 }
                } })),
            ),
            Tool::new(
                "media_preview",
                "Inspect one imported media asset and its timeline placements. Returns media metadata only: no image payload exists yet (still_frame.available is false), so clients must not treat it as a preview.",
                schema(json!({ "type": "object", "properties": {
                    "media_id": { "type": "integer", "minimum": 0 }
                }, "required": ["media_id"] })),
            ),
            Tool::new(
                "render_start",
                "Start an asynchronous export and return its job_id. Poll render_status for progress or call render_cancel.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string" },
                        "use_mark_range": { "type": "boolean", "default": false },
                        "range": { "type": "object", "properties": {
                            "start_ticks": { "type": "integer", "minimum": 0 },
                            "end_ticks": { "type": "integer", "minimum": 1 }
                        }, "required": ["start_ticks", "end_ticks"] },
                        "settings": { "type": "object", "properties": {
                            "width": { "type": "integer" }, "height": { "type": "integer" },
                            "frameRateNumerator": { "type": "integer" }, "frameRateDenominator": { "type": "integer" },
                            "useHardwareEncoding": { "type": "boolean" }
                        } }
                    }, "required": ["output_path"]
                })),
            ),
            Tool::new(
                "render_status",
                "Read queued, running, completed, failed, or cancelled render state and progress.",
                schema(
                    json!({ "type": "object", "properties": { "job_id": { "type": "integer", "minimum": 1 } }, "required": ["job_id"] }),
                ),
            ),
            Tool::new(
                "render_cancel",
                "Request cancellation of a queued or active render job.",
                schema(
                    json!({ "type": "object", "properties": { "job_id": { "type": "integer", "minimum": 1 } }, "required": ["job_id"] }),
                ),
            ),
            Tool::new(
                "render_project",
                "Render the active sequence to MP4 or WebM. In bridge mode this renders the project open in the desktop app.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string", "description": "Absolute .mp4 or .webm output path." },
                        "use_mark_range": { "type": "boolean", "default": false },
                        "range": { "type": "object", "description": "Optional timeline render range in project ticks, e.g. {start_ticks: 0, end_ticks: 508032000000} for a two-second clip at 30 fps.", "properties": {
                            "start_ticks": { "type": "integer", "minimum": 0 },
                            "end_ticks": { "type": "integer", "minimum": 1 }
                        }, "required": ["start_ticks", "end_ticks"] },
                        "settings": { "type": "object", "properties": {
                            "width": { "type": "integer" }, "height": { "type": "integer" },
                            "frameRateNumerator": { "type": "integer" }, "frameRateDenominator": { "type": "integer" },
                            "useHardwareEncoding": { "type": "boolean" }
                        } }
                    },
                    "required": ["output_path"]
                })),
            ),
        ]
    }

    async fn call(&self, request: CallToolRequestParam) -> Result<CallToolResult, Error> {
        let arguments = Value::Object(request.arguments.unwrap_or_default());
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
                    Error::invalid_params("command_run requires string `id`", None)
                })?;
                self.execute(
                    id.to_owned(),
                    arguments
                        .get("params")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                )
                .await
            }
            "command_batch" => {
                let steps = arguments
                    .get("steps")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        Error::invalid_params("command_batch requires `steps` array", None)
                    })?;
                let stop_on_error = arguments
                    .get("stop_on_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
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
                    match self
                        .execute(
                            id.to_owned(),
                            step.get("params").cloned().unwrap_or_else(|| json!({})),
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
            "doc_inspect" => self.document().await,
            "sequence_inspect" => self.document().await.and_then(|value| {
                value
                    .pointer("/project/project/sequence")
                    .cloned()
                    .ok_or_else(|| "active sequence not found".to_owned())
            }),
            "media_search" => self.media_search(&arguments).await,
            "media_preview" => self.media_preview(&arguments).await,
            "render_start" => self.start_render(&arguments).await,
            "render_status" => self.render_job_status(&arguments).await,
            "render_cancel" => self.cancel_render(&arguments).await,
            "render_project" => self.render_project(&arguments).await,
            _ => {
                return Err(Error::invalid_params(
                    format!("unknown tool: {}", request.name),
                    None,
                ));
            }
        };
        Ok(match result {
            Ok(value) => CallToolResult::success(vec![
                RawContent::text(
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
                )
                .optional_annotate(None),
            ]),
            Err(error) => {
                CallToolResult::error(vec![RawContent::text(error).optional_annotate(None)])
            }
        })
    }

    async fn render_project(&self, arguments: &Value) -> Result<Value, String> {
        let output_path = arguments
            .get("output_path")
            .and_then(Value::as_str)
            .ok_or_else(|| "render_project requires output_path".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("render_project", arguments.clone()).await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        let output_path = PathBuf::from(output_path);
        let use_mark_range = arguments
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let explicit_range = parse_render_range(&arguments)?;
        if explicit_range.is_some() && use_mark_range {
            return Err("Pass either range or use_mark_range, not both".to_owned());
        }
        let settings = arguments
            .get("settings")
            .filter(|value| value.is_object())
            .map(|value| serde_json::from_value(value.clone()).map_err(|error| error.to_string()))
            .transpose()?;
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let document = engine.document().clone();
            let range = if let Some(range) = explicit_range {
                Some(range)
            } else if use_mark_range {
                match (
                    document.project.sequence.mark_in,
                    document.project.sequence.mark_out,
                ) {
                    (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
                    _ => {
                        return Err(
                            "Set both In and Out points before rendering the marked range"
                                .to_owned(),
                        );
                    }
                }
            } else {
                None
            };
            let settings =
                settings.unwrap_or_else(|| saturn_render::RenderSettings::from_project(&document));
            drop(engine);
            let output = output_path.clone();
            saturn_render::render_project_range_with_settings(
                &document,
                &output,
                range,
                settings,
                |_| {},
            )
            .map(|()| json!({ "rendered": true, "output_path": output.to_string_lossy() }))
        })
        .await
        .map_err(|error| format!("render task failed: {error}"))?
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

    async fn start_render(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("render_start", arguments.clone()).await;
        }
        let output_path = arguments
            .get("output_path")
            .and_then(Value::as_str)
            .ok_or_else(|| "render_start requires output_path".to_owned())?;
        let output_path = PathBuf::from(output_path);
        let use_mark_range = arguments
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let explicit_range = parse_render_range(arguments)?;
        if explicit_range.is_some() && use_mark_range {
            return Err("Pass either range or use_mark_range, not both".into());
        }
        let settings_value = arguments.get("settings").filter(|value| value.is_object());
        let settings = settings_value
            .cloned()
            .map(serde_json::from_value::<saturn_render::RenderSettings>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        let (document, range, settings) = tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let document = engine.document().clone();
            let range = if let Some(range) = explicit_range {
                Some(range)
            } else if use_mark_range {
                match (
                    document.project.sequence.mark_in,
                    document.project.sequence.mark_out,
                ) {
                    (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
                    _ => {
                        return Err(
                            "Set both In and Out points before rendering the marked range"
                                .to_owned(),
                        );
                    }
                }
            } else {
                None
            };
            let settings =
                settings.unwrap_or_else(|| saturn_render::RenderSettings::from_project(&document));
            Ok((document, range, settings))
        })
        .await
        .map_err(|error| format!("project snapshot task failed: {error}"))??;
        let job_output = output_path.clone();
        let label = format!(
            "MCP render: {}",
            output_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        let (id, _) = self.jobs.submit_detached(
            label,
            JobKind::ExportRendering {
                output_path: output_path.clone(),
            },
            JobPriority::Immediate,
            move |ctx| {
                saturn_render::render_project_range_with_control(
                    &document,
                    &job_output,
                    range,
                    settings,
                    |progress| {
                        ctx.update_progress(
                            progress.fraction_percent as f32 / 100.0,
                            progress.phase,
                        )
                    },
                    || ctx.is_cancelled(),
                )?;
                Ok(Some(job_output.to_string_lossy().into_owned()))
            },
        );
        Ok(json!({"job_id": id.0, "state": "queued", "output_path": output_path}))
    }

    async fn render_job_status(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("job_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "render_status requires an unsigned job_id".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("render_status", json!({"job_id": id}))
                .await;
        }
        self.jobs
            .get_job(JobId(id))
            .map(|job| serde_json::to_value(job).unwrap_or(Value::Null))
            .ok_or_else(|| format!("Render job {id} was not found"))
    }

    async fn cancel_render(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("job_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "render_cancel requires an unsigned job_id".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("render_cancel", json!({"job_id": id}))
                .await;
        }
        Ok(json!({"job_id": id, "cancellation_requested": self.jobs.cancel(JobId(id))}))
    }

    async fn media_preview(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("media_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "media_preview requires an unsigned media_id".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("media_preview", arguments.clone()).await;
        }
        let document = self.document().await?;
        let media = document
            .pointer("/project/project/media")
            .and_then(Value::as_array)
            .ok_or_else(|| "Current project has no media collection".to_owned())?;
        let asset = media
            .iter()
            .find(|item| item.get("id").and_then(Value::as_u64) == Some(id))
            .cloned()
            .ok_or_else(|| format!("Media asset {id} was not found"))?;
        let sequence = document
            .pointer("/project/project/sequence")
            .ok_or_else(|| "Active sequence was not found".to_owned())?;
        let mut placements = Vec::new();
        if let Some(tracks) = sequence.get("tracks").and_then(Value::as_array) {
            for track in tracks {
                if let Some(clips) = track.get("clips").and_then(Value::as_array) {
                    for clip in clips {
                        if clip.get("media_id").and_then(Value::as_u64) == Some(id) {
                            placements.push(json!({"track_id": track.get("id"), "clip": clip}));
                        }
                    }
                }
            }
        }
        Ok(
            json!({"media": asset, "timeline_placements": placements, "still_frame": {"available": false, "reason": "still-frame extraction is not implemented; no image payload exists yet"}}),
        )
    }
}

fn parse_render_range(arguments: &Value) -> Result<Option<saturn_render::RenderRange>, String> {
    let Some(range) = arguments.get("range") else {
        return Ok(None);
    };
    let start = range
        .get("start_ticks")
        .and_then(Value::as_i64)
        .ok_or_else(|| "range.start_ticks must be a non-negative integer".to_owned())?;
    let end = range
        .get("end_ticks")
        .and_then(Value::as_i64)
        .ok_or_else(|| "range.end_ticks must be an integer greater than start_ticks".to_owned())?;
    if start < 0 || end <= start {
        return Err("Render range must have non-negative start_ticks and end_ticks greater than start_ticks".to_owned());
    }
    Ok(Some(saturn_render::RenderRange {
        start: saturn_core::Tick(start),
        end: saturn_core::Tick(end),
    }))
}

fn schema(value: Value) -> Arc<Map<String, Value>> {
    Arc::new(value.as_object().cloned().unwrap_or_default())
}

impl ServerHandler for EditorMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "easy-edits-pro".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            instructions: Some("Easy Edits Pro MCP server. Use command_list to discover editor engine commands, command_run to apply an edit, command_batch for ordered edits, doc_inspect or sequence_inspect to inspect the project, and render_project to export it. By default the server owns a headless session; start it with --bridge to edit the project in the running desktop app.".to_owned()),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: Self::tools(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, Error> {
        self.call(request).await
    }
}
