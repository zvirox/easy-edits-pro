//! Phase 4 media organisation: sequence markers.
//!
//! Every case drives the engine through the same `execute` path the UI, CLI,
//! and MCP use, so the tests cover dispatch, validation, undo, and the CSV
//! interchange format rather than a private helper.

use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::{json, Value};

fn engine() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Markers"}),
        ))
        .unwrap();
    engine
}

fn markers(engine: &mut EditorEngine) -> Vec<Value> {
    let status = engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap();
    status["document"]["project"]["sequence"]["markers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn execute(engine: &mut EditorEngine, id: &str, params: Value) -> Value {
    engine
        .execute(CommandRequest::new(id, params))
        .unwrap_or_else(|error| panic!("{id} failed: {error}"))
}

#[test]
fn add_marker_creates_a_point_marker_and_one_undo_step() {
    let mut engine = engine();
    let added = execute(
        &mut engine,
        "marker.add",
        json!({"at": TICKS_PER_SECOND, "kind": "chapter", "note": "Cold open"}),
    );
    assert_eq!(added["added"], true);
    assert_eq!(added["range"], false);

    let list = markers(&mut engine);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "Marker 1");
    assert_eq!(list[0]["kind"], "chapter");
    assert_eq!(list[0]["start_ticks"], json!(TICKS_PER_SECOND));
    assert_eq!(list[0]["duration_ticks"], json!(0));
    assert_eq!(list[0]["note"], "Cold open");

    execute(&mut engine, "edit.undo", Value::Null);
    assert!(markers(&mut engine).is_empty());
}

#[test]
fn add_range_marker_needs_an_increasing_span() {
    let mut engine = engine();
    let added = execute(
        &mut engine,
        "marker.add_range",
        json!({
            "start": TICKS_PER_SECOND,
            "end": TICKS_PER_SECOND * 3,
            "name": "Interview"
        }),
    );
    assert_eq!(added["duration"], json!(TICKS_PER_SECOND * 2));

    let list = markers(&mut engine);
    assert_eq!(list[0]["duration_ticks"], json!(TICKS_PER_SECOND * 2));

    let error = engine
        .execute(CommandRequest::new(
            "marker.add_range",
            json!({"start": TICKS_PER_SECOND * 2, "end": TICKS_PER_SECOND}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("before its end"), "{error}");
}

#[test]
fn markers_are_kept_in_timeline_order() {
    let mut engine = engine();
    execute(&mut engine, "marker.add", json!({"at": 300, "name": "Third"}));
    execute(&mut engine, "marker.add", json!({"at": 100, "name": "First"}));
    execute(&mut engine, "marker.add", json!({"at": 200, "name": "Second"}));
    let names: Vec<String> = markers(&mut engine)
        .iter()
        .map(|marker| marker["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["First", "Second", "Third"]);
}

#[test]
fn update_marker_retimes_and_renames_without_losing_the_kind() {
    let mut engine = engine();
    let id = execute(
        &mut engine,
        "marker.add",
        json!({"at": 100, "kind": "flash_cue", "name": "Flash"}),
    )["id"]
        .as_u64()
        .unwrap();

    execute(
        &mut engine,
        "marker.update",
        json!({"id": id, "at": 500, "duration": 250, "color": "#d9468f"}),
    );
    let list = markers(&mut engine);
    assert_eq!(list[0]["name"], "Flash");
    assert_eq!(list[0]["kind"], "flash_cue");
    assert_eq!(list[0]["start_ticks"], json!(500));
    assert_eq!(list[0]["duration_ticks"], json!(250));
    assert_eq!(list[0]["color"], "#d9468f");

    let error = engine
        .execute(CommandRequest::new(
            "marker.update",
            json!({"id": 99_999, "at": 0}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn update_accepts_the_kind_and_colour_strings_the_ui_sends() {
    let mut engine = engine();
    let id = execute(&mut engine, "marker.add", json!({"at": 100}))["id"]
        .as_u64()
        .unwrap();

    for kind in ["chapter", "flash_cue", "web_link", "segmentation", "comment"] {
        execute(
            &mut engine,
            "marker.update",
            json!({"id": id, "kind": kind, "color": "#d9468f", "note": "reviewed"}),
        );
        let list = markers(&mut engine);
        assert_eq!(list[0]["kind"], json!(kind));
        assert_eq!(list[0]["color"], "#d9468f");
        assert_eq!(list[0]["note"], "reviewed");
    }
}

#[test]
fn navigation_finds_the_next_and_previous_marker_around_the_playhead() {
    let mut engine = engine();
    for (at, name) in [(100_i64, "A"), (300, "B"), (500, "C")] {
        execute(&mut engine, "marker.add", json!({"at": at, "name": name}));
    }

    let next = execute(&mut engine, "marker.next", json!({"from": 100}));
    assert_eq!(next["target"], json!(300));
    assert_eq!(next["name"], "B");

    let previous = execute(&mut engine, "marker.previous", json!({"from": 300}));
    assert_eq!(previous["target"], json!(100));

    let end = execute(&mut engine, "marker.next", json!({"from": 900}));
    assert_eq!(end["target"], Value::Null);
}

#[test]
fn ripple_shifts_only_markers_after_the_anchor() {
    let mut engine = engine();
    execute(&mut engine, "marker.add", json!({"at": 100, "name": "Before"}));
    execute(&mut engine, "marker.add", json!({"at": 400, "name": "After"}));

    let report = execute(
        &mut engine,
        "marker.ripple",
        json!({"from": 200, "delta": TICKS_PER_SECOND}),
    );
    assert_eq!(report["shifted"], 1);

    let list = markers(&mut engine);
    assert_eq!(list[0]["start_ticks"], json!(100));
    assert_eq!(list[1]["start_ticks"], json!(400 + TICKS_PER_SECOND));

    // A negative delta that would pass zero clamps instead of wrapping.
    execute(
        &mut engine,
        "marker.ripple",
        json!({"from": 0, "delta": -TICKS_PER_SECOND * 100}),
    );
    assert_eq!(markers(&mut engine)[0]["start_ticks"], json!(0));
}

#[test]
fn split_turns_a_range_marker_into_two_contiguous_parts() {
    let mut engine = engine();
    execute(
        &mut engine,
        "marker.add_range",
        json!({"start": 1000, "end": 3000, "name": "Scene"}),
    );
    let report = execute(&mut engine, "marker.split", json!({"at": 2000}));
    assert_eq!(report["split"], 1);

    let list = markers(&mut engine);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["start_ticks"], json!(1000));
    assert_eq!(list[0]["duration_ticks"], json!(1000));
    assert_eq!(list[1]["start_ticks"], json!(2000));
    assert_eq!(list[1]["duration_ticks"], json!(1000));
    // Splitting outside a range is a no-op, not an error.
    let outside = execute(&mut engine, "marker.split", json!({"at": 9000}));
    assert_eq!(outside["split"], 0);
}

#[test]
fn csv_export_import_round_trips_with_zero_loss() {
    let mut engine = engine();
    execute(
        &mut engine,
        "marker.add",
        json!({
            "at": TICKS_PER_SECOND,
            "name": "Café, \"opening\"",
            "kind": "comment",
            "color": "#d9468f",
            "note": "Line one\nline two"
        }),
    );
    execute(
        &mut engine,
        "marker.add_range",
        json!({"start": 42, "end": 1042, "name": "範囲", "kind": "segmentation"}),
    );

    let exported = execute(&mut engine, "marker.export_csv", Value::Null);
    assert_eq!(exported["count"], 2);
    let csv = exported["csv"].as_str().unwrap();
    assert!(csv.starts_with("name,kind,start_ticks,duration_ticks,color,note"));

    execute(&mut engine, "marker.clear", Value::Null);
    assert!(markers(&mut engine).is_empty());

    let imported = execute(
        &mut engine,
        "marker.import_csv",
        json!({"csv": csv, "replace": true}),
    );
    assert_eq!(imported["imported"], 2);

    let restored = markers(&mut engine);
    assert_eq!(restored.len(), 2);
    assert_eq!(restored[0]["name"], "範囲");
    assert_eq!(restored[0]["kind"], "segmentation");
    assert_eq!(restored[0]["duration_ticks"], json!(1000));
    assert_eq!(restored[1]["name"], "Café, \"opening\"");
    assert_eq!(restored[1]["note"], "Line one\nline two");
    assert_eq!(restored[1]["color"], "#d9468f");
    assert_eq!(
        restored[1]["start_ticks"],
        json!(TICKS_PER_SECOND),
        "tick values must survive the trip exactly"
    );
}

#[test]
fn import_rejects_a_malformed_csv_without_mutating_the_project() {
    let mut engine = engine();
    execute(&mut engine, "marker.add", json!({"at": 100, "name": "Keep"}));
    let error = engine
        .execute(CommandRequest::new(
            "marker.import_csv",
            json!({"csv": "Bad,unknown_kind,10,0,,\n", "replace": true}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("unknown kind"), "{error}");
    let list = markers(&mut engine);
    assert_eq!(list.len(), 1, "a failed import must not clear markers");
    assert_eq!(list[0]["name"], "Keep");
}

#[test]
fn clear_and_remove_report_what_they_deleted() {
    let mut engine = engine();
    let first = execute(&mut engine, "marker.add", json!({"at": 100}))["id"]
        .as_u64()
        .unwrap();
    execute(&mut engine, "marker.add", json!({"at": 200}));

    let removed = execute(&mut engine, "marker.remove", json!({"id": first}));
    assert_eq!(removed["removed"], true);
    assert_eq!(markers(&mut engine).len(), 1);

    let cleared = execute(&mut engine, "marker.clear", Value::Null);
    assert_eq!(cleared["cleared"], 1);
    assert!(markers(&mut engine).is_empty());

    execute(&mut engine, "edit.undo", Value::Null);
    assert_eq!(markers(&mut engine).len(), 1, "clear must be undoable");
}
