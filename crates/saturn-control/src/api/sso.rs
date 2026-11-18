// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! REST API handlers for Enterprise SSO (OIDC & SAML 2.0).

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::handlers::ErrorResponse;
use crate::server::ControlServer;
use crate::sso::{OidcCallbackRequest, OidcLoginRequest, SamlCallbackRequest, SamlLoginRequest, SsoError};

/// `GET /auth/sso/status` & `GET /api/v1/auth/sso/status` — inspect SSO provider statuses.
pub async fn sso_status_handler(State(server): State<Arc<ControlServer>>) -> Response {
    let status = server.sso.status();
    (StatusCode::OK, Json(json!({
        "success": true,
        "status": status,
        "enabled": status.enabled,
        "oidc": status.oidc,
        "saml": status.saml,
    }))).into_response()
}

/// `GET /auth/sso/roles` & `GET /api/v1/auth/sso/roles` — inspect caller role and permissions.
pub async fn sso_roles_handler(
    State(server): State<Arc<ControlServer>>,
    headers: HeaderMap,
) -> Response {
    let endpoint = "auth.sso.roles";
    let start_time = Instant::now();
    let params = json!({});

    let header_val = headers
        .get("authorization")
        .or_else(|| headers.get("x-auth-token"))
        .and_then(|v| v.to_str().ok());

    let claims = match server.auth.validate_token(header_val) {
        Ok(c) => c,
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &params,
                "read",
                duration_ms,
                e.error_code(),
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    success: false,
                    error: e.to_string(),
                    code: Some(e.error_code().to_string()),
                }),
            )
                .into_response();
        }
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;
    let _ = server.audit.lock().record(
        &claims.session_id,
        &claims.actor,
        endpoint,
        &params,
        "read",
        duration_ms,
        "ok",
    );

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "session_id": claims.session_id,
            "actor": claims.actor,
            "role": claims.role.as_str(),
            "is_agent": claims.is_agent(),
            "permissions": {
                "can_approve_plan": claims.can_approve_plan(),
                "can_approve_delivery": claims.can_approve_delivery(),
                "can_submit_jobs": claims.can_submit_jobs(),
            },
            "available_roles": [
                "Requester",
                "Reviewer",
                "Approver",
                "Admin"
            ]
        })),
    )
        .into_response()
}

/// `POST /auth/sso/oidc/login` & `GET /auth/sso/oidc/login` — generate OIDC authorization URL.
pub async fn oidc_login_handler(
    State(server): State<Arc<ControlServer>>,
    Query(query_req): Query<OidcLoginRequest>,
    body: Option<Json<OidcLoginRequest>>,
) -> Response {
    let endpoint = "auth.sso.oidc_login";
    let start_time = Instant::now();

    // Prefer body if present, fallback to query parameters
    let req = body.map(|Json(b)| b).unwrap_or(query_req);
    let audit_params = json!({
        "provider": req.provider.as_deref().unwrap_or("oidc"),
        "has_redirect_uri": req.redirect_uri.is_some(),
        "has_state": req.state.is_some(),
    });

    match server.sso.oidc_login(&req) {
        Ok(res) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "read",
                duration_ms,
                "ok",
            );
            (StatusCode::OK, Json(json!({
                "success": true,
                "authorization_url": res.authorization_url,
                "state": res.state,
                "nonce": res.nonce,
            }))).into_response()
        }
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "read",
                duration_ms,
                e.error_code(),
            );
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            })).into_response()
        }
    }
}

/// `POST /auth/sso/oidc/callback` — validate OIDC claims/token and issue session token.
pub async fn oidc_callback_handler(
    State(server): State<Arc<ControlServer>>,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "auth.sso.oidc_callback";
    let start_time = Instant::now();

    let raw_body = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));

    // Deserialise request payload
    let req: OidcCallbackRequest = serde_json::from_value(raw_body.clone())
        .unwrap_or_default();

    // Audit params: redact token, id_token, secret
    let audit_params = json!({
        "has_code": req.code.is_some(),
        "has_id_token": req.id_token.is_some(),
        "has_claims": req.claims.is_some(),
        "state": req.state,
    });

    match server.sso.oidc_callback(&req) {
        Ok(success) => {
            let session_id = format!("sso-oidc-{}", Uuid::new_v4());
            let actor_name = format!("human:{}", success.subject);

            // Issue session token with mapped role
            let token = server.auth.issue_session_token_with_role(
                &session_id,
                &actor_name,
                success.role,
                false,
            );
            let _session = server.sessions.get_or_create(&session_id, &actor_name);

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                &session_id,
                &actor_name,
                endpoint,
                &audit_params,
                "admin",
                duration_ms,
                "ok",
            );

            (StatusCode::OK, Json(json!({
                "success": true,
                "token": token,
                "session_id": session_id,
                "actor": actor_name,
                "role": success.role.as_str(),
                "provider": "oidc",
                "display_name": success.display_name,
                "email": success.email,
            }))).into_response()
        }
        Err(e) => {
            let status = match e {
                SsoError::TokenExpired
                | SsoError::InvalidIssuer
                | SsoError::InvalidAudience
                | SsoError::InvalidToken(_) => StatusCode::UNAUTHORIZED,
                _ => StatusCode::BAD_REQUEST,
            };

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "admin",
                duration_ms,
                e.error_code(),
            );

            (status, Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            })).into_response()
        }
    }
}

/// `POST /auth/sso/saml/login` & `GET /auth/sso/saml/login` — generate SAML 2.0 AuthnRequest.
pub async fn saml_login_handler(
    State(server): State<Arc<ControlServer>>,
    Query(query_req): Query<SamlLoginRequest>,
    body: Option<Json<SamlLoginRequest>>,
) -> Response {
    let endpoint = "auth.sso.saml_login";
    let start_time = Instant::now();

    let req = body.map(|Json(b)| b).unwrap_or(query_req);
    let audit_params = json!({
        "provider": "saml",
        "has_relay_state": req.relay_state.is_some(),
    });

    match server.sso.saml_login(&req) {
        Ok(res) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "read",
                duration_ms,
                "ok",
            );
            (StatusCode::OK, Json(json!({
                "success": true,
                "sso_url": res.sso_url,
                "saml_request": res.saml_request,
                "relay_state": res.relay_state,
            }))).into_response()
        }
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "read",
                duration_ms,
                e.error_code(),
            );
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            })).into_response()
        }
    }
}

/// `POST /auth/sso/saml/callback` — parse and validate SAML assertion XML and issue session token.
pub async fn saml_callback_handler(
    State(server): State<Arc<ControlServer>>,
    body: Option<Json<Value>>,
) -> Response {
    let endpoint = "auth.sso.saml_callback";
    let start_time = Instant::now();

    let raw_body = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let req: SamlCallbackRequest = serde_json::from_value(raw_body).unwrap_or_default();

    // Redact saml_response in audit params
    let audit_params = json!({
        "has_saml_response": !req.saml_response.is_empty(),
        "relay_state": req.relay_state,
    });

    match server.sso.saml_callback(&req) {
        Ok(success) => {
            let session_id = format!("sso-saml-{}", Uuid::new_v4());
            let actor_name = format!("human:{}", success.subject);

            // Issue session token with mapped role
            let token = server.auth.issue_session_token_with_role(
                &session_id,
                &actor_name,
                success.role,
                false,
            );
            let _session = server.sessions.get_or_create(&session_id, &actor_name);

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                &session_id,
                &actor_name,
                endpoint,
                &audit_params,
                "admin",
                duration_ms,
                "ok",
            );

            (StatusCode::OK, Json(json!({
                "success": true,
                "token": token,
                "session_id": session_id,
                "actor": actor_name,
                "role": success.role.as_str(),
                "provider": "saml",
                "display_name": success.display_name,
                "email": success.email,
            }))).into_response()
        }
        Err(e) => {
            let status = match e {
                SsoError::SamlAssertionExpired
                | SsoError::SamlAssertionNotYetValid
                | SsoError::InvalidIssuer
                | SsoError::InvalidAudience => StatusCode::UNAUTHORIZED,
                _ => StatusCode::BAD_REQUEST,
            };

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let _ = server.audit.lock().record(
                "unauthenticated",
                "anonymous",
                endpoint,
                &audit_params,
                "admin",
                duration_ms,
                e.error_code(),
            );

            (status, Json(ErrorResponse {
                success: false,
                error: e.to_string(),
                code: Some(e.error_code().to_string()),
            })).into_response()
        }
    }
}
