// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Unix Domain Socket (UDS) server handling JSONL protocol, token auth, and EOF cleanup (AF9).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use saturn_commands::context::AppContext;
use saturn_core::actor::ActorId;
use saturn_core::catalogue::Tier;
use saturn_core::command::CommandRequest;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info};

use crate::audit::SharedAuditLogger;
use crate::auth::AuthManager;
use crate::project::{project_close, project_open, project_save, project_status};
use crate::provenance::{validate_path_containment, validate_untrusted_media_text};
use crate::session::{DaemonSession, SessionManager};

pub const DEFAULT_MAX_UDS_LINE_BYTES: usize = 1_048_576; // 1 MiB

pub struct UdsServer {
    socket_path: PathBuf,
    auth: Arc<AuthManager>,
    sessions: Arc<SessionManager>,
    audit: SharedAuditLogger,
    app: Arc<AppContext>,
    project_root: Option<PathBuf>,
}

impl UdsServer {
    pub fn new(
        socket_path: impl Into<PathBuf>,
        auth: Arc<AuthManager>,
        sessions: Arc<SessionManager>,
        audit: SharedAuditLogger,
        app: Arc<AppContext>,
        project_root: Option<PathBuf>,
    ) -> Self {
        Self {
            socket_path: socket_path.into(),
            auth,
            sessions,
            audit,
            app,
            project_root,
        }
    }

    pub async fn bind(&self) -> std::io::Result<UnixListener> {
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        let listener = UnixListener::bind(&self.socket_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o600));
        }
        info!("UDS daemon listening on {}", self.socket_path.display());
        Ok(listener)
    }

    pub async fn run(&self, listener: UnixListener) {
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let auth = self.auth.clone();
                    let sessions = self.sessions.clone();
                    let audit = self.audit.clone();
                    let app = self.app.clone();
                    let root = self.project_root.clone();
                    tokio::spawn(async move {
                        handle_client(stream, auth, sessions, audit, app, root).await;
                    });
                }
                Err(e) => {
                    error!("UDS accept error: {e}");
                    break;
                }
            }
        }
    }
}

struct ClientConnection {
    authenticated_session_id: Option<String>,
    authenticated_actor: Option<String>,
}

async fn handle_client(
    stream: UnixStream,
    auth: Arc<AuthManager>,
    sessions: Arc<SessionManager>,
    audit: SharedAuditLogger,
    app: Arc<AppContext>,
    project_root: Option<PathBuf>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut conn = ClientConnection {
        authenticated_session_id: None,
        authenticated_actor: None,
    };

    loop {
        line.clear();
        let bytes_read = match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF - client disconnected
            Ok(n) => n,
            Err(e) => {
                debug!("UDS read error: {e}");
                break;
            }
        };

        if bytes_read > DEFAULT_MAX_UDS_LINE_BYTES {
            let err_resp = json!({
                "ok": false,
                "error": "PayloadTooLarge",
                "message": "request exceeds max line length",
            });
            let _ = writer.write_all(format!("{err_resp}\n").as_bytes()).await;
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request_val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                let err_resp = json!({
                    "ok": false,
                    "error": "InvalidJson",
                    "message": "Malformed JSON line",
                });
                let _ = writer.write_all(format!("{err_resp}\n").as_bytes()).await;
                continue;
            }
        };

        let _start_time = Instant::now();
        let (response, _duration_ms) = process_uds_request(
            &request_val,
            &mut conn,
            &auth,
            &sessions,
            &audit,
            &app,
            project_root.as_deref(),
        ).await;

        let response_str = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        if writer.write_all(format!("{response_str}\n").as_bytes()).await.is_err() {
            break;
        }
        let _ = writer.flush().await;
    }

    // Client EOF / disconnect: perform clean cleanup
    if let Some(session_id) = conn.authenticated_session_id.take() {
        debug!(%session_id, "UDS client disconnected; performing cleanup");
        sessions.disconnect_cleanup(&session_id);
        let mut logger = audit.lock();
        let _ = logger.record(
            &session_id,
            conn.authenticated_actor.as_deref().unwrap_or("unknown"),
            "client.disconnect",
            &json!({}),
            "system",
            0,
            "cleanup_done",
        );
    }
}

async fn process_uds_request(
    req: &Value,
    conn: &mut ClientConnection,
    auth: &Arc<AuthManager>,
    sessions: &Arc<SessionManager>,
    audit: &SharedAuditLogger,
    app: &Arc<AppContext>,
    project_root: Option<&Path>,
) -> (Value, u64) {
    let start = Instant::now();
    let req_id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("unknown");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // Extract token if provided in request
    let token_in_req = req
        .get("token")
        .or_else(|| req.get("auth_token"))
        .or_else(|| params.get("token"))
        .or_else(|| params.get("auth_token"))
        .and_then(Value::as_str);

    // Handle initial auth handshake
    if method == "auth" {
        let auth_res = auth.validate_token(token_in_req);
        let duration = start.elapsed().as_millis() as u64;
        return match auth_res {
            Ok(claims) => {
                let target_session = params
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&claims.session_id)
                    .to_string();
                let actor = params
                    .get("actor")
                    .and_then(Value::as_str)
                    .unwrap_or(&claims.actor)
                    .to_string();

                conn.authenticated_session_id = Some(target_session.clone());
                conn.authenticated_actor = Some(actor.clone());
                let _session_arc = sessions.get_or_create(&target_session, &actor);

                let mut logger = audit.lock();
                let _ = logger.record(
                    &target_session,
                    &actor,
                    "auth",
                    &params,
                    "read",
                    duration,
                    "ok",
                );

                (
                    json!({
                        "id": req_id,
                        "ok": true,
                        "result": {
                            "authenticated": true,
                            "session_id": target_session,
                            "actor": actor,
                        }
                    }),
                    duration,
                )
            }
            Err(e) => {
                let mut logger = audit.lock();
                let _ = logger.record(
                    "unauthenticated",
                    "anonymous",
                    "auth",
                    &params,
                    "read",
                    duration,
                    e.error_code(),
                );
                (
                    json!({
                        "id": req_id,
                        "ok": false,
                        "error": e.error_code(),
                        "message": e.to_string(),
                    }),
                    duration,
                )
            }
        };
    }

    // For any other method, ensure connection or request is authenticated
    let (session_id, actor) = if let Some(ref s_id) = conn.authenticated_session_id {
        (s_id.clone(), conn.authenticated_actor.clone().unwrap_or_else(|| "agent".to_string()))
    } else {
        match auth.validate_token(token_in_req) {
            Ok(claims) => {
                let s_id = params
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&claims.session_id)
                    .to_string();
                let act = params
                    .get("actor")
                    .and_then(Value::as_str)
                    .unwrap_or(&claims.actor)
                    .to_string();
                conn.authenticated_session_id = Some(s_id.clone());
                conn.authenticated_actor = Some(act.clone());
                (s_id, act)
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                let mut logger = audit.lock();
                let _ = logger.record(
                    "unauthenticated",
                    "anonymous",
                    method,
                    &params,
                    "read",
                    duration,
                    e.error_code(),
                );
                return (
                    json!({
                        "id": req_id,
                        "ok": false,
                        "error": e.error_code(),
                        "message": e.to_string(),
                    }),
                    duration,
                );
            }
        }
    };

    let session_arc = sessions.get_or_create(&session_id, &actor);
    let mut session = session_arc.write();

    // Check AF2 Stop
    if method != "session.resume" && method != "session_resume" {
        if let Err(e) = session.check_state() {
            let duration = start.elapsed().as_millis() as u64;
            let mut logger = audit.lock();
            let _ = logger.record(&session_id, &actor, method, &params, "read", duration, e.error_code());
            return (
                json!({
                    "id": req_id,
                    "ok": false,
                    "error": e.error_code(),
                    "message": e.to_string(),
                }),
                duration,
            );
        }
    }

    // Check Budgets (rate limit and time budget)
    if let Err(e) = session.check_budgets() {
        let duration = start.elapsed().as_millis() as u64;
        let mut logger = audit.lock();
        let _ = logger.record(&session_id, &actor, method, &params, "read", duration, e.error_code());
        return (
            json!({
                "id": req_id,
                "ok": false,
                "error": e.error_code(),
                "message": e.to_string(),
            }),
            duration,
        );
    }

    // Dispatch methods
    let (tier, result) = dispatch_uds_method(&mut session, method, &params, app, project_root);
    let duration = start.elapsed().as_millis() as u64;

    let res_status = match &result {
        Ok(_) => "ok",
        Err((code, _)) => *code,
    };

    let mut logger = audit.lock();
    let _ = logger.record(
        &session_id,
        &actor,
        method,
        &params,
        tier.as_str(),
        duration,
        res_status,
    );

    let payload = match result {
        Ok(v) => json!({
            "id": req_id,
            "ok": true,
            "result": v,
        }),
        Err((code, msg)) => json!({
            "id": req_id,
            "ok": false,
            "error": code,
            "message": msg,
        }),
    };

    (payload, duration)
}

fn dispatch_uds_method(
    session: &mut DaemonSession,
    method: &str,
    params: &Value,
    app: &Arc<AppContext>,
    project_root: Option<&Path>,
) -> (Tier, Result<Value, (&'static str, String)>) {
    match method {
        "health" => (
            Tier::Read,
            Ok(json!({
                "status": "ok",
                "session_id": session.id,
                "generation": session.generation,
                "stopped": session.stopped,
            })),
        ),
        "project.open" | "project_open" => {
            let path_opt = params.get("path").and_then(Value::as_str);
            match project_open(session, path_opt, project_root) {
                Ok(v) => (Tier::Read, Ok(v)),
                Err(e) => (Tier::Read, Err((e.error_code(), e.to_string()))),
            }
        }
        "project.close" | "project_close" => match project_close(session) {
            Ok(v) => (Tier::Read, Ok(v)),
            Err(e) => (Tier::Read, Err((e.error_code(), e.to_string()))),
        },
        "project.status" | "project_status" => (Tier::Read, Ok(project_status(session))),
        "project.save" | "project_save" => {
            let path_opt = params.get("path").and_then(Value::as_str);
            match project_save(session, path_opt, project_root) {
                Ok(v) => (Tier::Filesystem, Ok(v)),
                Err(e) => (Tier::Filesystem, Err((e.error_code(), e.to_string()))),
            }
        }
        "session.stop" | "session_stop" => {
            let new_gen = session.stop();
            (
                Tier::Read,
                Ok(json!({
                    "stopped": true,
                    "session_id": session.id,
                    "generation": new_gen,
                })),
            )
        }
        "session.resume" | "session_resume" => {
            let caller_name = params
                .get("caller")
                .and_then(Value::as_str)
                .unwrap_or("human");
            let caller = if caller_name.contains("human") || caller_name == "admin" {
                ActorId::human()
            } else {
                ActorId::agent(caller_name)
            };
            match session.resume(&caller) {
                Ok(new_gen) => (
                    Tier::Read,
                    Ok(json!({
                        "resumed": true,
                        "session_id": session.id,
                        "generation": new_gen,
                    })),
                ),
                Err(e) => (Tier::Read, Err((e.error_code(), e.to_string()))),
            }
        }
        "session.status" | "session_status" => (
            Tier::Read,
            Ok(json!({
                "session_id": session.id,
                "actor": session.actor,
                "stopped": session.stopped,
                "generation": session.generation,
                "locks": session.locks.iter().collect::<Vec<_>>(),
                "created_at_unix_ms": session.created_at_unix_ms,
            })),
        ),
        "lock.clip" => {
            let clip_id = match params.get("clip_id").and_then(Value::as_u64) {
                Some(id) => id,
                None => return (Tier::Edit, Err(("InvalidParams", "missing clip_id".into()))),
            };
            let note = params.get("note").and_then(Value::as_str).unwrap_or("");
            if let Some(ref mut engine) = session.engine {
                match engine.document_mut().locks.lock_clip(
                    session.actor.clone(),
                    Some(session.id.clone()),
                    clip_id,
                    note,
                ) {
                    Ok(id) => {
                        session.locks.insert(id);
                        (Tier::Edit, Ok(json!({ "lock_id": id, "clip_id": clip_id })))
                    }
                    Err(e) => (Tier::Edit, Err(("LockConflict", e))),
                }
            } else {
                (Tier::Edit, Err(("ProjectNotOpen", "project not open".into())))
            }
        }
        "lock.release" => {
            let lock_id = match params.get("lock_id").and_then(Value::as_u64) {
                Some(id) => id,
                None => return (Tier::Edit, Err(("InvalidParams", "missing lock_id".into()))),
            };
            if let Some(ref mut engine) = session.engine {
                match engine.document_mut().locks.release_lock(lock_id, &session.actor) {
                    Ok(_) => {
                        session.locks.remove(&lock_id);
                        (Tier::Edit, Ok(json!({ "released": true, "lock_id": lock_id })))
                    }
                    Err(e) => (Tier::Edit, Err(("LockError", e))),
                }
            } else {
                (Tier::Edit, Err(("ProjectNotOpen", "project not open".into())))
            }
        }
        "tools.list" | "tools_list" | "commands.list" | "commands_list" => (
            Tier::Read,
            Ok(serde_json::to_value(app.registry.all_specs()).unwrap_or_else(|_| json!([]))),
        ),
        other => {
            // Check AF5 untrusted media text
            if let Err(e) = validate_untrusted_media_text(params, Tier::Edit) {
                return (Tier::Edit, Err((e.error_code(), e.to_string())));
            }

            // Path containment check on any 'path' or 'source' parameter
            if let Some(p) = params.get("path").or_else(|| params.get("source")).and_then(Value::as_str) {
                if let Err(e) = validate_path_containment(Path::new(p), project_root.or(session.project_root.as_deref())) {
                    return (Tier::Filesystem, Err((e.error_code(), e.to_string())));
                }
            }

            // Check if engine is available for direct core commands
            let ctx = session.actor_context();
            if let Some(ref mut engine) = session.engine {
                let cmd_id = if other == "command.execute" || other == "execute" {
                    params.get("command").and_then(Value::as_str).unwrap_or(other)
                } else {
                    other
                };
                let inner_params = if (other == "command.execute" || other == "execute") && params.get("params").is_some() {
                    params.get("params").cloned().unwrap_or_else(|| json!({}))
                } else {
                    params.clone()
                };

                let request = CommandRequest {
                    id: cmd_id.to_string(),
                    params: inner_params,
                    rationale: None,
                    confidence: None,
                };
                match engine.execute_with_context(request, &ctx) {
                    Ok(val) => (Tier::Edit, Ok(val)),
                    Err(err) => (Tier::Edit, Err(("CommandExecutionFailed", err.to_string()))),
                }
            } else {
                (
                    Tier::Read,
                    Err(("ProjectNotOpen", format!("cannot execute '{other}': no project is open in this session"))),
                )
            }
        }
    }
}
