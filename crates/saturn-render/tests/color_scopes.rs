// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 1.2 evidence: `color_read_scopes` end-to-end through the shared
//! payload builder — media mode (direct decode) and sequence mode (graded
//! `render_frame` output analyzed). Skips when the environment lacks the
//! GStreamer decode plugins or the fixture; run laptop-native for evidence.

use saturn_core::{CommandRequest, EditorEngine, Tick};
use saturn_render::preview_tools::color_scopes_payload;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// The media-mode decode and the sequence-mode GES render deadlock when
/// their GStreamer pipelines race each other's global init in parallel test
/// threads (observed: both hang forever). GStreamer work in this file is
/// serialized behind one gate; each test measures the same thing.
fn gstreamer_gate() -> MutexGuard<'static, ()> {
    static GATE: OnceLock<Mutex<()>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fixture() -> Option<PathBuf> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/saturn-tauri/tests/fixtures/editor-smoke.webm");
    if !fixture.is_file() {
        return None;
    }
    let _ = gstreamer::init();
    if gstreamer::ElementFactory::find("uridecodebin").is_none() {
        return None;
    }
    Some(fixture)
}

fn engine_with_fixture(fixture: &std::path::Path) -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            serde_json::json!({"name": "Scopes"}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            serde_json::json!({
                "name": fixture.file_name().unwrap().to_string_lossy(),
                "path": fixture.to_string_lossy(),
                "kind": "video"
            }),
        ))
        .unwrap();
    engine
}

#[test]
fn color_scopes_media_mode_reports_measurable_telemetry() {
    let _gate = gstreamer_gate();
    let Some(fixture) = fixture() else {
        eprintln!("skipping: no fixture or no GStreamer decode plugins");
        return;
    };
    let engine = engine_with_fixture(&fixture);
    let media_id = engine.document().project.media[0].id;
    let tick = Tick(saturn_core::time::TICKS_PER_SECOND); // 1 s into the fixture

    let payload =
        color_scopes_payload(engine.document(), &serde_json::json!({"media_id": media_id, "tick": tick.0}))
            .expect("media-mode scopes analyze");
    let analysis = &payload["analysis"];
    assert_eq!(payload["source"], "media editor-smoke.webm");
    assert_eq!(analysis["atTick"], tick.0);
    for key in ["lumaIreP10", "lumaIreP50", "lumaIreP90"] {
        let value = analysis[key].as_f64().expect(key);
        assert!((0.0..=100.0).contains(&value), "{key} = {value}");
    }
    for key in ["clippedHighlightsPct", "crushedShadowsPct", "vectorscopeMeanSatPct"] {
        let value = analysis[key].as_f64().expect(key);
        assert!((0.0..=100.0).contains(&value), "{key} = {value}");
    }
    assert_eq!(analysis["rgbParadeMedians"].as_array().map(Vec::len), Some(3));
    // The aggregate-score honesty note: no invented quality number is present.
    assert_eq!(payload["notes"]["aggregateScore"], serde_json::Value::Null);
}

#[test]
fn color_scopes_sequence_mode_renders_then_analyzes() {
    let _gate = gstreamer_gate();
    let Some(fixture) = fixture() else {
        eprintln!("skipping: no fixture or no GStreamer decode plugins");
        return;
    };
    let mut engine = engine_with_fixture(&fixture);
    let media_id = engine.document().project.media[0].id;
    let video_track_id = engine
        .document()
        .project
        .sequence
        .tracks
        .iter()
        .find(|track| matches!(track.kind, saturn_core::TrackKind::Video))
        .unwrap()
        .id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            serde_json::json!({"media_id": media_id, "track_id": video_track_id}),
        ))
        .unwrap();

    let tick = Tick(saturn_core::time::TICKS_PER_SECOND); // 1 s, inside the clip
    let payload = color_scopes_payload(
        engine.document(),
        &serde_json::json!({"at_tick": tick.0}),
    )
    .expect("sequence-mode scopes render + analyze");
    assert_eq!(payload["source"], "sequence render");
    let analysis = &payload["analysis"];
    assert_eq!(analysis["atTick"], tick.0);
    assert!(analysis["lumaIreP50"].is_number(), "graded frame analyzed");

    // The sequence-mode analysis temp frame must not linger in the previews
    // dir (it is analysis scratch, not an artifact).
    let previews = saturn_render::frame_extractor::previews_dir().unwrap();
    let stray = previews.join(format!("scopes-frame-{}.png", tick.0));
    assert!(!stray.exists(), "temp scopes frame must be removed: {stray:?}");
}

#[test]
fn color_scopes_param_errors_are_typed() {
    let engine = EditorEngine::default();
    // Neither mode selected.
    let error = color_scopes_payload(engine.document(), &serde_json::json!({})).unwrap_err();
    assert!(error.contains("media_id+tick or at_tick"), "unexpected: {error}");
    // Unknown media id.
    let error = color_scopes_payload(
        engine.document(),
        &serde_json::json!({"media_id": 4242, "tick": 0}),
    )
    .unwrap_err();
    assert!(error.contains("was not found"), "unexpected: {error}");
    // Malformed ROI.
    let error = color_scopes_payload(
        engine.document(),
        &serde_json::json!({"at_tick": 0, "roi": {"x": 1}}),
    )
    .unwrap_err();
    assert!(error.contains("roi must carry"), "unexpected: {error}");
}
