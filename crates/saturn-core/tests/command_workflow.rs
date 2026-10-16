use saturn_core::{CommandRequest, EditorEngine};
use serde_json::json;

#[test]
fn new_add_save_reopen_and_list_media_uses_one_command_path() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("workflow.saturn");
    let mut engine = EditorEngine::default();

    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Workflow"}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name":"sample.mov", "path":"/assets/sample.mov", "kind":"unknown"
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.update_media",
            json!({
                "path":"/assets/sample.mov", "kind":"video",
                "duration":254016000000_i64, "width":1920, "height":1080
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new("file.save", json!({"path":path})))
        .unwrap();

    let mut reopened = EditorEngine::default();
    reopened
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    let listed = reopened
        .execute(CommandRequest::new("media.list", serde_json::Value::Null))
        .unwrap();
    assert_eq!(listed[0]["name"], "sample.mov");
    assert_eq!(listed[0]["path"], "/assets/sample.mov");
    assert!(!reopened.is_dirty());
}

#[test]
fn project_status_and_timeline_clip_commands_round_trip_with_undo() {
    use saturn_core::TICKS_PER_SECOND;

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("timeline.saturn");
    let mut engine = EditorEngine::default();

    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Timeline workflow"}),
        ))
        .unwrap();
    let initial_status = engine
        .execute(CommandRequest::new(
            "project.status",
            serde_json::Value::Null,
        ))
        .unwrap();
    assert_eq!(
        initial_status["document"]["project"]["name"],
        "Timeline workflow"
    );
    assert_eq!(initial_status["path"], serde_json::Value::Null);
    assert_eq!(initial_status["is_dirty"], true);

    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name":"scene.mp4", "path":"/assets/scene.mp4", "kind":"video",
                "duration": TICKS_PER_SECOND * 10, "width": 1920, "height": 1080
            }),
        ))
        .unwrap();
    let media_id = media["media_id"].as_u64().unwrap();

    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id":media_id,"track_id":1,"duration":TICKS_PER_SECOND * 2}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "clip.set_color",
            json!({"track_id":1,"clip_index":0,"exposure":1.25,"contrast":1.2,"saturation":0.8}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "track.set_gain_db",
            json!({"track_id":2,"gain_db":-6.0}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "clip.set_keyframe",
            json!({"track_id":1,"clip_index":0,"property":"position_x","time":TICKS_PER_SECOND / 2,"value":120.0}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "clip.set_keyframe",
            json!({"track_id":1,"clip_index":0,"property":"position_x","time":TICKS_PER_SECOND / 2,"value":240.0}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id":media_id,"track_id":1,"duration":TICKS_PER_SECOND * 2}),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[1]
            .timeline_start
            .0,
        TICKS_PER_SECOND * 2
    );

    let moved = engine
        .execute(CommandRequest::new(
            "timeline.move_clip",
            json!({
                "track_id":1,"clip_index":0,
                "timeline_start":TICKS_PER_SECOND * 5
            }),
        ))
        .unwrap();
    assert_eq!(moved["track_id"], 1);
    assert_eq!(moved["clip_index"], 1);
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[1]
            .timeline_start
            .0,
        TICKS_PER_SECOND * 5
    );

    let audio_drop = engine.execute(CommandRequest::new(
        "timeline.move_clip",
        json!({
            "track_id":1,"clip_index":1,"target_track_id":2,
            "timeline_start":TICKS_PER_SECOND * 3
        }),
    ));
    assert!(audio_drop.is_err());
    assert!(
        engine.document().project.sequence.tracks[1]
            .clips
            .is_empty()
    );

    engine
        .execute(CommandRequest::new(
            "timeline.remove_clip",
            json!({"track_id":1,"clip_index":1}),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 2);

    engine
        .execute(CommandRequest::new("file.save", json!({"path":path})))
        .unwrap();
    let saved_status = engine
        .execute(CommandRequest::new(
            "project.status",
            serde_json::Value::Null,
        ))
        .unwrap();
    assert_eq!(saved_status["is_dirty"], false);
    assert_eq!(saved_status["path"], path.to_string_lossy().as_ref());

    let mut reopened = EditorEngine::default();
    reopened
        .execute(CommandRequest::new("file.open", json!({"path":path})))
        .unwrap();
    assert_eq!(
        reopened.document().project.sequence.tracks[0].clips.len(),
        2
    );
    assert_eq!(
        reopened.document().project.sequence.tracks[0].clips[1]
            .timeline_start
            .0,
        TICKS_PER_SECOND * 5
    );
    assert_eq!(
        reopened.document().project.sequence.tracks[0].clips[1]
            .color
            .exposure,
        1.25
    );
    assert_eq!(
        reopened.document().project.sequence.tracks[0].clips[1]
            .keyframes
            .len(),
        1
    );
    assert_eq!(
        reopened.document().project.sequence.tracks[0].clips[1].keyframes[0].value,
        240.0
    );
    assert_eq!(reopened.document().project.sequence.tracks[1].gain_db, -6.0);
    assert!(!reopened.is_dirty());
}

#[test]
fn transitions_and_title_clips_workflow_with_undo() {
    use saturn_core::{TICKS_PER_SECOND, TransitionKind};

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("transition_title.saturn");
    let mut engine = EditorEngine::default();

    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Transitions & Titles"}),
        ))
        .unwrap();

    // Add title clip
    let title_res = engine
        .execute(CommandRequest::new(
            "timeline.add_title",
            json!({
                "text": "Intro Title",
                "subtitle": "Chapter 1",
                "style": "lower_third",
                "duration": TICKS_PER_SECOND * 3,
            }),
        ))
        .unwrap();
    let title_media_id = title_res["media_id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);
    assert_eq!(engine.document().project.media.len(), 1);

    // Update title properties
    engine
        .execute(CommandRequest::new(
            "title.update",
            json!({
                "media_id": title_media_id,
                "text": "Intro Title Updated",
                "color": "#38bdf8",
            }),
        ))
        .unwrap();
    let updated_media = &engine.document().project.media[0];
    assert_eq!(updated_media.title.as_ref().unwrap().text, "Intro Title Updated");
    assert_eq!(updated_media.title.as_ref().unwrap().color, "#38bdf8");

    // Add transition on Track 1
    let trans_res = engine
        .execute(CommandRequest::new(
            "timeline.add_transition",
            json!({
                "track_id": 1,
                "cut_at": TICKS_PER_SECOND * 3,
                "duration": TICKS_PER_SECOND,
                "kind": "crossfade",
            }),
        ))
        .unwrap();
    let trans_id = trans_res["transition_id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].transitions.len(), 1);
    assert_eq!(
        engine.document().project.sequence.tracks[0].transitions[0].kind,
        TransitionKind::Crossfade
    );

    // Save and reload
    engine
        .execute(CommandRequest::new("file.save", json!({"path": path})))
        .unwrap();

    let mut loaded = EditorEngine::default();
    loaded
        .execute(CommandRequest::new("file.open", json!({"path": path})))
        .unwrap();
    assert_eq!(loaded.document().project.sequence.tracks[0].transitions.len(), 1);
    assert_eq!(loaded.document().project.media[0].title.as_ref().unwrap().text, "Intro Title Updated");

    // Test remove transition with undo
    loaded
        .execute(CommandRequest::new(
            "timeline.remove_transition",
            json!({"track_id": 1, "transition_id": trans_id}),
        ))
        .unwrap();
    assert_eq!(loaded.document().project.sequence.tracks[0].transitions.len(), 0);

    loaded
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(loaded.document().project.sequence.tracks[0].transitions.len(), 1);
}

