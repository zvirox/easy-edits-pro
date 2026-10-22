//! Phase 4 media organisation: bins, asset properties, and metadata.
//!
//! These drive the same command ids the UI uses, so a rename of a command or
//! parameter in the engine breaks the test before it breaks the editor.

use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::{json, Value};

fn engine_with_media() -> (EditorEngine, u64) {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Media organisation"}),
        ))
        .unwrap();
    let added = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "interview.mp4",
                "path": "/assets/interview.mp4",
                "kind": "video",
                "duration": TICKS_PER_SECOND * 30,
                "width": 1920,
                "height": 1080
            }),
        ))
        .unwrap();
    (engine, added["media_id"].as_u64().unwrap())
}

fn project(engine: &mut EditorEngine) -> Value {
    engine
        .execute(CommandRequest::new("project.status", Value::Null))
        .unwrap()["document"]["project"]
        .clone()
}

fn run(engine: &mut EditorEngine, id: &str, params: Value) -> Value {
    engine
        .execute(CommandRequest::new(id, params))
        .unwrap_or_else(|error| panic!("{id} failed: {error}"))
}

#[test]
fn bin_lifecycle_moves_media_and_is_fully_undoable() {
    let (mut engine, media_id) = engine_with_media();
    let created = run(
        &mut engine,
        "media.create_bin",
        json!({"name": "B-Roll", "color": "#10b981"}),
    );
    let bin_id = created["bin"]["id"].as_u64().unwrap();
    assert_eq!(created["bin"]["name"], "B-Roll");
    assert_eq!(project(&mut engine)["bins"][0]["name"], "B-Roll");

    run(
        &mut engine,
        "media.rename_bin",
        json!({"bin_id": bin_id, "name": "B-Roll 4K"}),
    );
    assert_eq!(project(&mut engine)["bins"][0]["name"], "B-Roll 4K");

    run(
        &mut engine,
        "media.move_to_bin",
        json!({"media_id": media_id, "bin_id": bin_id}),
    );
    assert_eq!(project(&mut engine)["media"][0]["bin_id"], json!(bin_id));

    run(&mut engine, "media.delete_bin", json!({"bin_id": bin_id}));
    let after = project(&mut engine);
    assert!(after["bins"].as_array().unwrap().is_empty());
    assert_eq!(
        after["media"][0]["bin_id"],
        Value::Null,
        "deleting a bin returns its media to the root"
    );

    // One undo restores the bin and the media placement.
    run(&mut engine, "edit.undo", Value::Null);
    let restored = project(&mut engine);
    assert_eq!(restored["bins"][0]["name"], "B-Roll 4K");
    assert_eq!(restored["media"][0]["bin_id"], json!(bin_id));
}

#[test]
fn move_to_bin_rejects_an_unknown_target_without_mutating() {
    let (mut engine, media_id) = engine_with_media();
    let error = engine
        .execute(CommandRequest::new(
            "media.move_to_bin",
            json!({"media_id": media_id, "bin_id": 9999}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("not found"), "{error}");
    assert_eq!(project(&mut engine)["media"][0]["bin_id"], Value::Null);
}

#[test]
fn update_properties_writes_metadata_in_one_undo_step() {
    let (mut engine, media_id) = engine_with_media();
    let updated = run(
        &mut engine,
        "media.update_properties",
        json!({
            "media_id": media_id,
            "name": "Interview master",
            "color_label": "#f59e0b",
            "tags": ["interview", "4k"],
            "comment": "Recorded on the roof, audio is clean.",
            "scene": "12B",
            "take": "3"
        }),
    );
    assert_eq!(updated["scene"], "12B");
    assert_eq!(updated["take"], "3");
    assert_eq!(updated["comment"], "Recorded on the roof, audio is clean.");

    let media = &project(&mut engine)["media"][0];
    assert_eq!(media["name"], "Interview master");
    assert_eq!(media["tags"], json!(["interview", "4k"]));
    assert_eq!(media["color_label"], "#f59e0b");
    assert_eq!(media["comment"], "Recorded on the roof, audio is clean.");

    run(&mut engine, "edit.undo", Value::Null);
    let reverted = &project(&mut engine)["media"][0];
    assert_eq!(reverted["name"], "interview.mp4");
    assert_eq!(reverted["comment"], "");
}

#[test]
fn oversized_metadata_is_rejected_before_the_undo_checkpoint() {
    let (mut engine, media_id) = engine_with_media();
    run(
        &mut engine,
        "media.update_properties",
        json!({"media_id": media_id, "scene": "12B"}),
    );

    let error = engine
        .execute(CommandRequest::new(
            "media.update_properties",
            json!({"media_id": media_id, "comment": "x".repeat(4001)}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("4000"), "{error}");
    assert_eq!(
        project(&mut engine)["media"][0]["scene"],
        "12B",
        "a rejected update must not touch the project"
    );

    // The rejected call must not have pushed a history entry either: one undo
    // rolls back the valid scene edit.
    run(&mut engine, "edit.undo", Value::Null);
    assert_eq!(project(&mut engine)["media"][0]["scene"], "");
}

#[test]
fn asset_metadata_survives_save_and_reload() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("organised.saturn");
    let (mut engine, media_id) = engine_with_media();
    run(
        &mut engine,
        "media.update_properties",
        json!({
            "media_id": media_id,
            "comment": "needs a re-record",
            "scene": "5A",
            "take": "2"
        }),
    );
    run(&mut engine, "media.create_bin", json!({"name": "Selects"}));
    run(&mut engine, "file.save", json!({"path": path}));

    let reloaded = saturn_core::ProjectDocument::load(&path).unwrap();
    let media = &reloaded.project.media[0];
    assert_eq!(media.comment, "needs a re-record");
    assert_eq!(media.scene, "5A");
    assert_eq!(media.take, "2");
    assert_eq!(reloaded.project.bins[0].name, "Selects");
    assert_eq!(reloaded.schema_version, saturn_core::project::PROJECT_SCHEMA_VERSION);
}

#[test]
fn v13_projects_reopen_with_graphics_arrays_and_captions_intact() {
    // Build a current project that carries captions ..., save it, then
    // rewrite it as a v13 file (no `graphics` on sequences, no
    // `graphic_templates`, version field 13) to simulate a pre-Phase-5-slice-2
    // project on disk.
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("legacy.saturn");
    let mut engine = saturn_core::EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            serde_json::json!({"name": "Legacy"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "subtitle.add",
            serde_json::json!({"start_ticks": 0, "end_ticks": 254_016_000_000i64, "text": "Hi"}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new("file.save", serde_json::json!({"path": path})))
        .unwrap();

    // Demote the file on disk to v13.
    let mut raw = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(&path).unwrap(),
    )
    .unwrap();
    {
        let project = raw.get_mut("project").unwrap();
        let sequence = project.get_mut("sequence").unwrap();
        let sequence = sequence.as_object_mut().unwrap();
        sequence.remove("graphics");
        project
            .as_object_mut()
            .unwrap()
            .remove("graphic_templates");
        let sequences = project.get_mut("sequences").unwrap();
        if let Some(sequences) = sequences.as_array_mut() {
            for sequence in sequences.iter_mut() {
                sequence
                    .as_object_mut()
                    .unwrap()
                    .remove("graphics");
            }
        }
        raw.as_object_mut()
            .unwrap()
            .insert("schema_version".into(), serde_json::json!(13));
    }
    std::fs::write(&path, serde_json::to_string(&raw).unwrap()).unwrap();

    let reloaded = saturn_core::ProjectDocument::load(&path).unwrap();
    assert_eq!(
        reloaded.schema_version,
        saturn_core::project::PROJECT_SCHEMA_VERSION,
        "v13 must migrate up to the current version"
    );
    assert_eq!(reloaded.project.sequence.graphics, Vec::new());
    assert!(reloaded.project.graphic_templates.is_empty());
    // Earlier phase data survives the graphics migration untouched.
    assert_eq!(reloaded.project.sequence.caption_tracks.len(), 1);
    assert_eq!(reloaded.project.sequence.caption_tracks[0].cues[0].text, "Hi");
}
