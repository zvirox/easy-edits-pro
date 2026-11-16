// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! REST API handlers for the Runner service (TS9).
//!
//! Routes:
//!   GET  /runner/status          — runner status summary
//!   POST /runner/start           — start the runner
//!   POST /runner/stop            — stop the runner
//!   GET  /runner/jobs            — list all jobs
//!   POST /runner/jobs/:id/cancel — cancel a specific job

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::api::{authenticate_and_check, record_audit_and_respond};
use crate::handlers::ErrorResponse;
use crate::runner::byo_endpoint::ByoEndpoint;
use crate::runner::{RunnerError, WorkerBudget};
use crate::server::ControlServer;

/// Request body for POST /runner/start.
#[derive(Debug, Deserialize)]
pub struct RunnerStartRequest {
    #[serde(default)]
    pub max_concurrent_workers: Option<usize>,
}

/// Request body for POST /runner/jobs (enqueue a new job).
#[derive(Debug, Deserialize)]
pub struct EnqueueJobRequest {
    pub tasksheet_id: String,
    #[serde(default)]
    pub budget: Option<WorkerBudget>,
    #[serde(default)]
    pub endpoint: Option<ByoEndpoint>,
}

fn runner_error_response(err: &RunnerError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match err {
        RunnerError::JobNotFound { .. } => StatusCode::NOT_FOUND,
        RunnerError::RunnerNotRunning => StatusCode::CONFLICT,
        RunnerError::RunnerAlreadyRunning => StatusCode::CONFLICT,
        RunnerError::Endpoint(_) => StatusCode::UNPROCESSABLE_ENTITY,
    };
    (
        status,
        Json(ErrorResponse {
            success: false,
            error: err.to_string(),
            code: Some(err.error_code().to_string()),
        }),
    )
}

/// `GET /runner/status` — return runner status and job counts.
pub async fn runner_status_handler(
    headers: HeaderMap,
    State(server): State<Arc<ControlServer>>,
) -> Response {
    let params = json!({});
    let ctx = match authenticate_and_check(&headers, &server, "runner.status", &params, "ts9") {
        Ok(c) => c,
        Err(e) => return e,
    };

    let status = server.runner.status();
    let body = serde_json::to_value(&status).unwrap_or(json!({}));
    record_audit_and_respond(
        &server,
        &ctx.session_id,
        &ctx.actor,
        "runner.status",
        &params,
        "ts9",
        ctx.start_time,
        Ok(body),
    )
}

/// `POST /runner/start` — start the runner service.
pub async fn runner_start_handler(
    headers: HeaderMap,
    State(server): State<Arc<ControlServer>>,
    Json(body): Json<Option<RunnerStartRequest>>,
) -> Response {
    let params = json!({ "request": body.is_some() });
    let ctx = match authenticate_and_check(&headers, &server, "runner.start", &params, "ts9") {
        Ok(c) => c,
        Err(e) => return e,
    };

    match server.runner.start() {
        Ok(()) => {
            let status = server.runner.status();
            let resp = serde_json::to_value(&status).unwrap_or(json!({"running": true}));
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.start",
                &params,
                "ts9",
                ctx.start_time,
                Ok(resp),
            )
        }
        Err(e) => {
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.start",
                &params,
                "ts9",
                ctx.start_time,
                Err((runner_error_response(&e).0, e.error_code(), e.to_string())),
            )
        }
    }
}

/// `POST /runner/stop` — stop the runner service.
pub async fn runner_stop_handler(
    headers: HeaderMap,
    State(server): State<Arc<ControlServer>>,
) -> Response {
    let params = json!({});
    let ctx = match authenticate_and_check(&headers, &server, "runner.stop", &params, "ts9") {
        Ok(c) => c,
        Err(e) => return e,
    };

    match server.runner.stop() {
        Ok(()) => {
            let status = server.runner.status();
            let resp = serde_json::to_value(&status).unwrap_or(json!({"running": false}));
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.stop",
                &params,
                "ts9",
                ctx.start_time,
                Ok(resp),
            )
        }
        Err(e) => {
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.stop",
                &params,
                "ts9",
                ctx.start_time,
                Err((runner_error_response(&e).0, e.error_code(), e.to_string())),
            )
        }
    }
}

/// `GET /runner/jobs` — list all jobs.
pub async fn runner_jobs_list_handler(
    headers: HeaderMap,
    State(server): State<Arc<ControlServer>>,
) -> Response {
    let params = json!({});
    let ctx = match authenticate_and_check(&headers, &server, "runner.jobs.list", &params, "ts9") {
        Ok(c) => c,
        Err(e) => return e,
    };

    let jobs = server.runner.list_jobs();
    // Redact endpoint credentials in each job before serialising
    let safe_jobs: Vec<Value> = jobs
        .into_iter()
        .map(|j| {
            let mut v = serde_json::to_value(&j).unwrap_or(json!({}));
            if let Some(ep_safe) = j.audit_safe_endpoint() {
                v["endpoint"] = ep_safe;
            }
            v
        })
        .collect();
    let resp = json!({ "jobs": safe_jobs });
    record_audit_and_respond(
        &server,
        &ctx.session_id,
        &ctx.actor,
        "runner.jobs.list",
        &params,
        "ts9",
        ctx.start_time,
        Ok(resp),
    )
}

/// `POST /runner/jobs/:id/cancel` — cancel a job.
pub async fn runner_job_cancel_handler(
    headers: HeaderMap,
    State(server): State<Arc<ControlServer>>,
    Path(id): Path<String>,
) -> Response {
    let params = json!({ "job_id": id });
    let ctx = match authenticate_and_check(&headers, &server, "runner.jobs.cancel", &params, "ts9") {
        Ok(c) => c,
        Err(e) => return e,
    };

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Invalid job ID: '{id}'"),
                    code: Some("InvalidArgument".to_string()),
                }),
            )
                .into_response();
        }
    };

    match server.runner.cancel_job(uuid) {
        Ok(()) => {
            let resp = json!({ "cancelled": true, "job_id": id });
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.jobs.cancel",
                &params,
                "ts9",
                ctx.start_time,
                Ok(resp),
            )
        }
        Err(e) => {
            record_audit_and_respond(
                &server,
                &ctx.session_id,
                &ctx.actor,
                "runner.jobs.cancel",
                &params,
                "ts9",
                ctx.start_time,
                Err((runner_error_response(&e).0, e.error_code(), e.to_string())),
            )
        }
    }
}
