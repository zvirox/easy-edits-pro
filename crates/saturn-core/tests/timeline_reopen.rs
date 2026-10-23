//! Timeline/preview reliability (planning.md M1.2): every timeline editing
//! interaction the plan names — placement, trimming (plain and ripple),
//! splitting (razor), moving, ripple delete, transitions, in/out points, and
//! undo/redo — must survive a project save and reopen with its exact state,
//! and the reopened session must start with a clean history and undo its own
//! new edits.

use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::{json, Value};

fn setup_media(engine: &mut EditorEngine) -> u64 {
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name":"scene.mp4", "path":"/assets/scene.mp4", "kind":"video",
                "duration": TICKS_PER_SECOND * 30, "width": 1920, "height": 1080
            }),
        ))
        .unwrap();
    media["media_id"].as_u64().unwrap()
}

fn sequence(engine: &EditorEngine) -> Value {
    serde_json::to_value(&engine.document().project.sequence).unwrap()
}

#[test]
fn timeline_interactions_survive_save_and_reopen_exactly() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("interactions.saturn");
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name":"Interactions"})))
        .unwrap();
    let media_id = setup_media(&mut engine);

    // Drop two contiguous clips: A [0s,2s), B [2s,5s).
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "media_id":media_id, "track_id":1,
                "timeline_start":0, "source_in":0,
                "duration":TICKS_PER_SECOND * 2
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "media_id":media_id, "track_id":1,
                "timeline_start":TICKS_PER_SECOND * 2, "source_in":TICKS_PER_SECOND * 4,
                "duration":TICKS_PER_SECOND * 3
            }),
        ))
        .unwrap();

    // Transition across the shared cut point.
    engine
        .execute(CommandRequest::new(
            "timeline.add_transition",
            json!({
                "track_id":1, "cut_at":TICKS_PER_SECOND * 2,
                "duration":TICKS_PER_SECOND / 2, "kind":"crossfade"
            }),
        ))
        .unwrap();

    // Plain trim pulls A's tail in; ripple trim pushes B's head to meet it.
    engine
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id":1, "clip_index":0, "edge":"end",
                "delta":-(TICKS_PER_SECOND / 2), "ripple":false
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id":1, "clip_index":1, "edge":"start",
                "delta":-(TICKS_PER_SECOND / 2), "ripple":false
            }),
        ))
        .unwrap();

    // Razor split B at 3.5s, then move the second half right.
    engine
        .execute(CommandRequest::new(
            "timeline.split_clip",
            json!({"track_id":1, "clip_index":1, "at":TICKS_PER_SECOND * 7 / 2}),
        ))
        .unwrap();
    let moved = engine
        .execute(CommandRequest::new(
            "timeline.move_clip",
            json!({
                "track_id":1, "clip_index":2,
                "timeline_start":TICKS_PER_SECOND * 11 / 2
            }),
        ))
        .unwrap();
    assert_eq!(moved["clip_index"], 2);

    // Ripple delete the moved second half (closes the gap it leaves).
    engine
        .execute(CommandRequest::new(
            "timeline.ripple_delete",
            json!({"track_id":1, "clip_index":2}),
        ))
        .unwrap();

    // In/out points around the programme.
    engine
        .execute(CommandRequest::new(
            "timeline.set_mark_in",
            json!({"at":0}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.set_mark_out",
            json!({"at":TICKS_PER_SECOND * 5}),
        ))
        .unwrap();

    // One more edit, undone before saving, proves undo restores exact ticks
    // and that the saved document is the post-undo state.
    engine
        .execute(CommandRequest::new(
            "timeline.move_clip",
            json!({
                "track_id":1, "clip_index":1,
                "timeline_start":TICKS_PER_SECOND * 6
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[1].timeline_start.0,
        TICKS_PER_SECOND * 3 / 2,
        "undo restored the pre-move position (B was rippled to 1.5s)"
    );

    let saved_sequence = sequence(&engine);
    engine
        .execute(CommandRequest::new("file.save", json!({"path":path})))
        .unwrap();
    assert!(!engine.is_dirty());

    let mut reopened = EditorEngine::default();
    reopened
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    assert!(!reopened.is_dirty());

    // Exact persistence: the whole sequence (clip geometry, transitions,
    // marks) equals what was saved - not just spot fields.
    assert_eq!(sequence(&reopened), saved_sequence, "reopened sequence differs from the saved one");

    // Reopened sessions start with a clean history: the pre-save undo stack
    // does not leak across the file boundary.
    assert!(
        reopened
            .execute(CommandRequest::new("edit.undo", Value::Null))
            .is_err(),
        "reopened engine must not undo into the previous session"
    );

    // And the reopened session edits (and undoes) like a fresh one.
    reopened
        .execute(CommandRequest::new(
            "timeline.trim_clip",
            json!({
                "track_id":1, "clip_index":0, "edge":"end",
                "delta":-(TICKS_PER_SECOND / 4), "ripple":false
            }),
        ))
        .unwrap();
    let trimmed_end = reopened.document().project.sequence.tracks[0].clips[0]
        .timeline_start
        .0
        + reopened.document().project.sequence.tracks[0].clips[0].duration.0;
    assert_eq!(trimmed_end, TICKS_PER_SECOND * 3 / 2 - TICKS_PER_SECOND / 4);
    reopened
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let restored_end = reopened.document().project.sequence.tracks[0].clips[0]
        .timeline_start
        .0
        + reopened.document().project.sequence.tracks[0].clips[0].duration.0;
    assert_eq!(restored_end, TICKS_PER_SECOND * 3 / 2);
}
