// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Enterprise Single Sign-On (SSO) subsystem supporting OIDC and SAML 2.0.

pub mod oidc;
pub mod rbac;
pub mod saml;

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use oidc::{OidcCallbackRequest, OidcConfig, OidcLoginRequest, OidcLoginResponse};
pub use rbac::Role;
pub use saml::{SamlCallbackRequest, SamlConfig, SamlLoginRequest, SamlLoginResponse};

/// Error types emitted during SSO authentication and token verification.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum SsoError {
    #[error("MissingSubject: subject / NameID is required")]
    MissingSubject,
    #[error("InvalidIssuer: token issuer does not match configuration")]
    InvalidIssuer,
    #[error("InvalidAudience: token audience does not match client ID")]
    InvalidAudience,
    #[error("TokenExpired: OIDC token has expired")]
    TokenExpired,
    #[error("InvalidToken: {0}")]
    InvalidToken(String),
    #[error("InvalidSamlResponse: invalid base64 or unreadable SAML response")]
    InvalidSamlResponse,
    #[error("InvalidSamlXml: malformed SAML assertion XML")]
    InvalidSamlXml,
    #[error("SamlAssertionExpired: SAML assertion conditions expired")]
    SamlAssertionExpired,
    #[error("SamlAssertionNotYetValid: SAML assertion conditions not yet valid")]
    SamlAssertionNotYetValid,
    #[error("ProviderNotConfigured: {0}")]
    ProviderNotConfigured(String),
    #[error("Internal: {0}")]
    Internal(String),
}

impl SsoError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::MissingSubject => "MissingSubject",
            Self::InvalidIssuer => "InvalidIssuer",
            Self::InvalidAudience => "InvalidAudience",
            Self::TokenExpired => "TokenExpired",
            Self::InvalidToken(_) => "InvalidToken",
            Self::InvalidSamlResponse => "InvalidSamlResponse",
            Self::InvalidSamlXml => "InvalidSamlXml",
            Self::SamlAssertionExpired => "SamlAssertionExpired",
            Self::SamlAssertionNotYetValid => "SamlAssertionNotYetValid",
            Self::ProviderNotConfigured(_) => "ProviderNotConfigured",
            Self::Internal(_) => "Internal",
        }
    }
}

/// Result of successful SSO authentication.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SsoAuthSuccess {
    pub subject: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Role,
    pub provider: String,
}

/// Overall SSO configuration for control server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SsoConfig {
    pub enabled: bool,
    pub oidc: Option<OidcConfig>,
    pub saml: Option<SamlConfig>,
    pub group_mappings: Option<HashMap<String, Role>>,
}

/// Provider status summary returned by `/auth/sso/status`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SsoStatusResponse {
    pub enabled: bool,
    pub oidc: ProviderStatus,
    pub saml: ProviderStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub configured: bool,
    pub provider_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

/// SSO Subsystem Manager.
#[derive(Clone)]
pub struct SsoManager {
    config: Arc<RwLock<SsoConfig>>,
}

impl SsoManager {
    pub fn new(config: SsoConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub fn config(&self) -> SsoConfig {
        self.config.read().clone()
    }

    pub fn set_config(&self, config: SsoConfig) {
        *self.config.write() = config;
    }

    pub fn configure_oidc(&self, oidc: OidcConfig) {
        let mut cfg = self.config.write();
        cfg.enabled = true;
        cfg.oidc = Some(oidc);
    }

    pub fn configure_saml(&self, saml: SamlConfig) {
        let mut cfg = self.config.write();
        cfg.enabled = true;
        cfg.saml = Some(saml);
    }

    pub fn status(&self) -> SsoStatusResponse {
        let cfg = self.config.read();
        SsoStatusResponse {
            enabled: cfg.enabled,
            oidc: ProviderStatus {
                configured: cfg.oidc.is_some(),
                provider_name: "oidc".to_string(),
                issuer: cfg.oidc.as_ref().map(|o| o.issuer.clone()),
                entity_id: None,
            },
            saml: ProviderStatus {
                configured: cfg.saml.is_some(),
                provider_name: "saml".to_string(),
                issuer: None,
                entity_id: cfg.saml.as_ref().map(|s| s.idp_entity_id.clone()),
            },
        }
    }

    pub fn oidc_login(&self, req: &OidcLoginRequest) -> Result<OidcLoginResponse, SsoError> {
        let cfg = self.config.read();
        let oidc = cfg
            .oidc
            .as_ref()
            .ok_or_else(|| SsoError::ProviderNotConfigured("OIDC is not configured".into()))?;
        oidc::generate_authorization_url(oidc, req)
    }

    pub fn oidc_callback(&self, req: &OidcCallbackRequest) -> Result<SsoAuthSuccess, SsoError> {
        let cfg = self.config.read();
        let oidc = cfg
            .oidc
            .as_ref()
            .ok_or_else(|| SsoError::ProviderNotConfigured("OIDC is not configured".into()))?;
        oidc::verify_oidc_claims(oidc, req, cfg.group_mappings.as_ref())
    }

    pub fn saml_login(&self, req: &SamlLoginRequest) -> Result<SamlLoginResponse, SsoError> {
        let cfg = self.config.read();
        let saml = cfg
            .saml
            .as_ref()
            .ok_or_else(|| SsoError::ProviderNotConfigured("SAML is not configured".into()))?;
        saml::generate_authn_request(saml, req)
    }

    pub fn saml_callback(&self, req: &SamlCallbackRequest) -> Result<SsoAuthSuccess, SsoError> {
        let cfg = self.config.read();
        let saml = cfg
            .saml
            .as_ref()
            .ok_or_else(|| SsoError::ProviderNotConfigured("SAML is not configured".into()))?;
        saml::parse_and_validate_saml_response(saml, req, cfg.group_mappings.as_ref())
    }
}
