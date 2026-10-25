//! Three-point edits, edit-point inspection, and durable settings.

use saturn_core::{CommandRequest, DispatchError, EditorEngine, TICKS_PER_SECOND};
use serde_json::{json, Value};

fn setup(engine: &mut EditorEngine) -> u64 {
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Three point"}),
        ))
        .unwrap();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "scene.mp4",
                "path": "/assets/scene.mp4",
                "kind": "video",
                "duration": TICKS_PER_SECOND * 30,
                "width": 1920,
                "height": 1080
            }),
        ))
        .unwrap();
    media["media_id"].as_u64().unwrap()
}

fn add_clip(engine: &mut EditorEngine, media_id: u64, start: i64, duration: i64, source: i64) {
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "media_id": media_id,
                "track_id": 1,
                "timeline_start": start,
                "source_in": source,
                "duration": duration
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

fn insert(engine: &mut EditorEngine, media_id: u64, at: i64, duration: i64) -> Value {
    engine
        .execute(CommandRequest::new(
            "timeline.insert_clip",
            json!({
                "media_id": media_id,
                "track_id": 1,
                "at": at,
                "source_in": 0,
                "duration": duration
            }),
        ))
        .unwrap()
}

fn overwrite(engine: &mut EditorEngine, media_id: u64, at: i64, duration: i64) -> Value {
    engine
        .execute(CommandRequest::new(
            "timeline.overwrite_clip",
            json!({
                "media_id": media_id,
                "track_id": 1,
                "at": at,
                "source_in": TICKS_PER_SECOND,
                "duration": duration
            }),
        ))
        .unwrap()
}

#[test]
fn insert_splits_at_the_playhead_and_ripples_everything_downstream() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    add_clip(&mut engine, media, 0, TICKS_PER_SECOND * 4, 0);
    add_clip(
        &mut engine,
        media,
        TICKS_PER_SECOND * 6,
        TICKS_PER_SECOND * 2,
        0,
    );

    let result = insert(
        &mut engine,
        media,
        TICKS_PER_SECOND * 2,
        TICKS_PER_SECOND * 3,
    );
    assert_eq!(result["mode"], "insert");

    let clips = clips(&mut engine);
    let spans: Vec<(i64, i64)> = clips
        .iter()
        .map(|clip| {
            (
                clip["timeline_start"].as_i64().unwrap(),
                clip["duration"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            (0, TICKS_PER_SECOND * 2),
            (TICKS_PER_SECOND * 2, TICKS_PER_SECOND * 3),
            (TICKS_PER_SECOND * 5, TICKS_PER_SECOND * 2),
            (TICKS_PER_SECOND * 9, TICKS_PER_SECOND * 2),
        ]
    );
    // The right half of the split clip keeps source continuity.
    assert_eq!(
        clips[2]["source_in"].as_i64().unwrap(),
        TICKS_PER_SECOND * 2
    );
}

#[test]
fn overwrite_keeps_outside_handles_and_drops_covered_clips() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    add_clip(&mut engine, media, 0, TICKS_PER_SECOND * 4, 0);
    add_clip(
        &mut engine,
        media,
        TICKS_PER_SECOND * 4,
        TICKS_PER_SECOND * 2,
        0,
    );

    overwrite(
        &mut engine,
        media,
        TICKS_PER_SECOND * 2,
        TICKS_PER_SECOND * 4,
    );

    let clips = clips(&mut engine);
    let spans: Vec<(i64, i64)> = clips
        .iter()
        .map(|clip| {
            (
                clip["timeline_start"].as_i64().unwrap(),
                clip["duration"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            (0, TICKS_PER_SECOND * 2),
            (TICKS_PER_SECOND * 2, TICKS_PER_SECOND * 4),
        ]
    );
    // The overwriting clip keeps the Source In supplied by the monitor.
    assert_eq!(
        clips.last().unwrap()["source_in"].as_i64().unwrap(),
        TICKS_PER_SECOND
    );
}

#[test]
fn insert_is_one_undo_entry() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    add_clip(&mut engine, media, 0, TICKS_PER_SECOND * 2, 0);
    insert(&mut engine, media, TICKS_PER_SECOND, TICKS_PER_SECOND);

    assert_eq!(clips(&mut engine).len(), 3);
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert_eq!(clips(&mut engine).len(), 1);
    assert_eq!(
        clips(&mut engine)[0]["duration"].as_i64().unwrap(),
        TICKS_PER_SECOND * 2
    );
}

#[test]
fn inspect_reports_clip_bounds_and_trim_room() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    add_clip(
        &mut engine,
        media,
        TICKS_PER_SECOND,
        TICKS_PER_SECOND * 4,
        TICKS_PER_SECOND * 2,
    );

    let clip = engine
        .execute(CommandRequest::new(
            "timeline.inspect",
            json!({"track_id": 1, "clip_index": 0}),
        ))
        .unwrap();
    assert_eq!(clip["start"].as_i64().unwrap(), TICKS_PER_SECOND);
    assert_eq!(clip["end"].as_i64().unwrap(), TICKS_PER_SECOND * 5);
    assert_eq!(clip["source_in"].as_i64().unwrap(), TICKS_PER_SECOND * 2);

    let trim = engine
        .execute(CommandRequest::new(
            "trim.inspect",
            json!({"track_id": 1, "clip_index": 0, "edge": "end"}),
        ))
        .unwrap();
    assert_eq!(trim["cut"].as_i64().unwrap(), TICKS_PER_SECOND * 5);
    assert_eq!(trim["tail_room"].as_i64().unwrap(), TICKS_PER_SECOND * 24);
    assert_eq!(
        trim["head_room"].as_i64().unwrap(),
        TICKS_PER_SECOND * 4 - TICKS_PER_SECOND / 30
    );
}

#[test]
fn settings_round_trip_through_one_json_file() {
    let path = std::env::temp_dir().join(format!(
        "eep-settings-{}-{}.json",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&path);
    let mut engine = EditorEngine::default();
    engine.load_settings(&path);
    engine
        .execute(CommandRequest::new(
            "prefs.set",
            json!({"values": {"theme": "light", "snap": false}}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "shortcuts.set",
            json!({"shortcuts": {"playhead.end": "Ctrl+E"}}),
        ))
        .unwrap();
    // G2 purged the layout-preset family with the MDI panel engine; the
    // settings file carries no `workspaces` state and the commands now
    // resolve to the standard unknown-command error.
    let err = engine
        .execute(CommandRequest::new(
            "workspace.save",
            json!({"preset": {"id": "custom_rough", "label": "Rough", "workspace": "edit", "layouts": {}}}),
        ))
        .unwrap_err();
    assert!(
        matches!(err, DispatchError::UnknownCommand(ref id) if id == "workspace.save"),
        "workspace.save should be purged, got {err:?}"
    );

    // G4 purged the placeholder workspace switchers (library / ai /
    // export): the generic `workspace.*` interception slices the id into
    // selectDavinciWorkspace, so specs naming workspaces that do not exist
    // would brick the router. The real surfaces keep working switchers.
    for purged in ["workspace.library", "workspace.ai", "workspace.export"] {
        let err = engine
            .execute(CommandRequest::new(purged, Value::Null))
            .unwrap_err();
        assert!(
            matches!(err, DispatchError::UnknownCommand(ref id) if id == purged),
            "{purged} should be purged, got {err:?}"
        );
    }

    let mut loaded = EditorEngine::default();
    loaded.load_settings(&path);
    let prefs = loaded
        .execute(CommandRequest::new("prefs.get", Value::Null))
        .unwrap();
    assert_eq!(prefs["prefs"]["theme"], "light");
    assert_eq!(prefs["prefs"]["snap"], false);
    let shortcuts = loaded
        .execute(CommandRequest::new("shortcuts.list", Value::Null))
        .unwrap();
    assert_eq!(shortcuts["shortcuts"]["playhead.end"], "Ctrl+E");
    let err = loaded
        .execute(CommandRequest::new("workspace.list", Value::Null))
        .unwrap_err();
    assert!(
        matches!(err, DispatchError::UnknownCommand(ref id) if id == "workspace.list"),
        "workspace.list should be purged, got {err:?}"
    );
    let _ = std::fs::remove_file(path);
}
