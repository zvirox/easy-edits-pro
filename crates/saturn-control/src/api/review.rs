// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Review & Human-in-the-Loop REST handlers (TS7).

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use saturn_core::command::CommandRequest;
use serde_json::{Value, json};

use super::{authenticate_and_check, record_audit_and_respond};
use crate::server::ControlServer;

/// `GET /review/notes` or `GET /api/v1/review/notes` — list review notes.
pub async fn review_notes_list_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "review.note_list";
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
                Ok(val) => {
                    let notes = val.get("notes").cloned().unwrap_or(val);
                    Ok(json!({
                        "success": true,
                        "notes": notes,
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

/// `POST /review/notes` or `POST /api/v1/review/notes` — add review note.
pub async fn review_notes_add_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "review.note_add";
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
                    let note_id = val
                        .get("note")
                        .and_then(|n| n.get("id"))
                        .or_else(|| val.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("");

                    server.broadcast(json!({
                        "type": "review_note_added",
                        "note_id": note_id,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "added": true,
                        "note": val.get("note").cloned().unwrap_or(val),
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

/// `POST /review/notes/:id/resolve` or `POST /api/v1/review/notes/:id/resolve` — resolve review note.
pub async fn review_notes_resolve_handler(
    State(server): State<Arc<ControlServer>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "review.note_resolve";
    let mut payload = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));

    if let Value::Object(ref mut map) = payload {
        map.insert("note_id".to_string(), Value::String(id.clone()));
    } else {
        payload = json!({
            "note_id": id,
            "resolution": payload.get("resolution").and_then(Value::as_str).unwrap_or("accepted"),
        });
    }

    let tier = "edit";

    let ctx = match authenticate_and_check(&headers, &server, endpoint, &payload, tier) {
        Ok(c) => c,
        Err(res) => return res,
    };

    let result = {
        let mut session = ctx.session_arc.write();
        let act_ctx = session.actor_context();
        if let Some(ref mut engine) = session.engine {
            let req = CommandRequest::new(endpoint, payload.clone());
            match engine.execute_with_context(req, &act_ctx) {
                Ok(val) => {
                    server.broadcast(json!({
                        "type": "review_note_resolved",
                        "note_id": id,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "resolved": true,
                        "note_id": id,
                        "result": val,
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

/// `GET /review/changes` or `GET /api/v1/review/changes` — list agent changes.
pub async fn review_changes_list_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "review.list_changes";
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
                Ok(val) => {
                    let changes = val.get("changes").cloned().unwrap_or(val);
                    Ok(json!({
                        "success": true,
                        "changes": changes,
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

/// `POST /review/round/open` or `POST /api/v1/review/round/open` — open review round.
pub async fn review_round_open_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "review.round_open";
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
                    let round_num = val.get("round_number").and_then(Value::as_u64).unwrap_or(1);
                    server.broadcast(json!({
                        "type": "review_round_opened",
                        "round_number": round_num,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "round_opened": true,
                        "round_number": round_num,
                        "details": val,
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

/// `POST /review/round/close` or `POST /api/v1/review/round/close` — close review round.
pub async fn review_round_close_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "review.round_close";
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
                    let round_num = val.get("round_number").and_then(Value::as_u64).unwrap_or(1);
                    let outcome = val.get("outcome").and_then(Value::as_str).unwrap_or("approved");

                    server.broadcast(json!({
                        "type": "review_round_closed",
                        "round_number": round_num,
                        "outcome": outcome,
                        "session_id": ctx.session_id,
                    }));

                    Ok(json!({
                        "success": true,
                        "round_closed": true,
                        "round_number": round_num,
                        "outcome": outcome,
                        "details": val,
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
