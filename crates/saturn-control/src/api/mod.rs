// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! REST API module for saturn-control daemon (TS7).
//!
//! Exposes Task Sheet, Review, Question, and Verification endpoints
//! under `/api/v1` and root paths with AF9 auth, budgets, and audit integration.

pub mod questions;
pub mod review;
pub mod runner;
pub mod tasksheet;
pub mod verify;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use parking_lot::RwLock;
use serde_json::Value;

use crate::handlers::ErrorResponse;
use crate::server::ControlServer;
use crate::session::DaemonSession;

pub struct AuthSessionContext {
    pub session_arc: Arc<RwLock<DaemonSession>>,
    pub session_id: String,
    pub actor: String,
    pub start_time: Instant,
}

/// Validates bearer authentication token, session state (fenced on AF2 Stop),
/// and rate/time budgets before passing control to the endpoint logic.
pub fn authenticate_and_check(
    headers: &HeaderMap,
    server: &ControlServer,
    endpoint_name: &str,
    params: &Value,
    tier: &str,
) -> Result<AuthSessionContext, Response> {
    let start_time = Instant::now();
    let header_val = headers
        .get("authorization")
        .or_else(|| headers.get("x-auth-token"))
        .and_then(|v| v.to_str().ok());

    let claims = match server.auth.validate_token(header_val) {
        Ok(c) => c,
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut logger = server.audit.lock();
            let _ = logger.record(
                "unauthenticated",
                "anonymous",
                endpoint_name,
                params,
                tier,
                duration_ms,
                e.error_code(),
            );

            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    success: false,
                    error: e.to_string(),
                    code: Some(e.error_code().to_string()),
                }),
            )
                .into_response());
        }
    };

    let session_arc = server.sessions.get_or_create(&claims.session_id, &claims.actor);
    {
        let mut session = session_arc.write();

        if let Err(e) = session.check_state() {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut logger = server.audit.lock();
            let _ = logger.record(
                &claims.session_id,
                &claims.actor,
                endpoint_name,
                params,
                tier,
                duration_ms,
                e.error_code(),
            );

            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    success: false,
                    error: e.to_string(),
                    code: Some(e.error_code().to_string()),
                }),
            )
                .into_response());
        }

        if let Err(e) = session.check_budgets() {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut logger = server.audit.lock();
            let _ = logger.record(
                &claims.session_id,
                &claims.actor,
                endpoint_name,
                params,
                tier,
                duration_ms,
                e.error_code(),
            );

            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    success: false,
                    error: e.to_string(),
                    code: Some(e.error_code().to_string()),
                }),
            )
                .into_response());
        }
    }

    Ok(AuthSessionContext {
        session_arc,
        session_id: claims.session_id,
        actor: claims.actor,
        start_time,
    })
}

/// Records the audit log entry and builds the HTTP response.
pub fn record_audit_and_respond(
    server: &ControlServer,
    session_id: &str,
    actor: &str,
    endpoint_name: &str,
    params: &Value,
    tier: &str,
    start_time: Instant,
    result: Result<Value, (StatusCode, &'static str, String)>,
) -> Response {
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let mut logger = server.audit.lock();

    match result {
        Ok(body) => {
            let _ = logger.record(
                session_id,
                actor,
                endpoint_name,
                params,
                tier,
                duration_ms,
                "ok",
            );
            (StatusCode::OK, Json(body)).into_response()
        }
        Err((status, code, err_msg)) => {
            let _ = logger.record(
                session_id,
                actor,
                endpoint_name,
                params,
                tier,
                duration_ms,
                code,
            );
            (
                status,
                Json(ErrorResponse {
                    success: false,
                    error: err_msg,
                    code: Some(code.to_string()),
                }),
            )
                .into_response()
        }
    }
}

/// Mounts all TS7 web API routes onto the given axum Router.
pub fn mount_api_routes(mut router: Router<Arc<ControlServer>>) -> Router<Arc<ControlServer>> {
    // Task Sheet routes
    router = router
        .route("/tasksheet", get(tasksheet::tasksheet_get_handler))
        .route("/tasksheet", post(tasksheet::tasksheet_post_handler))
        .route("/tasksheet/submit", post(tasksheet::tasksheet_submit_handler))
        .route("/tasksheet/plan/submit", post(tasksheet::tasksheet_plan_submit_handler))
        .route("/tasksheet/plan/approve", post(tasksheet::tasksheet_plan_approve_handler))
        .route("/tasksheet/plan/reject", post(tasksheet::tasksheet_plan_reject_handler))
        .route("/tasksheet/report", get(tasksheet::tasksheet_report_get_handler))
        .route("/tasksheet/report", post(tasksheet::tasksheet_report_post_handler))
        .route("/tasksheet/run_report", post(tasksheet::tasksheet_report_post_handler))
        .route("/api/v1/tasksheet", get(tasksheet::tasksheet_get_handler))
        .route("/api/v1/tasksheet", post(tasksheet::tasksheet_post_handler))
        .route("/api/v1/tasksheet/submit", post(tasksheet::tasksheet_submit_handler))
        .route("/api/v1/tasksheet/plan/submit", post(tasksheet::tasksheet_plan_submit_handler))
        .route("/api/v1/tasksheet/plan/approve", post(tasksheet::tasksheet_plan_approve_handler))
        .route("/api/v1/tasksheet/plan/reject", post(tasksheet::tasksheet_plan_reject_handler))
        .route("/api/v1/tasksheet/report", get(tasksheet::tasksheet_report_get_handler))
        .route("/api/v1/tasksheet/report", post(tasksheet::tasksheet_report_post_handler))
        .route("/api/v1/tasksheet/run_report", post(tasksheet::tasksheet_report_post_handler));

    // Question routes
    router = router
        .route("/tasksheet/questions", post(questions::questions_ask_handler))
        .route("/tasksheet/questions", get(questions::questions_list_handler))
        .route("/tasksheet/questions/:id/answer", post(questions::questions_answer_handler))
        .route("/api/v1/tasksheet/questions", post(questions::questions_ask_handler))
        .route("/api/v1/tasksheet/questions", get(questions::questions_list_handler))
        .route("/api/v1/tasksheet/questions/:id/answer", post(questions::questions_answer_handler));

    // Review routes
    router = router
        .route("/review/notes", get(review::review_notes_list_handler))
        .route("/review/notes", post(review::review_notes_add_handler))
        .route("/review/notes/:id/resolve", post(review::review_notes_resolve_handler))
        .route("/review/changes", get(review::review_changes_list_handler))
        .route("/review/round/open", post(review::review_round_open_handler))
        .route("/review/round/close", post(review::review_round_close_handler))
        .route("/api/v1/review/notes", get(review::review_notes_list_handler))
        .route("/api/v1/review/notes", post(review::review_notes_add_handler))
        .route("/api/v1/review/notes/:id/resolve", post(review::review_notes_resolve_handler))
        .route("/api/v1/review/changes", get(review::review_changes_list_handler))
        .route("/api/v1/review/round/open", post(review::review_round_open_handler))
        .route("/api/v1/review/round/close", post(review::review_round_close_handler));

    // Verification & Delivery routes
    router = router
        .route("/verify/run", post(verify::verify_run_handler))
        .route("/delivery/verify", post(verify::delivery_verify_handler))
        .route("/api/v1/verify/run", post(verify::verify_run_handler))
        .route("/api/v1/delivery/verify", post(verify::delivery_verify_handler));

    // Runner & Multi-Job Execution routes (TS9)
    router = router
        .route("/runner/status", get(runner::runner_status_handler))
        .route("/runner/start", post(runner::runner_start_handler))
        .route("/runner/stop", post(runner::runner_stop_handler))
        .route("/runner/jobs", get(runner::runner_jobs_list_handler))
        .route("/runner/jobs/:id/cancel", post(runner::runner_job_cancel_handler))
        .route("/api/v1/runner/status", get(runner::runner_status_handler))
        .route("/api/v1/runner/start", post(runner::runner_start_handler))
        .route("/api/v1/runner/stop", post(runner::runner_stop_handler))
        .route("/api/v1/runner/jobs", get(runner::runner_jobs_list_handler))
        .route("/api/v1/runner/jobs/:id/cancel", post(runner::runner_job_cancel_handler));

    router
}
