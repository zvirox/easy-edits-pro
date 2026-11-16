// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Authentication management and token verification for saturn-control daemon (AF9).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum AuthError {
    #[error("MissingAuthToken: authentication token is required")]
    MissingAuthToken,
    #[error("InvalidAuthToken: invalid authentication token")]
    InvalidAuthToken,
}

impl AuthError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::MissingAuthToken => "MissingAuthToken",
            Self::InvalidAuthToken => "InvalidAuthToken",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthClaims {
    pub session_id: String,
    pub actor: String,
    pub issued_at_unix_ms: u64,
}

#[derive(Clone)]
pub struct AuthManager {
    master_token: Option<String>,
    session_tokens: Arc<RwLock<HashMap<String, AuthClaims>>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl AuthManager {
    pub fn new(master_token: Option<String>) -> Self {
        Self {
            master_token,
            session_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn master_token(&self) -> Option<&str> {
        self.master_token.as_deref()
    }

    pub fn issue_session_token(&self, session_id: &str, actor: &str) -> String {
        let token = format!("saturn-{}", Uuid::new_v4());
        let claims = AuthClaims {
            session_id: session_id.to_string(),
            actor: actor.to_string(),
            issued_at_unix_ms: now_ms(),
        };
        self.session_tokens.write().insert(token.clone(), claims);
        token
    }

    pub fn register_token(&self, token: &str, session_id: &str, actor: &str) {
        let claims = AuthClaims {
            session_id: session_id.to_string(),
            actor: actor.to_string(),
            issued_at_unix_ms: now_ms(),
        };
        self.session_tokens.write().insert(token.to_string(), claims);
    }

    pub fn validate_token(&self, token_opt: Option<&str>) -> Result<AuthClaims, AuthError> {
        let raw = match token_opt {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => return Err(AuthError::MissingAuthToken),
        };

        let token = extract_bearer_token(raw);

        if let Some(ref master) = self.master_token {
            if token == master.trim() {
                return Ok(AuthClaims {
                    session_id: "master-session".to_string(),
                    actor: "daemon-admin".to_string(),
                    issued_at_unix_ms: now_ms(),
                });
            }
        }

        if let Some(claims) = self.session_tokens.read().get(token) {
            return Ok(claims.clone());
        }

        Err(AuthError::InvalidAuthToken)
    }

    pub fn revoke_token(&self, token: &str) -> bool {
        let normalized = extract_bearer_token(token.trim());
        self.session_tokens.write().remove(normalized).is_some()
    }

    pub fn revoke_session(&self, session_id: &str) {
        let mut tokens = self.session_tokens.write();
        tokens.retain(|_, claims| claims.session_id != session_id);
    }
}

pub fn extract_bearer_token(input: &str) -> &str {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("Bearer ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("bearer ") {
        rest.trim()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_returns_typed_error() {
        let auth = AuthManager::new(Some("secret-master".into()));
        assert_eq!(auth.validate_token(None), Err(AuthError::MissingAuthToken));
        assert_eq!(auth.validate_token(Some("")), Err(AuthError::MissingAuthToken));
        assert_eq!(auth.validate_token(Some("   ")), Err(AuthError::MissingAuthToken));
    }

    #[test]
    fn invalid_token_returns_typed_error() {
        let auth = AuthManager::new(Some("secret-master".into()));
        assert_eq!(
            auth.validate_token(Some("wrong-token")),
            Err(AuthError::InvalidAuthToken)
        );
        assert_eq!(
            auth.validate_token(Some("Bearer wrong-token")),
            Err(AuthError::InvalidAuthToken)
        );
    }

    #[test]
    fn valid_master_token_succeeds() {
        let auth = AuthManager::new(Some("secret-master".into()));
        let claims = auth.validate_token(Some("secret-master")).unwrap();
        assert_eq!(claims.actor, "daemon-admin");

        let claims_bearer = auth.validate_token(Some("Bearer secret-master")).unwrap();
        assert_eq!(claims_bearer.actor, "daemon-admin");
    }

    #[test]
    fn session_token_issue_validate_and_revoke() {
        let auth = AuthManager::new(None);
        let token = auth.issue_session_token("session-123", "agent-alpha");
        let claims = auth.validate_token(Some(&token)).unwrap();
        assert_eq!(claims.session_id, "session-123");
        assert_eq!(claims.actor, "agent-alpha");

        assert!(auth.revoke_token(&token));
        assert_eq!(auth.validate_token(Some(&token)), Err(AuthError::InvalidAuthToken));
    }
}
