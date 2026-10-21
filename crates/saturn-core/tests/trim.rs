//! Trim maths, ripple behaviour, and gesture-scoped undo for Phase 2.

use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::{Value, json};

fn setup(engine: &mut EditorEngine) -> u64 {
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "Trim"})))
        .unwrap();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "scene.mp4",
                "path": "/assets/scene.mp4",
                "kind": "video",
                "duration": TICKS_PER_SECOND * 20,
                "width": 1920,
                "height": 1080
            }),
        ))
        .unwrap();
    media["media_id"].as_u64().unwrap()
}

fn add_clip(
    engine: &mut EditorEngine,
    media_id: u64,
    start_seconds: i64,
    duration_seconds: i64,
    source_seconds: i64,
) {
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "media_id": media_id,
                "track_id": 1,
                "timeline_start": TICKS_PER_SECOND * start_seconds,
                "source_in": TICKS_PER_SECOND * source_seconds,
                "duration": TICKS_PER_SECOND * duration_seconds
            }),
        ))
        .unwrap();
}

fn clips(engine: &mut EditorEngine) -> Vec<Value> {
    let status = engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap();
    status["document"]["project"]["sequence"]["tracks"][0]["clips"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn trim(
    engine: &mut EditorEngine,
    clip_index: usize,
    edge: &str,
    delta_seconds: i64,
    ripple: bool,
) -> Value {
    engine
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id": 1,
                "clip_index": clip_index,
                "edge": edge,
                "delta": TICKS_PER_SECOND * delta_seconds,
                "ripple": ripple
            }),
        ))
        .unwrap()
}

#[test]
fn plain_trim_moves_the_edge_and_preserves_source_continuity() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 2, 4, 1);

    let trimmed = trim(&mut engine, 0, "start", 1, false);
    assert_eq!(trimmed["timeline_start"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(trimmed["duration"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(trimmed["source_in"], json!(TICKS_PER_SECOND * 2));

    let trimmed = trim(&mut engine, 0, "end", -1, false);
    assert_eq!(trimmed["timeline_start"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(trimmed["duration"], json!(TICKS_PER_SECOND * 2));
    assert_eq!(trimmed["source_in"], json!(TICKS_PER_SECOND * 2));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let clip = &clips(&mut engine)[0];
    assert_eq!(clip["timeline_start"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(clip["duration"], json!(TICKS_PER_SECOND * 3));
}

#[test]
fn ripple_trim_head_keeps_the_clip_anchored_and_closes_the_gap() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 0, 2, 0);
    add_clip(&mut engine, media_id, 2, 2, 0);
    add_clip(&mut engine, media_id, 4, 2, 0);

    let trimmed = trim(&mut engine, 0, "start", 1, true);
    assert_eq!(trimmed["timeline_start"], json!(0));
    assert_eq!(trimmed["duration"], json!(TICKS_PER_SECOND));

    let timeline = clips(&mut engine);
    assert_eq!(timeline[0]["source_in"], json!(TICKS_PER_SECOND));
    assert_eq!(timeline[1]["timeline_start"], json!(TICKS_PER_SECOND));
    assert_eq!(timeline[2]["timeline_start"], json!(TICKS_PER_SECOND * 3));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let timeline = clips(&mut engine);
    assert_eq!(timeline[1]["timeline_start"], json!(TICKS_PER_SECOND * 2));
    assert_eq!(timeline[2]["timeline_start"], json!(TICKS_PER_SECOND * 4));
}

#[test]
fn ripple_trim_tail_then_undo_restores_exact_ticks() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 0, 2, 0);
    add_clip(&mut engine, media_id, 2, 2, 0);
    add_clip(&mut engine, media_id, 4, 2, 0);

    trim(&mut engine, 0, "end", 1, true);
    let timeline = clips(&mut engine);
    assert_eq!(timeline[0]["duration"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(timeline[1]["timeline_start"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(timeline[2]["timeline_start"], json!(TICKS_PER_SECOND * 5));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let timeline = clips(&mut engine);
    assert_eq!(timeline[0]["duration"], json!(TICKS_PER_SECOND * 2));
    assert_eq!(timeline[1]["timeline_start"], json!(TICKS_PER_SECOND * 2));
    assert_eq!(timeline[2]["timeline_start"], json!(TICKS_PER_SECOND * 4));
}

#[test]
fn trim_rejects_ranges_outside_the_media_and_keeps_history_clean() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 0, 2, 18);

    let error = engine
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "edge": "end",
                "delta": TICKS_PER_SECOND,
                "ripple": false
            }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("media duration"), "{error}");

    let error = engine
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "edge": "end",
                "delta": -TICKS_PER_SECOND * 2,
                "ripple": false
            }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("whole clip"), "{error}");

    // Failed trims never checkpoint, so the next undo removes the clip that
    // was added before them rather than restoring a rejected trim.
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert!(clips(&mut engine).is_empty());
}

#[test]
fn one_gesture_wraps_trim_and_move_into_a_single_undo_entry() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 0, 2, 0);

    engine.begin_gesture();
    trim(&mut engine, 0, "end", 1, false);
    engine
        .execute(CommandRequest::new(
            "timeline.move_clip",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "target_track_id": 1,
                "timeline_start": TICKS_PER_SECOND * 2
            }),
        ))
        .unwrap();
    engine.end_gesture();

    let timeline = clips(&mut engine);
    assert_eq!(timeline[0]["duration"], json!(TICKS_PER_SECOND * 3));
    assert_eq!(timeline[0]["timeline_start"], json!(TICKS_PER_SECOND * 2));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let timeline = clips(&mut engine);
    assert_eq!(timeline[0]["duration"], json!(TICKS_PER_SECOND * 2));
    assert_eq!(timeline[0]["timeline_start"], json!(0));
}

#[test]
fn transition_updates_change_kind_and_duration_in_place() {
    let mut engine = EditorEngine::default();
    let media_id = setup(&mut engine);
    add_clip(&mut engine, media_id, 0, 2, 0);
    add_clip(&mut engine, media_id, 2, 2, 0);

    let added = engine
        .execute(CommandRequest::new(
            "timeline.add_transition",
            json!({
                "track_id": 1,
                "cut_at": TICKS_PER_SECOND * 2,
                "duration": TICKS_PER_SECOND / 2,
                "kind": "crossfade"
            }),
        ))
        .unwrap();
    let transition_id = added["transition_id"].as_u64().unwrap();

    engine
        .execute(CommandRequest::new(
            "timeline.update_transition",
            json!({
                "track_id": 1,
                "transition_id": transition_id,
                "kind": "dip_to_black",
                "duration": TICKS_PER_SECOND,
                "cut_at": TICKS_PER_SECOND * 2
            }),
        ))
        .unwrap();

    let status = engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap();
    let transition = &status["document"]["project"]["sequence"]["tracks"][0]["transitions"][0];
    assert_eq!(transition["kind"], "dip_to_black");
    assert_eq!(transition["duration"], json!(TICKS_PER_SECOND));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let status = engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap();
    let transition = &status["document"]["project"]["sequence"]["tracks"][0]["transitions"][0];
    assert_eq!(transition["kind"], "crossfade");
}
