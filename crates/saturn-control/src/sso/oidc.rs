// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! OpenID Connect (OIDC) authentication provider for Enterprise SSO.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use super::rbac::{Role, map_claims_to_role};
use super::{SsoAuthSuccess, SsoError};

/// OpenID Connect provider configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub jwks_url: String,
    pub redirect_uri: String,
    #[serde(default = "default_allow_simulated_claims")]
    pub allow_simulated_claims: bool,
}

fn default_allow_simulated_claims() -> bool {
    true
}

impl OidcConfig {
    pub fn new(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        jwks_url: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            jwks_url: jwks_url.into(),
            redirect_uri: redirect_uri.into(),
            allow_simulated_claims: true,
        }
    }

    pub fn with_simulated_claims(mut self, allow: bool) -> Self {
        self.allow_simulated_claims = allow;
        self
    }
}

/// Request to initiate OIDC login.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OidcLoginRequest {
    pub provider: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub scope: Option<String>,
}

/// Response returned from OIDC login initialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcLoginResponse {
    pub authorization_url: String,
    pub state: String,
    pub nonce: String,
}

/// Request received at the OIDC callback endpoint.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OidcCallbackRequest {
    pub code: Option<String>,
    pub state: Option<String>,
    pub id_token: Option<String>,
    pub claims: Option<Value>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generate authorization redirect URL for OIDC provider.
pub fn generate_authorization_url(
    config: &OidcConfig,
    req: &OidcLoginRequest,
) -> Result<OidcLoginResponse, SsoError> {
    let state = req.state.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let nonce = Uuid::new_v4().to_string();
    let redirect_uri = req
        .redirect_uri
        .as_deref()
        .unwrap_or(&config.redirect_uri);

    let auth_endpoint = format!("{}/protocol/openid-connect/auth", config.issuer.trim_end_matches('/'));
    let mut url = Url::parse(&auth_endpoint)
        .or_else(|_| Url::parse(&format!("{}/authorize", config.issuer.trim_end_matches('/'))))
        .map_err(|e| SsoError::Internal(format!("Invalid issuer URL: {e}")))?;

    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("response_type", "code")
        .append_pair("scope", req.scope.as_deref().unwrap_or("openid profile email groups"))
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", &state)
        .append_pair("nonce", &nonce);

    Ok(OidcLoginResponse {
        authorization_url: url.to_string(),
        state,
        nonce,
    })
}

/// Decode base64url JWT payload string into JSON `Value`.
pub fn parse_jwt_payload(jwt: &str) -> Result<Value, SsoError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(SsoError::InvalidToken(
            "JWT must have 3 parts separated by dots".to_string(),
        ));
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| {
            // Fallback with standard base64 if url-safe without pad fails
            use base64::engine::general_purpose::STANDARD;
            let mut padded = parts[1].to_string();
            while padded.len() % 4 != 0 {
                padded.push('=');
            }
            STANDARD.decode(padded)
        })
        .map_err(|e| SsoError::InvalidToken(format!("Failed to base64 decode JWT payload: {e}")))?;

    serde_json::from_slice(&payload_bytes)
        .map_err(|e| SsoError::InvalidToken(format!("Invalid JSON in JWT payload: {e}")))
}

/// Helper to create a simulated encoded JWT token for testing.
pub fn create_simulated_jwt(claims: &Value) -> String {
    let header = serde_json::json!({
        "alg": "HS256",
        "typ": "JWT"
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string());
    let payload_b64 = URL_SAFE_NO_PAD.encode(claims.to_string());
    let signature_b64 = URL_SAFE_NO_PAD.encode("simulated-signature-bytes");
    format!("{header_b64}.{payload_b64}.{signature_b64}")
}

/// Verify OIDC claims against provider configuration and extract user identity.
pub fn verify_oidc_claims(
    config: &OidcConfig,
    req: &OidcCallbackRequest,
    custom_mapping: Option<&HashMap<String, Role>>,
) -> Result<SsoAuthSuccess, SsoError> {
    // Resolve claims: either from `id_token` or simulated `claims` if permitted
    let claims = match (&req.id_token, &req.claims) {
        (Some(token), _) => parse_jwt_payload(token)?,
        (None, Some(c)) => {
            if !config.allow_simulated_claims && !cfg!(test) {
                return Err(SsoError::InvalidToken(
                    "Simulated claims disabled in production; id_token required".into(),
                ));
            }
            c.clone()
        }
        (None, None) => {
            return Err(SsoError::MissingSubject);
        }
    };

    // 1. Verify `sub` claim
    let sub = claims
        .get("sub")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or(SsoError::MissingSubject)?;

    // 2. Verify `iss` claim
    if let Some(iss) = claims.get("iss").and_then(Value::as_str) {
        let expected_clean = config.issuer.trim_end_matches('/');
        let actual_clean = iss.trim_end_matches('/');
        if expected_clean != actual_clean {
            return Err(SsoError::InvalidIssuer);
        }
    } else {
        return Err(SsoError::InvalidIssuer);
    }

    // 3. Verify `aud` claim
    let aud_matches = match claims.get("aud") {
        Some(Value::String(s)) => s == &config.client_id,
        Some(Value::Array(arr)) => arr.iter().any(|item| item.as_str() == Some(&config.client_id)),
        _ => false,
    };
    if !aud_matches {
        return Err(SsoError::InvalidAudience);
    }

    let now = now_secs();

    // 4. Verify `exp` claim
    if let Some(exp) = claims.get("exp").and_then(Value::as_u64) {
        if exp < now {
            return Err(SsoError::TokenExpired);
        }
    }

    // 5. Verify `nbf` claim (not before)
    if let Some(nbf) = claims.get("nbf").and_then(Value::as_u64) {
        if nbf > now {
            return Err(SsoError::InvalidToken("Token not yet valid (nbf)".into()));
        }
    }

    // 6. Verify `iat` claim (issued at, allowing 5m clock skew)
    if let Some(iat) = claims.get("iat").and_then(Value::as_u64) {
        if iat > now + 300 {
            return Err(SsoError::InvalidToken("Token issued in future (iat)".into()));
        }
    }

    // 5. Extract role
    let role = map_claims_to_role(&claims, custom_mapping);

    // 6. Extract display name and email
    let display_name = claims
        .get("name")
        .or_else(|| claims.get("preferred_username"))
        .or_else(|| claims.get("nickname"))
        .and_then(Value::as_str)
        .map(String::from);

    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .map(String::from);

    Ok(SsoAuthSuccess {
        subject: sub.to_string(),
        display_name,
        email,
        role,
        provider: "oidc".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_config() -> OidcConfig {
        OidcConfig::new(
            "https://idp.example.com",
            "saturn-client-1",
            "client-secret-123",
            "https://idp.example.com/.well-known/jwks.json",
            "http://localhost:3000/callback",
        )
    }

    #[test]
    fn test_generate_auth_url() {
        let cfg = sample_config();
        let req = OidcLoginRequest {
            state: Some("state-xyz".into()),
            ..Default::default()
        };
        let res = generate_authorization_url(&cfg, &req).unwrap();
        assert!(res.authorization_url.contains("client_id=saturn-client-1"));
        assert!(res.authorization_url.contains("state=state-xyz"));
        assert_eq!(res.state, "state-xyz");
    }

    #[test]
    fn test_valid_simulated_claims() {
        let cfg = sample_config();
        let exp = now_secs() + 3600;
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://idp.example.com",
            "aud": "saturn-client-1",
            "exp": exp,
            "groups": ["video-approvers"]
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        let success = verify_oidc_claims(&cfg, &req, None).unwrap();
        assert_eq!(success.subject, "usr_approver");
        assert_eq!(success.role, Role::Approver);
        assert_eq!(success.provider, "oidc");
    }

    #[test]
    fn test_expired_token_rejected() {
        let cfg = sample_config();
        let exp = now_secs() - 100;
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://idp.example.com",
            "aud": "saturn-client-1",
            "exp": exp,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        assert_eq!(verify_oidc_claims(&cfg, &req, None), Err(SsoError::TokenExpired));
    }

    #[test]
    fn test_invalid_issuer_rejected() {
        let cfg = sample_config();
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://wrong.idp.com",
            "aud": "saturn-client-1",
            "exp": now_secs() + 3600,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        assert_eq!(verify_oidc_claims(&cfg, &req, None), Err(SsoError::InvalidIssuer));
    }

    #[test]
    fn test_invalid_audience_rejected() {
        let cfg = sample_config();
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://idp.example.com",
            "aud": "wrong-client",
            "exp": now_secs() + 3600,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        assert_eq!(verify_oidc_claims(&cfg, &req, None), Err(SsoError::InvalidAudience));
    }

    #[test]
    fn test_missing_subject_rejected() {
        let cfg = sample_config();
        let claims = json!({
            "iss": "https://idp.example.com",
            "aud": "saturn-client-1",
            "exp": now_secs() + 3600,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        assert_eq!(verify_oidc_claims(&cfg, &req, None), Err(SsoError::MissingSubject));
    }

    #[test]
    fn test_future_nbf_rejected() {
        let cfg = sample_config();
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://idp.example.com",
            "aud": "saturn-client-1",
            "exp": now_secs() + 7200,
            "nbf": now_secs() + 3600,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        let res = verify_oidc_claims(&cfg, &req, None);
        assert!(matches!(res, Err(SsoError::InvalidToken(msg)) if msg.contains("nbf")));
    }

    #[test]
    fn test_future_iat_rejected() {
        let cfg = sample_config();
        let claims = json!({
            "sub": "usr_approver",
            "iss": "https://idp.example.com",
            "aud": "saturn-client-1",
            "exp": now_secs() + 7200,
            "iat": now_secs() + 3600,
        });
        let req = OidcCallbackRequest {
            claims: Some(claims),
            ..Default::default()
        };
        let res = verify_oidc_claims(&cfg, &req, None);
        assert!(matches!(res, Err(SsoError::InvalidToken(msg)) if msg.contains("iat")));
    }
}
