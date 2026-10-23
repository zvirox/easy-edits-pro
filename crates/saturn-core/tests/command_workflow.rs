use saturn_core::{CommandRequest, EditorEngine, EffectPayload, TICKS_PER_SECOND};
use serde_json::{Value, json};

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
    assert_eq!(
        updated_media.title.as_ref().unwrap().text,
        "Intro Title Updated"
    );
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
    assert_eq!(
        engine.document().project.sequence.tracks[0]
            .transitions
            .len(),
        1
    );
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
    assert_eq!(
        loaded.document().project.sequence.tracks[0]
            .transitions
            .len(),
        1
    );
    assert_eq!(
        loaded.document().project.media[0]
            .title
            .as_ref()
            .unwrap()
            .text,
        "Intro Title Updated"
    );

    // Test remove transition with undo
    loaded
        .execute(CommandRequest::new(
            "timeline.remove_transition",
            json!({"track_id": 1, "transition_id": trans_id}),
        ))
        .unwrap();
    assert_eq!(
        loaded.document().project.sequence.tracks[0]
            .transitions
            .len(),
        0
    );

    loaded
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        loaded.document().project.sequence.tracks[0]
            .transitions
            .len(),
        1
    );
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
        loaded.document().project.sequence.tracks[0].clips[0]
            .effects
            .len(),
        1
    );

    loaded
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        loaded.document().project.sequence.tracks[0].clips[0]
            .effects
            .len(),
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
    assert_eq!(engine.document().project.sequence.tracks[0].pan, -0.75);

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
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Roll & Slide Test"}),
        ))
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
        .execute(CommandRequest::new(
            "project.new",
            json!({"name":"Rate & Link Test"}),
        ))
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
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].group_id,
        Some(group_id)
    );
    assert_eq!(
        engine.document().project.sequence.tracks[1].clips[0].group_id,
        Some(group_id)
    );

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
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0]
            .timeline_start
            .0,
        2 * TICKS_PER_SECOND
    );
    assert_eq!(
        engine.document().project.sequence.tracks[1].clips[0]
            .timeline_start
            .0,
        2 * TICKS_PER_SECOND
    );

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
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0]
            .duration
            .0,
        8 * TICKS_PER_SECOND
    );
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].speed,
        0.5
    );

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
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0]
            .duration
            .0,
        2 * TICKS_PER_SECOND
    );

    // Undo rate stretch
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].speed,
        0.5
    );
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].speed,
        1.0
    );
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0]
            .duration
            .0,
        4 * TICKS_PER_SECOND
    );

    // 4. Unlink clips
    engine
        .execute(CommandRequest::new(
            "timeline.unlink_clips",
            json!({
                "clips": [{"track_id": v_track_id, "clip_index": 0}]
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].group_id,
        None
    );
    assert_eq!(
        engine.document().project.sequence.tracks[1].clips[0].group_id,
        None
    );

    // Undo unlink
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].group_id,
        Some(group_id)
    );
    assert_eq!(
        engine.document().project.sequence.tracks[1].clips[0].group_id,
        Some(group_id)
    );
}

#[test]
fn audio_processing_and_color_hue_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({ "name": "Audio & Color Test" }),
        ))
        .unwrap();

    let a1_id = engine.document().project.sequence.tracks[1].id;

    // 1. Set Track Audio Processing (EQ, Compressor, Denoise)
    engine
        .execute(CommandRequest::new(
            "track.set_audio_processing",
            json!({
                "track_id": a1_id,
                "eq_enabled": true,
                "eq_low_gain": 3.5,
                "eq_mid_gain": -2.0,
                "eq_high_gain": 4.0,
                "comp_enabled": true,
                "comp_threshold": -18.0,
                "comp_ratio": 3.5,
                "comp_attack": 0.01,
                "comp_release": 0.15,
                "denoise_enabled": true,
                "denoise_amount": 0.8
            }),
        ))
        .unwrap();

    let ap = &engine.document().project.sequence.tracks[1].audio_processing;
    assert!(ap.eq_enabled);
    assert_eq!(ap.eq_low_gain, 3.5);
    assert_eq!(ap.eq_mid_gain, -2.0);
    assert_eq!(ap.eq_high_gain, 4.0);
    assert!(ap.comp_enabled);
    assert_eq!(ap.comp_threshold, -18.0);
    assert_eq!(ap.comp_ratio, 3.5);
    assert!(ap.denoise_enabled);
    assert_eq!(ap.denoise_amount, 0.8);

    // Undo audio processing
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let ap_undone = &engine.document().project.sequence.tracks[1].audio_processing;
    assert!(!ap_undone.eq_enabled);
    assert_eq!(ap_undone.eq_low_gain, 0.0);
    assert!(!ap_undone.comp_enabled);

    // Redo audio processing
    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[1]
            .audio_processing
            .eq_enabled
    );
}

#[test]
fn subtitles_crud_and_undo_workflow() {
    let mut engine = EditorEngine::default();

    // 1. Add subtitle 1
    let res1 = engine
        .execute(CommandRequest::new(
            "subtitle.add",
            json!({
                "start_ticks": 254_016_000_000i64, // 1.0s
                "end_ticks": 254_016_000_000i64 * 4, // 4.0s
                "text": "First caption line"
            }),
        ))
        .unwrap();
    let sub1_id = res1["id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 1);
    assert_eq!(
        engine.document().project.sequence.caption_tracks[0].cues[0].text,
        "First caption line"
    );

    // 2. Add subtitle 2 (earlier time to verify sorting)
    let res2 = engine
        .execute(CommandRequest::new(
            "subtitle.add",
            json!({
                "start_ticks": 0,
                "end_ticks": 254_016_000_000i64,
                "text": "Intro caption"
            }),
        ))
        .unwrap();
    let sub2_id = res2["id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 2);
    // Intro should be first after sort
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues[0].id, sub2_id);
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues[1].id, sub1_id);

    // 3. Update subtitle 1 text
    engine
        .execute(CommandRequest::new(
            "subtitle.update",
            json!({
                "id": sub1_id,
                "text": "Updated caption text"
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.caption_tracks[0].cues[1].text,
        "Updated caption text"
    );

    // 4. Undo update
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.caption_tracks[0].cues[1].text,
        "First caption line"
    );

    // 5. Redo update
    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.caption_tracks[0].cues[1].text,
        "Updated caption text"
    );

    // 6. Remove subtitle 2
    engine
        .execute(CommandRequest::new(
            "subtitle.remove",
            json!({ "id": sub2_id }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 1);
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues[0].id, sub1_id);

    // 7. Clear subtitles
    engine
        .execute(CommandRequest::new("subtitle.clear", json!({})))
        .unwrap();
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 0);

    // Undo clear
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 1);
}

#[test]
fn keyframe_bezier_interpolation_workflow() {
    let mut engine = EditorEngine::default();
    let media_path = "/assets/clip.mov";
    let add_media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "clip.mov",
                "path": media_path,
                "kind": "video",
                "duration": 254_016_000_000i64 * 10
            }),
        ))
        .unwrap();
    let media_id = add_media["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({ "media_id": media_id, "track_id": 1, "timeline_start": 0 }),
        ))
        .unwrap();

    // Set linear keyframe
    engine
        .execute(CommandRequest::new(
            "clip.set_keyframe",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "property": "opacity",
                "time": 0,
                "value": 0.0,
                "interpolation": "linear"
            }),
        ))
        .unwrap();

    // Set bezier keyframe with tangent handles
    engine
        .execute(CommandRequest::new(
            "clip.set_keyframe",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "property": "opacity",
                "time": 254_016_000_000i64 * 2,
                "value": 1.0,
                "interpolation": "bezier",
                "handle_in_x": 0.42,
                "handle_in_y": 0.0,
                "handle_out_x": 0.58,
                "handle_out_y": 1.0
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert_eq!(clip.keyframes.len(), 2);
    assert_eq!(
        clip.keyframes[0].interpolation,
        saturn_core::InterpolationKind::Linear
    );
    assert_eq!(
        clip.keyframes[1].interpolation,
        saturn_core::InterpolationKind::Bezier
    );
    assert_eq!(clip.keyframes[1].handle_in_x, Some(0.42));
    assert_eq!(clip.keyframes[1].handle_out_y, Some(1.0));
}

#[test]
fn lut_effect_workflow() {
    let mut engine = EditorEngine::default();
    let media_path = "/assets/clip.mov";
    let add_media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "clip.mov",
                "path": media_path,
                "kind": "video",
                "duration": 254_016_000_000i64 * 10
            }),
        ))
        .unwrap();
    let media_id = add_media["media_id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({ "media_id": media_id, "track_id": 1, "timeline_start": 0 }),
        ))
        .unwrap();

    // Add 3D LUT effect
    let res = engine
        .execute(CommandRequest::new(
            "clip.add_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "kind": "lut",
                "name": "Cinematic Film LUT"
            }),
        ))
        .unwrap();
    let eff_id = res["effect_id"].as_u64().unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert_eq!(clip.effects.len(), 1);
    assert_eq!(clip.effects[0].kind(), saturn_core::EffectKind::Lut);

    // Update LUT effect payload
    engine
        .execute(CommandRequest::new(
            "clip.update_effect",
            json!({
                "track_id": 1,
                "clip_index": 0,
                "effect_id": eff_id,
                "payload": {
                    "kind": "lut",
                    "params": {
                        "name": "Warm Vintage",
                        "intensity": 0.85,
                        "preset_id": "vintage"
                    }
                }
            }),
        ))
        .unwrap();

    let updated = &engine.document().project.sequence.tracks[0].clips[0].effects[0];
    if let saturn_core::EffectPayload::Lut(ref p) = updated.payload {
        assert_eq!(p.name, "Warm Vintage");
        assert_eq!(p.intensity, 0.85);
        assert_eq!(p.preset_id.as_deref(), Some("vintage"));
    } else {
        panic!("Expected Lut payload");
    }
}

#[test]
fn color_wheels_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    let m_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({ "name": "color_sample.mp4", "path": "/media/color_sample.mp4", "kind": "video" }),
        ))
        .unwrap();
    let media_id = m_res["media_id"].as_u64().unwrap();

    let clip_res = engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": 1,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 500,
            }),
        ))
        .unwrap();
    let clip_index = clip_res["clip_index"].as_u64().unwrap() as usize;

    // Set interactive color wheels
    engine
        .execute(CommandRequest::new(
            "clip.set_color_wheels",
            json!({
                "track_id": 1,
                "clip_index": clip_index,
                "wheels": {
                    "lift": { "red": 0.1, "green": -0.05, "blue": 0.0, "luma": -0.2 },
                    "gamma": { "red": -0.02, "green": 0.05, "blue": 0.1, "luma": 0.05 },
                    "gain": { "red": 0.15, "green": 0.1, "blue": -0.1, "luma": 0.1 },
                    "offset": { "red": 0.0, "green": 0.0, "blue": 0.0, "luma": 0.0 }
                }
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[clip_index];
    assert_eq!(clip.color.wheels.lift.red, 0.1);
    assert_eq!(clip.color.wheels.lift.luma, -0.2);
    assert_eq!(clip.color.wheels.gain.red, 0.15);

    // Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let undone = &engine.document().project.sequence.tracks[0].clips[clip_index];
    assert_eq!(undone.color.wheels.lift.red, 0.0);
    assert_eq!(undone.color.wheels.lift.luma, 0.0);

    // Test Redo
    engine
        .execute(CommandRequest::new("edit.redo", Value::Null))
        .unwrap();
    let redone = &engine.document().project.sequence.tracks[0].clips[clip_index];
    assert_eq!(redone.color.wheels.lift.red, 0.1);
    assert_eq!(redone.color.wheels.lift.luma, -0.2);
}

#[test]
fn multicam_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    let m1_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({ "name": "cam_a.mp4", "path": "/media/cam_a.mp4", "kind": "video" }),
        ))
        .unwrap();
    let m1_id = m1_res["media_id"].as_u64().unwrap();

    let m2_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({ "name": "cam_b.mp4", "path": "/media/cam_b.mp4", "kind": "video" }),
        ))
        .unwrap();
    let m2_id = m2_res["media_id"].as_u64().unwrap();

    let clip_res = engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": 1,
                "media_id": m1_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 600,
            }),
        ))
        .unwrap();
    let clip_index = clip_res["clip_index"].as_u64().unwrap() as usize;

    // Create Multicam group
    let multi_res = engine
        .execute(CommandRequest::new(
            "multicam.create",
            json!({
                "name": "Interview Multicam",
                "angles": [
                    { "id": 1, "name": "Host (Cam A)", "media_id": m1_id, "sync_offset_ticks": 0 },
                    { "id": 2, "name": "Guest (Cam B)", "media_id": m2_id, "sync_offset_ticks": 100 }
                ],
                "sync_mode": "audio_waveform"
            }),
        ))
        .unwrap();
    let multicam_id = multi_res["id"].as_u64().unwrap();

    assert_eq!(engine.document().project.sequence.multicam_clips.len(), 1);
    assert_eq!(
        engine.document().project.sequence.multicam_clips[0].name,
        "Interview Multicam"
    );

    // Switch active angle to Angle 2 (Cam B)
    engine
        .execute(CommandRequest::new(
            "multicam.switch_angle",
            json!({
                "track_id": 1,
                "clip_index": clip_index,
                "angle_id": 2
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[clip_index];
    assert_eq!(clip.multicam_id, Some(multicam_id));
    assert_eq!(clip.multicam_angle_id, Some(2));
    assert_eq!(clip.media_id, m2_id);
    assert_eq!(clip.source_in, saturn_core::Tick(100));

    // Sync multicam with detected audio offsets
    engine
        .execute(CommandRequest::new(
            "multicam.sync",
            json!({
                "multicam_id": multicam_id,
                "sync_mode": "audio_waveform",
                "offsets": { "1": 0, "2": 250 }
            }),
        ))
        .unwrap();

    let group = &engine.document().project.sequence.multicam_clips[0];
    assert_eq!(group.angles[1].sync_offset_ticks, saturn_core::Tick(250));

    // Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.multicam_clips[0].angles[1].sync_offset_ticks,
        saturn_core::Tick(100)
    );
}

#[test]
fn motion_tracking_and_pinning_workflow() {
    let mut engine = EditorEngine::default();
    let m_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({ "name": "motion.mp4", "path": "/media/motion.mp4", "kind": "video" }),
        ))
        .unwrap();
    let media_id = m_res["media_id"].as_u64().unwrap();

    let clip_res = engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": 1,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 500,
            }),
        ))
        .unwrap();
    let clip_index = clip_res["clip_index"].as_u64().unwrap() as usize;

    // Set motion tracking
    engine
        .execute(CommandRequest::new(
            "clip.set_motion_tracking",
            json!({
                "track_id": 1,
                "clip_index": clip_index,
                "tracking": {
                    "id": 1,
                    "name": "Skateboarder",
                    "mode": "stabilize",
                    "points": [
                        { "time": 0, "x": 0.5, "y": 0.5, "confidence": 0.99 },
                        { "time": 250, "x": 0.52, "y": 0.49, "confidence": 0.97 },
                        { "time": 500, "x": 0.55, "y": 0.48, "confidence": 0.95 }
                    ],
                    "pinned_clip_id": null,
                    "smoothness": 0.75
                }
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[clip_index];
    assert!(clip.tracking.is_some());
    let tr = clip.tracking.as_ref().unwrap();
    assert_eq!(tr.name, "Skateboarder");
    assert_eq!(tr.points.len(), 3);
    assert_eq!(tr.smoothness, 0.75);

    // Pin a sticker/title clip to track
    engine
        .execute(CommandRequest::new(
            "clip.pin_to_track",
            json!({
                "track_id": 1,
                "clip_index": clip_index,
                "pinned_clip_id": 99
            }),
        ))
        .unwrap();

    let pinned = engine.document().project.sequence.tracks[0].clips[clip_index]
        .tracking
        .as_ref()
        .unwrap();
    assert_eq!(pinned.pinned_clip_id, Some(99));

    // Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let unpinned = engine.document().project.sequence.tracks[0].clips[clip_index]
        .tracking
        .as_ref()
        .unwrap();
    assert_eq!(unpinned.pinned_clip_id, None);
}

#[test]
fn fairlight_bus_routing_and_automation_workflow() {
    let mut engine = EditorEngine::default();

    // 1. Set track bus routing
    engine
        .execute(CommandRequest::new(
            "track.set_bus_route",
            json!({
                "track_id": 2,
                "bus": "submix1"
            }),
        ))
        .unwrap();

    assert_eq!(
        engine.document().project.sequence.tracks[1].bus,
        saturn_core::BusTarget::Submix1
    );

    // 2. Set fader volume & pan automation
    engine
        .execute(CommandRequest::new(
            "track.set_automation",
            json!({
                "track_id": 2,
                "automation": {
                    "mode": "read",
                    "volume_points": [
                        { "time": 0, "value": 0.0 },
                        { "time": 254016000000_i64, "value": -6.0 }
                    ],
                    "pan_points": [
                        { "time": 0, "value": -0.5 },
                        { "time": 254016000000_i64, "value": 0.5 }
                    ]
                }
            }),
        ))
        .unwrap();

    let auto = &engine.document().project.sequence.tracks[1].automation;
    assert_eq!(auto.mode, saturn_core::AutomationMode::Read);
    assert_eq!(auto.volume_points.len(), 2);
    assert_eq!(auto.pan_points.len(), 2);

    // 3. Add Audio Plugin (Parametric EQ / VST3)
    engine
        .execute(CommandRequest::new(
            "track.add_audio_plugin",
            json!({
                "track_id": 2,
                "plugin": {
                    "id": 101,
                    "name": "Fairlight 4-Band Studio EQ",
                    "plugin_format": "vst3",
                    "enabled": true,
                    "preset": "Warm Dialogue",
                    "params": {
                        "gain_low": 2.0,
                        "gain_high": -1.5
                    }
                }
            }),
        ))
        .unwrap();

    assert_eq!(
        engine.document().project.sequence.tracks[1].plugins.len(),
        1
    );
    assert_eq!(
        engine.document().project.sequence.tracks[1].plugins[0].name,
        "Fairlight 4-Band Studio EQ"
    );

    // 4. Toggle plugin
    engine
        .execute(CommandRequest::new(
            "track.toggle_audio_plugin",
            json!({
                "track_id": 2,
                "plugin_id": 101,
                "enabled": false
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[1].plugins[0].enabled,
        false
    );

    // 5. Set Audio Bus Gain
    engine
        .execute(CommandRequest::new(
            "bus.set_gain_db",
            json!({
                "target": "master",
                "gain_db": -2.5
            }),
        ))
        .unwrap();

    let master_bus = engine
        .document()
        .project
        .sequence
        .buses
        .iter()
        .find(|b| b.target == saturn_core::BusTarget::Master)
        .unwrap();
    assert_eq!(master_bus.gain_db, -2.5);

    // Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let undone_bus = engine
        .document()
        .project
        .sequence
        .buses
        .iter()
        .find(|b| b.target == saturn_core::BusTarget::Master)
        .unwrap();
    assert_eq!(undone_bus.gain_db, 0.0);
}

#[test]
fn job_manager_pause_resume_batch_workflow() {
    let manager = saturn_core::JobManager::new(2);
    assert!(!manager.is_paused());

    manager.pause();
    assert!(manager.is_paused());

    let (job_id, _cancel, rx) = manager.submit(
        "Batch Job 1",
        saturn_core::JobKind::BatchExport {
            output_path: std::path::PathBuf::from("/out/test.mp4"),
            preset: "YouTube 4K".into(),
        },
        saturn_core::JobPriority::Normal,
        |ctx| {
            ctx.update_progress(0.5, "Encoding");
            Ok(Some("/out/test.mp4".into()))
        },
    );

    // While paused, job stays Queued
    let job_info = manager.get_job(job_id).unwrap();
    assert_eq!(job_info.state, saturn_core::JobState::Queued);

    // Resume queue -> job executes to Completed
    manager.resume();
    assert!(!manager.is_paused());

    let result = rx.recv().unwrap();
    assert_eq!(result.unwrap().as_deref(), Some("/out/test.mp4"));
}

#[test]
fn fusion_node_graph_workflow_with_undo() {
    let mut engine = EditorEngine::default();

    // 1. Add MediaSource node
    let res = engine
        .execute(CommandRequest::new(
            "nodegraph.add_node",
            json!({
                "kind": "media_source",
                "title": "MediaIn1",
                "pos_x": 100.0,
                "pos_y": 150.0,
                "params": {"media_id": 1}
            }),
        ))
        .unwrap();
    let src_id = res.get("node_id").unwrap().as_u64().unwrap();

    // 2. Add Keyer node
    let res = engine
        .execute(CommandRequest::new(
            "nodegraph.add_node",
            json!({
                "kind": "keyer",
                "title": "ChromaKey1",
                "pos_x": 300.0,
                "pos_y": 150.0,
                "params": {"color": "#00ff00"}
            }),
        ))
        .unwrap();
    let keyer_id = res.get("node_id").unwrap().as_u64().unwrap();

    // 3. Add OutputSink node
    let res = engine
        .execute(CommandRequest::new(
            "nodegraph.add_node",
            json!({
                "kind": "output_sink",
                "title": "MediaOut1",
                "pos_x": 500.0,
                "pos_y": 150.0,
            }),
        ))
        .unwrap();
    let out_id = res.get("node_id").unwrap().as_u64().unwrap();

    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    assert_eq!(graph.nodes.len(), 3);

    // 4. Connect MediaSource -> Keyer, and Keyer -> OutputSink
    engine
        .execute(CommandRequest::new(
            "nodegraph.connect",
            json!({
                "from_node": src_id,
                "from_socket": "output",
                "to_node": keyer_id,
                "to_socket": "input",
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "nodegraph.connect",
            json!({
                "from_node": keyer_id,
                "from_socket": "output",
                "to_node": out_id,
                "to_socket": "input",
            }),
        ))
        .unwrap();

    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    assert_eq!(graph.connections.len(), 2);

    // 5. Update Keyer position and params
    engine
        .execute(CommandRequest::new(
            "nodegraph.update_node",
            json!({
                "node_id": keyer_id,
                "pos_x": 320.0,
                "pos_y": 160.0,
                "params": {"color": "#00ff00", "tolerance": 45.0}
            }),
        ))
        .unwrap();

    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    let keyer = graph.nodes.iter().find(|n| n.id == keyer_id).unwrap();
    assert_eq!(keyer.pos_x, 320.0);
    assert_eq!(keyer.params["tolerance"], 45.0);

    // 6. Add RotoSpline Mask
    engine
        .execute(CommandRequest::new(
            "nodegraph.add_mask",
            json!({
                "mask": {
                    "id": 0,
                    "name": "Garbage Matte",
                    "closed": true,
                    "inverted": false,
                    "feather": 12.0,
                    "opacity": 1.0,
                    "points": [
                        {"x": 10.0, "y": 10.0, "handle_in": [0.0, 0.0], "handle_out": [0.0, 0.0]},
                        {"x": 100.0, "y": 10.0, "handle_in": [0.0, 0.0], "handle_out": [0.0, 0.0]},
                        {"x": 100.0, "y": 100.0, "handle_in": [0.0, 0.0], "handle_out": [0.0, 0.0]},
                        {"x": 10.0, "y": 100.0, "handle_in": [0.0, 0.0], "handle_out": [0.0, 0.0]},
                    ]
                }
            }),
        ))
        .unwrap();

    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    assert_eq!(graph.masks.len(), 1);
    assert_eq!(graph.masks[0].name, "Garbage Matte");
    assert_eq!(graph.masks[0].feather, 12.0);

    // 7. Remove Keyer node (should also clean up connections attached to it)
    engine
        .execute(CommandRequest::new(
            "nodegraph.remove_node",
            json!({"node_id": keyer_id}),
        ))
        .unwrap();

    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.connections.len(), 0);

    // 8. Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let graph = engine
        .document()
        .project
        .sequence
        .node_graph
        .as_ref()
        .unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.connections.len(), 2);
}

#[test]
fn optical_flow_speed_ramp_and_motion_blur_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    let media_id = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "skate.mp4",
                "path": "/media/skate.mp4",
                "kind": "video",
                "duration": 10 * TICKS_PER_SECOND
            }),
        ))
        .unwrap()
        .get("media_id")
        .unwrap()
        .as_u64()
        .unwrap();

    let track_id = engine.document().project.sequence.tracks[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 5 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();

    // 1. Set Speed Ramp
    engine
        .execute(CommandRequest::new(
            "clip.set_speed_ramp",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "speed_ramp": {
                    "points": [
                        {"time": 0, "speed_factor": 1.0, "bezier_tangent": [0.0, 0.0]},
                        {"time": 2 * TICKS_PER_SECOND, "speed_factor": 0.25, "bezier_tangent": [0.2, -0.5]},
                        {"time": 4 * TICKS_PER_SECOND, "speed_factor": 2.0, "bezier_tangent": [0.1, 0.8]}
                    ],
                    "optical_flow_interpolation": true,
                    "frame_blend": false
                }
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    let ramp = clip.speed_ramp.as_ref().unwrap();
    assert_eq!(ramp.points.len(), 3);
    assert_eq!(ramp.points[1].speed_factor, 0.25);
    assert!(ramp.optical_flow_interpolation);

    // 2. Set Motion Blur
    engine
        .execute(CommandRequest::new(
            "clip.set_motion_blur",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "motion_blur": {
                    "enabled": true,
                    "shutter_angle_deg": 270.0,
                    "samples": 12
                }
            }),
        ))
        .unwrap();

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    let mb = clip.motion_blur.as_ref().unwrap();
    assert_eq!(mb.shutter_angle_deg, 270.0);
    assert_eq!(mb.samples, 12);

    // 3. Test Undo
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert!(clip.motion_blur.is_none());
}

#[test]
fn speech_to_subtitles_and_ripple_deletion_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name":"interview.mp4", "path":"/assets/interview.mp4", "kind":"video"}),
        ))
        .unwrap();
    let media_id = media["media_id"].as_u64().unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;

    // Add 10-second clip (10 * TICKS_PER_SECOND)
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 10 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();

    // 1. Transcribe speech segments into subtitle track
    let segments = json!([
        {
            "id": 1,
            "start_ticks": 1 * TICKS_PER_SECOND,
            "end_ticks": 3 * TICKS_PER_SECOND,
            "text": "Hello world and welcome.",
            "confidence": 0.98,
            "words": [
                {"word": "Hello", "start_ticks": 1 * TICKS_PER_SECOND, "end_ticks": 1500 * (TICKS_PER_SECOND / 1000), "confidence": 0.99, "is_filler": false},
                {"word": "world", "start_ticks": 1500 * (TICKS_PER_SECOND / 1000), "end_ticks": 2 * TICKS_PER_SECOND, "confidence": 0.99, "is_filler": false}
            ]
        },
        {
            "id": 2,
            "start_ticks": 4 * TICKS_PER_SECOND,
            "end_ticks": 5 * TICKS_PER_SECOND,
            "text": "Um, so basically...",
            "confidence": 0.92,
            "words": [
                {"word": "Um", "start_ticks": 4 * TICKS_PER_SECOND, "end_ticks": 4500 * (TICKS_PER_SECOND / 1000), "confidence": 0.95, "is_filler": true},
                {"word": "basically", "start_ticks": 4500 * (TICKS_PER_SECOND / 1000), "end_ticks": 5 * TICKS_PER_SECOND, "confidence": 0.95, "is_filler": true}
            ]
        },
        {
            "id": 3,
            "start_ticks": 7 * TICKS_PER_SECOND,
            "end_ticks": 9 * TICKS_PER_SECOND,
            "text": "Final concluding words.",
            "confidence": 0.99,
            "words": []
        }
    ]);

    let res = engine
        .execute(CommandRequest::new(
            "speech.transcribe_to_subtitles",
            json!({ "segments": segments, "replace_existing": true }),
        ))
        .unwrap();
    assert_eq!(res["transcribed"], true);
    assert_eq!(res["subtitles_count"], 3);
    assert_eq!(engine.document().project.sequence.caption_tracks[0].cues.len(), 3);

    // 2. Ripple delete filler words (removes 4s..5s interval = 1s duration)
    let res = engine
        .execute(CommandRequest::new(
            "speech.ripple_delete_fillers",
            json!({ "segments": segments }),
        ))
        .unwrap();
    assert!(res["deleted_ranges_count"].as_u64().unwrap() >= 1);

    // Sequence duration should now be 9s (10s - 1s)
    let dur = engine.document().project.sequence.duration().0;
    assert_eq!(dur, 9 * TICKS_PER_SECOND);

    // 3. Undo ripple delete of filler words
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.duration().0,
        10 * TICKS_PER_SECOND
    );
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);
}

#[test]
fn timeline_interop_edl_fcpxml_premiere_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name":"scene1.mp4", "path":"/assets/scene1.mp4", "kind":"video"}),
        ))
        .unwrap();
    let media_id = media["media_id"].as_u64().unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;

    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 1 * TICKS_PER_SECOND,
                "duration": 4 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();

    // 1. Export CMX 3600 EDL and re-import through engine command
    let edl_str = saturn_core::export_cmx3600_edl(engine.document()).unwrap();
    assert!(edl_str.contains("TITLE: Sequence 1"));

    let res = engine
        .execute(CommandRequest::new(
            "interop.import_edl",
            json!({
                "edl": edl_str,
                "project_name": "EDL Project"
            }),
        ))
        .unwrap();
    assert_eq!(res["imported"], true);
    assert_eq!(engine.document().project.name, "EDL Project");
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);

    // 2. Undo CMX 3600 EDL import
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.name, "Untitled project");

    // 3. Export FCPXML and re-import
    let fcpxml_str = saturn_core::export_fcpxml(engine.document()).unwrap();
    assert!(fcpxml_str.contains("<fcpxml version=\"1.9\">"));

    let res = engine
        .execute(CommandRequest::new(
            "interop.import_fcpxml",
            json!({ "xml": fcpxml_str }),
        ))
        .unwrap();
    assert_eq!(res["imported"], true);

    // 4. Export Premiere XML and re-import
    let prem_xml = saturn_core::export_premiere_xml(engine.document()).unwrap();
    assert!(prem_xml.contains("<xmeml version=\"5\">"));

    let res = engine
        .execute(CommandRequest::new(
            "interop.import_premiere_xml",
            json!({ "xml": prem_xml }),
        ))
        .unwrap();
    assert_eq!(res["imported"], true);
}

#[test]
fn nested_sequences_and_compound_clips_workflow() {
    let mut engine = EditorEngine::default();
    let res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "a-roll.mp4",
                "path": "/media/a-roll.mp4",
                "kind": "video",
                "duration": 20 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();
    let media_id = res["media_id"].as_u64().unwrap();
    let v_track_id = engine.document().project.sequence.tracks[0].id;

    // Add 2 clips to V1: [0s to 4s], [4s to 10s]
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": v_track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 4 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": v_track_id,
                "media_id": media_id,
                "timeline_start": 4 * TICKS_PER_SECOND,
                "source_in": 0,
                "duration": 6 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();

    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 2);

    // 1. Create Compound Clip grouping both clips
    let compound_res = engine
        .execute(CommandRequest::new(
            "timeline.create_compound_clip",
            json!({
                "name": "Interview Sequence Compound",
                "target_track_id": v_track_id,
                "clips": [
                    {"track_id": v_track_id, "clip_index": 0},
                    {"track_id": v_track_id, "clip_index": 1}
                ]
            }),
        ))
        .unwrap();

    let compound_media_id = compound_res["compound_media_id"].as_u64().unwrap();
    let nested_seq_id = compound_res["sequence_id"].as_u64().unwrap();
    assert_eq!(
        compound_res["duration"].as_i64().unwrap(),
        10 * TICKS_PER_SECOND
    );

    // Active sequence track V1 now has 1 single compound clip spanning 10s
    let v_track = &engine.document().project.sequence.tracks[0];
    assert_eq!(v_track.clips.len(), 1);
    assert_eq!(v_track.clips[0].media_id, compound_media_id);
    assert_eq!(v_track.clips[0].duration.0, 10 * TICKS_PER_SECOND);
    assert_eq!(
        v_track.clips[0].compound_sequence_id,
        Some(compound_media_id)
    );

    // Project sequences list contains the nested sequence
    assert_eq!(engine.document().project.sequences.len(), 1);
    assert_eq!(engine.document().project.sequences[0].id, nested_seq_id);
    assert_eq!(
        engine.document().project.sequences[0].name,
        "Interview Sequence Compound"
    );

    // Media asset created
    let compound_media = engine
        .document()
        .project
        .media
        .iter()
        .find(|m| m.id == compound_media_id)
        .unwrap();
    assert_eq!(
        compound_media.kind,
        saturn_core::project::MediaKind::CompoundSequence
    );
    assert!(compound_media.nested_sequence.is_some());

    // 2. Switch Sequence into the nested sequence
    let switch_res = engine
        .execute(CommandRequest::new(
            "project.switch_sequence",
            json!({ "sequence_id": nested_seq_id }),
        ))
        .unwrap();
    assert_eq!(switch_res["switched"], true);
    assert_eq!(engine.document().project.sequence.id, nested_seq_id);
    assert_eq!(
        engine.document().project.sequence.name,
        "Interview Sequence Compound"
    );
    // Inside the nested sequence, there are 2 clips
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 2);

    // Switch back to root sequence
    let root_seq_id = engine.document().project.sequences[0].id;
    let switch_back_res = engine
        .execute(CommandRequest::new(
            "project.switch_sequence",
            json!({ "sequence_id": root_seq_id }),
        ))
        .unwrap();
    assert_eq!(switch_back_res["switched"], true);
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);

    // 3. Decompose Compound Clip in Place
    let decomp_res = engine
        .execute(CommandRequest::new(
            "timeline.decompose_compound_clip",
            json!({ "track_id": v_track_id, "clip_index": 0 }),
        ))
        .unwrap();
    assert_eq!(decomp_res["decomposed"], true);
    assert_eq!(decomp_res["unpacked_clips_count"], 2);

    // Timeline tracks now have the unpacked 2 clips restored
    let restored_v_track = &engine.document().project.sequence.tracks[0];
    assert_eq!(restored_v_track.clips.len(), 2);
    assert_eq!(restored_v_track.clips[0].timeline_start.0, 0);
    assert_eq!(restored_v_track.clips[0].duration.0, 4 * TICKS_PER_SECOND);
    assert_eq!(
        restored_v_track.clips[1].timeline_start.0,
        4 * TICKS_PER_SECOND
    );
    assert_eq!(restored_v_track.clips[1].duration.0, 6 * TICKS_PER_SECOND);

    // 4. Undo decomposition -> restores compound clip
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 1);
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0].compound_sequence_id,
        Some(compound_media_id)
    );

    // Redo decomposition -> expands again
    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks[0].clips.len(), 2);
}

#[test]
fn media_bins_properties_and_missing_detection_workflow() {
    let mut engine = EditorEngine::default();

    // 1. Import media
    let add_res = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "broll1.mp4",
                "path": "/media/broll1.mp4",
                "kind": "video",
                "duration": 5 * TICKS_PER_SECOND,
                "width": 3840,
                "height": 2160,
                "fps": 59.94,
                "codec": "h264",
                "audio_channels": 2,
            }),
        ))
        .unwrap();
    let media_id = add_res["media_id"].as_u64().unwrap();
    assert_eq!(engine.document().project.media[0].fps, Some(59.94));
    assert_eq!(
        engine.document().project.media[0].codec.as_deref(),
        Some("h264")
    );
    assert_eq!(engine.document().project.media[0].audio_channels, Some(2));

    // 2. Create Media Bin
    let bin_res = engine
        .execute(CommandRequest::new(
            "media.create_bin",
            json!({
                "name": "B-Roll Shots",
                "color": "#10b981",
            }),
        ))
        .unwrap();
    let bin_id = bin_res["bin"]["id"].as_u64().unwrap();
    assert_eq!(engine.document().project.bins.len(), 1);
    assert_eq!(engine.document().project.bins[0].name, "B-Roll Shots");

    // 3. Rename Bin
    engine
        .execute(CommandRequest::new(
            "media.rename_bin",
            json!({
                "bin_id": bin_id,
                "name": "Cinematic B-Roll",
            }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.bins[0].name, "Cinematic B-Roll");

    // 4. Move media to bin
    engine
        .execute(CommandRequest::new(
            "media.move_to_bin",
            json!({
                "media_id": media_id,
                "bin_id": bin_id,
            }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.media[0].bin_id, Some(bin_id));

    // 5. Update media properties (color label, tags, rename)
    engine
        .execute(CommandRequest::new(
            "media.update_properties",
            json!({
                "media_id": media_id,
                "name": "Drone_Beach_Sunset.mp4",
                "color_label": "emerald",
                "tags": ["drone", "sunset", "4k"],
            }),
        ))
        .unwrap();
    let media = &engine.document().project.media[0];
    assert_eq!(media.name, "Drone_Beach_Sunset.mp4");
    assert_eq!(media.color_label.as_deref(), Some("emerald"));
    assert_eq!(media.tags, vec!["drone", "sunset", "4k"]);

    // 6. Check missing media (mock path doesn't exist on disk)
    let check_res = engine
        .execute(CommandRequest::new(
            "media.check_missing",
            serde_json::Value::Null,
        ))
        .unwrap();
    assert_eq!(check_res["checked"], true);
    assert_eq!(check_res["missing_count"], 1);
    assert!(engine.document().project.media[0].is_missing);

    // 7. Delete Bin -> media resets bin_id to None
    engine
        .execute(CommandRequest::new(
            "media.delete_bin",
            json!({ "bin_id": bin_id }),
        ))
        .unwrap();
    assert!(engine.document().project.bins.is_empty());
    assert_eq!(engine.document().project.media[0].bin_id, None);

    // 8. Undo bin deletion -> restores bin and media bin_id
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(engine.document().project.bins.len(), 1);
    assert_eq!(engine.document().project.media[0].bin_id, Some(bin_id));
}

#[test]
fn timeline_add_remove_track_and_ripple_gap_workflow() {
    let mut engine = EditorEngine::default();

    // Initial project has 2 tracks (V1, A1)
    assert_eq!(engine.document().project.sequence.tracks.len(), 2);
    assert_eq!(engine.document().project.sequence.tracks[0].name, "V1");
    assert_eq!(engine.document().project.sequence.tracks[1].name, "A1");

    // 1. Add Video Track V2
    let add_v_res = engine
        .execute(CommandRequest::new(
            "timeline.add_track",
            json!({ "kind": "video" }),
        ))
        .unwrap();
    let v2_id = add_v_res["track_id"].as_u64().unwrap();
    assert_eq!(add_v_res["name"], "V2");
    assert_eq!(engine.document().project.sequence.tracks.len(), 3);

    // 2. Add Audio Track A2
    let add_a_res = engine
        .execute(CommandRequest::new(
            "timeline.add_track",
            json!({ "kind": "audio", "name": "SFX Lane" }),
        ))
        .unwrap();
    let _a2_id = add_a_res["track_id"].as_u64().unwrap();
    assert_eq!(add_a_res["name"], "SFX Lane");
    assert_eq!(engine.document().project.sequence.tracks.len(), 4);

    // 3. Locked track cannot be removed
    engine
        .execute(CommandRequest::new(
            "track.set_locked",
            json!({ "track_id": v2_id, "locked": true }),
        ))
        .unwrap();
    let locked_remove_err = engine.execute(CommandRequest::new(
        "timeline.remove_track",
        json!({ "track_id": v2_id }),
    ));
    assert!(locked_remove_err.is_err());

    // Unlock and remove V2
    engine
        .execute(CommandRequest::new(
            "track.set_locked",
            json!({ "track_id": v2_id, "locked": false }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.remove_track",
            json!({ "track_id": v2_id }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.tracks.len(), 3);

    // 4. Test Ripple Delete Gap
    // Setup media and clips on V1: Clip A [0..3s], Clip B [6..10s] -> empty gap of 3s at [3s..6s]
    let add_media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({ "name": "clip.mp4", "path": "/media/clip.mp4", "kind": "video" }),
        ))
        .unwrap();
    let media_id = add_media["media_id"].as_u64().unwrap();
    let v1_id = engine.document().project.sequence.tracks[0].id;

    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({ "track_id": v1_id, "media_id": media_id, "timeline_start": 0, "source_in": 0, "duration": 3 * TICKS_PER_SECOND }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({ "track_id": v1_id, "media_id": media_id, "timeline_start": 6 * TICKS_PER_SECOND, "source_in": 0, "duration": 4 * TICKS_PER_SECOND }),
        ))
        .unwrap();

    let clips = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips[0].timeline_start.0, 0);
    assert_eq!(clips[1].timeline_start.0, 6 * TICKS_PER_SECOND);

    // Ripple delete gap at 4.5s (which is inside [3s..6s])
    let gap_res = engine
        .execute(CommandRequest::new(
            "timeline.ripple_delete_gap",
            json!({ "track_id": v1_id, "at_tick": (4.5 * TICKS_PER_SECOND as f64) as i64 }),
        ))
        .unwrap();
    assert_eq!(gap_res["gap_deleted"], true);
    assert_eq!(gap_res["gap_start"].as_i64().unwrap(), 3 * TICKS_PER_SECOND);
    assert_eq!(gap_res["gap_end"].as_i64().unwrap(), 6 * TICKS_PER_SECOND);

    // Clip B should now start right at 3s!
    let clips_after = &engine.document().project.sequence.tracks[0].clips;
    assert_eq!(clips_after[1].timeline_start.0, 3 * TICKS_PER_SECOND);

    // 5. Undo gap ripple delete -> Clip B returns to 6s
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[1]
            .timeline_start
            .0,
        6 * TICKS_PER_SECOND
    );

    // Redo gap ripple delete -> Clip B returns to 3s
    engine
        .execute(CommandRequest::new("edit.redo", serde_json::Value::Null))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[1]
            .timeline_start
            .0,
        3 * TICKS_PER_SECOND
    );
}

#[test]
fn track_groups_curves_qualifier_and_custom_shader_workflow_with_undo() {
    let mut engine = EditorEngine::default();
    // Add media and clip
    let add_media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "cinematic.mp4",
                "path": "/media/cinematic.mp4",
                "kind": "video",
                "duration": 10 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();
    let media_id = add_media["media_id"].as_u64().unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 5 * TICKS_PER_SECOND
            }),
        ))
        .unwrap();

    // 1. Create Track Group
    let group_res = engine
        .execute(CommandRequest::new(
            "timeline.create_track_group",
            json!({
                "name": "Dialogue Group",
                "color": "#3b82f6"
            }),
        ))
        .unwrap();
    assert_eq!(group_res["created"], true);
    let group_id = group_res["group"]["id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.track_groups.len(), 1);

    // 2. Assign Track to Group
    engine
        .execute(CommandRequest::new(
            "timeline.assign_track_group",
            json!({
                "track_id": track_id,
                "group_id": group_id
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].group_id,
        Some(group_id)
    );

    // 3. Toggle Group Collapsed
    let toggle_res = engine
        .execute(CommandRequest::new(
            "timeline.toggle_track_group_collapsed",
            json!({ "group_id": group_id }),
        ))
        .unwrap();
    assert_eq!(toggle_res["collapsed"], true);
    assert_eq!(
        engine.document().project.sequence.track_groups[0].collapsed,
        true
    );

    // 4. Update Group Properties (Mute/Solo/Lock)
    engine
        .execute(CommandRequest::new(
            "timeline.set_track_group_properties",
            json!({
                "group_id": group_id,
                "name": "Vocals & Dialogue",
                "muted": true
            }),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.track_groups[0].name,
        "Vocals & Dialogue"
    );
    assert_eq!(
        engine.document().project.sequence.track_groups[0].muted,
        true
    );
    assert_eq!(engine.document().project.sequence.tracks[0].muted, true);

    // 5. Update RGB Curves
    let curves = json!({
        "master": [{"x": 0.0, "y": 0.0}, {"x": 0.5, "y": 0.6}, {"x": 1.0, "y": 1.0}],
        "red": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
        "green": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
        "blue": [{"x": 0.0, "y": 0.0}, {"x": 0.4, "y": 0.3}, {"x": 1.0, "y": 1.0}]
    });
    engine
        .execute(CommandRequest::new(
            "clip.update_rgb_curves",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "curves": curves
            }),
        ))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[0].clips[0]
            .color
            .curves
            .is_some()
    );

    // 6. Update HSL Qualifier
    let qualifier = json!({
        "hue_center": 0.1,
        "hue_width": 0.15,
        "hue_softness": 0.05,
        "sat_min": 0.2,
        "sat_max": 0.9,
        "sat_softness": 0.1,
        "luma_min": 0.1,
        "luma_max": 0.95,
        "luma_softness": 0.08,
        "invert": false,
        "blur_radius": 2.0,
        "hue_shift": 15.0,
        "sat_boost": 1.2,
        "exposure_shift": 0.3
    });
    engine
        .execute(CommandRequest::new(
            "clip.update_hsl_qualifier",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "qualifier": qualifier
            }),
        ))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[0].clips[0]
            .color
            .qualifier
            .is_some()
    );

    // 7. Add Custom GLSL Shader Effect
    let shader_res = engine
        .execute(CommandRequest::new(
            "clip.add_custom_shader",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "name": "Scanlines Plugin",
                "shader_code": "void main() { gl_FragColor = vec4(1.0); }",
                "preset_id": "crt",
                "uniforms": { "u_intensity": 0.75 }
            }),
        ))
        .unwrap();
    assert_eq!(shader_res["added"], true);
    let shader_effect_id = shader_res["effect_id"].as_u64().unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips[0]
            .effects
            .len(),
        1
    );

    // 8. Update Custom Shader Uniforms
    engine
        .execute(CommandRequest::new(
            "clip.update_custom_shader",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "effect_id": shader_effect_id,
                "uniforms": { "u_intensity": 0.9 }
            }),
        ))
        .unwrap();
    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    if let EffectPayload::CustomShader(p) = &clip.effects[0].payload {
        assert_eq!(p.uniforms.get("u_intensity"), Some(&0.9));
    } else {
        panic!("Effect should be CustomShader");
    }

    // 9. Undo & Redo Verification
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    if let EffectPayload::CustomShader(p) =
        &engine.document().project.sequence.tracks[0].clips[0].effects[0].payload
    {
        assert_eq!(p.uniforms.get("u_intensity"), Some(&0.75));
    }

    // 10. Delete Track Group
    engine
        .execute(CommandRequest::new(
            "timeline.delete_track_group",
            json!({ "group_id": group_id }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.track_groups.len(), 0);
    assert_eq!(engine.document().project.sequence.tracks[0].group_id, None);
}

#[test]
fn graphics_crud_transform_and_undo() {
    let mut engine = EditorEngine::default();

    // 1. Add a text graphic
    let res = engine
        .execute(CommandRequest::new(
            "graphic.add",
            json!({
                "kind": "text",
                "name": "Lower third",
                "start_ticks": 0,
                "duration_ticks": 5 * 254_016_000_000i64,
                "text": "Jane Doe"
            }),
        ))
        .unwrap();
    let text_id = res["id"].as_u64().unwrap();
    assert_eq!(engine.document().project.sequence.graphics.len(), 1);
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.kind, saturn_core::project::GraphicKind::Text);
    assert_eq!(graphic.text.as_ref().unwrap().content, "Jane Doe");
    assert!(graphic.fill.is_some(), "text graphics default to a fill");

    // 2. Transform updates validate and mutate
    engine
        .execute(CommandRequest::new(
            "graphic.set_transform",
            json!({
                "id": text_id,
                "x": 120.0,
                "y": 820.0,
                "opacity": 0.8
            }),
        ))
        .unwrap();
    let transform = &engine.document().project.sequence.graphics[0].transform;
    assert_eq!((transform.x, transform.y), (120.0, 820.0));
    assert_eq!(transform.opacity, 0.8);

    // 3. Out-of-range opacity is rejected without touching state and without
    //    pushing a history entry
    let error = engine
        .execute(CommandRequest::new(
            "graphic.set_transform",
            json!({ "id": text_id, "opacity": 1.5 }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("opacity"), "{error}");
    assert_eq!(
        engine.document().project.sequence.graphics[0].transform.opacity,
        0.8
    );

    // 4. Text edits land on the text payload only
    engine
        .execute(CommandRequest::new(
            "graphic.set_text",
            json!({ "id": text_id, "text": "Jane Doe, Editor", "font_size": 64.0 }),
        ))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.text.as_ref().unwrap().content, "Jane Doe, Editor");
    assert_eq!(graphic.text.as_ref().unwrap().font_size, 64.0);
    assert_eq!(graphic.transform.x, 120.0, "text edits must not move the box");

    // 5. Undo rolls the text edit back
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.text.as_ref().unwrap().content, "Jane Doe");
    assert_eq!(graphic.text.as_ref().unwrap().font_size, 42.0);

    // 6. Duplicate copies with a fresh id
    let res = engine
        .execute(CommandRequest::new(
            "graphic.duplicate",
            json!({ "id": text_id }),
        ))
        .unwrap();
    let copy_id = res["id"].as_u64().unwrap();
    assert_ne!(copy_id, text_id);
    assert_eq!(engine.document().project.sequence.graphics.len(), 2);

    // 7. Remove both
    engine
        .execute(CommandRequest::new("graphic.remove", json!({ "id": text_id })))
        .unwrap();
    engine
        .execute(CommandRequest::new("graphic.remove", json!({ "id": copy_id })))
        .unwrap();
    assert!(engine.document().project.sequence.graphics.is_empty());
}

#[test]
fn graphics_fill_stroke_shadow_and_shape_kinds() {
    let mut engine = EditorEngine::default();

    let res = engine
        .execute(CommandRequest::new(
            "graphic.add",
            json!({ "kind": "rectangle", "name": "Backdrop" }),
        ))
        .unwrap();
    let rect_id = res["id"].as_u64().unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.kind, saturn_core::project::GraphicKind::Rectangle);
    assert!(graphic.text.is_none(), "shapes carry no text payload");

    engine
        .execute(CommandRequest::new(
            "graphic.set_fill",
            json!({ "id": rect_id, "color": "#204060", "opacity": 0.9 }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "graphic.set_stroke",
            json!({ "id": rect_id, "color": "#ffffff", "width": 3.0 }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "graphic.set_shadow",
            json!({ "id": rect_id, "blur": 12.0, "offset_y": 4.0 }),
        ))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.fill.as_ref().unwrap().color, "#204060");
    assert_eq!(graphic.stroke.as_ref().unwrap().width, 3.0);
    assert_eq!(graphic.shadow.as_ref().unwrap().blur, 12.0);

    // Each property command checkpoints separately, so one undo reverts only
    // the shadow.
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert!(graphic.shadow.is_none());
    assert_eq!(graphic.stroke.as_ref().unwrap().width, 3.0);

    // Negative stroke width is rejected
    let error = engine
        .execute(CommandRequest::new(
            "graphic.set_stroke",
            json!({ "id": rect_id, "width": -1.0 }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("negative"), "{error}");
}

#[test]
fn graphics_align_distribute_and_arrange() {
    let mut engine = EditorEngine::default();

    let mut ids = Vec::new();
    for x in [10.0, 300.0, 800.0] {
        let res = engine
            .execute(CommandRequest::new(
                "graphic.add",
                json!({ "kind": "rectangle" }),
            ))
            .unwrap();
        let id = res["id"].as_u64().unwrap();
        engine
            .execute(CommandRequest::new(
                "graphic.set_transform",
                json!({ "id": id, "x": x, "y": 100.0, "width": 100.0, "height": 50.0 }),
            ))
            .unwrap();
        ids.push(id);
    }

    // Align left: every box starts at x = 10.
    engine
        .execute(CommandRequest::new(
            "graphic.align",
            json!({ "ids": ids, "mode": "left" }),
        ))
        .unwrap();
    for graphic in &engine.document().project.sequence.graphics {
        assert_eq!(graphic.transform.x, 10.0);
    }

    // Distribute horizontal from the stacked state: span 10..110, each box
    // width 100, so equal gaps of 0 keep extremities anchored — boxes fan out
    // only when there is room. Reset positions first.
    let spread = [10.0, 400.0, 900.0];
    for (id, x) in ids.iter().zip(spread) {
        engine
            .execute(CommandRequest::new(
                "graphic.set_transform",
                json!({ "id": id, "x": x }),
            ))
            .unwrap();
    }
    engine
        .execute(CommandRequest::new(
            "graphic.distribute",
            json!({ "ids": ids, "axis": "horizontal" }),
        ))
        .unwrap();
    // Total extent 300, span 10..1000, gap = (990 - 300) / 2 = 345: positions
    // land at 10, 455, 900.
    let xs: Vec<f64> = engine
        .document()
        .project
        .sequence
        .graphics
        .iter()
        .map(|graphic| graphic.transform.x)
        .collect();
    let sorted = {
        let mut sorted = xs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted
    };
    assert_eq!(sorted, vec![10.0, 455.0, 900.0]);
    assert_eq!(xs[0], 10.0, "first extremity stays anchored");

    // Aligning a single graphic is rejected
    let error = engine
        .execute(CommandRequest::new(
            "graphic.align",
            json!({ "ids": [ids[0]], "mode": "left" }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("two"), "{error}");

    // Arrange: move first to front, check its layer wins
    engine
        .execute(CommandRequest::new(
            "graphic.arrange",
            json!({ "id": ids[0], "order": "front" }),
        ))
        .unwrap();
    let front_layer = engine.document().project.sequence.graphics[0].layer;
    let others = engine.document().project.sequence.graphics[1].layer;
    assert!(front_layer > others, "front must raise the layer");

    engine
        .execute(CommandRequest::new(
            "graphic.arrange",
            json!({ "id": ids[0], "order": "backward" }),
        ))
        .unwrap();
    let lowered = engine.document().project.sequence.graphics[0].layer;
    assert!(lowered < front_layer, "backward must lower the layer");
}

#[test]
fn graphics_templates_save_apply_and_survive_reload() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("graphics.saturn");
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "Graphics"})))
        .unwrap();

    let res = engine
        .execute(CommandRequest::new(
            "graphic.add",
            json!({ "kind": "text", "name": "Title style", "text": "S1 E1" }),
        ))
        .unwrap();
    let graphic_id = res["id"].as_u64().unwrap();
    engine
        .execute(CommandRequest::new(
            "graphic.set_text",
            json!({ "id": graphic_id, "font_family": "Inter", "leading": 1.4 }),
        ))
        .unwrap();

    let res = engine
        .execute(CommandRequest::new(
            "graphic.template_save",
            json!({ "graphic_id": graphic_id, "name": "Episode title" }),
        ))
        .unwrap();
    let template_id = res["id"].as_u64().unwrap();

    // Apply creates a new graphic from the template body
    engine
        .execute(CommandRequest::new(
            "graphic.set_transform",
            json!({ "id": graphic_id, "x": 42.0 }),
        ))
        .unwrap();
    let res = engine
        .execute(CommandRequest::new(
            "graphic.template_apply",
            json!({ "template_id": template_id }),
        ))
        .unwrap();
    let applied_id = res["id"].as_u64().unwrap();
    assert_ne!(applied_id, graphic_id);
    assert_eq!(engine.document().project.sequence.graphics.len(), 2);

    // Save and reopen: templates and graphics persist
    engine
        .execute(CommandRequest::new("file.save", json!({"path": path})))
        .unwrap();
    let mut reopened = EditorEngine::default();
    reopened
        .execute(CommandRequest::new("file.open", json!({"path": path})))
        .unwrap();
    assert_eq!(reopened.document().project.graphic_templates.len(), 1);
    assert_eq!(
        reopened.document().project.graphic_templates[0].name,
        "Episode title"
    );
    // Template body froze the pre-move transform (x=0), not live state.
    let res = reopened
        .execute(CommandRequest::new(
            "graphic.template_apply",
            json!({ "template_id": template_id, "start_ticks": 5 * 254_016_000_000i64 }),
        ))
        .unwrap();
    let applied = res["id"].as_u64().unwrap();
    let graphic = reopened
        .document()
        .project
        .sequence
        .graphics
        .iter()
        .find(|graphic| graphic.id == applied)
        .unwrap();
    assert_eq!(graphic.transform.x, 0.0, "template freezes the saved look");
    assert_eq!(graphic.start_ticks.0, 5 * 254_016_000_000i64);
}

#[test]
fn fonts_list_returns_sorted_families_from_the_shared_context() {
    let mut engine = EditorEngine::default();
    let res = engine
        .execute(CommandRequest::new("fonts.list", serde_json::Value::Null))
        .unwrap();
    let fonts: Vec<String> = res["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap().to_owned())
        .collect();
    assert!(!fonts.is_empty(), "fonts.list must expose system families");
    for window in fonts.windows(2) {
        assert!(
            window[0].to_lowercase() <= window[1].to_lowercase(),
            "fonts.list must be case-insensitively sorted: {window:?}"
        );
    }
    // A second call must succeed (shared, lazily-built TextContext).
    let again = engine
        .execute(CommandRequest::new("fonts.list", serde_json::Value::Null))
        .unwrap();
    assert_eq!(again["count"], serde_json::json!(fonts.len()));
}

#[test]
fn graphics_flags_lock_blocks_property_mutations_but_not_unlock() {
    let mut engine = EditorEngine::default();
    let res = engine
        .execute(CommandRequest::new(
            "graphic.add",
            json!({ "kind": "text", "text": "Locked title" }),
        ))
        .unwrap();
    let id = res["id"].as_u64().unwrap();

    // Hide the graphic and lock it.
    engine
        .execute(CommandRequest::new(
            "graphic.set_flags",
            json!({ "id": id, "visible": false, "locked": true }),
        ))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert!(!graphic.visible);
    assert!(graphic.locked);

    // Property mutations are rejected while locked...
    for (command, params) in [
        ("graphic.set_transform", json!({ "id": id, "x": 50.0 })),
        ("graphic.set_text", json!({ "id": id, "text": "no" })),
        ("graphic.set_fill", json!({ "id": id, "color": "#000000" })),
        ("graphic.set_stroke", json!({ "id": id, "width": 1.0 })),
        ("graphic.set_shadow", json!({ "id": id, "blur": 4.0 })),
    ] {
        let error = engine
            .execute(CommandRequest::new(command, params))
            .unwrap_err();
        assert!(error.to_string().contains("locked"), "{command}: {error}");
    }

    // ...and the locked graphic's data is untouched.
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.transform.x, 0.0);

    // Unlocking itself must still work, then edits flow again.
    engine
        .execute(CommandRequest::new(
            "graphic.set_flags",
            json!({ "id": id, "locked": false }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "graphic.set_transform",
            json!({ "id": id, "x": 50.0 }),
        ))
        .unwrap();
    assert_eq!(engine.document().project.sequence.graphics[0].transform.x, 50.0);

    // Undo rolls the whole gesture chain: the last checkpoint was the unlock,
    // then the transform. Two undos land back at locked + x=0.
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    engine
        .execute(CommandRequest::new("edit.undo", serde_json::Value::Null))
        .unwrap();
    let graphic = &engine.document().project.sequence.graphics[0];
    assert_eq!(graphic.transform.x, 0.0);
    assert!(graphic.locked);
}
