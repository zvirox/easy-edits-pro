// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! SAML 2.0 authentication provider and roxmltree assertion parsing for Enterprise SSO.

use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::rbac::{Role, map_saml_attributes_to_role};
use super::{SsoAuthSuccess, SsoError};

/// SAML 2.0 Identity Provider configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamlConfig {
    pub idp_entity_id: String,
    pub idp_sso_url: String,
    pub idp_certificate: String,
    pub sp_entity_id: String,
}

impl SamlConfig {
    pub fn new(
        idp_entity_id: impl Into<String>,
        idp_sso_url: impl Into<String>,
        idp_certificate: impl Into<String>,
        sp_entity_id: impl Into<String>,
    ) -> Self {
        Self {
            idp_entity_id: idp_entity_id.into(),
            idp_sso_url: idp_sso_url.into(),
            idp_certificate: idp_certificate.into(),
            sp_entity_id: sp_entity_id.into(),
        }
    }
}

/// Request to initiate SAML 2.0 login.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SamlLoginRequest {
    pub relay_state: Option<String>,
}

/// Response returned from SAML 2.0 login initiation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SamlLoginResponse {
    pub sso_url: String,
    pub saml_request: String,
    pub relay_state: Option<String>,
}

/// Request received at the SAML 2.0 callback endpoint.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SamlCallbackRequest {
    pub saml_response: String,
    pub relay_state: Option<String>,
}

/// Generate SAML 2.0 AuthnRequest XML and encode it in base64.
pub fn generate_authn_request(
    config: &SamlConfig,
    req: &SamlLoginRequest,
) -> Result<SamlLoginResponse, SsoError> {
    let id = format!("_{}", Uuid::new_v4());
    let issue_instant = Utc::now().to_rfc3339();

    let authn_xml = format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{id}" Version="2.0" IssueInstant="{issue_instant}" Destination="{dest}" AssertionConsumerServiceURL="{dest}"><saml:Issuer>{sp}</saml:Issuer></samlp:AuthnRequest>"#,
        dest = config.idp_sso_url,
        sp = config.sp_entity_id
    );

    let saml_request_b64 = STANDARD.encode(authn_xml.as_bytes());

    Ok(SamlLoginResponse {
        sso_url: config.idp_sso_url.clone(),
        saml_request: saml_request_b64,
        relay_state: req.relay_state.clone(),
    })
}

/// Parse date-time string in RFC 3339 or ISO 8601 format.
fn parse_saml_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // Try standard ISO-8601 UTC representation (e.g. 2026-11-18T12:00:00Z)
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
        .ok()
}

/// Parse and validate SAML 2.0 assertion using `roxmltree`.
pub fn parse_and_validate_saml_response(
    config: &SamlConfig,
    req: &SamlCallbackRequest,
    custom_mapping: Option<&HashMap<String, Role>>,
) -> Result<SsoAuthSuccess, SsoError> {
    let raw = req.saml_response.trim();
    if raw.is_empty() {
        return Err(SsoError::InvalidSamlResponse);
    }

    // 1. Decode base64 SAML response XML
    let xml_bytes = if raw.starts_with('<') {
        raw.as_bytes().to_vec()
    } else {
        // Strip whitespace/newlines before base64 decoding
        let clean_b64: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        STANDARD
            .decode(&clean_b64)
            .map_err(|_| SsoError::InvalidSamlResponse)?
    };

    let xml_str = std::str::from_utf8(&xml_bytes)
        .map_err(|_| SsoError::InvalidSamlResponse)?;

    // 2. Parse XML Document with roxmltree
    let doc = roxmltree::Document::parse(xml_str)
        .map_err(|_| SsoError::InvalidSamlXml)?;

    // 3. Validate Issuer against config.idp_entity_id
    let mut found_issuer = false;
    for node in doc.descendants() {
        if node.tag_name().name() == "Issuer" {
            if let Some(text) = node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    found_issuer = true;
                    if trimmed != config.idp_entity_id {
                        return Err(SsoError::InvalidIssuer);
                    }
                }
            }
        }
    }
    if !found_issuer {
        return Err(SsoError::InvalidIssuer);
    }

    // 4. Validate Audience against config.sp_entity_id
    let mut has_audience_restriction = false;
    let mut audiences = Vec::new();
    for node in doc.descendants() {
        if node.tag_name().name() == "AudienceRestriction" {
            has_audience_restriction = true;
        }
        if node.tag_name().name() == "Audience" {
            if let Some(text) = node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    audiences.push(trimmed);
                }
            }
        }
    }
    if has_audience_restriction && audiences.is_empty() {
        return Err(SsoError::InvalidAudience);
    }
    if !audiences.is_empty() && !audiences.iter().any(|a| *a == config.sp_entity_id) {
        return Err(SsoError::InvalidAudience);
    }

    // 5. Extract Subject NameID
    let mut name_id_opt = None;
    for node in doc.descendants() {
        if node.tag_name().name() == "NameID" {
            if let Some(text) = node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    name_id_opt = Some(trimmed.to_string());
                    break;
                }
            }
        }
    }

    let subject = name_id_opt.ok_or(SsoError::MissingSubject)?;

    // 6. Validate Conditions (NotBefore & NotOnOrAfter)
    let now = Utc::now();
    for node in doc.descendants() {
        if node.tag_name().name() == "Conditions" {
            if let Some(not_on_or_after_str) = node.attribute("NotOnOrAfter") {
                if let Some(not_on_or_after) = parse_saml_datetime(not_on_or_after_str) {
                    if now >= not_on_or_after {
                        return Err(SsoError::SamlAssertionExpired);
                    }
                }
            }

            if let Some(not_before_str) = node.attribute("NotBefore") {
                if let Some(not_before) = parse_saml_datetime(not_before_str) {
                    if now < not_before {
                        return Err(SsoError::SamlAssertionNotYetValid);
                    }
                }
            }
        }
    }

    // 5. Extract AttributeStatements
    let mut attributes: HashMap<String, Vec<String>> = HashMap::new();
    for node in doc.descendants() {
        if node.tag_name().name() == "Attribute" {
            let attr_name = node
                .attribute("Name")
                .or_else(|| node.attribute("FriendlyName"));

            if let Some(name) = attr_name {
                let mut vals = Vec::new();
                for child in node.children() {
                    if child.tag_name().name() == "AttributeValue" {
                        if let Some(txt) = child.text() {
                            vals.push(txt.trim().to_string());
                        }
                    }
                }
                if !vals.is_empty() {
                    attributes.entry(name.to_string()).or_default().extend(vals);
                }
            }
        }
    }

    // 6. Map attributes to Role
    let role = map_saml_attributes_to_role(&attributes, custom_mapping);

    // 7. Extract optional display name and email
    let display_name = attributes
        .get("displayName")
        .or_else(|| attributes.get("name"))
        .or_else(|| attributes.get("cn"))
        .and_then(|v| v.first().cloned());

    let email = attributes
        .get("email")
        .or_else(|| attributes.get("emailAddress"))
        .or_else(|| attributes.get("mail"))
        .and_then(|v| v.first().cloned())
        .or_else(|| {
            if subject.contains('@') {
                Some(subject.clone())
            } else {
                None
            }
        });

    Ok(SsoAuthSuccess {
        subject,
        display_name,
        email,
        role,
        provider: "saml".to_string(),
    })
}

/// Helper to create valid sample SAML XML assertion string for testing.
pub fn create_test_saml_assertion(
    subject: &str,
    role: &str,
    not_before: &str,
    not_on_or_after: &str,
    issuer: &str,
    audience: &str,
) -> String {
    format!(
        r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="_resp123" Version="2.0">
  <saml2:Issuer>{issuer}</saml2:Issuer>
  <saml2:Assertion ID="_assert123" Version="2.0">
    <saml2:Issuer>{issuer}</saml2:Issuer>
    <saml2:Subject>
      <saml2:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">{subject}</saml2:NameID>
    </saml2:Subject>
    <saml2:Conditions NotBefore="{not_before}" NotOnOrAfter="{not_on_or_after}">
      <saml2:AudienceRestriction>
        <saml2:Audience>{audience}</saml2:Audience>
      </saml2:AudienceRestriction>
    </saml2:Conditions>
    <saml2:AttributeStatement>
      <saml2:Attribute Name="Role">
        <saml2:AttributeValue>{role}</saml2:AttributeValue>
      </saml2:Attribute>
    </saml2:AttributeStatement>
  </saml2:Assertion>
</saml2p:Response>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> SamlConfig {
        SamlConfig::new(
            "https://idp.example.com/entity",
            "https://idp.example.com/sso",
            "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8A...",
            "saturn-sp-1",
        )
    }

    #[test]
    fn test_generate_authn_request() {
        let cfg = sample_config();
        let req = SamlLoginRequest {
            relay_state: Some("relay-123".into()),
        };
        let res = generate_authn_request(&cfg, &req).unwrap();
        assert_eq!(res.sso_url, "https://idp.example.com/sso");
        assert_eq!(res.relay_state, Some("relay-123".into()));
        assert!(!res.saml_request.is_empty());
    }

    #[test]
    fn test_valid_saml_assertion_parsing() {
        let cfg = sample_config();
        let xml = create_test_saml_assertion(
            "editor@example.com",
            "Approver",
            "2020-01-01T00:00:00Z",
            "2099-01-01T00:00:00Z",
            "https://idp.example.com/entity",
            "saturn-sp-1",
        );
        let b64 = STANDARD.encode(xml.as_bytes());
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        let success = parse_and_validate_saml_response(&cfg, &req, None).unwrap();
        assert_eq!(success.subject, "editor@example.com");
        assert_eq!(success.role, Role::Approver);
        assert_eq!(success.provider, "saml");
    }

    #[test]
    fn test_mismatched_saml_issuer_rejected() {
        let cfg = sample_config();
        let xml = create_test_saml_assertion(
            "editor@example.com",
            "Approver",
            "2020-01-01T00:00:00Z",
            "2099-01-01T00:00:00Z",
            "https://rogue-idp.com",
            "saturn-sp-1",
        );
        let b64 = STANDARD.encode(xml.as_bytes());
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        assert_eq!(
            parse_and_validate_saml_response(&cfg, &req, None),
            Err(SsoError::InvalidIssuer)
        );
    }

    #[test]
    fn test_mismatched_saml_audience_rejected() {
        let cfg = sample_config();
        let xml = create_test_saml_assertion(
            "editor@example.com",
            "Approver",
            "2020-01-01T00:00:00Z",
            "2099-01-01T00:00:00Z",
            "https://idp.example.com/entity",
            "wrong-sp-audience",
        );
        let b64 = STANDARD.encode(xml.as_bytes());
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        assert_eq!(
            parse_and_validate_saml_response(&cfg, &req, None),
            Err(SsoError::InvalidAudience)
        );
    }

    #[test]
    fn test_expired_saml_assertion_rejected() {
        let cfg = sample_config();
        let xml = create_test_saml_assertion(
            "editor@example.com",
            "Approver",
            "2020-01-01T00:00:00Z",
            "2021-01-01T00:00:00Z",
            "https://idp.example.com/entity",
            "saturn-sp-1",
        );
        let b64 = STANDARD.encode(xml.as_bytes());
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        assert_eq!(
            parse_and_validate_saml_response(&cfg, &req, None),
            Err(SsoError::SamlAssertionExpired)
        );
    }

    #[test]
    fn test_malformed_xml_rejected() {
        let cfg = sample_config();
        let b64 = STANDARD.encode("not valid xml");
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        assert_eq!(
            parse_and_validate_saml_response(&cfg, &req, None),
            Err(SsoError::InvalidSamlXml)
        );
    }

    #[test]
    fn test_missing_subject_rejected() {
        let cfg = sample_config();
        let xml = r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion"><saml2:Issuer>https://idp.example.com/entity</saml2:Issuer><saml2:Conditions NotOnOrAfter="2099-01-01T00:00:00Z"/></saml2:Assertion>"#;
        let b64 = STANDARD.encode(xml.as_bytes());
        let req = SamlCallbackRequest {
            saml_response: b64,
            relay_state: None,
        };
        assert_eq!(
            parse_and_validate_saml_response(&cfg, &req, None),
            Err(SsoError::MissingSubject)
        );
    }
}
