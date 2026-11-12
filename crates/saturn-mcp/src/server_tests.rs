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
async fn beat_grid_errors_and_forwards_exact_bridge_request() {
    let mcp = EditorMcp::new(EditorEngine::default());
    let args = json!({"video_track_id":1,"music_track_id":2,"subdivision":"quarter","strength":1.0});
    assert!(mcp.timeline_beat_grid_quantize(&args).await.unwrap_err().contains("continuous video strip"));
    let path = std::env::temp_dir().join(format!("eep-beat-tool-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&path).unwrap();
    let client = EditorMcp::bridge_with_path(path.clone());
    let expected = args.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "timeline_beat_grid_quantize");
        assert_eq!(request["params"], expected);
        write.write_all(b"{\"error\":\"no stable pulse\"}\n").await.unwrap();
    });
    assert!(client.timeline_beat_grid_quantize(&args).await.unwrap_err().contains("no stable pulse"));
    server.await.unwrap(); std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn beat_grid_headless_prepares_applies_and_returns_measurements() {
    use saturn_core::{audio_edit::audio_clip, MediaAsset, MediaKind, ProjectDocument, Tick, TICKS_PER_SECOND};
    let f = |n| Tick(n * (TICKS_PER_SECOND / 30));
    let dir = std::env::temp_dir().join(format!("eep-beat-headless-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pulse.wav");
    let mut samples = vec![0.0; 16000 * 8];
    for at in (8000..samples.len()-480).step_by(8000) {
        for i in 0..480 { samples[at+i] = 0.5 * (1.0-i as f32/480.0); }
    }
    saturn_render::audio_edit_files::write_wav(&mut std::fs::File::create(&path).unwrap(), &samples, 1, 16000).unwrap();
    let mut d = ProjectDocument::new("MCP beat edit");
    d.project.media = vec![
        MediaAsset { id: 3, path: "/picture.mkv".into(), kind: MediaKind::Video, duration: Some(f(600)), ..Default::default() },
        MediaAsset { id: 4, path, kind: MediaKind::Audio, duration: Some(f(240)), audio_channels: Some(1), ..Default::default() },
    ];
    let mut right = audio_clip(3, f(82), f(98)); right.source_in = f(60);
    d.project.sequence.tracks[0].clips = vec![audio_clip(3, f(0), f(82)), right];
    d.project.sequence.tracks[1].clips = vec![audio_clip(4, f(0), f(240))];
    d.project.next_id = 5;
    let mut engine = EditorEngine::default(); engine.restore_recovery(d, None).unwrap();
    let mcp = EditorMcp::new(engine);
    let result = mcp.timeline_beat_grid_quantize(&json!({"music_track_id":2,"video_track_id":1,"subdivision":"quarter","strength":1})).await.unwrap();
    assert_eq!(result["applied"]["changed_cuts"], 1);
    assert_eq!(result["applied"]["cuts"][0]["tick"], json!(f(75)));
    assert!((result["telemetry"]["tempo_bpm"].as_f64().unwrap()-120.0).abs()<0.01);
    assert_eq!(mcp.headless_document().await.unwrap().project.sequence.tracks[0].clips[1].timeline_start, f(75));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn match_cut_errors_and_forwards_exact_bridge_request() {
    let mcp = EditorMcp::new(EditorEngine::default());
    let args = json!({"track_id":1,"left_clip_index":0,"right_clip_index":1,"mode":"motion"});
    assert!(mcp.edit_match_cut_align(&args).await.unwrap_err().contains("Outgoing clip"));
    let path = std::env::temp_dir().join(format!("eep-match-tool-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&path).unwrap();
    let client = EditorMcp::bridge_with_path(path.clone());
    let expected = args.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "edit_match_cut_align");
        assert_eq!(request["params"], expected);
        write.write_all(b"{\"error\":\"no measurable motion\"}\n").await.unwrap();
    });
    assert!(client.edit_match_cut_align(&args).await.unwrap_err().contains("no measurable motion"));
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn audio_tool_errors_and_forwards_exact_bridge_request() {
    let mcp = EditorMcp::new(EditorEngine::default());
    let error = mcp.audio_process(&json!({"operation": "duck", "dialogue_track_id": 2, "music_track_id": 3})).await.unwrap_err();
    assert!(error.contains("Save the project"), "{error}");
    let path = std::env::temp_dir().join(format!("eep-audio-tool-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&path).unwrap();
    let client = EditorMcp::bridge_with_path(path.clone());
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
    let client = EditorMcp::bridge_with_path(path.clone());

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

#[tokio::test]
async fn list_tools_default_is_bounded_and_filters_by_profile() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    let tools = mcp.active_tools().await;
    assert!(
        tools.len() <= 30,
        "default tools must be <= 30, got {}",
        tools.len()
    );
    assert_eq!(
        tools.len(),
        16,
        "expected exactly 14 core + 2 rough-cut tools"
    );

    let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"tools.list_profiles"));
    assert!(names.contains(&"tools.load_profile"));
    assert!(names.contains(&"command_list"));
    assert!(names.contains(&"command_run"));
    assert!(names.contains(&"command_batch"));
    assert!(names.contains(&"session.stop"));
    assert!(names.contains(&"session.resume"));
    assert!(names.contains(&"session.status"));
    assert!(names.contains(&"history.undo_change"));
    assert!(names.contains(&"doc_inspect"));
    assert!(names.contains(&"sequence_inspect"));
    assert!(names.contains(&"media_search"));
    assert!(names.contains(&"media_preview"));
    assert!(names.contains(&"vision_extract_frame_sheet"));
    assert!(names.contains(&"edit_match_cut_align"));
    assert!(names.contains(&"media_semantic_query"));
    assert!(!names.contains(&"color_match_shot_to_reference"));
    assert!(!names.contains(&"audio_process"));
}

#[tokio::test]
async fn profile_discovery_and_loading_enables_new_tools() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    // 1. tools.list_profiles
    let list_req = CallToolRequestParams::new("tools.list_profiles");
    let res = mcp.call(list_req).await.unwrap();
    let res_text = match &res.content[0].raw {
        RawContent::Text(t) => &t.text,
        _ => panic!("expected text content"),
    };
    let json: Value = serde_json::from_str(res_text).unwrap();
    assert_eq!(json["mode"], "autopilot");
    assert!(
        json["active_profiles"]
            .as_array()
            .unwrap()
            .contains(&json!("rough-cut"))
    );
    let profiles = json["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 13);
    let colour_prof = profiles.iter().find(|p| p["name"] == "colour").unwrap();
    assert_eq!(colour_prof["active"], false);

    // 2. tools.load_profile("colour")
    let mut load_params = serde_json::Map::new();
    load_params.insert("profile".to_string(), json!("colour"));
    let load_req = CallToolRequestParams::new("tools.load_profile").with_arguments(load_params);
    let load_res = mcp.call(load_req).await.unwrap();
    let load_text = match &load_res.content[0].raw {
        RawContent::Text(t) => &t.text,
        _ => panic!("expected text content"),
    };
    let load_json: Value = serde_json::from_str(load_text).unwrap();
    assert_eq!(load_json["ok"], true);
    assert_eq!(load_json["newly_activated"], true);

    // 3. list_tools now includes colour tools
    let tools = mcp.active_tools().await;
    assert!(tools.len() <= 30);
    let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"color_match_shot_to_reference"));
    assert!(names.contains(&"color_read_scopes"));
}

#[tokio::test]
async fn wrong_profile_returns_typed_profile_required_hint() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    // Calling a colour command without loading colour profile
    let mut run_params = serde_json::Map::new();
    run_params.insert("id".to_string(), json!("color.node_add"));
    let req = CallToolRequestParams::new("command_run").with_arguments(run_params);
    let err = mcp.call(req).await.unwrap_err();
    assert!(
        err.message
            .contains("ProfileRequired: command 'color.node_add' requires profile 'colour'")
    );
    assert!(err.message.contains("Call tools.load_profile."));

    // Calling dedicated colour tool without loading colour profile
    let mut match_params = serde_json::Map::new();
    match_params.insert("src_track_id".to_string(), json!(0));
    match_params.insert("src_clip_index".to_string(), json!(0));
    match_params.insert("ref_track_id".to_string(), json!(0));
    match_params.insert("ref_clip_index".to_string(), json!(0));
    let tool_req = CallToolRequestParams::new("color_match_shot_to_reference")
        .with_arguments(match_params);
    let err = mcp.call(tool_req).await.unwrap_err();
    assert!(err.message.contains(
        "ProfileRequired: tool 'color_match_shot_to_reference' requires profile 'colour'"
    ));
    assert!(err.message.contains("Call tools.load_profile."));
}

#[tokio::test]
async fn command_batch_enforces_profiles_and_stops_on_error() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    let mut batch_params = serde_json::Map::new();
    batch_params.insert(
        "steps".to_string(),
        json!([
            { "id": "color.node_add", "params": {} },
            { "id": "edit.undo", "params": {} }
        ]),
    );
    batch_params.insert("stop_on_error".to_string(), json!(true));
    let req = CallToolRequestParams::new("command_batch").with_arguments(batch_params);
    let res = mcp.call(req).await.unwrap();
    let res_text = match &res.content[0].raw {
        RawContent::Text(t) => &t.text,
        _ => panic!("expected text content"),
    };
    let json: Value = serde_json::from_str(res_text).unwrap();
    assert_eq!(json["failed"], 1);
    assert_eq!(json["completed"], 0);
    assert!(json["results"][0]["error"].as_str().unwrap().contains(
        "ProfileRequired: command 'color.node_add' requires profile 'colour'"
    ));
}

#[tokio::test]
async fn loading_profile_cannot_elevate_permission_tiers() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    // Load export profile
    let mut load_params = serde_json::Map::new();
    load_params.insert("profile".to_string(), json!("export"));
    mcp.call(
        CallToolRequestParams::new("tools.load_profile").with_arguments(load_params),
    )
    .await
    .unwrap();

    // render_project requires Filesystem tier; session only has [Read, Edit]
    let mut render_params = serde_json::Map::new();
    render_params.insert("output_path".to_string(), json!("/tmp/output.mp4"));
    let req = CallToolRequestParams::new("render_project").with_arguments(render_params);
    let err = mcp.call(req).await.unwrap_err();
    assert!(
        err.message
            .contains("PermissionDenied: tool 'render_project' requires tier 'filesystem'")
    );
}

#[tokio::test]
async fn review_session_mode_is_strictly_read_only() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new_with_policy(engine, SessionPolicy::review());

    // Tools in review mode
    let tools = mcp.active_tools().await;
    assert!(tools.len() <= 30);

    // Mutating command is refused
    let mut run_params = serde_json::Map::new();
    run_params.insert("id".to_string(), json!("timeline.split_clip"));
    run_params.insert(
        "params".to_string(),
        json!({"track_id": 0, "clip_index": 0, "at": {"ticks": 100}}),
    );
    let req = CallToolRequestParams::new("command_run").with_arguments(run_params);
    let err = mcp.call(req).await.unwrap_err();
    assert!(
        err.message
            .contains("PermissionDenied: session in review mode is read-only")
    );
}

#[tokio::test]
async fn mcp_session_stop_resume_and_atomic_batch() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "MCP AF2 Test"})))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    // 1. session.status
    let req = CallToolRequestParams::new("session.status");
    let res = mcp.call(req).await.unwrap();
    assert!(!res.is_error.unwrap_or(false));

    // 2. session.stop
    let mut stop_args = serde_json::Map::new();
    stop_args.insert("session_id".to_string(), json!("agent-mcp-1"));
    let req = CallToolRequestParams::new("session.stop").with_arguments(stop_args);
    let res = mcp.call(req).await.unwrap();
    assert!(!res.is_error.unwrap_or(false));

    // Verify stopped
    let mut status_args = serde_json::Map::new();
    status_args.insert("session_id".to_string(), json!("agent-mcp-1"));
    let req = CallToolRequestParams::new("session.status").with_arguments(status_args);
    let res = mcp.call(req).await.unwrap();
    let text = res.content[0].as_text().unwrap();
    assert!(text.text.contains("\"is_stopped\": true"));

    // 3. session.resume with human authority
    let mut resume_args = serde_json::Map::new();
    resume_args.insert("session_id".to_string(), json!("agent-mcp-1"));
    resume_args.insert("caller".to_string(), json!("human"));
    let req = CallToolRequestParams::new("session.resume").with_arguments(resume_args);
    let res = mcp.call(req).await.unwrap();
    assert!(!res.is_error.unwrap_or(false));

    // 4. command_batch with atomic: true rollback on error
    let mut load_args = serde_json::Map::new();
    load_args.insert("profile".to_string(), json!("admin"));
    let req = CallToolRequestParams::new("tools.load_profile").with_arguments(load_args);
    let _ = mcp.call(req).await.unwrap();

    let mut batch_args = serde_json::Map::new();
    batch_args.insert("atomic".to_string(), json!(true));
    batch_args.insert(
        "steps".to_string(),
        json!([
            {"id": "project.branch_create", "params": {"name": "rollback-branch"}},
            {"id": "timeline.remove_clip", "params": {"track_id": 9999, "clip_index": 0}}
        ]),
    );
    let req = CallToolRequestParams::new("command_batch").with_arguments(batch_args);
    let res = mcp.call(req).await.unwrap();
    assert!(res.is_error.unwrap_or(false));
}

#[tokio::test]
async fn mcp_command_run_with_provenance_and_brief() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "MCP AF4 Test"})))
        .unwrap();
    let mcp = EditorMcp::new(engine);

    // 1. Load admin profile
    let mut load_args = serde_json::Map::new();
    load_args.insert("profile".to_string(), json!("admin"));
    let _ = mcp.call(CallToolRequestParams::new("tools.load_profile").with_arguments(load_args)).await.unwrap();

    // 2. Set brief (from human authority via context)
    *mcp.context.write().await = ActorContext::human();
    let mut set_args = serde_json::Map::new();
    set_args.insert("id".to_string(), json!("project.brief_set"));
    set_args.insert("params".to_string(), json!({"goal": "MCP Brief Test"}));
    let res = mcp.call(CallToolRequestParams::new("command_run").with_arguments(set_args)).await.unwrap();
    assert!(!res.is_error.unwrap_or(false));

    // 3. Switch to agent context and try to rewrite brief (must fail)
    *mcp.context.write().await = ActorContext::agent("agent-test", "session-1");
    let mut set_agent_args = serde_json::Map::new();
    set_agent_args.insert("id".to_string(), json!("project.brief_set"));
    set_agent_args.insert("params".to_string(), json!({"goal": "Agent Overwrite"}));
    let res = mcp.call(CallToolRequestParams::new("command_run").with_arguments(set_agent_args)).await.unwrap();
    assert!(res.is_error.unwrap_or(false));

    // 4. Get brief as agent (must succeed)
    let mut get_args = serde_json::Map::new();
    get_args.insert("id".to_string(), json!("project.brief_get"));
    let res = mcp.call(CallToolRequestParams::new("command_run").with_arguments(get_args)).await.unwrap();
    assert!(!res.is_error.unwrap_or(false));
    let text = res.content[0].as_text().unwrap();
    assert!(text.text.contains("MCP Brief Test"));
}

#[tokio::test]
async fn mcp_verify_run_headless_measured_evidence() {
    let engine = EditorEngine::default();
    let mcp = EditorMcp::new(engine);

    // 1. Load review profile
    let load_res = mcp
        .call(CallToolRequestParams::new("tools.load_profile").with_arguments({
            let mut m = serde_json::Map::new();
            m.insert("profile".into(), json!("review"));
            m
        }))
        .await
        .unwrap();
    assert!(!load_res.is_error.unwrap_or(false));

    // 2. Call verify.run
    let verify_args = {
        let mut m = serde_json::Map::new();
        m.insert("preset".into(), json!("web-1080p"));
        m.insert("checks".into(), json!(["project.resolution_aspect"]));
        m
    };
    let res = mcp
        .call(CallToolRequestParams::new("verify.run").with_arguments(verify_args))
        .await
        .unwrap();
    assert!(!res.is_error.unwrap_or(false));
    let text = res.content[0].as_text().unwrap();
    let report: Value = serde_json::from_str(&text.text).unwrap();
    assert_eq!(report["preset"], "web-1080p");
    assert!(report["notes"]["aggregate_score"].is_null());
}
