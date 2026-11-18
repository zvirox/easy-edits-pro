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

pub use crate::sso::rbac::Role;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum AuthError {
    #[error("MissingAuthToken: authentication token is required")]
    MissingAuthToken,
    #[error("InvalidAuthToken: invalid authentication token")]
    InvalidAuthToken,
    #[error("AdminRequired: this endpoint requires the daemon master token")]
    AdminRequired,
    #[error("TokenIssuanceDisabled: the daemon was started without a master token")]
    TokenIssuanceDisabled,
    #[error("InvalidIdentity: {0}")]
    InvalidIdentity(String),
}

impl AuthError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::MissingAuthToken => "MissingAuthToken",
            Self::InvalidAuthToken => "InvalidAuthToken",
            Self::AdminRequired => "AdminRequired",
            Self::TokenIssuanceDisabled => "TokenIssuanceDisabled",
            Self::InvalidIdentity(_) => "InvalidIdentity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthClaims {
    pub session_id: String,
    pub actor: String,
    pub issued_at_unix_ms: u64,
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub is_agent: bool,
}

impl AuthClaims {
    pub fn is_agent(&self) -> bool {
        self.is_agent
    }

    pub fn can_approve_plan(&self) -> bool {
        !self.is_agent() && (self.role == Role::Approver || self.role == Role::Admin)
    }

    pub fn can_approve_delivery(&self) -> bool {
        !self.is_agent() && (self.role == Role::Approver || self.role == Role::Admin)
    }

    pub fn can_submit_jobs(&self) -> bool {
        self.role == Role::Requester || self.role == Role::Approver || self.role == Role::Admin
    }
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
        let is_agent = actor.starts_with("agent");
        let role = if is_agent {
            Role::Requester
        } else {
            Role::Approver
        };
        self.issue_session_token_with_role(session_id, actor, role, is_agent)
    }

    pub fn issue_session_token_with_role(
        &self,
        session_id: &str,
        actor: &str,
        role: Role,
        is_agent: bool,
    ) -> String {
        let token = format!("saturn-{}", Uuid::new_v4());
        let claims = AuthClaims {
            session_id: session_id.to_string(),
            actor: actor.to_string(),
            issued_at_unix_ms: now_ms(),
            role,
            is_agent,
        };
        self.session_tokens.write().insert(token.clone(), claims);
        token
    }

    pub fn register_token(&self, token: &str, session_id: &str, actor: &str) {
        let is_agent = actor.starts_with("agent");
        let role = if is_agent {
            Role::Requester
        } else {
            Role::Approver
        };
        self.register_token_with_role(token, session_id, actor, role, is_agent);
    }

    pub fn register_token_with_role(
        &self,
        token: &str,
        session_id: &str,
        actor: &str,
        role: Role,
        is_agent: bool,
    ) {
        let claims = AuthClaims {
            session_id: session_id.to_string(),
            actor: actor.to_string(),
            issued_at_unix_ms: now_ms(),
            role,
            is_agent,
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
            if constant_time_eq(token.as_bytes(), master.trim().as_bytes()) {
                return Ok(AuthClaims {
                    session_id: "master-session".to_string(),
                    actor: "daemon-admin".to_string(),
                    issued_at_unix_ms: now_ms(),
                    role: Role::Admin,
                    is_agent: false,
                });
            }
        }

        if let Some(claims) = self.session_tokens.read().get(token) {
            return Ok(claims.clone());
        }

        Err(AuthError::InvalidAuthToken)
    }

    /// True only when `token_opt` is the configured master token (constant-time compare).
    pub fn is_master(&self, token_opt: Option<&str>) -> bool {
        let (Some(raw), Some(master)) = (token_opt, self.master_token.as_deref()) else {
            return false;
        };
        let token = extract_bearer_token(raw);
        !token.is_empty() && constant_time_eq(token.as_bytes(), master.trim().as_bytes())
    }

    /// Gate for admin-only endpoints such as `POST /auth/token`.
    pub fn require_master(&self, token_opt: Option<&str>) -> Result<(), AuthError> {
        if self.master_token.is_none() {
            return Err(AuthError::TokenIssuanceDisabled);
        }
        match token_opt {
            None => Err(AuthError::MissingAuthToken),
            Some(t) if t.trim().is_empty() => Err(AuthError::MissingAuthToken),
            Some(_) if self.is_master(token_opt) => Ok(()),
            Some(_) => Err(AuthError::AdminRequired),
        }
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

/// Compare two byte strings without early exit on the first differing byte.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u32;
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= u32::from(x ^ y);
    }
    diff == 0
}

/// Session ids and actor names come from the caller of `/auth/token`; restrict them to a
/// safe charset and reserve the identities the daemon itself uses for the master token.
pub fn validate_identity(session_id: &str, actor: &str) -> Result<(), AuthError> {
    let ok = |v: &str| {
        !v.is_empty()
            && v.len() <= 64
            && v.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '@'))
    };
    if !ok(session_id) {
        return Err(AuthError::InvalidIdentity("session_id must be 1-64 chars of [A-Za-z0-9-_.:@]".into()));
    }
    if !ok(actor) {
        return Err(AuthError::InvalidIdentity("actor must be 1-64 chars of [A-Za-z0-9-_.:@]".into()));
    }
    if session_id == "master-session" || actor == "daemon-admin" {
        return Err(AuthError::InvalidIdentity("reserved identity".into()));
    }
    Ok(())
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
    fn require_master_rejects_session_tokens_and_unset_master() {
        let auth = AuthManager::new(Some("secret-master".into()));
        let session = auth.issue_session_token("s1", "agent");
        assert_eq!(auth.require_master(None), Err(AuthError::MissingAuthToken));
        assert_eq!(auth.require_master(Some(&session)), Err(AuthError::AdminRequired));
        assert_eq!(auth.require_master(Some("Bearer secret-master")), Ok(()));
        let open = AuthManager::new(None);
        assert_eq!(open.require_master(Some("anything")), Err(AuthError::TokenIssuanceDisabled));
    }

    #[test]
    fn identities_are_validated_and_reserved_names_blocked() {
        assert!(validate_identity("job-1", "human:reviewer").is_ok());
        assert!(validate_identity("master-session", "agent").is_err());
        assert!(validate_identity("s", "daemon-admin").is_err());
        assert!(validate_identity("", "agent").is_err());
        assert!(validate_identity("a b", "agent").is_err());
        assert!(validate_identity(&"x".repeat(65), "agent").is_err());
    }

    #[test]
    fn constant_time_eq_matches_normal_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
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
