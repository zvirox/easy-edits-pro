use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use crate::project::MediaKind;
use serde_json::{json, Value};

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
