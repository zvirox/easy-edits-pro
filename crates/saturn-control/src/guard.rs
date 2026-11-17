// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Browser-origin guard for the control server.
//!
//! The daemon is meant for non-browser clients and for a same-origin Web Desk. Any request
//! that carries an `Origin` header must come from the same authority as the `Host` header;
//! this blocks cross-site WebSocket hijacking and cross-site form posts against a daemon
//! bound to localhost. Requests with no `Origin` (curl, agents, UDS bridges) are unaffected.

use axum::{
    Json,
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::handlers::ErrorResponse;

/// Authority (`host[:port]`) portion of an `Origin` value, lower-cased.
fn origin_authority(origin: &str) -> Option<String> {
    let rest = origin.split_once("://")?.1;
    let authority = rest.split('/').next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Some(authority.to_ascii_lowercase())
}

/// True when the request is not a cross-origin browser request.
pub fn origin_allowed(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let (Some(auth), Some(host)) = (origin_authority(origin), host) else {
        return false;
    };
    auth == host.trim().to_ascii_lowercase()
}

pub async fn origin_guard(request: Request, next: Next) -> Response {
    let origin = request.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let host = request.headers().get(header::HOST).and_then(|v| v.to_str().ok());
    if !origin_allowed(origin, host) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                success: false,
                error: "CrossOriginDenied: cross-origin requests are not allowed".into(),
                code: Some("CrossOriginDenied".into()),
            }),
        )
            .into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_and_non_browser_requests_pass() {
        assert!(origin_allowed(None, Some("127.0.0.1:8080")));
        assert!(origin_allowed(Some("http://127.0.0.1:8080"), Some("127.0.0.1:8080")));
        assert!(origin_allowed(Some("http://LOCALHOST:8080"), Some("localhost:8080")));
    }

    #[test]
    fn cross_origin_null_and_malformed_are_denied() {
        assert!(!origin_allowed(Some("https://evil.example"), Some("127.0.0.1:8080")));
        assert!(!origin_allowed(Some("null"), Some("127.0.0.1:8080")));
        assert!(!origin_allowed(Some("http://a@127.0.0.1:8080"), Some("127.0.0.1:8080")));
        assert!(!origin_allowed(Some("http://127.0.0.1:8080"), None));
    }
}
