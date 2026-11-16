// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Bring-Your-Own (BYO) model endpoint configuration and validation (TS9).
//!
//! Supports local endpoints (Ollama, vLLM, llama.cpp on localhost or Unix socket)
//! and cloud endpoints (Anthropic, OpenAI, Gemini, OpenRouter).
//! Credentials are redacted before appearing in audit logs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from BYO endpoint validation.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum EndpointError {
    #[error("InvalidEndpoint: {message}")]
    InvalidEndpoint { message: String },
    #[error("UnsupportedProvider: '{provider}' is not a recognised model provider")]
    UnsupportedProvider { provider: String },
    #[error("MissingCredential: {message}")]
    MissingCredential { message: String },
}

impl EndpointError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidEndpoint { .. } => "InvalidEndpoint",
            Self::UnsupportedProvider { .. } => "UnsupportedProvider",
            Self::MissingCredential { .. } => "MissingCredential",
        }
    }
}

/// Local inference server variants (no cloud credential required).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalEndpoint {
    /// Ollama REST endpoint (default: http://localhost:11434/v1).
    Ollama { base_url: String, model: String },
    /// vLLM OpenAI-compatible endpoint.
    Vllm { base_url: String, model: String },
    /// llama.cpp server OpenAI-compatible endpoint.
    LlamaCpp { base_url: String, model: String },
    /// Generic OpenAI-compatible endpoint on localhost.
    GenericLocal { base_url: String, model: String },
}

impl LocalEndpoint {
    /// Return the base URL for the endpoint.
    pub fn base_url(&self) -> &str {
        match self {
            Self::Ollama { base_url, .. }
            | Self::Vllm { base_url, .. }
            | Self::LlamaCpp { base_url, .. }
            | Self::GenericLocal { base_url, .. } => base_url,
        }
    }

    pub fn model(&self) -> &str {
        match self {
            Self::Ollama { model, .. }
            | Self::Vllm { model, .. }
            | Self::LlamaCpp { model, .. }
            | Self::GenericLocal { model, .. } => model,
        }
    }

    /// Validate that the base_url is local (localhost, 127.x, ::1, or Unix socket path).
    pub fn validate(&self) -> Result<(), EndpointError> {
        let url = self.base_url();
        let is_local = url.starts_with("http://localhost")
            || url.starts_with("http://127.")
            || url.starts_with("http://[::1]")
            || url.starts_with("unix://")
            || url.starts_with('/'); // Unix socket path
        if !is_local {
            return Err(EndpointError::InvalidEndpoint {
                message: format!(
                    "Local endpoint base_url must be localhost or a Unix socket path, got '{url}'"
                ),
            });
        }
        Ok(())
    }
}

/// Cloud provider variants (require an API key / credential).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum CloudEndpoint {
    Anthropic { model: String, api_key: String },
    OpenAi { model: String, api_key: String },
    Gemini { model: String, api_key: String },
    OpenRouter { model: String, api_key: String },
}

impl CloudEndpoint {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Anthropic { .. } => "anthropic",
            Self::OpenAi { .. } => "openai",
            Self::Gemini { .. } => "gemini",
            Self::OpenRouter { .. } => "openrouter",
        }
    }

    pub fn model(&self) -> &str {
        match self {
            Self::Anthropic { model, .. }
            | Self::OpenAi { model, .. }
            | Self::Gemini { model, .. }
            | Self::OpenRouter { model, .. } => model,
        }
    }

    /// Validate that the api_key is non-empty.
    pub fn validate(&self) -> Result<(), EndpointError> {
        let key = match self {
            Self::Anthropic { api_key, .. }
            | Self::OpenAi { api_key, .. }
            | Self::Gemini { api_key, .. }
            | Self::OpenRouter { api_key, .. } => api_key,
        };
        if key.trim().is_empty() {
            return Err(EndpointError::MissingCredential {
                message: format!(
                    "api_key for provider '{}' must not be empty",
                    self.provider_name()
                ),
            });
        }
        Ok(())
    }

    /// Return a redacted version safe for audit logs (api_key replaced).
    pub fn redacted(&self) -> CloudEndpointRedacted {
        CloudEndpointRedacted {
            provider: self.provider_name().to_string(),
            model: self.model().to_string(),
            api_key: "[REDACTED]".to_string(),
        }
    }
}

/// Audit-safe representation of a cloud endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEndpointRedacted {
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

/// A BYO model endpoint: either local or cloud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ByoEndpoint {
    Local(LocalEndpoint),
    Cloud(CloudEndpoint),
}

impl ByoEndpoint {
    /// Validate the endpoint configuration.
    pub fn validate(&self) -> Result<(), EndpointError> {
        match self {
            Self::Local(ep) => ep.validate(),
            Self::Cloud(ep) => ep.validate(),
        }
    }

    /// Return an audit-safe JSON value with secrets redacted.
    pub fn audit_safe(&self) -> serde_json::Value {
        match self {
            Self::Local(ep) => serde_json::json!({
                "type": "local",
                "base_url": ep.base_url(),
                "model": ep.model(),
            }),
            Self::Cloud(ep) => {
                let r = ep.redacted();
                serde_json::json!({
                    "type": "cloud",
                    "provider": r.provider,
                    "model": r.model,
                    "api_key": r.api_key,
                })
            }
        }
    }

    /// Parse a provider string and build a cloud endpoint.
    pub fn from_cloud_parts(
        provider: &str,
        model: String,
        api_key: String,
    ) -> Result<Self, EndpointError> {
        let ep = match provider.to_lowercase().as_str() {
            "anthropic" => CloudEndpoint::Anthropic { model, api_key },
            "openai" => CloudEndpoint::OpenAi { model, api_key },
            "gemini" => CloudEndpoint::Gemini { model, api_key },
            "openrouter" => CloudEndpoint::OpenRouter { model, api_key },
            other => {
                return Err(EndpointError::UnsupportedProvider {
                    provider: other.to_string(),
                });
            }
        };
        ep.validate()?;
        Ok(Self::Cloud(ep))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ollama_validates_localhost() {
        let ep = ByoEndpoint::Local(LocalEndpoint::Ollama {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3".to_string(),
        });
        assert!(ep.validate().is_ok());
    }

    #[test]
    fn local_endpoint_rejects_remote_url() {
        let ep = LocalEndpoint::GenericLocal {
            base_url: "http://example.com/v1".to_string(),
            model: "x".to_string(),
        };
        let err = ep.validate().unwrap_err();
        assert_eq!(err.error_code(), "InvalidEndpoint");
    }

    #[test]
    fn cloud_endpoint_rejects_empty_key() {
        let ep = CloudEndpoint::Anthropic {
            model: "claude-3".to_string(),
            api_key: "".to_string(),
        };
        let err = ep.validate().unwrap_err();
        assert_eq!(err.error_code(), "MissingCredential");
    }

    #[test]
    fn cloud_endpoint_audit_safe_redacts_key() {
        let ep = ByoEndpoint::Cloud(CloudEndpoint::OpenAi {
            model: "gpt-4o".to_string(),
            api_key: "sk-secret123".to_string(),
        });
        let safe = ep.audit_safe();
        assert_eq!(safe["api_key"], "[REDACTED]");
        assert_eq!(safe["model"], "gpt-4o");
    }

    #[test]
    fn from_cloud_parts_unsupported_provider() {
        let err = ByoEndpoint::from_cloud_parts("cohere", "m".to_string(), "k".to_string())
            .unwrap_err();
        assert_eq!(err.error_code(), "UnsupportedProvider");
    }

    #[test]
    fn from_cloud_parts_valid() {
        let ep = ByoEndpoint::from_cloud_parts("gemini", "gemini-pro".to_string(), "my-key".to_string())
            .unwrap();
        assert!(matches!(ep, ByoEndpoint::Cloud(CloudEndpoint::Gemini { .. })));
    }
}
