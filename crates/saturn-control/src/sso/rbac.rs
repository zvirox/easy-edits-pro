// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Role-Based Access Control (RBAC) and IdP group/claim mapping for Enterprise SSO.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Enterprise roles for Web Desk / saturn-control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub enum Role {
    #[default]
    #[serde(alias = "requester", alias = "REQUESTER")]
    Requester,
    #[serde(alias = "reviewer", alias = "REVIEWER")]
    Reviewer,
    #[serde(alias = "approver", alias = "APPROVER")]
    Approver,
    #[serde(alias = "admin", alias = "ADMIN")]
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requester => "Requester",
            Self::Reviewer => "Reviewer",
            Self::Approver => "Approver",
            Self::Admin => "Admin",
        }
    }

    /// Available roles in order of increasing privilege.
    pub fn all_roles() -> &'static [Role] {
        &[Role::Requester, Role::Reviewer, Role::Approver, Role::Admin]
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Map string value to a role based on explicit match or known substring heuristics.
pub fn parse_role_or_group(value: &str) -> Option<Role> {
    let trimmed = value.trim().trim_start_matches('/');
    let lower = trimmed.to_lowercase();

    if lower == "admin" || lower == "admins" || lower.contains("admin") {
        return Some(Role::Admin);
    }
    if lower == "approver" || lower == "approvers" || lower.contains("approv") {
        return Some(Role::Approver);
    }
    if lower == "reviewer" || lower == "reviewers" || lower.contains("review") {
        return Some(Role::Reviewer);
    }
    if lower == "requester" || lower == "requesters" || lower.contains("request") {
        return Some(Role::Requester);
    }
    None
}

/// Helper to select the highest role from candidate roles.
fn highest_role(candidates: &[Role]) -> Role {
    candidates
        .iter()
        .copied()
        .max()
        .unwrap_or(Role::Requester)
}

/// Extract and map roles from OIDC JWT claims JSON (Okta, Azure AD, Keycloak, etc.).
pub fn map_claims_to_role(
    claims: &serde_json::Value,
    custom_mapping: Option<&HashMap<String, Role>>,
) -> Role {
    let mut detected_roles = Vec::new();

    let check_string = |val: &str, roles: &mut Vec<Role>| {
        let clean = val.trim().trim_start_matches('/');
        if let Some(custom) = custom_mapping {
            if let Some(&role) = custom.get(clean).or_else(|| custom.get(val.trim())) {
                roles.push(role);
                return;
            }
        }
        if let Some(role) = parse_role_or_group(clean) {
            roles.push(role);
        }
    };

    let check_array_or_string = |val_opt: Option<&serde_json::Value>, roles: &mut Vec<Role>| {
        if let Some(val) = val_opt {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        check_string(s, roles);
                    }
                }
            } else if let Some(s) = val.as_str() {
                check_string(s, roles);
            }
        }
    };

    // 1. Direct role/roles claim
    check_array_or_string(claims.get("role"), &mut detected_roles);
    check_array_or_string(claims.get("roles"), &mut detected_roles);

    // 2. Okta / generic: `groups` claim
    check_array_or_string(claims.get("groups"), &mut detected_roles);
    check_array_or_string(claims.get("group"), &mut detected_roles);

    // 3. Azure AD / Microsoft Entra ID: App Roles and URI claims
    check_array_or_string(
        claims.get("http://schemas.microsoft.com/ws/2008/06/identity/claims/role"),
        &mut detected_roles,
    );
    check_array_or_string(
        claims.get("http://schemas.microsoft.com/ws/2008/06/identity/claims/groups"),
        &mut detected_roles,
    );

    // 4. Keycloak: `realm_access.roles` and `resource_access.<client>.roles`
    if let Some(realm_access) = claims.get("realm_access") {
        check_array_or_string(realm_access.get("roles"), &mut detected_roles);
    }
    if let Some(resource_access) = claims.get("resource_access").and_then(|v| v.as_object()) {
        for (_client, client_val) in resource_access {
            check_array_or_string(client_val.get("roles"), &mut detected_roles);
        }
    }

    highest_role(&detected_roles)
}

/// Extract and map roles from SAML 2.0 attribute statements.
pub fn map_saml_attributes_to_role(
    attributes: &HashMap<String, Vec<String>>,
    custom_mapping: Option<&HashMap<String, Role>>,
) -> Role {
    let mut detected_roles = Vec::new();

    let check_string = |val: &str, roles: &mut Vec<Role>| {
        let clean = val.trim().trim_start_matches('/');
        if let Some(custom) = custom_mapping {
            if let Some(&role) = custom.get(clean).or_else(|| custom.get(val.trim())) {
                roles.push(role);
                return;
            }
        }
        if let Some(role) = parse_role_or_group(clean) {
            roles.push(role);
        }
    };

    for (name, values) in attributes {
        let name_lower = name.to_lowercase();
        let is_role_attr = name_lower == "role"
            || name_lower == "roles"
            || name_lower == "group"
            || name_lower == "groups"
            || name_lower == "memberof"
            || name_lower.ends_with("/claims/role")
            || name_lower.ends_with("/claims/groups");

        if is_role_attr {
            for val in values {
                check_string(val, &mut detected_roles);
            }
        }
    }

    highest_role(&detected_roles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_okta_group_mapping() {
        let claims = json!({
            "sub": "user123",
            "groups": ["video-approvers", "everyone"]
        });
        assert_eq!(map_claims_to_role(&claims, None), Role::Approver);

        let claims_req = json!({
            "sub": "user123",
            "groups": ["video-requesters"]
        });
        assert_eq!(map_claims_to_role(&claims_req, None), Role::Requester);
    }

    #[test]
    fn test_azure_ad_roles_mapping() {
        let claims = json!({
            "sub": "azure-user",
            "roles": ["Approver"]
        });
        assert_eq!(map_claims_to_role(&claims, None), Role::Approver);

        let claims_uri = json!({
            "sub": "azure-user-2",
            "http://schemas.microsoft.com/ws/2008/06/identity/claims/role": ["Admin"]
        });
        assert_eq!(map_claims_to_role(&claims_uri, None), Role::Admin);
    }

    #[test]
    fn test_keycloak_roles_and_groups() {
        let claims = json!({
            "sub": "kc-user",
            "realm_access": {
                "roles": ["default-roles-eep", "approver"]
            },
            "groups": ["/video-approvers"]
        });
        assert_eq!(map_claims_to_role(&claims, None), Role::Approver);
    }

    #[test]
    fn test_fallback_to_requester() {
        let claims = json!({
            "sub": "random-user",
            "groups": ["marketing", "external"]
        });
        assert_eq!(map_claims_to_role(&claims, None), Role::Requester);
    }

    #[test]
    fn test_saml_attribute_mapping() {
        let mut attrs = HashMap::new();
        attrs.insert("Role".to_string(), vec!["Approver".to_string()]);
        assert_eq!(map_saml_attributes_to_role(&attrs, None), Role::Approver);

        let mut attrs_groups = HashMap::new();
        attrs_groups.insert("groups".to_string(), vec!["video-approvers".to_string()]);
        assert_eq!(map_saml_attributes_to_role(&attrs_groups, None), Role::Approver);
    }
}
