// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Tests for the MCP server surface (`server.rs`).

use super::*;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};

#[tokio::test]
async fn audio_tool_errors_and_forwards_exact_bridge_request() {
    let mcp = EditorMcp::new(EditorEngine::default());
    let error = mcp.audio_process(&json!({"operation": "duck", "dialogue_track_id": 2, "music_track_id": 3})).await.unwrap_err();
    assert!(error.contains("Save the project"), "{error}");
    let path = std::env::temp_dir().join(format!("eep-audio-tool-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&path).unwrap();
    let client = EditorMcp { backend: Backend::Bridge(path.clone()), jobs: Arc::new(JobManager::new(1)) };
    let args = json!({"operation": "normalize_master", "standard": "streaming", "true_peak_dbtp": -1.0});
    let expected = args.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "audio_process");
        assert_eq!(request["params"], expected);
        write.write_all(b"{\"error\":\"locked track\"}\n").await.unwrap();
    });
    assert!(client.audio_process(&args).await.unwrap_err().contains("locked track"));
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn media_preview_never_returns_a_source_path_placeholder() {    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP preview"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mp4", "path": "/tmp/nonexistent-clip.mp4", "kind": "video"}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;

    let mcp = EditorMcp::new(engine);
    let result = mcp
        .media_preview(&json!({"media_id": media_id}))
        .await
        .expect("media_preview on a real asset");
    assert!(result.get("preview_path").is_none(), "no source-path placeholder");
    assert_eq!(result["still_frame"]["available"], false);

    // Unknown ids are a typed error, not a fabricated payload.
    let missing = mcp.media_preview(&json!({"media_id": 4242})).await;
    assert!(missing.is_err());
}

#[tokio::test]
async fn bridge_client_reconnects_after_server_restart() {
    let path =
        std::env::temp_dir().join(format!("eep-mcp-reconnect-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let client = EditorMcp {
        backend: Backend::Bridge(path.clone()),
        jobs: Arc::new(JobManager::new(1)),
    };

    for expected in ["first", "second"] {
        let listener = UnixListener::bind(&path).expect("bind bridge socket");
        let response = expected.to_owned();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept bridge request");
            let (read, mut write) = stream.into_split();
            let mut line = String::new();
            BufReader::new(read)
                .read_line(&mut line)
                .await
                .expect("read request");
            let request: Value = serde_json::from_str(&line).expect("valid request json");
            assert_eq!(request["method"], "lifecycle_test");
            write
                .write_all(json!({"result": response}).to_string().as_bytes())
                .await
                .expect("write response");
            write.write_all(b"\n").await.expect("terminate response");
        });
        let result = client
            .bridge_call("lifecycle_test", json!({}))
            .await
            .expect("bridge reconnect call");
        assert_eq!(result, expected);
        server.await.expect("mock bridge task");
        let _ = std::fs::remove_file(&path);
    }

}
#[tokio::test]
async fn vision_tools_reject_missing_sources_with_typed_errors() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP vision"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mp4", "path": "/tmp/nonexistent-clip.mp4", "kind": "video"}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;

    // Frame sheet: audio media has no frames to sheet (added before the
    // engine moves into the MCP instance).
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "song.wav", "path": "/tmp/nonexistent-song.wav", "kind": "audio"}),
        ))
        .unwrap();
    let audio_id = engine.document().project.media[1].id;

    let mcp = EditorMcp::new(engine);

    // Frame sheet: a source that is not on disk is a typed error.
    let error = mcp
        .vision_extract_frame_sheet(&json!({
            "media_id": media_id, "start_tick": 0, "end_tick": 1_000
        }))
        .await
        .expect_err("missing source must be a typed error, not a placeholder");
    assert!(error.contains("missing"), "unexpected error: {error}");

    let error = mcp
        .vision_extract_frame_sheet(&json!({
            "media_id": audio_id, "start_tick": 0, "end_tick": 1_000
        }))
        .await
        .expect_err("audio must be rejected with a reason");
    assert!(error.contains("audio"), "unexpected error: {error}");

    // Proxy: span over 10 seconds is rejected before any render starts.
    let error = mcp
        .render_preview_clip_proxy(&json!({
            "start_tick": 0,
            "end_tick": 11 * saturn_core::time::TICKS_PER_SECOND
        }))
        .await
        .expect_err("over-long proxy must be rejected");
    assert!(error.contains("10 seconds"), "unexpected error: {error}");

    // Proxy: an empty timeline is rejected.
    let error = mcp
        .render_preview_clip_proxy(&json!({"start_tick": 0, "end_tick": 1_000}))
        .await
        .expect_err("empty timeline must be rejected");
    assert!(error.contains("no clips"), "unexpected error: {error}");
}

#[tokio::test]
async fn color_read_scopes_rejects_bad_requests_with_typed_errors() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP scopes"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "song.wav", "path": "/tmp/nonexistent-song.wav", "kind": "audio"}),
        ))
        .unwrap();
    let audio_id = engine.document().project.media[0].id;
    let mcp = EditorMcp::new(engine);

    // Neither mode selected is a usage error, not a placeholder.
    let error = mcp
        .color_read_scopes(&json!({}))
        .await
        .expect_err("modeless call must be a typed error");
    assert!(error.contains("media_id+tick or at_tick"), "unexpected: {error}");

    // Audio has no video frames to scope.
    let error = mcp
        .color_read_scopes(&json!({"media_id": audio_id, "tick": 0}))
        .await
        .expect_err("audio media must be rejected with a reason");
    assert!(error.contains("audio"), "unexpected error: {error}");
}

#[tokio::test]
async fn audio_read_telemetry_rejects_bad_requests_with_typed_errors() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP audio"}),
        ))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    // Neither mode selected is a usage error, not a placeholder.
    let error = mcp
        .audio_read_telemetry(&json!({}))
        .await
        .expect_err("modeless call must be a typed error");
    assert!(error.contains("media_id, or start_tick+end_tick"), "unexpected: {error}");

    // Unknown media id.
    let error = mcp
        .audio_read_telemetry(&json!({"media_id": 4242}))
        .await
        .expect_err("unknown media must be a typed error");
    assert!(error.contains("was not found"), "unexpected error: {error}");

    // Track scoping is honestly rejected instead of silently ignored.
    let error = mcp
        .audio_read_telemetry(&json!({"start_tick": 0, "end_tick": 1, "track_id": 7}))
        .await
        .expect_err("track scoping must be rejected until implemented");
    assert!(error.contains("track-scoped"), "unexpected error: {error}");
}

#[tokio::test]
async fn semantic_query_and_cut_continuity_reject_bad_requests_honestly() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "MCP semantic"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mp4", "path": "/tmp/nonexistent-clip.mp4", "kind": "video"}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;
    let mcp = EditorMcp::new(engine);

    // Blank / missing query is a usage error.
    let error = mcp
        .media_semantic_query(&json!({}))
        .await
        .expect_err("query required");
    assert!(error.contains("query"), "unexpected: {error}");

    // In-memory (never-saved) projects have no sidecar path — honest error,
    // not an empty result.
    let error = mcp
        .media_semantic_query(&json!({"query": "green grass"}))
        .await
        .expect_err("unsaved project must error");
    assert!(error.contains("saved project"), "unexpected: {error}");

    // Unknown media id for the cut tool.
    let error = mcp
        .edit_evaluate_cut_continuity(&json!({"media_id": 4242, "cut_tick": 500}))
        .await
        .expect_err("unknown media must be a typed error");
    assert!(error.contains("was not found"), "unexpected: {error}");

    // Known media but missing source file: typed error naming the path.
    let error = mcp
        .edit_evaluate_cut_continuity(&json!({"media_id": media_id, "cut_tick": 500}))
        .await
        .expect_err("missing source must be a typed error");
    assert!(error.contains("missing"), "unexpected: {error}");

    // cut_tick 0 has no outgoing frame.
    let error = mcp
        .edit_evaluate_cut_continuity(&json!({"media_id": media_id, "cut_tick": 0}))
        .await
        .expect_err("cut at the first frame has no outgoing side");
    assert!(error.contains("at least 1"), "unexpected: {error}");
}

#[test]
fn image_block_in_the_response_becomes_a_real_image_content_block() {
    // continue_text_only strips the base64 blob; the text stays readable JSON.
    let value = json!({
        "ok": true,
        "png_path": "/previews/sheet.png",
        "image_block": {"data": "AAAA", "mime_type": "image/jpeg"},
        "still_frame": {"available": true, "data_base64": "AAAA"}
    });
    let text = continue_text_only(&value);
    let parsed: Value = serde_json::from_str(&text).expect("text stays valid JSON");
    assert!(parsed.get("image_block").is_none());
    assert!(parsed["still_frame"].get("data_base64").is_none());
    assert_eq!(parsed["png_path"], "/previews/sheet.png");
}

#[tokio::test]
async fn color_match_shot_reports_typed_errors_headless() {
    use saturn_core::CommandRequest;
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/tmp/eep-mcp-match-a.mp4", "kind": "video"}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "b.mp4", "path": "/tmp/eep-mcp-match-b.mp4", "kind": "video"}),
        ))
        .unwrap();
    let media_a = engine.document().project.media[0].id;
    let media_b = engine.document().project.media[1].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_a, "track_id": 1, "timeline_start": 0}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_b, "track_id": 1, "timeline_start": 0}),
        ))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    // Unknown clip index: a typed parameter error, never a fake success.
    let error = mcp
        .color_match_shot_to_reference(&json!({
            "src_track_id": 1, "src_clip_index": 9,
            "ref_track_id": 1, "ref_clip_index": 1,
        }))
        .await
        .expect_err("missing clip must be a typed error");
    assert!(error.contains("does not exist"), "unexpected: {error}");

    // Missing source file is a typed error naming the file.
    let error = mcp
        .color_match_shot_to_reference(&json!({
            "src_track_id": 1, "src_clip_index": 0,
            "ref_track_id": 1, "ref_clip_index": 1,
        }))
        .await
        .expect_err("missing file must be a typed error");
    assert!(error.contains("source file is missing"), "unexpected: {error}");
}
