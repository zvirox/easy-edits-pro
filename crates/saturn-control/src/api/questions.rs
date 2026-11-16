// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet Questions REST handlers (TS7).

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use saturn_core::actor::{ActorContext, ActorId};
use saturn_core::command::CommandRequest;
use serde_json::{Value, json};

use super::{authenticate_and_check, record_audit_and_respond};
use crate::server::ControlServer;

/// `POST /tasksheet/questions` or `POST /api/v1/tasksheet/questions` — ask structured question.
pub async fn questions_ask_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.question_ask";
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
                    let q_id = val.get("question_id").and_then(Value::as_str).unwrap_or("");
                    let question_text = params.get("question").and_then(Value::as_str).unwrap_or("");
                    let status = val.get("status").and_then(Value::as_str).unwrap_or("needs_input");

                    server.broadcast(json!({
                        "type": "question_asked",
                        "question_id": q_id,
                        "question": question_text,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "asked": true,
                        "question_id": q_id,
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

/// `POST /tasksheet/questions/:id/answer` or `POST /api/v1/tasksheet/questions/:id/answer` — answer question.
pub async fn questions_answer_handler(
    State(server): State<Arc<ControlServer>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "tasksheet.question_answer";
    let mut payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));

    // Ensure question_id is set from path
    if let Value::Object(ref mut map) = payload {
        map.insert("question_id".to_string(), Value::String(id.clone()));
    } else {
        payload = json!({
            "question_id": id,
            "option_id": payload.get("option_id").and_then(Value::as_str).unwrap_or(""),
        });
    }

    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &payload, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = ActorContext {
            actor: ActorId::human(),
            session_id: session.id.clone(),
            branch: saturn_core::branch::MAIN_BRANCH.to_string(),
            generation: session.generation,
        };
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, payload.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    let opt_id = payload.get("option_id").and_then(Value::as_str).unwrap_or("");
                    let sheet_val = val.get("sheet").cloned().unwrap_or(val);
                    let status = sheet_val.get("status").and_then(Value::as_str).unwrap_or("running");

                    server.broadcast(json!({
                        "type": "question_answered",
                        "question_id": id,
                        "option_id": opt_id,
                        "status": status,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "answered": true,
                        "question_id": id,
                        "option_id": opt_id,
                        "status": status,
                        "sheet": sheet_val,
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
        &payload,
        tier,
        ctx.start_time,
        result,
    )
}

/// `GET /tasksheet/questions` or `GET /api/v1/tasksheet/questions` — list questions.
pub async fn questions_list_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "tasksheet.questions.list";
    let params = json!({});
    let tier = "read";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &params, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let session = ctx.session_arc.read();
        if let Some(ref engine) = session.engine {
            let questions = engine
                .document()
                .project
                .task_sheet
                .as_ref()
                .map(|s| s.questions.clone())
                .unwrap_or_default();

            Ok(json!({
                "success": true,
                "questions": questions,
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
