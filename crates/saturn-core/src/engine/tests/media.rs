use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use serde_json::{json, Value};

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

// --- E.4 seam: proxy attach vs the missing cache ----------------------------
// `media.attach_proxy` is the only writer of `proxy_path` and it validates
// the file exists at attach time. The preview cache prunes by age, so an
// attached path can later dangle; nothing consumes proxy_path for playback
// today, so the pinned contract is: attach requires a real file, detach is
// undoable, and a missing source media is still attachable (the proxy outlives
// the relink).

#[test]
fn proxy_attach_requires_an_existing_file_and_is_undoable() {
    let mut engine = engine_with_video_media();
    let media_id = engine.document().project.media[0].id;

    // A proxy path that is not a file must be rejected up front.
    let error = engine
        .execute(CommandRequest::new(
            "media.attach_proxy",
            json!({"media_id": media_id, "path": "/nowhere/proxy.webm"}),
        ))
        .unwrap_err();
    assert!(
        error.to_string().contains("does not exist"),
        "attach must validate the proxy file: {error}"
    );
    assert!(engine.document().project.media[0].proxy_path.is_none());

    // Real file: attaches and survives undo.
    let real = tempfile::NamedTempFile::new().unwrap();
    engine
        .execute(CommandRequest::new(
            "media.attach_proxy",
            json!({"media_id": media_id, "path": real.path().to_string_lossy()}),
        ))
        .unwrap();
    assert!(engine.document().project.media[0].proxy_path.is_some());

    engine.execute(CommandRequest::new("edit.undo", Value::Null)).unwrap();
    assert!(
        engine.document().project.media[0].proxy_path.is_none(),
        "attaching a proxy participates in undo"
    );
}

#[test]
fn proxy_detach_reports_when_nothing_is_attached() {
    let mut engine = engine_with_video_media();
    let media_id = engine.document().project.media[0].id;

    let error = engine
        .execute(CommandRequest::new(
            "media.detach_proxy",
            json!({"media_id": media_id}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("no proxy attached"));
}

#[test]
fn proxy_unknown_media_id_is_rejected() {
    let mut engine = engine_with_video_media();
    let real = tempfile::NamedTempFile::new().unwrap();
    for id in ["media.attach_proxy", "media.detach_proxy"] {
        let error = engine
            .execute(CommandRequest::new(id, json!({"media_id": 99_999, "path": real.path().to_string_lossy()})))
            .unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{id}: {error}");
    }
}
