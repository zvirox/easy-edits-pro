// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet REST handlers (TS7).

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use saturn_core::actor::{ActorContext, ActorId};
use saturn_core::command::CommandRequest;
use saturn_core::tasksheet::TaskSheet;
use serde_json::{Value, json};

use super::{authenticate_and_check, record_audit_and_respond};
use crate::handlers::ErrorResponse;
use crate::server::ControlServer;

/// `GET /tasksheet` or `GET /api/v1/tasksheet` — fetch active task sheet.
pub async fn tasksheet_get_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "tasksheet.get";
    let params = json!({});
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
                Ok(val) => Ok(json!({
                    "success": true,
                    "status": val.get("status").and_then(Value::as_str).unwrap_or("unknown"),
                    "sheet": val.get("sheet").cloned().unwrap_or(val),
                })),
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

/// `POST /tasksheet` or `POST /api/v1/tasksheet` — create or update task sheet.
pub async fn tasksheet_post_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, "tasksheet.post", &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let (endpoint, result) = {
        let mut session = ctx.session_arc.write();
        let act_ctx = session.actor_context();
        if let Some(ref mut engine) = session.engine {
            let already_exists = engine.document().project.task_sheet.is_some();
            let force_create = params.get("create").and_then(Value::as_bool).unwrap_or(false);
            let cmd_id = if !already_exists || force_create {
                "tasksheet.create"
            } else {
                "tasksheet.update"
            };

            let req = CommandRequest::new(cmd_id, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let sheet_val = val.get("sheet").cloned().unwrap_or_else(|| val.clone());
                    let status = sheet_val.get("status").and_then(Value::as_str).unwrap_or("draft");
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": cmd_id,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));
                    (
                        cmd_id,
                        Ok(json!({
                            "success": true,
                            "command": cmd_id,
                            "sheet": sheet_val,
                        })),
                    )
                }
                Err(e) => (cmd_id, Err((StatusCode::BAD_REQUEST, "CommandExecutionFailed", e.to_string()))),
            }
        } else {
            (
                "tasksheet.post",
                Err((
                    StatusCode::BAD_REQUEST,
                    "ProjectNotOpen",
                    "no project is currently open in this session".into(),
                )),
            )
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

/// `POST /tasksheet/submit` or `POST /api/v1/tasksheet/submit` — submit sheet to ready status.
pub async fn tasksheet_submit_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.submit";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    if !ctx.claims.can_submit_jobs() {
        let _ = server.audit.lock().record(
            &ctx.session_id,
            &ctx.actor,
            endpoint,
            &params,
            tier,
            ctx.start_time.elapsed().as_millis() as u64,
            "PermissionDenied",
        );
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "PermissionDenied: caller does not have permission to submit task sheets".into(),
                code: Some("PermissionDenied".into()),
            }),
        )
            .into_response();
    }

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = session.actor_context();
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let status = val.get("status").and_then(Value::as_str).unwrap_or("ready");
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": endpoint,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));
                    Ok(json!({
                        "success": true,
                        "submitted": true,
                        "status": status,
                        "sheet": val.get("sheet").cloned().unwrap_or(val),
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

/// `POST /tasksheet/plan/submit` or `POST /api/v1/tasksheet/plan/submit` — submit execution plan.
pub async fn tasksheet_plan_submit_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.plan_submit";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

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
                    let status = val.get("status").and_then(Value::as_str).unwrap_or("awaiting_plan_approval");
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": endpoint,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));
                    Ok(json!({
                        "success": true,
                        "plan_submitted": true,
                        "status": status,
                        "sheet": val.get("sheet").cloned().unwrap_or(val),
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

/// `POST /tasksheet/plan/approve` or `POST /api/v1/tasksheet/plan/approve` — human approves plan.
pub async fn tasksheet_plan_approve_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.plan_approve";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    if ctx.claims.is_agent() {
        let _ = server.audit.lock().record(
            &ctx.session_id,
            &ctx.actor,
            endpoint,
            &params,
            tier,
            ctx.start_time.elapsed().as_millis() as u64,
            "AgentApprovalForbidden",
        );
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "AgentApprovalForbidden: agent actors cannot approve plans".into(),
                code: Some("AgentApprovalForbidden".into()),
            }),
        )
            .into_response();
    }

    if !ctx.claims.can_approve_plan() {
        let _ = server.audit.lock().record(
            &ctx.session_id,
            &ctx.actor,
            endpoint,
            &params,
            tier,
            ctx.start_time.elapsed().as_millis() as u64,
            "PermissionDenied",
        );
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "PermissionDenied: caller does not have permission to approve plans".into(),
                code: Some("PermissionDenied".into()),
            }),
        )
            .into_response();
    }

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = ActorContext {
            actor: ActorId::human(),
            session_id: session.id.clone(),
            branch: saturn_core::branch::MAIN_BRANCH.to_string(),
            generation: session.generation,
        };
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let status = val.get("status").and_then(Value::as_str).unwrap_or("running");
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": endpoint,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));
                    Ok(json!({
                        "success": true,
                        "approved": true,
                        "status": status,
                        "sheet": val.get("sheet").cloned().unwrap_or(val),
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

/// `POST /tasksheet/plan/reject` or `POST /api/v1/tasksheet/plan/reject` — human rejects plan.
pub async fn tasksheet_plan_reject_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.plan_reject";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    if ctx.claims.is_agent() {
        let _ = server.audit.lock().record(
            &ctx.session_id,
            &ctx.actor,
            endpoint,
            &params,
            tier,
            ctx.start_time.elapsed().as_millis() as u64,
            "AgentApprovalForbidden",
        );
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "AgentApprovalForbidden: agent actors cannot reject plans".into(),
                code: Some("AgentApprovalForbidden".into()),
            }),
        )
            .into_response();
    }

    if !ctx.claims.can_approve_plan() {
        let _ = server.audit.lock().record(
            &ctx.session_id,
            &ctx.actor,
            endpoint,
            &params,
            tier,
            ctx.start_time.elapsed().as_millis() as u64,
            "PermissionDenied",
        );
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "PermissionDenied: caller does not have permission to reject plans".into(),
                code: Some("PermissionDenied".into()),
            }),
        )
            .into_response();
    }

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = ActorContext {
            actor: ActorId::human(),
            session_id: session.id.clone(),
            branch: saturn_core::branch::MAIN_BRANCH.to_string(),
            generation: session.generation,
        };
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, params.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let status = val.get("status").and_then(Value::as_str).unwrap_or("planning");
                    let reason = val.get("reason").and_then(Value::as_str).unwrap_or("");
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": endpoint,
                        "status": status,
                        "reason": reason,
                        "session_id": ctx.session_id,
                    }));
                    Ok(json!({
                        "success": true,
                        "rejected": true,
                        "status": status,
                        "reason": reason,
                        "sheet": val.get("sheet").cloned().unwrap_or(val),
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

/// `GET /tasksheet/report` or `GET /api/v1/tasksheet/report` — fetch run report.
pub async fn tasksheet_report_get_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "tasksheet.report";
    let params = json!({});
    let tier = "read";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let session = ctx.session_arc.read();
        if let Some(ref engine) = session.engine {
            let sheet = engine
                .document()
                .project
                .task_sheet
                .clone()
                .unwrap_or_else(|| TaskSheet::from_brief(&engine.document().project.brief));

            Ok(json!({
                "success": true,
                "status": sheet.status.as_str(),
                "sheet_id": sheet.id,
                "plan": sheet.plan,
                "questions": sheet.questions,
                "rounds": sheet.rounds,
                "transitions": sheet.transitions,
            }))
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

/// `POST /tasksheet/report` or `POST /api/v1/tasksheet/report` — update run report.
pub async fn tasksheet_report_post_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.run_report";
    let params = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let tier = "edit";

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
                    server.broadcast(json!({
                        "type": "tasksheet_transition",
                        "event": endpoint,
                        "session_id": ctx.session_id,
                    }));
                    Ok(json!({
                        "success": true,
                        "reported": true,
                        "sheet": val.get("sheet").cloned().unwrap_or(val),
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
