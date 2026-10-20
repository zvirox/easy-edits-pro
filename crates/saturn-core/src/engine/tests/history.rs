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
