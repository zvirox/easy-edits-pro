// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MCP integration tests for `capture_read_events` (CET v0.1 + AF5 isolation).

use super::*;
use saturn_core::tasksheet::cet::CetGenerator;

#[tokio::test]
async fn mcp_capture_read_events_enforces_profile_containment_and_af5_isolation() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({ "name": "MCP CET Inspection" }),
        ))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    let tmp_dir = std::env::temp_dir().join(format!("eep-mcp-cet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(tmp_dir.clone());

    let cet_path = tmp_dir.join("desktop_capture.cet.jsonl");
    let injected_title = "INJECTED_DESTRUCTIVE_TARGET /etc/shadow";
    let mut generator = CetGenerator::new("desktop_capture.mkv", 30.0, 1920, 1080);
    generator.emit_window("Browser", injected_title, 0.0, 0.0, 1.0, 1.0);
    generator.emit_mark("chapter", "Setup Step");
    generator.advance_ms(400);
    generator.emit_click(0.35, 0.42, "left");
    generator.advance_ms(800);
    std::fs::write(&cet_path, generator.to_jsonl()).unwrap();

    // 1. Default session lacks Filesystem tier -> PermissionDenied
    let mut read_args = serde_json::Map::new();
    read_args.insert("path".to_string(), json!(cet_path.to_string_lossy()));
    let tier_err = mcp
        .call(CallToolRequestParams::new("capture_read_events").with_arguments(read_args.clone()))
        .await
        .unwrap_err();
    assert!(tier_err.message.contains("PermissionDenied"));

    // Grant Filesystem tier without `ingest`/`review` profile -> ProfileRequired
    mcp.policy
        .write()
        .await
        .granted_tiers
        .insert(saturn_core::catalogue::Tier::Filesystem);

    let profile_err = mcp
        .call(CallToolRequestParams::new("capture_read_events").with_arguments(read_args.clone()))
        .await
        .unwrap_err();
    assert!(profile_err.message.contains("ProfileRequired"));

    // Load `review` profile -> path traversal (`..`) still blocked
    let mut load_args = serde_json::Map::new();
    load_args.insert("profile".to_string(), json!("review"));
    mcp.call(CallToolRequestParams::new("tools.load_profile").with_arguments(load_args))
        .await
        .unwrap();

    let mut traversal_args = serde_json::Map::new();
    traversal_args.insert("path".to_string(), json!("../../etc/passwd.cet.jsonl"));
    let trav_res = mcp
        .call(CallToolRequestParams::new("capture_read_events").with_arguments(traversal_args))
        .await
        .unwrap();
    assert!(trav_res.is_error.unwrap_or(false));
    let trav_text = trav_res.content[0].as_text().unwrap().text.as_str();
    assert!(trav_text.contains("PermissionDenied"), "unexpected error: {trav_text}");

    // 2. Valid CET sidecar read succeeds with bounded events, proposals, and AF5 untrusted marking
    let res = mcp
        .call(CallToolRequestParams::new("capture_read_events").with_arguments(read_args))
        .await
        .unwrap();
    assert!(!res.is_error.unwrap_or(false));
    let payload: Value = serde_json::from_str(&res.content[0].as_text().unwrap().text).unwrap();
    assert!(payload["untrusted_notice"]
        .as_str()
        .unwrap()
        .contains("AF5_UNTRUSTED_CET"));
    assert!(payload["proposals"]["zoom_proposals"].as_array().unwrap().len() >= 1);

    // 3. Passing CET untrusted string into file save / tasksheet title must be refused by OriginTracker
    let mut load_ts = serde_json::Map::new();
    load_ts.insert("profile".to_string(), json!("tasksheet"));
    mcp.call(CallToolRequestParams::new("tools.load_profile").with_arguments(load_ts))
        .await
        .unwrap();

    let mut exact_save = serde_json::Map::new();
    exact_save.insert("id".to_string(), json!("file.save"));
    exact_save.insert("params".to_string(), json!({ "path": injected_title }));
    let blocked = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(exact_save))
        .await
        .unwrap_err();
    assert!(
        blocked.message.contains("PermissionDenied"),
        "expected AF5 refusal on CET untrusted string, got: {}",
        blocked.message
    );

    let mut ts_update = serde_json::Map::new();
    ts_update.insert("id".to_string(), json!("tasksheet.update"));
    ts_update.insert("params".to_string(), json!({ "title": injected_title }));
    let ts_blocked = mcp
        .call(CallToolRequestParams::new("command_run").with_arguments(ts_update))
        .await
        .unwrap_err();
    assert!(
        ts_blocked.message.contains("PermissionDenied")
            && ts_blocked.message.contains("UntrustedOrigin"),
        "expected AF5 UntrustedOrigin refusal on tasksheet.update, got: {}",
        ts_blocked.message
    );
}
