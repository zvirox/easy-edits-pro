use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use serde_json::json;

fn engine_with_video_media() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/a.mp4", "kind": crate::project::MediaKind::Video}),
        ))
        .unwrap();
    engine
}

// --- Tick-arithmetic robustness --------------------------------------------
// Effectively infinite tick values must yield clean errors (or saturate),
// never an integer-overflow panic: a panic unwinds through the editor's
// state mutex and bricks every later command.

#[test]
fn cue_nudge_overflow_is_a_clean_error_not_a_panic() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "caption.track_add",
            json!({"name": "C"}),
        ))
        .unwrap();
    let track_id = engine.document().project.sequence.caption_tracks[0].id;
    let cue_start: i64 = i64::MAX - 2_000;
    engine
        .execute(CommandRequest::new(
            "caption.cue_add",
            json!({"track_id": track_id, "start_ticks": cue_start,
                   "end_ticks": cue_start + 700, "text": "hi"}),
        ))
        .unwrap();
    let cue_id = engine.document().project.sequence.caption_tracks[0].cues[0].id;

    let error = engine
        .execute(CommandRequest::new(
            "caption.cue_nudge",
            json!({"track_id": track_id, "id": cue_id, "delta_ticks": 5_000}),
        ))
        .unwrap_err();
    assert!(
        error.to_string().contains("overflow"),
        "nudge past i64::MAX must be rejected, got: {error}"
    );
    // The cue itself is untouched.
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues[0].start_ticks.0, cue_start);
}

#[test]
fn clip_timeline_tick_bounds_are_rejected_at_load() {
    let mut engine = engine_with_video_media();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": engine.document().project.media[0].id,
                   "timeline_start": 0, "duration": 100_000_000}),
        ))
        .unwrap();

    // start + duration must not be able to overflow i64
    for (label, start, duration) in [
        ("start near i64::MAX", i64::MAX - 10, 100_000_000_i64),
        ("negative start", -5, 100_000_000),
    ] {
        let mut raw = serde_json::to_value(engine.document()).unwrap();
        if let Some(clip) = raw
            .get_mut("project")
            .and_then(|project| project.get_mut("sequence"))
            .and_then(|sequence| sequence.get_mut("tracks"))
            .and_then(|tracks| tracks.as_array_mut())
            .and_then(|tracks| tracks.first_mut())
            .and_then(|track| track.get_mut("clips"))
            .and_then(|clips| clips.as_array_mut())
            .and_then(|clips| clips.first_mut())
        {
            clip["timeline_start"] = json!(start);
            clip["duration"] = json!(duration);
        } else {
            panic!("could not reach the clip in the serialized document");
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.eep");
        std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
        let error = crate::project::ProjectDocument::load(&path).unwrap_err();
        assert!(
            error.contains("overflow"),
            "{label} must be rejected at load, got: {error}"
        );
    }
}

#[test]
fn gap_scan_survives_huge_command_created_ticks() {
    let mut engine = engine_with_video_media();
    let media_id = engine.document().project.media[0].id;
    // A command-created clip may carry a legal-but-huge start; the gap scans
    // behind timeline.ripple_delete_gap must saturate instead of panicking.
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id,
                   "timeline_start": i64::MAX - 1_000, "duration": 500}),
        ))
        .unwrap();
    // Either a clean error or a clean result - both fine, panic is not.
    let _ = engine.execute(CommandRequest::new(
        "sequence.inspect",
        json!({}),
    ));
    let _ = engine.execute(CommandRequest::new(
        "timeline.ripple_delete_gap",
        json!({"at_tick": 1}),
    ));
}

#[test]
fn compound_clip_of_huge_ticks_is_a_clean_error_not_a_panic() {
    let mut engine = engine_with_video_media();
    let media_id = engine.document().project.media[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id,
                   "timeline_start": i64::MAX - 1_000, "duration": 500}),
        ))
        .unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let _ = engine.execute(CommandRequest::new(
        "timeline.create_compound_clip",
        json!({"clips": [{"track_id": track_id, "clip_index": 0}]}),
    ));
}

// --- Checkpoint hygiene ------------------------------------------------------
// Disk-derived scan commands deliberately stay OUT of undo history (pinned by
// phase4_media: "a missing-media scan changes flags, not undo history"), while
// project.update_media mutates real authored data and must checkpoint.

#[test]
fn scan_commands_add_no_history() {
    for command in ["media.check_missing", "media.refresh_changed"] {
        let mut engine = engine_with_video_media(); // /a.mp4 does not exist
        assert!(!engine.document().project.media[0].is_missing);

        engine
            .execute(CommandRequest::new(command, json!({})))
            .unwrap();
        assert!(
            engine.document().project.media[0].is_missing,
            "{command} must flag the missing media"
        );

        // Exactly ONE undo must be left: the original add_media. If the scan
        // pushed history, this undo would only revert the scan instead.
        engine.execute(CommandRequest::new("edit.undo", json!({}))).unwrap();
        assert!(
            engine.document().project.media.is_empty(),
            "{command} must not consume undo history"
        );
        let error = engine
            .execute(CommandRequest::new("edit.undo", json!({})))
            .unwrap_err();
        assert!(
            error.to_string().to_lowercase().contains("nothing to undo"),
            "{command} must not push undo entries, got: {error}"
        );
    }
}

#[test]
fn update_media_is_undoable() {
    let mut engine = engine_with_video_media();
    engine
        .execute(CommandRequest::new(
            "project.update_media",
            json!({"path": "/a.mp4", "kind": crate::project::MediaKind::Audio,
                   "duration": 1_000, "width": 640, "height": 360}),
        ))
        .unwrap();
    assert!(matches!(
        engine.document().project.media[0].kind,
        crate::project::MediaKind::Audio
    ));
    engine.execute(CommandRequest::new("edit.undo", json!({}))).unwrap();
    assert!(matches!(
        engine.document().project.media[0].kind,
        crate::project::MediaKind::Video
    ));
}
