//! Sequence depth: create, duplicate, rename, switch, and delete.

use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::{json, Value};

fn setup(engine: &mut EditorEngine) -> u64 {
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Sequences"}),
        ))
        .unwrap();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "scene.mp4",
                "path": "/assets/scene.mp4",
                "kind": "video",
                "duration": TICKS_PER_SECOND * 20,
                "width": 1920,
                "height": 1080
            }),
        ))
        .unwrap();
    media["media_id"].as_u64().unwrap()
}

fn document(engine: &mut EditorEngine) -> Value {
    engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap()["document"]
        .clone()
}

fn active_id(engine: &mut EditorEngine) -> u64 {
    document(engine)["project"]["sequence"]["id"]
        .as_u64()
        .unwrap()
}

fn sequence_ids(engine: &mut EditorEngine) -> Vec<u64> {
    let doc = document(engine);
    let mut ids = vec![doc["project"]["sequence"]["id"].as_u64().unwrap()];
    ids.extend(
        doc["project"]["sequences"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sequence| sequence["id"].as_u64().unwrap()),
    );
    ids
}

fn active_track_id(engine: &mut EditorEngine, kind: &str) -> u64 {
    document(engine)["project"]["sequence"]["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|track| track["kind"] == kind)
        .and_then(|track| track["id"].as_u64())
        .unwrap()
}

fn add_clip(engine: &mut EditorEngine, media_id: u64, track_id: u64) {
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "media_id": media_id,
                "track_id": track_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": TICKS_PER_SECOND * 2
            }),
        ))
        .unwrap();
}

#[test]
fn create_adds_and_activates_a_unique_sequence() {
    let mut engine = EditorEngine::default();
    setup(&mut engine);
    let first = active_id(&mut engine);

    let created = engine
        .execute(CommandRequest::new("sequence.create", json!({})))
        .unwrap();
    let second = created["sequence_id"].as_u64().unwrap();
    assert_ne!(first, second);
    assert_eq!(created["activated"], true);
    assert_eq!(active_id(&mut engine), second);

    let doc = document(&mut engine);
    assert_eq!(doc["project"]["sequences"].as_array().unwrap().len(), 1);
    assert_eq!(
        doc["project"]["sequences"][0]["id"].as_u64().unwrap(),
        first
    );
    assert_eq!(
        doc["project"]["sequence"]["tracks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn duplicate_copies_content_and_is_one_undo_entry() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    let first = active_id(&mut engine);
    let video_track = active_track_id(&mut engine, "video");
    add_clip(&mut engine, media, video_track);

    let duplicate = engine
        .execute(CommandRequest::new("sequence.duplicate", json!({})))
        .unwrap();
    let second = duplicate["sequence_id"].as_u64().unwrap();
    assert_eq!(duplicate["activated"], true);
    assert_ne!(first, second);
    assert_eq!(active_id(&mut engine), second);

    let doc = document(&mut engine);
    let clips = doc["project"]["sequence"]["tracks"][0]["clips"]
        .as_array()
        .unwrap();
    assert_eq!(clips.len(), 1);

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert_eq!(active_id(&mut engine), first);
    assert_eq!(sequence_ids(&mut engine), vec![first]);
}

#[test]
fn rename_rejects_blank_names_and_keeps_names_unique() {
    let mut engine = EditorEngine::default();
    setup(&mut engine);
    let first = active_id(&mut engine);
    let second = engine
        .execute(CommandRequest::new(
            "sequence.create",
            json!({"name": "Main"}),
        ))
        .unwrap()["sequence_id"]
        .as_u64()
        .unwrap();

    let renamed = engine
        .execute(CommandRequest::new(
            "sequence.rename",
            json!({"sequence_id": first, "name": "Main"}),
        ))
        .unwrap();
    assert_eq!(renamed["name"], "Main 2");

    let blank = engine.execute(CommandRequest::new(
        "sequence.rename",
        json!({"sequence_id": second, "name": "  "}),
    ));
    assert!(blank.is_err());
}

#[test]
fn switching_sequences_keeps_independent_edits() {
    let mut engine = EditorEngine::default();
    let media = setup(&mut engine);
    let first = active_id(&mut engine);
    let video_track = active_track_id(&mut engine, "video");
    add_clip(&mut engine, media, video_track);

    let second = engine
        .execute(CommandRequest::new(
            "sequence.create",
            json!({"name": "B roll"}),
        ))
        .unwrap()["sequence_id"]
        .as_u64()
        .unwrap();
    let video_track = active_track_id(&mut engine, "video");
    add_clip(&mut engine, media, video_track);

    engine
        .execute(CommandRequest::new(
            "project.switch_sequence",
            json!({"sequence_id": first}),
        ))
        .unwrap();
    let doc = document(&mut engine);
    assert_eq!(doc["project"]["sequence"]["id"].as_u64().unwrap(), first);
    assert_eq!(
        doc["project"]["sequence"]["tracks"][0]["clips"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    engine
        .execute(CommandRequest::new(
            "project.switch_sequence",
            json!({"sequence_id": second}),
        ))
        .unwrap();
    let doc = document(&mut engine);
    assert_eq!(doc["project"]["sequence"]["id"].as_u64().unwrap(), second);
    assert_eq!(
        doc["project"]["sequence"]["tracks"][0]["clips"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn duplicate_and_delete_work_on_inactive_sequences() {
    let mut engine = EditorEngine::default();
    setup(&mut engine);
    let first = active_id(&mut engine);
    let second = engine
        .execute(CommandRequest::new(
            "sequence.create",
            json!({"name": "Second"}),
        ))
        .unwrap()["sequence_id"]
        .as_u64()
        .unwrap();

    engine
        .execute(CommandRequest::new(
            "project.switch_sequence",
            json!({"sequence_id": first}),
        ))
        .unwrap();

    let copy = engine
        .execute(CommandRequest::new(
            "sequence.duplicate",
            json!({"sequence_id": second, "name": "Second copy"}),
        ))
        .unwrap();
    assert_eq!(copy["activated"], false);
    assert_eq!(active_id(&mut engine), first);
    let copy_id = copy["sequence_id"].as_u64().unwrap();
    assert!(sequence_ids(&mut engine).contains(&copy_id));

    let deleted = engine
        .execute(CommandRequest::new(
            "sequence.delete",
            json!({"sequence_id": second}),
        ))
        .unwrap();
    assert_eq!(deleted["deleted"], true);
    assert_eq!(active_id(&mut engine), first);
    assert!(!sequence_ids(&mut engine).contains(&second));
}

#[test]
fn deleting_active_sequence_activates_a_remaining_one() {
    let mut engine = EditorEngine::default();
    setup(&mut engine);
    let first = active_id(&mut engine);
    let second = engine
        .execute(CommandRequest::new(
            "sequence.create",
            json!({"name": "Second"}),
        ))
        .unwrap()["sequence_id"]
        .as_u64()
        .unwrap();

    let deleted = engine
        .execute(CommandRequest::new("sequence.delete", json!({})))
        .unwrap();
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["active_sequence_id"].as_u64().unwrap(), first);
    assert_eq!(active_id(&mut engine), first);
    assert_eq!(sequence_ids(&mut engine), vec![first]);
    assert!(!sequence_ids(&mut engine).contains(&second));

    // The last sequence is protected.
    let last = engine.execute(CommandRequest::new("sequence.delete", json!({})));
    assert!(last.is_err());
    assert_eq!(sequence_ids(&mut engine), vec![first]);
}
