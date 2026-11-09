// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

#[test]
fn media_preview_reports_honest_absence_of_still_frames() {
    let mut engine = saturn_core::EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP preview"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mp4", "path": "/assets/clip.mp4", "kind": "video"}),
        ))
        .unwrap();
    let document = engine.document().clone();
    let media_id = document.project.media[0].id;

    let payload = saturn_render::preview_tools::media_preview_payload(&document, media_id, None)
        .unwrap();
    assert!(payload.get("preview_path").is_none(), "no source-path placeholder");
    assert_eq!(payload["still_frame"]["available"], false);
    assert!(payload["still_frame"]["reason"].is_string(), "absence carries a reason");
    assert!(payload.get("image_block").is_none(), "no fabricated image block");
    assert_eq!(
        payload["media"]["id"],
        serde_json::to_value(media_id).unwrap()
    );

    // Unknown ids stay a typed error, never a fabricated preview.
    assert!(saturn_render::preview_tools::media_preview_payload(&document, 999, None).is_err());
}

#[test]
fn frame_sheet_rejects_missing_sources_and_bad_ranges() {
    let mut engine = saturn_core::EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP sheet"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mp4", "path": "/assets/clip.mp4", "kind": "video"}),
        ))
        .unwrap();
    let document = engine.document().clone();
    let media_id = document.project.media[0].id;

    // A source that is not on disk is a typed error, not a placeholder.
    let error = saturn_render::preview_tools::frame_sheet_payload(
        &document,
        &json!({"media_id": media_id, "start_tick": 0, "end_tick": 1_000}),
    )
    .unwrap_err();
    assert!(error.contains("missing"), "unexpected error: {error}");

    // An end past the (unknown) duration with no file never gets that far;
    // a decodable-file span check is exercised in the extractor tests.
    let error = saturn_render::preview_tools::frame_sheet_payload(
        &document,
        &json!({"start_tick": 0, "end_tick": 1_000}),
    )
    .unwrap_err();
    assert!(error.contains("media_id"), "unexpected error: {error}");
}

#[tokio::test]
async fn bridge_socket_prep_covers_stale_live_and_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("bridge.sock");

    // A stale socket file (no listener answers) is removed and replaced.
    std::fs::write(&path, b"not a socket").unwrap();
    let listener = prepare_bridge_socket(&path).await.expect("stale socket recovery");
    drop(listener);
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600,
        "bridge socket is private to the user"
    );
    // Dropping a listener leaves its socket file behind - that stale file
    // is exactly what the crash-recovery path must clear on next start.
    let _ = std::fs::remove_file(&path);

    // A live listener owns the socket: the second editor must refuse to
    // start its bridge instead of stealing it.
    let owner = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let error = prepare_bridge_socket(&path).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
    drop(owner);
    let _ = std::fs::remove_file(&path);

    // Fresh bind works, creating parent directories as needed.
    let nested = folder.path().join("deep/new/dir");
    let fresh = prepare_bridge_socket(&nested.join("bridge.sock"))
        .await
        .expect("fresh bind with created parents");
    drop(fresh);
}
