use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use serde_json::{json, Value};

/// Packet 0.1 tests: the unified engine-owned viewport state (playhead,
/// selection, export queue, workspace) responds over the command surface so
/// MCP, the CLI and the desktop UI share one deterministic state.

fn engine_with_timeline_clip() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/a.mp4", "kind": crate::project::MediaKind::Video}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id, "timeline_start": 0, "duration": 100_000_000_000i64}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id, "timeline_start": 200_000_000_000i64, "duration": 100_000_000_000i64}),
        ))
        .unwrap();
    engine
}

fn viewport(response: &Result<Value, crate::engine::DispatchError>) -> Value {
    let response = response.as_ref().unwrap_or_else(|error| {
        panic!("viewport command failed: {error:?}")
    });
    response
        .get("viewport")
        .cloned()
        .expect("viewport commands return a viewport snapshot")
}

// --- Playhead ---------------------------------------------------------------

#[test]
fn playhead_start_and_end_land_on_zero_and_duration() {
    let mut engine = engine_with_timeline_clip();
    let start = viewport(&engine.execute(CommandRequest::new("playhead.start", json!({}))));
    assert_eq!(start["playhead_tick"], 0);
    let end = viewport(&engine.execute(CommandRequest::new("playhead.end", json!({}))));
    assert_eq!(end["playhead_tick"], 300_000_000_000i64);
}

#[test]
fn playhead_nudges_move_by_one_frame_and_clamp() {
    let mut engine = engine_with_timeline_clip();
    let ticks_per_frame = engine
        .document()
        .project
        .settings
        .frame_rate
        .frame_ticks()
        .unwrap_or(10_584_000_000)
        .max(1);
    let nudged = viewport(&engine.execute(CommandRequest::new(
        "playhead.nudge_forward_1",
        json!({}),
    )));
    assert_eq!(nudged["playhead_tick"], ticks_per_frame);
    // Saturation, not an overflow panic, at the far end.
    let _ = engine.execute(CommandRequest::new("playhead.end", json!({})));
    let back = viewport(&engine.execute(CommandRequest::new(
        "playhead.nudge_back_5",
        json!({}),
    )));
    assert_eq!(
        back["playhead_tick"],
        300_000_000_000i64 - 5 * ticks_per_frame
    );
}

#[test]
fn playhead_next_and_previous_edit_walk_cut_points() {
    let mut engine = engine_with_timeline_clip();
    // Edit points are 0, 100e9, 200e9, 300e9; nudging the `> current` search
    // skips the current cut.
    let next = viewport(&engine.execute(CommandRequest::new("playhead.next_edit", json!({}))));
    assert_eq!(next["playhead_tick"], 100_000_000_000i64);
    engine.execute(CommandRequest::new("playhead.end", json!({}))).unwrap();
    let previous = viewport(&engine.execute(CommandRequest::new(
        "playhead.previous_edit",
        json!({}),
    )));
    assert_eq!(previous["playhead_tick"], 200_000_000_000i64);
}

#[test]
fn playhead_next_edit_without_further_cut_is_a_clean_error() {
    let mut engine = engine_with_timeline_clip();
    engine.execute(CommandRequest::new("playhead.end", json!({}))).unwrap();
    let error = engine
        .execute(CommandRequest::new("playhead.next_edit", json!({})))
        .unwrap_err();
    assert!(error.to_string().contains("No edit point"));
}

#[test]
fn playhead_go_to_timecode_parses_and_rejects() {
    let mut engine = engine_with_timeline_clip();
    let moved = viewport(&engine.execute(CommandRequest::new(
        "playhead.go_to_timecode",
        json!({"timecode": "00:00:00:10"}),
    )));
    let ticks_per_frame = engine
        .document()
        .project
        .settings
        .frame_rate
        .frame_ticks()
        .unwrap_or(10_584_000_000)
        .max(1);
    assert_eq!(moved["playhead_tick"], 10 * ticks_per_frame);    let error = engine
        .execute(CommandRequest::new(
            "playhead.go_to_timecode",
            json!({"timecode": "not-a-timecode"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("Invalid timecode"));
}

#[test]
fn playhead_go_to_in_requires_marks() {
    let mut engine = engine_with_timeline_clip();
    let error = engine
        .execute(CommandRequest::new("playhead.go_to_in", json!({})))
        .unwrap_err();
    assert!(error.to_string().contains("No mark in"));
}

// --- Selection ----------------------------------------------------------------

#[test]
fn timeline_select_validates_clip_index() {
    let mut engine = engine_with_timeline_clip();
    // timeline.add_clip appends to V1: two clips exist.
    let track_id = engine.document().project.sequence.tracks[0].id;
    let ok = viewport(&engine.execute(CommandRequest::new(
        "timeline.select",
        json!({"track_id": track_id, "clip_index": 1}),
    )));
    assert_eq!(ok["selected_clips"].as_array().unwrap().len(), 1);
    assert_eq!(ok["selected_clips"][0]["clip_index"], 1);
    let error = engine
        .execute(CommandRequest::new(
            "timeline.select",
            json!({"track_id": track_id, "clip_index": 9}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("does not exist"));
}

#[test]
fn timeline_select_next_and_prev_clip() {
    let mut engine = engine_with_timeline_clip();
    let track_id = engine.document().project.sequence.tracks[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.select",
            json!({"track_id": track_id, "clip_index": 0}),
        ))
        .unwrap();
    let next = viewport(&engine.execute(CommandRequest::new(
        "timeline.selectNextClip",
        json!({}),
    )));
    assert_eq!(next["selected_clips"][0]["clip_index"], 1);
    let error = engine
        .execute(CommandRequest::new("timeline.selectNextClip", json!({})))
        .unwrap_err();
    assert!(error.to_string().contains("No next clip"));
    let prev = viewport(&engine.execute(CommandRequest::new(
        "timeline.selectPrevClip",
        json!({}),
    )));
    assert_eq!(prev["selected_clips"][0]["clip_index"], 0);
}

#[test]
fn timeline_select_gap_requires_an_empty_range() {
    let mut engine = engine_with_timeline_clip();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let selected = viewport(&engine.execute(CommandRequest::new(
        "timeline.selectGap",
        json!({"track_id": track_id, "start_tick": 150_000_000_000i64, "end_tick": 180_000_000_000i64}),
    )));
    assert_eq!(selected["selected_gap"]["start_tick"], 150_000_000_000i64);
    assert!(selected["selected_clips"].as_array().unwrap().is_empty());
    // Overlapping the second clip must be rejected.
    let error = engine
        .execute(CommandRequest::new(
            "timeline.selectGap",
            json!({"track_id": track_id, "start_tick": 210_000_000_000i64, "end_tick": 220_000_000_000i64}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("contains clips"));
}

#[test]
fn edit_select_all_and_clip_at_playhead_and_label_group() {
    let mut engine = engine_with_timeline_clip();
    let all = viewport(&engine.execute(CommandRequest::new("edit.select_all", json!({}))));
    assert_eq!(all["selected_clips"].as_array().unwrap().len(), 2);
    // Park the playhead inside the second clip (frame 29 of 30fps lands in
    // [200e9, 300e9) with the default project rate).
    let ticks_per_frame = engine
        .document()
        .project
        .settings
        .frame_rate
        .frame_ticks()
        .unwrap_or(10_584_000_000)
        .max(1);
    assert!(29 * ticks_per_frame >= 200_000_000_000);
    engine
        .execute(CommandRequest::new(
            "playhead.go_to_timecode",
            json!({"timecode": "00:00:00:29"}),
        ))
        .unwrap();
    let at_playhead = viewport(&engine.execute(CommandRequest::new(
        "edit.select_clip_at_playhead",
        json!({}),
    )));
    let selection = at_playhead["selected_clips"].as_array().unwrap();
    assert_eq!(selection.len(), 1);
    assert_eq!(selection[0]["clip_index"], 1);
    let error = engine
        .execute(CommandRequest::new(
            "edit.select_label_group",
            json!({"label": "no-such-label"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("No clips"));
}

// --- Workspace + projected state ------------------------------------------------

#[test]
fn workspace_switch_is_engine_state_and_rejects_unknown() {
    let mut engine = EditorEngine::default();
    let switched = viewport(&engine.execute(CommandRequest::new(
        "workspace.color",
        json!({}),
    )));
    assert_eq!(switched["active_workspace"], "color");
    // Unknown workspace ids are not in the catalogue, so the engine's
    // find_command gate reports them before dispatch.
    let error = engine
        .execute(CommandRequest::new("workspace.does_not_exist", json!({})))
        .unwrap_err();
    assert!(
        error.to_string().contains("Unknown command"),
        "got: {error}"
    );
}

#[test]
fn viewport_reports_targeted_tracks_from_targeting_state() {
    let mut engine = EditorEngine::default();
    let snapshot = viewport(&engine.execute(CommandRequest::new("viewport.inspect", json!({}))));
    assert_eq!(
        snapshot["targeted_video_tracks"].as_array().unwrap(),
        &[1]
    );
    assert_eq!(snapshot["active_workspace"], "edit");
}

#[test]
fn viewport_inspect_junk_params_cap_stop_are_clean_errors() {
    let mut engine = EditorEngine::default();
    let error = engine
        .execute(CommandRequest::new(
            "playhead.start",
            json!({"unexpected": 1}),
        ))
        .unwrap_err();
    assert!(
        matches!(
            error,
            crate::engine::DispatchError::InvalidParameters(_)
        ),
        "junk params must be rejected, got: {error:?}"
    );
    engine
        .execute(CommandRequest::new("export.queue.start", json!({})))
        .unwrap();
    let snapshot = viewport(&engine.execute(CommandRequest::new("viewport.inspect", json!({}))));
    assert_eq!(snapshot["export_queue_paused"], false);
}

// --- Export queue ----------------------------------------------------------

#[test]
fn export_queue_lifecycle_add_remove_retry_and_bounds() {
    let mut engine = EditorEngine::default();
    let added = viewport(&engine.execute(CommandRequest::new(
        "export.queue.add",
        json!({"output_path": "/out/a.mp4"}),
    )));
    assert_eq!(added["export_queue"].as_array().unwrap().len(), 1);
    assert_eq!(added["export_queue"][0]["state"], "queued");

    let id = added["export_queue"][0]["id"].as_u64().unwrap();
    // Cancel then retry flips the state machine back to queued.
    let cancelled = viewport(&engine.execute(CommandRequest::new(
        "export.queue.cancel",
        json!({"id": id}),
    )));
    assert_eq!(cancelled["export_queue"][0]["state"], "cancelled");
    let retried = viewport(&engine.execute(CommandRequest::new(
        "export.queue.retry",
        json!({"id": id}),
    )));
    assert_eq!(retried["export_queue"][0]["state"], "queued");

    // Unknown ids are clean errors.
    let error = engine
        .execute(CommandRequest::new(
            "export.queue.cancel",
            json!({"id": 999}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("does not exist"));

    // Empty output path rejected.
    let error = engine
        .execute(CommandRequest::new(
            "export.queue.add",
            json!({"output_path": "   "}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("non-empty"));

    let listed = engine
        .execute(CommandRequest::new("export.queue.list", json!({})))
        .unwrap();
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);

    engine.execute(CommandRequest::new("export.queue.stop", json!({}))).unwrap();
    let paused = engine
        .execute(CommandRequest::new("export.queue.list", json!({})))
        .unwrap();
    assert_eq!(paused["paused"], true);
    engine.execute(CommandRequest::new("export.queue.start", json!({}))).unwrap();

    engine.execute(CommandRequest::new("export.queue.clear", json!({}))).unwrap();
    let empty = engine
        .execute(CommandRequest::new("export.queue.list", json!({})))
        .unwrap();
    assert_eq!(empty["items"].as_array().unwrap().len(), 0);
}

#[test]
fn export_queue_move_reorders_entries() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "export.queue.add",
            json!({"output_path": "/out/first.mp4"}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "export.queue.add",
            json!({"output_path": "/out/second.mp4"}),
        ))
        .unwrap();
    let snapshot = viewport(&engine.execute(CommandRequest::new("viewport.inspect", json!({}))));
    let items = snapshot["export_queue"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let first_id = items[0]["id"].as_u64().unwrap();
    let second_id = items[1]["id"].as_u64().unwrap();
    // Move the second entry to the front.
    engine
        .execute(CommandRequest::new(
            "export.queue.move",
            json!({"id": second_id, "position": 0}),
        ))
        .unwrap();
    let moved = viewport(&engine.execute(CommandRequest::new("viewport.inspect", json!({}))));
    let items = moved["export_queue"].as_array().unwrap();
    assert_eq!(items[0]["id"].as_u64(), Some(second_id));
    assert_eq!(items[1]["id"].as_u64(), Some(first_id));
}
