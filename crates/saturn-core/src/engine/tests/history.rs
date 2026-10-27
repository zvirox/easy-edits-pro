use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use crate::project::MediaKind;
use serde_json::{Value, json};

#[test]
fn commands_report_reasons_and_undo_redo_project_mutations() {
    let mut engine = EditorEngine::default();
    assert_eq!(
        engine
            .command_status("edit.undo")
            .unwrap()
            .reason
            .as_deref(),
        Some("Nothing to undo")
    );
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name":"clip.mp4", "path":"/clips/clip.mp4", "kind": MediaKind::Video
            }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.media.len(), 1);
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert!(engine.document().project.media.is_empty());
    engine
        .execute(CommandRequest::new("edit.redo", Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.media[0].name, "clip.mp4");
}

#[test]
fn mark_in_and_mark_out_are_undoable_sequence_state() {
    let mut engine = EditorEngine::default();
    let mark = crate::time::TICKS_PER_SECOND;

    engine
        .execute(CommandRequest::new(
            "timeline.set_mark_in",
            json!({ "at": mark }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.set_mark_out",
            json!({ "at": mark * 2 }),
        ))
        .unwrap();

    let sequence = &engine.document().project.sequence;
    assert_eq!(sequence.mark_in.map(|value| value.0), Some(mark));
    assert_eq!(sequence.mark_out.map(|value| value.0), Some(mark * 2));

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.mark_out, None);
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .mark_in
            .map(|value| value.0),
        Some(mark)
    );

    engine
        .execute(CommandRequest::new("timeline.clear_marks", Value::Null))
        .unwrap();
    assert!(engine.document().project.sequence.mark_in.is_none());
    assert!(engine.document().project.sequence.mark_out.is_none());
}

// --- E.4 seam: undo across autosave checkpoints ----------------------------
// autosave_once snapshots engine.document() while undo/redo hold editor
// history; restoring that snapshot must discard the old history (an undo
// after restore would rewind to a pre-recovery document), and it must leave
// the engine dirty so the autosave loop re-arms over the restored state.

#[test]
fn restore_recovery_clears_history_and_re_arms_autosave() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/a.mp4", "kind": MediaKind::Video}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "b.mp4", "path": "/b.mp4", "kind": MediaKind::Video}),
        ))
        .unwrap();
    assert!(
        engine
            .command_status("edit.undo")
            .unwrap()
            .enabled,
        "two mutations should make undo available"
    );

    let recovered = crate::project::ProjectDocument::new("Recovered");
    engine.restore_recovery(recovered, None).unwrap();

    assert_eq!(engine.document().project.name, "Recovered");
    assert!(
        !engine.command_status("edit.undo").unwrap().enabled,
        "restoring an autosave must not leave history that rewinds across it"
    );
    assert!(!engine.command_status("edit.redo").unwrap().enabled);
    // dirty again: the autosave loop re-arms so the restored document itself
    // becomes the new recovery checkpoint.
    assert!(engine.is_dirty());

    // History rebuilds cleanly from the restored document.
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "c.mp4", "path": "/c.mp4", "kind": MediaKind::Video}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert!(engine.document().project.media.is_empty());
}

// --- E.4 seam: save/reopen with disabled clips (clip.set_enabled) ----------

fn engine_with_one_clip() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/a.mp4", "kind": MediaKind::Video}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": 1, "media_id": media_id}),
        ))
        .unwrap();
    engine
}

#[test]
fn clip_set_enabled_is_undoable_and_redoable() {
    let mut engine = engine_with_one_clip();
    engine
        .execute(CommandRequest::new(
            "clip.set_enabled",
            json!({"track_id": 1, "clip_index": 0, "enabled": false}),
        ))
        .unwrap();
    assert!(!engine.document().project.sequence.tracks[0].clips[0].enabled);

    engine.execute(CommandRequest::new("edit.undo", Value::Null)).unwrap();
    assert!(engine.document().project.sequence.tracks[0].clips[0].enabled);

    engine.execute(CommandRequest::new("edit.redo", Value::Null)).unwrap();
    assert!(!engine.document().project.sequence.tracks[0].clips[0].enabled);
}

#[test]
fn disabled_clip_state_survives_a_persistence_round_trip() {
    let mut engine = engine_with_one_clip();
    engine
        .execute(CommandRequest::new(
            "clip.set_enabled",
            json!({"track_id": 1, "clip_index": 0, "enabled": false}),
        ))
        .unwrap();

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("disabled.saturn");
    engine.document().save_atomic(&path).unwrap();

    // A fresh engine re-opening the file keeps the clip disabled - the UI
    // renders "on the timeline but silent" after save/reopen.
    let loaded = crate::project::ProjectDocument::load(&path).unwrap();
    let mut reopened = EditorEngine::default();
    reopened.restore_recovery(loaded, Some(path.clone())).unwrap();
    assert!(!reopened.document().project.sequence.tracks[0].clips[0].enabled);

    // Pre-Enable projects have no `enabled` field on clips; serde defaults
    // them to visible so an old save never loses footage.
    let mut raw = serde_json::to_value(engine.document()).unwrap();
    if let Some(tracks) = raw
        .get_mut("project")
        .and_then(|project| project.get_mut("sequence"))
        .and_then(|sequence| sequence.get_mut("tracks"))
        .and_then(|tracks| tracks.as_array_mut())
    {
        for track in tracks {
            if let Some(clips) = track.get_mut("clips").and_then(|clips| clips.as_array_mut()) {
                for clip in clips {
                    if let Some(clip) = clip.as_object_mut() {
                        clip.remove("enabled");
                    }
                }
            }
        }
    }
    let legacy = std::fs::File::create(&path).unwrap();
    serde_json::to_writer(legacy, &raw).unwrap();
    let legacy_doc = crate::project::ProjectDocument::load(&path).unwrap();
    assert!(
        legacy_doc.project.sequence.tracks[0].clips[0].enabled,
        "a project saved before clip.enable must load with clips enabled"
    );
}
