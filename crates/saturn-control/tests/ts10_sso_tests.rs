// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Conformance integration test suite for TS10: Enterprise SSO & RBAC.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use saturn_commands::{
    CommandRegistry, ShortcutManager, UndoManager,
    context::{AppContext, AppState, EventBus},
    register_builtin_commands,
};
use saturn_control::auth::Role;
use saturn_control::server::{ControlServer, DaemonConfig};
use saturn_control::sso::saml::create_test_saml_assertion;
use saturn_control::sso::{OidcConfig, SamlConfig, SsoConfig};
use saturn_core::tasksheet::{compute_blake3_hex, compute_sha256_hex};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const TEST_MASTER: &str = "test-master-sso-token";

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn create_test_delivery_package_fixture(pkg_dir: &Path) {
    let exports_dir = pkg_dir.join("exports");
    std::fs::create_dir_all(&exports_dir).expect("create exports dir");
    let export_file = exports_dir.join("main_cut.mp4");
    let media_bytes = b"fake-video-payload-for-ts10";
    std::fs::write(&export_file, media_bytes).expect("write export file");

    let b3_media = compute_blake3_hex(media_bytes);
    let s256_media = compute_sha256_hex(media_bytes);

    let manifest = json!({
        "schema": "delivery/1",
        "created_at_unix_ms": 1000,
        "created_by": "human:approver",
        "session_id": "ts10-session",
        "project_name": "TS10 Test",
        "project_format": "saturn",
        "project_schema_version": 1,
        "hash_algorithms": ["blake3", "sha256"],
        "staged_exports_count": 1,
        "changelog_entries_count": 0,
        "review_rounds_count": 0,
        "files": [{
            "relative_path": "exports/main_cut.mp4",
            "role": "export",
            "format": "mp4",
            "size_bytes": media_bytes.len(),
            "blake3": b3_media,
            "sha256": s256_media
        }]
    });

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("serialize manifest");
    std::fs::write(pkg_dir.join("manifest.json"), &manifest_bytes).expect("write manifest.json");

    let b3_manifest = compute_blake3_hex(&manifest_bytes);
    let s256_manifest = compute_sha256_hex(&manifest_bytes);

    let b3_content = format!("{b3_manifest}  manifest.json\n{b3_media}  exports/main_cut.mp4\n");
    let s256_content = format!("{s256_manifest}  manifest.json\n{s256_media}  exports/main_cut.mp4\n");

    std::fs::write(pkg_dir.join("CHECKSUMS.blake3"), b3_content).expect("write CHECKSUMS.blake3");
    std::fs::write(pkg_dir.join("SHA256SUMS"), s256_content).expect("write SHA256SUMS");
}

struct SsoHarness {
    server: Arc<ControlServer>,
    router: Router,
    _temp_dir: TempDir,
    audit_path: PathBuf,
}

impl SsoHarness {
    async fn new(sso_config: SsoConfig) -> Self {
        let dir = tempfile::tempdir().expect("create temp dir");
        let audit_path = dir.path().join("audit.jsonl");
        let project_root = dir.path().join("projects");
        std::fs::create_dir_all(&project_root).expect("create project root");

        let config = DaemonConfig {
            master_token: Some(TEST_MASTER.to_string()),
            project_root: Some(project_root),
            audit_log_path: Some(audit_path.clone()),
            max_audit_entries: 5000,
            sso_config: Some(sso_config),
            ..Default::default()
        };

        let registry = CommandRegistry::new();
        register_builtin_commands(&registry).expect("register builtin commands");

        let app = Arc::new(AppContext {
            state: Arc::new(RwLock::new(AppState::default())),
            registry: Arc::new(registry),
            undo: Arc::new(UndoManager::new(100)),
            shortcuts: Arc::new(ShortcutManager::new()),
            events: Arc::new(EventBus::new()),
        });
        let server = Arc::new(ControlServer::with_config(app, config));
        let router = server.router();

        Self {
            server,
            router,
            _temp_dir: dir,
            audit_path,
        }
    }

    async fn call(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.router.clone().oneshot(req).await.expect("route request");
        let status = res.status();
        let bytes = res
            .into_body()
            .collect()
            .await
            .expect("read body bytes")
            .to_bytes();

        let val: Value = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }))
        };

        (status, val)
    }

    async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method("GET").uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        self.call(builder.body(Body::empty()).unwrap()).await
    }

    async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        self.call(
            builder
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
    }

    async fn open_project(&self, token: &str) {
        let (status, body) = self.post("/project/open", json!({}), Some(token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["opened"], true);
    }
}

fn default_test_sso_config() -> SsoConfig {
    SsoConfig {
        enabled: true,
        oidc: Some(OidcConfig::new(
            "https://idp.example.com",
            "saturn-control",
            "super-secret-client-secret",
            "https://idp.example.com/.well-known/jwks.json",
            "http://localhost:3000/auth/callback",
        )),
        saml: Some(SamlConfig::new(
            "https://saml.idp.com",
            "https://saml.idp.com/sso",
            "MIIDVzCCAj+gAwIBAgIJAPk12345...",
            "saturn-control",
        )),
        group_mappings: None,
    }
}

#[tokio::test]
async fn test_01_oidc_login_and_callback_flow_with_simulated_claims() {
    let harness = SsoHarness::new(default_test_sso_config()).await;

    // 1. Initiate OIDC Login
    let login_payload = json!({
        "provider": "okta",
        "redirect_uri": "http://localhost:3000/auth/callback"
    });
    let (status, body) = harness.post("/auth/sso/oidc/login", login_payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["authorization_url"].as_str().unwrap().contains("saturn-control"));
    assert!(!body["state"].as_str().unwrap().is_empty());

    // 2. OIDC Callback with simulated valid claims
    let exp = now_secs() + 3600;
    let claims = json!({
        "sub": "usr_approver",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": exp,
        "name": "Jane Approver",
        "email": "jane@example.com",
        "groups": ["video-approvers"]
    });
    let callback_payload = json!({
        "claims": claims,
        "state": body["state"].as_str().unwrap()
    });
    let (status, callback_res) = harness.post("/auth/sso/oidc/callback", callback_payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(callback_res["success"], true);
    assert_eq!(callback_res["actor"], "human:usr_approver");
    assert_eq!(callback_res["role"], "Approver");
    let session_token = callback_res["token"].as_str().expect("session token");
    assert!(session_token.starts_with("saturn-"));

    // 3. Inspect roles endpoint using minted session token
    let (status, roles_res) = harness.get("/auth/sso/roles", Some(session_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(roles_res["role"], "Approver");
    assert_eq!(roles_res["permissions"]["can_approve_plan"], true);
    assert_eq!(roles_res["permissions"]["can_approve_delivery"], true);
    assert_eq!(roles_res["permissions"]["can_submit_jobs"], true);
    assert_eq!(roles_res["is_agent"], false);
}

#[tokio::test]
async fn test_02_oidc_claims_validation_error_cases() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    let now = now_secs();

    // 1. Expired token
    let expired_claims = json!({
        "sub": "user_exp",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now - 300,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": expired_claims }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "TokenExpired");

    // 2. Invalid Issuer
    let bad_iss_claims = json!({
        "sub": "user_bad_iss",
        "iss": "https://attacker-idp.com",
        "aud": "saturn-control",
        "exp": now + 3600,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": bad_iss_claims }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidIssuer");

    // 3. Invalid Audience
    let bad_aud_claims = json!({
        "sub": "user_bad_aud",
        "iss": "https://idp.example.com",
        "aud": "unrelated-client-id",
        "exp": now + 3600,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": bad_aud_claims }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidAudience");

    // 4. Missing Subject
    let missing_sub_claims = json!({
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": missing_sub_claims }), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["code"], "MissingSubject");

    // 5. Future nbf (not before)
    let future_nbf_claims = json!({
        "sub": "user_future_nbf",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 7200,
        "nbf": now + 3600,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": future_nbf_claims }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidToken");

    // 6. Future iat (issued at)
    let future_iat_claims = json!({
        "sub": "user_future_iat",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 7200,
        "iat": now + 3600,
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": future_iat_claims }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidToken");
}

#[tokio::test]
async fn test_03_saml_login_and_assertion_parsing_with_role_mapping() {
    let harness = SsoHarness::new(default_test_sso_config()).await;

    // 1. SAML Login request
    let login_payload = json!({ "relay_state": "state_saml_123" });
    let (status, body) = harness.post("/auth/sso/saml/login", login_payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["sso_url"], "https://saml.idp.com/sso");
    assert!(!body["saml_request"].as_str().unwrap().is_empty());

    // 2. SAML Callback with valid XML Assertion
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    let assertion_xml = create_test_saml_assertion(
        "editor@example.com",
        "Approver",
        "2020-01-01T00:00:00Z",
        "2099-01-01T00:00:00Z",
        "https://saml.idp.com",
        "saturn-control",
    );
    let b64_assertion = STANDARD.encode(assertion_xml.as_bytes());

    let (status, callback_res) = harness
        .post(
            "/auth/sso/saml/callback",
            json!({ "saml_response": b64_assertion, "relay_state": "state_saml_123" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(callback_res["success"], true);
    assert_eq!(callback_res["actor"], "human:editor@example.com");
    assert_eq!(callback_res["role"], "Approver");
    assert_eq!(callback_res["provider"], "saml");
    let session_token = callback_res["token"].as_str().expect("session token");

    // 3. Verify roles endpoint with SAML-issued token
    let (status, roles_res) = harness.get("/api/v1/auth/sso/roles", Some(session_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(roles_res["role"], "Approver");
    assert_eq!(roles_res["permissions"]["can_approve_plan"], true);
}

#[tokio::test]
async fn test_04_saml_error_cases_and_expired_assertions() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    // 1. Malformed Base64 / Invalid XML
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": "!!NotBase64!!" }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["code"], "InvalidSamlResponse");

    let bad_xml_b64 = STANDARD.encode("not valid xml");
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": bad_xml_b64 }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["code"], "InvalidSamlXml");

    // 2. Expired Assertion (NotOnOrAfter in past)
    let expired_xml = create_test_saml_assertion(
        "editor@example.com",
        "Approver",
        "2020-01-01T00:00:00Z",
        "2021-01-01T00:00:00Z",
        "https://saml.idp.com",
        "saturn-control",
    );
    let expired_b64 = STANDARD.encode(expired_xml.as_bytes());
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": expired_b64 }), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "SamlAssertionExpired");

    // 3. Missing Subject NameID
    let missing_nameid_xml = r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion"><saml2:Issuer>https://saml.idp.com</saml2:Issuer><saml2:Conditions NotOnOrAfter="2099-01-01T00:00:00Z"><saml2:AudienceRestriction><saml2:Audience>saturn-control</saml2:Audience></saml2:AudienceRestriction></saml2:Conditions></saml2:Assertion>"#;
    let missing_sub_b64 = STANDARD.encode(missing_nameid_xml.as_bytes());
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": missing_sub_b64 }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["code"], "MissingSubject");

    // 4. Mismatched IdP Entity ID (Issuer) -> 401 Unauthorized
    let bad_issuer_xml = create_test_saml_assertion(
        "editor@example.com",
        "Approver",
        "2020-01-01T00:00:00Z",
        "2099-01-01T00:00:00Z",
        "https://rogue-attacker-idp.com/entity",
        "saturn-control",
    );
    let bad_issuer_b64 = STANDARD.encode(bad_issuer_xml.as_bytes());
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": bad_issuer_b64 }), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidIssuer");

    // 5. Mismatched SP Audience -> 401 Unauthorized
    let bad_aud_xml = create_test_saml_assertion(
        "editor@example.com",
        "Approver",
        "2020-01-01T00:00:00Z",
        "2099-01-01T00:00:00Z",
        "https://saml.idp.com",
        "https://another-sp.example.com",
    );
    let bad_aud_b64 = STANDARD.encode(bad_aud_xml.as_bytes());
    let (status, res) = harness
        .post("/auth/sso/saml/callback", json!({ "saml_response": bad_aud_b64 }), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["code"], "InvalidAudience");
}

#[tokio::test]
async fn test_05_idp_group_mapping_okta_azure_keycloak() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    let now = now_secs();

    // Okta video-approvers -> Approver
    let okta_claims = json!({
        "sub": "okta_user_1",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
        "groups": ["video-approvers"]
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": okta_claims }), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["role"], "Approver");

    // Azure AD App Role -> Approver
    let azure_claims = json!({
        "sub": "azure_user_1",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
        "roles": ["Approver"]
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": azure_claims }), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["role"], "Approver");

    // Keycloak realm_access.roles -> Approver
    let kc_claims = json!({
        "sub": "kc_user_1",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
        "realm_access": { "roles": ["approver"] }
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": kc_claims }), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["role"], "Approver");

    // Okta video-requesters -> Requester
    let okta_req_claims = json!({
        "sub": "okta_req_1",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
        "groups": ["video-requesters"]
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": okta_req_claims }), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["role"], "Requester");

    // Admin group -> Admin
    let admin_claims = json!({
        "sub": "admin_user_1",
        "iss": "https://idp.example.com",
        "aud": "saturn-control",
        "exp": now + 3600,
        "groups": ["admins"]
    });
    let (status, res) = harness.post("/auth/sso/oidc/callback", json!({ "claims": admin_claims }), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["role"], "Admin");
}

#[tokio::test]
async fn test_06_rbac_requester_role_permissions_and_rejections() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    let req_token = harness.server.auth.issue_session_token_with_role(
        "sess-req",
        "human:requester",
        Role::Requester,
        false,
    );
    harness.open_project(&req_token).await;

    // 1. Requester can submit Task Sheet
    let sheet_payload = json!({
        "id": "sheet_req_01",
        "title": "Requester Task Sheet",
        "deliverables": [{ "id": "d1", "name": "Output 1", "preset": "web-1080p" }]
    });
    let (status, res) = harness.post("/tasksheet", sheet_payload, Some(&req_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["success"], true);

    let (status, res) = harness.post("/tasksheet/submit", json!({}), Some(&req_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["submitted"], true);

    // 2. Plan submission is allowed
    let plan_payload = json!({
        "steps": [{ "id": 1, "title": "Cut", "status": "pending" }]
    });
    let (status, res) = harness.post("/tasksheet/plan/submit", plan_payload, Some(&req_token)).await;
    assert_eq!(status, StatusCode::OK, "res was: {:?}", res);

    // 3. Plan approval REJECTED with 403 Forbidden for Requester
    let (status, res) = harness.post("/tasksheet/plan/approve", json!({}), Some(&req_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(res["code"], "PermissionDenied");

    // 4. Delivery verification REJECTED with 403 Forbidden for Requester
    let canonical_root = harness.server.project_root.as_ref().unwrap().canonicalize().unwrap();
    let pkg_dir = canonical_root.join("test_pkg_req");
    create_test_delivery_package_fixture(&pkg_dir);
    let (status, res) = harness.post(
        "/delivery/verify",
        json!({ "path": pkg_dir.to_str().unwrap() }),
        Some(&req_token),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(res["code"], "PermissionDenied");

    // 5. Requester can add review notes in active review session
    let req2_token = harness.server.auth.issue_session_token_with_role(
        "sess-req-2",
        "human:requester",
        Role::Requester,
        false,
    );
    harness.open_project(&req2_token).await;
    let note_payload = json!({
        "note_id": "note_1",
        "timecode_ms": 1000,
        "author": "human:requester",
        "text": "Please fix lighting"
    });
    let (status, res) = harness.post("/review/notes", note_payload, Some(&req2_token)).await;
    assert_eq!(status, StatusCode::OK, "res was: {:?}", res);
    assert_eq!(res["added"], true);
    assert_eq!(res["added"], true);
}

#[tokio::test]
async fn test_07_rbac_approver_and_admin_approval_permissions() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    let approver_token = harness.server.auth.issue_session_token_with_role(
        "sess-appr",
        "human:approver",
        Role::Approver,
        false,
    );
    harness.open_project(&approver_token).await;

    // Fast-forward to awaiting plan approval
    harness.post("/tasksheet/submit", json!({}), Some(&approver_token)).await;
    harness.post("/tasksheet/plan/submit", json!({ "steps": [{ "id": 1, "title": "Grade", "status": "pending" }] }), Some(&approver_token)).await;

    // 1. Approver successfully approves plan
    let (status, res) = harness.post("/tasksheet/plan/approve", json!({ "reason": "Looks good" }), Some(&approver_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["approved"], true);
    assert_eq!(res["status"], "running");

    // 2. Approver verifies delivery
    let canonical_root = harness.server.project_root.as_ref().unwrap().canonicalize().unwrap();
    let pkg_dir = canonical_root.join("test_pkg_appr");
    create_test_delivery_package_fixture(&pkg_dir);
    let (status, res) = harness.post(
        "/delivery/verify",
        json!({ "path": pkg_dir.to_str().unwrap() }),
        Some(&approver_token),
    ).await;
    assert_eq!(status, StatusCode::OK, "res was: {:?}", res);
    assert_eq!(res["verified"], true);

    // 3. Admin has full permissions across all operations
    let admin_token = harness.server.auth.issue_session_token_with_role(
        "sess-admin",
        "human:admin",
        Role::Admin,
        false,
    );
    harness.open_project(&admin_token).await;
    let (status, res) = harness.post(
        "/delivery/verify",
        json!({ "path": pkg_dir.to_str().unwrap() }),
        Some(&admin_token),
    ).await;
    assert_eq!(status, StatusCode::OK, "res was: {:?}", res);
    assert_eq!(res["verified"], true);
}

#[tokio::test]
async fn test_08_rbac_agent_actor_approval_invariant_strict_rejection() {
    let harness = SsoHarness::new(default_test_sso_config()).await;

    // Issue agent token explicitly
    let agent_token = harness.server.auth.issue_session_token_with_role(
        "sess-agent",
        "agent-worker-1",
        Role::Requester,
        true,
    );
    harness.open_project(&agent_token).await;

    // Fast-forward to awaiting plan approval
    harness.post("/tasksheet/submit", json!({}), Some(&agent_token)).await;
    harness.post("/tasksheet/plan/submit", json!({ "steps": [{ "id": 1, "title": "AutoCut", "status": "pending" }] }), Some(&agent_token)).await;

    // 1. STRICT INVARIANT: Agent cannot approve plan -> 403 Forbidden AgentApprovalForbidden
    let (status, res) = harness.post("/tasksheet/plan/approve", json!({}), Some(&agent_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(res["code"], "AgentApprovalForbidden");

    // 2. STRICT INVARIANT: Agent cannot approve or verify delivery -> 403 Forbidden AgentApprovalForbidden
    let canonical_root = harness.server.project_root.as_ref().unwrap().canonicalize().unwrap();
    let agent_pkg_dir = canonical_root.join("test_pkg_agent");
    create_test_delivery_package_fixture(&agent_pkg_dir);
    let (status, res) = harness.post(
        "/delivery/verify",
        json!({ "path": agent_pkg_dir.to_str().unwrap() }),
        Some(&agent_token),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(res["code"], "AgentApprovalForbidden");

    let (status, res) = harness.post(
        "/delivery/approve",
        json!({ "path": agent_pkg_dir.to_str().unwrap() }),
        Some(&agent_token),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(res["code"], "AgentApprovalForbidden");
}

#[tokio::test]
async fn test_09_audit_jsonl_credential_and_secret_redaction() {
    let harness = SsoHarness::new(default_test_sso_config()).await;
    let now = now_secs();

    // Post with client_secret and sensitive tokens
    let sensitive_oidc = json!({
        "claims": {
            "sub": "secret_user",
            "iss": "https://idp.example.com",
            "aud": "saturn-control",
            "exp": now + 3600,
            "client_secret": "raw-secret-12345",
            "access_token": "raw-access-token-9999",
            "password": "super-secret-password",
        }
    });
    harness.post("/auth/sso/oidc/callback", sensitive_oidc, None).await;

    // Read audit.jsonl lines
    let file = File::open(&harness.audit_path).expect("open audit.jsonl");
    let reader = BufReader::new(file);

    let mut record_count = 0;
    for line in reader.lines() {
        let l = line.expect("read line");
        if l.trim().is_empty() {
            continue;
        }
        record_count += 1;

        // Invariant 1: Zero plaintext secrets in log file
        assert!(!l.contains("raw-secret-12345"), "Plaintext secret found in audit log!");
        assert!(!l.contains("raw-access-token-9999"), "Plaintext access token found in audit log!");
        assert!(!l.contains("super-secret-password"), "Plaintext password found in audit log!");

        // Invariant 2: Argument hash is exactly 64 hex characters (SHA-256)
        let record: Value = serde_json::from_str(&l).expect("parse json line");
        let hash = record["argument_hash"].as_str().expect("argument_hash string");
        assert_eq!(hash.len(), 64, "argument_hash must be a 64-char hex string");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "argument_hash must be hex");
    }
    assert!(record_count > 0, "Audit log should have recorded entries");
}

#[tokio::test]
async fn test_10_sso_status_and_roles_inspection_endpoints() {
    let harness = SsoHarness::new(default_test_sso_config()).await;

    // 1. GET /auth/sso/status & /api/v1/auth/sso/status (public)
    let (status, res) = harness.get("/auth/sso/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["enabled"], true);
    assert_eq!(res["oidc"]["configured"], true);
    assert_eq!(res["saml"]["configured"], true);

    let (status, res_v1) = harness.get("/api/v1/auth/sso/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res_v1["enabled"], true);

    // 2. GET /auth/sso/roles without token -> 401 Unauthorized
    let (status, _) = harness.get("/auth/sso/roles", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 3. GET /auth/sso/roles with Approver token -> returns role and permissions
    let approver_token = harness.server.auth.issue_session_token_with_role(
        "sess-inspect",
        "human:approver",
        Role::Approver,
        false,
    );
    let (status, roles_res) = harness.get("/api/v1/auth/sso/roles", Some(&approver_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(roles_res["role"], "Approver");
    assert_eq!(roles_res["actor"], "human:approver");
    assert_eq!(roles_res["permissions"]["can_approve_plan"], true);
    assert_eq!(roles_res["permissions"]["can_approve_delivery"], true);
    assert_eq!(roles_res["permissions"]["can_submit_jobs"], true);
    assert_eq!(roles_res["available_roles"].as_array().unwrap().len(), 4);
}
