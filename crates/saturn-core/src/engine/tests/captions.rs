use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use serde_json::{json, Value};

fn engine_with_two_caption_tracks() -> (EditorEngine, u64, u64) {
    let mut engine = EditorEngine::default();
    let first = engine
        .execute(CommandRequest::new("caption.track_add", json!({"name": "A"})))
        .unwrap();
    let second = engine
        .execute(CommandRequest::new("caption.track_add", json!({"name": "B"})))
        .unwrap();
    let first_id = first
        .get("id")
        .and_then(Value::as_u64)
        .expect("track_add returns id");
    let second_id = second
        .get("id")
        .and_then(Value::as_u64)
        .expect("track_add returns id");
    (engine, first_id, second_id)
}

fn visibility(engine: &EditorEngine) -> Vec<bool> {
    engine
        .document()
        .project
        .sequence
        .caption_tracks
        .iter()
        .map(|track| track.visible)
        .collect()
}

// --- E.4 seam: caption-track solo state after sequence/document switches ---
// The editor holds `activeTrackId` across document replacements. When a
// switch leaves the stored id pointing at a track the live sequence does not
// have, the engine contract is: unknown ids hide/show NOTHING (never the
// wrong track). The UI solo command must therefore resolve its effective
// track (active, else the first) before calling these.

#[test]
fn caption_visibility_commands_ignore_unknown_track_ids() {
    let (mut engine, first_id, second_id) = engine_with_two_caption_tracks();

    engine
        .execute(CommandRequest::new(
            "caption.tracks_hide",
            json!({"track_ids": [99_999]}),
        ))
        .unwrap();
    assert_eq!(
        visibility(&engine),
        vec![true, true],
        "hiding a stale/unknown id must not touch the real tracks"
    );

    engine
        .execute(CommandRequest::new(
            "caption.tracks_show",
            json!({"track_ids": [99_999]}),
        ))
        .unwrap();
    assert_eq!(visibility(&engine), vec![true, true]);

    // The UI solo recipe with the RESOLVED id: hide everything, show one.
    engine
        .execute(CommandRequest::new("caption.tracks_hide", json!({})))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "caption.tracks_show",
            json!({"track_ids": [first_id]}),
        ))
        .unwrap();
    assert_eq!(
        visibility(&engine),
        vec![true, false],
        "solo shows exactly the active track ({first_id} vs {second_id})"
    );
}

#[test]
fn caption_visibility_is_undoable() {
    let (mut engine, first_id, _second_id) = engine_with_two_caption_tracks();
    engine
        .execute(CommandRequest::new(
            "caption.tracks_hide",
            json!({"track_ids": [first_id]}),
        ))
        .unwrap();
    assert_eq!(visibility(&engine), vec![false, true]);

    engine.execute(CommandRequest::new("edit.undo", Value::Null)).unwrap();
    assert_eq!(visibility(&engine), vec![true, true]);
}
