// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Verification and Delivery Quality Control REST handlers (TS7).

use std::path::Path;
use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use saturn_core::command::CommandRequest;
use serde_json::{Value, json};

use super::{authenticate_and_check, record_audit_and_respond};
use crate::provenance::validate_path_containment;
use crate::server::ControlServer;

/// `POST /verify/run` or `POST /api/v1/verify/run` — execute sequence verification.
pub async fn verify_run_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "verify.run";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "read";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = session.actor_context();
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let preset = val.get("preset").and_then(Value::as_str).unwrap_or("web-1080p");
                    let failed_count = val
                        .get("summary")
                        .and_then(|s| s.get("failed"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let passed = failed_count == 0;

                    server.broadcast(json!({
                        "type": "verification_completed",
                        "preset": preset,
                        "passed": passed,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "verified": true,
                        "report": val,
                    }))
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, "CommandExecutionFailed", e.to_string())),
            }
        } else {
            Err((
                StatusCode::BAD_REQUEST,
                "ProjectNotOpen",
                "no project is currently open in this session".into(),
            ))
        }
    };

    record_audit_and_respond(
        &server,
        &ctx.session_id,
        &ctx.actor,
        endpoint,
        &params,
        tier,
        ctx.start_time,
        result,
    )
}

/// `POST /delivery/verify` or `POST /api/v1/delivery/verify` — verify packaged delivery bundle.
pub async fn delivery_verify_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "delivery.verify";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "filesystem";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let mut session = ctx.session_arc.write();

        // Path containment validation
        if let Some(p) = params.get("path").and_then(Value::as_str) {
            let effective_root = server.project_root.as_deref().or(session.project_root.as_deref());
            if let Err(e) = validate_path_containment(Path::new(p), effective_root) {
                return record_audit_and_respond(
                    &server,
                    &ctx.session_id,
                    &ctx.actor,
                    endpoint,
                    &params,
                    tier,
                    ctx.start_time,
                    Err((StatusCode::FORBIDDEN, e.error_code(), e.to_string())),
                );
            }
        }

        let act_ctx = session.actor_context();
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    server.broadcast(json!({
                        "type": "delivery_verified",
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "verified": true,
                        "verification": val,
                    }))
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, "CommandExecutionFailed", e.to_string())),
            }
        } else {
            Err((
                StatusCode::BAD_REQUEST,
                "ProjectNotOpen",
                "no project is currently open in this session".into(),
            ))
        }
    };

    record_audit_and_respond(
        &server,
        &ctx.session_id,
        &ctx.actor,
        endpoint,
        &params,
        tier,
        ctx.start_time,
        result,
    )
}
