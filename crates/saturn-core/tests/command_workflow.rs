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

#[test]
fn effect_stack_workflow_with_undo() {
    use saturn_core::EffectKind;

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("effect_stack.saturn");
    let mut engine = EditorEngine::default();

    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Effect Stack Test"}),
        ))
        .unwrap();

    let media_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "clip.mp4",
                "path": "/tmp/clip.mp4",
                "kind": "video"
            }),
        ))
        .unwrap();
    let media_id = media_res["media_id"].as_u64().unwrap();

    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": 1,
                "media_id": media_id,
                "duration": 500
            }),
        ))
        .unwrap();

    // 1. Add Blur effect
    let add_blur = engine
        .execute(CommandRequest::new(
            "clip.add_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "kind": "gaussian_blur"
            }),
        ))
        .unwrap();
    let blur_id = add_blur["effect_id"].as_u64().unwrap();

    // 2. Add Crop effect
    let add_crop = engine
        .execute(CommandRequest::new(
            "clip.add_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "kind": "crop"
            }),
        ))
        .unwrap();
    let crop_id = add_crop["effect_id"].as_u64().unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert_eq!(clip.effects.len(), 2);
    assert_eq!(clip.effects[0].kind(), EffectKind::GaussianBlur);
    assert_eq!(clip.effects[1].kind(), EffectKind::Crop);

    // 3. Update Crop effect parameters
    engine
        .execute(CommandRequest::new(
            "clip.update_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "effect_id": crop_id,
                "payload": {
                    "kind": "crop",
                    "params": {
                        "top": 10.0,
                        "bottom": 10.0,
                        "left": 5.0,
                        "right": 5.0
                    }
                }
            }),
        ))
        .unwrap();

    // 4. Toggle Blur effect off
    engine
        .execute(CommandRequest::new(
            "clip.toggle_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "effect_id": blur_id,
                "enabled": false
            }),
        ))
        .unwrap();
    assert!(!engine.document().project.sequence.tracks[0].clips[0].effects[0].enabled);

    // 5. Reorder effects (move crop to index 0)
    engine
        .execute(CommandRequest::new(
            "clip.reorder_effects",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "from_index": 1,
                "to_index": 0
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].effects[0].id,
        crop_id
    );

    // 6. Test undo/redo
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].effects[0].id,
        blur_id
    );

    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].effects[0].id,
        crop_id
    );

    // 7. Save and reload
    engine
        .execute(CommandRequest::new("file.save", json!({"path": path})))
        .unwrap();

    let mut loaded = EditorEngine::default();
    loaded
        .execute(CommandRequest::new("file.open", json!({"path": path})))
        .unwrap();
    let loaded_clip = &loaded.document().project.sequence.tracks[0].clips[0];
    assert_eq!(loaded_clip.effects.len(), 2);
    assert_eq!(loaded_clip.effects[0].id, crop_id);
    assert_eq!(loaded_clip.effects[1].id, blur_id);

    // 8. Remove effect
    loaded
        .execute(CommandRequest::new(
            "clip.remove_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "effect_id": crop_id
            }),
        ))
        .unwrap();
    assert_eq!(
        loaded.document().project.sequence.tracks[0].clips[0].effects.len(),
        1
    );

    loaded
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        loaded.document().project.sequence.tracks[0].clips[0].effects.len(),
        2
    );
}

#[test]
fn track_audio_and_lock_controls_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Audio and Lock Test"}),
        ))
        .unwrap();

    let track_id = engine.document().project.sequence.tracks[0].id;

    // 1. Set track pan
    engine
        .execute(CommandRequest::new(
            "track.set_pan",
            json!({"track_id": track_id, "pan": -0.75}),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].pan,
        -0.75
    );

    // 2. Set track mute
    engine
        .execute(CommandRequest::new(
            "track.set_muted",
            json!({"track_id": track_id, "muted": true}),
        ))
        .unwrap();
    assert!(engine.document().project.sequence.tracks[0].muted);

    // 3. Set track solo
    engine
        .execute(CommandRequest::new(
            "track.set_solo",
            json!({"track_id": track_id, "solo": true}),
        ))
        .unwrap();
    assert!(engine.document().project.sequence.tracks[0].solo);

    // 4. Set track locked
    engine
        .execute(CommandRequest::new(
            "track.set_locked",
            json!({"track_id": track_id, "locked": true}),
        ))
        .unwrap();
    assert!(engine.document().project.sequence.tracks[0].locked);

    // 5. Verify editing on locked track fails
    let err = engine.execute(CommandRequest::new(
        "timeline.add_clip",
        json!({"track_id": track_id, "media_id": 1}),
    ));
    assert!(err.is_err());

    // 6. Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert!(!engine.document().project.sequence.tracks[0].locked);

    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert!(!engine.document().project.sequence.tracks[0].solo);

    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert!(!engine.document().project.sequence.tracks[0].muted);

    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].pan, 0.0);
}

#[test]
fn rolling_and_slide_edit_workflow_with_undo() {
    use saturn_core::TICKS_PER_SECOND;

    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name":"Roll & Slide Test"})))
        .unwrap();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name":"clip.mp4", "path":"/assets/clip.mp4", "kind":"video"}),
        ))
        .unwrap();
    let media_id = media["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "project.update_media",
            json!({"path":"/assets/clip.mp4", "kind":"video", "duration": 20 * TICKS_PER_SECOND}),
        ))
        .unwrap();

    let track_id = engine.document().project.sequence.tracks[0].id;

    // Add 3 adjacent clips (3 seconds each)
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": track_id, "media_id": media_id, "timeline_start": 0, "source_in": 0, "duration": 3 * TICKS_PER_SECOND}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": track_id, "media_id": media_id, "timeline_start": 3 * TICKS_PER_SECOND, "source_in": 0, "duration": 3 * TICKS_PER_SECOND}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": track_id, "media_id": media_id, "timeline_start": 6 * TICKS_PER_SECOND, "source_in": 0, "duration": 3 * TICKS_PER_SECOND}),
        ))
        .unwrap();

    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 3);

    // 1. Perform Rolling Edit: Move boundary between Clip 0 and Clip 1 right by 1s (+TICKS_PER_SECOND)
    engine
        .execute(CommandRequest::new(
            "timeline.roll_edit",
            json!({"track_id": track_id, "left_clip_index": 0, "delta": TICKS_PER_SECOND}),
        ))
        .unwrap();

    let clips = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips[0].duration.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[1].timeline_start.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[1].source_in.0, 1 * TICKS_PER_SECOND);
    assert_eq!(clips[1].duration.0, 2 * TICKS_PER_SECOND);

    // Undo Rolling Edit
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let clips = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips[0].duration.0, 3 * TICKS_PER_SECOND);
    assert_eq!(clips[1].timeline_start.0, 3 * TICKS_PER_SECOND);
    assert_eq!(clips[1].source_in.0, 0);
    assert_eq!(clips[1].duration.0, 3 * TICKS_PER_SECOND);

    // Redo Rolling Edit
    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    let clips = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips[0].duration.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[1].timeline_start.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[1].source_in.0, 1 * TICKS_PER_SECOND);

    // 2. Perform Slide Edit: Slide Clip 1 right by 0.5s
    let delta = TICKS_PER_SECOND / 2;
    engine
        .execute(CommandRequest::new(
            "timeline.slide_clip",
            json!({"track_id": track_id, "clip_index": 1, "delta": delta}),
        ))
        .unwrap();

    let clips = &engine.document().project.sequence.tracks[0].clips;
    // Clip 0 duration expands to fill gap: 4s + 0.5s = 4.5s
    assert_eq!(clips[0].duration.0, 4 * TICKS_PER_SECOND + delta);
    // Clip 1 start moves right by 0.5s: 4s + 0.5s = 4.5s, duration stays 2s
    assert_eq!(clips[1].timeline_start.0, 4 * TICKS_PER_SECOND + delta);
    assert_eq!(clips[1].duration.0, 2 * TICKS_PER_SECOND);
    // Clip 2 start moves right by 0.5s: 6s + 0.5s = 6.5s, source_in increases by 0.5s, duration shrinks by 0.5s
    assert_eq!(clips[2].timeline_start.0, 6 * TICKS_PER_SECOND + delta);
    assert_eq!(clips[2].source_in.0, delta);
    assert_eq!(clips[2].duration.0, 3 * TICKS_PER_SECOND - delta);

    // Undo Slide Edit
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let clips = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips[0].duration.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[1].timeline_start.0, 4 * TICKS_PER_SECOND);
    assert_eq!(clips[2].timeline_start.0, 6 * TICKS_PER_SECOND);
    assert_eq!(clips[2].source_in.0, 0);
    assert_eq!(clips[2].duration.0, 3 * TICKS_PER_SECOND);
}

#[test]
fn rate_stretch_and_clip_link_workflow_with_undo() {
    use saturn_core::TICKS_PER_SECOND;

    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name":"Rate & Link Test"})))
        .unwrap();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name":"video.mp4", "path":"/assets/video.mp4", "kind":"video"}),
        ))
        .unwrap();
    let media_id = media["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "project.update_media",
            json!({"path":"/assets/video.mp4", "kind":"video", "duration": 30 * TICKS_PER_SECOND}),
        ))
        .unwrap();

    let v_track_id = engine.document().project.sequence.tracks[0].id;
    let a_track_id = engine.document().project.sequence.tracks[1].id;

    // Add video clip and audio clip
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": v_track_id, "media_id": media_id, "timeline_start": 0, "source_in": 0, "duration": 4 * TICKS_PER_SECOND}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"track_id": a_track_id, "media_id": media_id, "timeline_start": 0, "source_in": 0, "duration": 4 * TICKS_PER_SECOND}),
        ))
        .unwrap();

    // 1. Link the two clips
    let link_res = engine
        .execute(CommandRequest::new(
            "timeline.link_clips",
            json!({
                "clips": [
                    {"track_id": v_track_id, "clip_index": 0},
                    {"track_id": a_track_id, "clip_index": 0}
                ]
            }),
        ))
        .unwrap();
    assert!(link_res["linked"].as_bool().unwrap());
    let group_id = link_res["group_id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].group_id, Some(group_id));
    assert_eq!(engine.document().project.sequence.tracks[1].clips[0].group_id, Some(group_id));

    // 2. Batch move linked clips together by +2s
    engine
        .execute(CommandRequest::new(
            "timeline.move_clips_batch",
            json!({
                "moves": [
                    {"track_id": v_track_id, "clip_index": 0, "timeline_start": 2 * TICKS_PER_SECOND},
                    {"track_id": a_track_id, "clip_index": 0, "timeline_start": 2 * TICKS_PER_SECOND}
                ]
            }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].timeline_start.0, 2 * TICKS_PER_SECOND);
    assert_eq!(engine.document().project.sequence.tracks[1].clips[0].timeline_start.0, 2 * TICKS_PER_SECOND);

    // 3. Rate Stretch video clip from 4s to 8s (slow motion -> 0.5x speed)
    let stretch_res = engine
        .execute(CommandRequest::new(
            "timeline.rate_stretch",
            json!({
                "track_id": v_track_id,
                "clip_index": 0,
                "new_duration": 8 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();
    assert!(stretch_res["stretched"].as_bool().unwrap());
    assert_eq!(stretch_res["speed"].as_f64().unwrap(), 0.5);
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].duration.0, 8 * TICKS_PER_SECOND);
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].speed, 0.5);

    // Test setting speed directly: 2.0x (fast motion)
    let speed_res = engine
        .execute(CommandRequest::new(
            "timeline.rate_stretch",
            json!({
                "track_id": v_track_id,
                "clip_index": 0,
                "speed": 2.0
            }),
        ))
        .unwrap();
    assert_eq!(speed_res["speed"].as_f64().unwrap(), 2.0);
    // At 2x speed, the 4s content takes 2s timeline duration
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].duration.0, 2 * TICKS_PER_SECOND);

    // Undo rate stretch
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].speed, 0.5);
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].speed, 1.0);
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].duration.0, 4 * TICKS_PER_SECOND);

    // 4. Unlink clips
    engine
        .execute(CommandRequest::new(
            "timeline.unlink_clips",
            json!({
                "clips": [{"track_id": v_track_id, "clip_index": 0}]
            }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].group_id, None);
    assert_eq!(engine.document().project.sequence.tracks[1].clips[0].group_id, None);

    // Undo unlink
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips[0].group_id, Some(group_id));
    assert_eq!(engine.document().project.sequence.tracks[1].clips[0].group_id, Some(group_id));
}

