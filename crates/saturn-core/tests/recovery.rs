//! Crash-recovery contract (planning.md M1.1): an autosave snapshot must
//! restore into a fresh engine with the edits intact, undo history cleared,
//! an explicit dirty flag, and the recovered document must survive a normal
//! save-and-reopen afterwards.

use saturn_core::{CommandRequest, EditorEngine, ProjectDocument, TICKS_PER_SECOND};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn run(engine: &mut EditorEngine, id: &str, params: Value) -> Value {
    engine
        .execute(CommandRequest::new(id, params))
        .unwrap_or_else(|error| panic!("{id} failed: {error}"))
}

/// A engine with unsaved real work: project, imported media, one placed clip.
fn engine_with_work() -> (EditorEngine, u64, u64) {
    let mut engine = EditorEngine::default();
    run(&mut engine, "project.new", json!({"name": "Recovery project"}));
    let media = run(
        &mut engine,
        "project.add_media",
        json!({
            "name": "b-roll.mp4",
            "path": "/assets/b-roll.mp4",
            "kind": "video",
            "duration": TICKS_PER_SECOND * 10,
            "width": 1920,
            "height": 1080
        }),
    );
    let media_id = media["media_id"].as_u64().unwrap();
    let track_id = run(&mut engine, "project.status", Value::Null)
        ["document"]["project"]["sequence"]["tracks"][0]["id"]
        .as_u64()
        .unwrap();
    run(
        &mut engine,
        "timeline.add_clip",
        json!({"track_id": track_id, "media_id": media_id}),
    );
    (engine, media_id, track_id)
}

fn autosave_snapshot(engine: &EditorEngine, folder: &Path) -> PathBuf {
    let path = folder.join("autosave.saturn");
    engine
        .document()
        .save_atomic(&path)
        .unwrap_or_else(|error| panic!("snapshot failed: {error}"));
    path
}

#[test]
fn recovery_snapshot_restores_edits_into_a_fresh_engine() {
    let (engine, _media_id, track_id) = engine_with_work();
    let folder = tempfile::tempdir().unwrap();
    let snapshot_path = autosave_snapshot(&engine, folder.path());
    assert!(snapshot_path.is_file(), "snapshot must exist on disk");

    // Crash: everything except the snapshot file is gone.
    let loaded =
        ProjectDocument::load(&snapshot_path).unwrap_or_else(|error| panic!("load: {error}"));
    let mut recovered = EditorEngine::default();
    recovered
        .restore_recovery(loaded, Some(snapshot_path.clone()))
        .unwrap_or_else(|error| panic!("restore: {error}"));

    // The edits are back, and the engine treats them as unsaved work on the
    // originally open file.
    let document = recovered.document();
    assert_eq!(document.project.name, "Recovery project");
    assert_eq!(document.project.media.len(), 1);
    assert_eq!(document.project.sequence.tracks[0].id, track_id);
    assert_eq!(document.project.sequence.tracks[0].clips.len(), 1);
    assert_eq!(recovered.current_path(), Some(snapshot_path.as_path()));
    assert!(
        recovered.is_dirty(),
        "restored recovery must be flagged unsaved so a crash cannot lose it again"
    );
    assert!(
        !recovered.command_status("edit.undo").unwrap().enabled,
        "undo history cannot survive a crash: the recovered state is the truth"
    );
}

#[test]
fn recovered_edits_survive_a_normal_save_and_reopen() {
    let (engine, _media_id, _track_id) = engine_with_work();
    let folder = tempfile::tempdir().unwrap();
    let snapshot_path = autosave_snapshot(&engine, folder.path());

    let loaded = ProjectDocument::load(&snapshot_path).unwrap();
    let mut recovered = EditorEngine::default();
    recovered.restore_recovery(loaded, Some(snapshot_path)).unwrap();

    // The user saves the recovered work under the real project path.
    let saved_path = folder.path().join("recovered.saturn");
    run(
        &mut recovered,
        "file.save",
        json!({"path": saved_path.to_string_lossy()}),
    );
    assert!(
        !recovered.is_dirty(),
        "saving the recovery must clear the dirty flag"
    );

    // Reopening later shows exactly the recovered edits.
    let mut reopened = EditorEngine::default();
    run(
        &mut reopened,
        "file.open",
        json!({"path": saved_path.to_string_lossy()}),
    );
    let document = reopened.document();
    assert_eq!(document.project.name, "Recovery project");
    assert_eq!(document.project.sequence.tracks[0].clips.len(), 1);
}
