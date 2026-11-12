// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MCP tests for TS5 delivery packaging & filesystem tier enforcement.

use super::*;

#[tokio::test]
async fn mcp_delivery_package_enforces_filesystem_tier_and_verifies_checksums() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({ "name": "MCP TS5 Delivery" }),
        ))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    // Load `tasksheet` profile without Filesystem tier grant -> PermissionDenied
    let mut load_args = serde_json::Map::new();
    load_args.insert("profile".to_string(), json!("tasksheet"));
    let load_res = mcp
        .call(CallToolRequestParams::new("tools.load_profile").with_arguments(load_args))
        .await
        .unwrap();
    assert!(!load_res.is_error.unwrap_or(false));

    let tmp_root = std::env::temp_dir().join(format!("eep-mcp-ts5-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_root);
    std::fs::create_dir_all(&tmp_root).unwrap();
    struct TempDirCleanup(std::path::PathBuf);
    impl Drop for TempDirCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = TempDirCleanup(tmp_root.clone());
    let pkg_dir = tmp_root.join("mcp-ts5-bundle");

    let mut pkg_args = serde_json::Map::new();
    pkg_args.insert("id".to_string(), json!("delivery.package"));
    pkg_args.insert(
        "params".to_string(),
        json!({ "path": pkg_dir.to_string_lossy() }),
    );
    let denied_err = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(pkg_args.clone()))
        .await
        .unwrap_err();
    assert!(denied_err.message.contains("PermissionDenied"));
    assert!(!pkg_dir.exists());

    // Grant Filesystem tier on session policy
    mcp.policy
        .write()
        .await
        .granted_tiers
        .insert(saturn_core::catalogue::Tier::Filesystem);

    // Writing outside allowed roots (e.g., /etc/...) must be blocked by OriginTracker
    let mut outside_args = serde_json::Map::new();
    outside_args.insert("id".to_string(), json!("delivery.package"));
    outside_args.insert(
        "params".to_string(),
        json!({ "path": "/etc/eep-mcp-disallowed-delivery" }),
    );
    let outside_err = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(outside_args))
        .await
        .unwrap_err();
    assert!(outside_err.message.contains("PermissionDenied"));
    assert!(outside_err.message.contains("unknown origin"));

    // Valid target inside temp_dir succeeds and verifies cleanly
    let ok_res = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(pkg_args))
        .await
        .unwrap();
    assert!(!ok_res.is_error.unwrap_or(false));
    let payload: Value =
        serde_json::from_str(&ok_res.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(payload["packaged"], json!(true));
    assert_eq!(payload["verified"], json!(true));

    let mut verify_args = serde_json::Map::new();
    verify_args.insert("id".to_string(), json!("delivery.verify"));
    verify_args.insert(
        "params".to_string(),
        json!({ "path": pkg_dir.to_string_lossy() }),
    );
    let verify_res = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(verify_args))
        .await
        .unwrap();
    assert!(!verify_res.is_error.unwrap_or(false));
    let v_payload: Value =
        serde_json::from_str(&verify_res.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(v_payload["verified"], json!(true));
    assert_eq!(v_payload["verification"]["blake3_verified"], json!(true));
    assert_eq!(v_payload["verification"]["sha256_verified"], json!(true));
}
