// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

use saturn_core::{
    ActorContext, ActorId, CommandRequest, EditorEngine,
    lock::LockScope,
};
use serde_json::json;

fn setup_test_engine() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "AF2 Test Project"})))
        .expect("project.new failed");
    let media = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "clip_a.mp4", "path": "/assets/clip_a.mp4", "kind": "video"}),
        ))
        .expect("add_media failed");
    let media_id = media["media_id"].as_u64().expect("media_id");
    let track_id = engine.document().project.sequence.tracks[0].id;

    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 0,
                "source_in": 0,
                "duration": 1000,
            }),
        ))
        .expect("add_clip 1 failed");
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": 1000,
                "source_in": 0,
                "duration": 1000,
            }),
        ))
        .expect("add_clip 2 failed");
    engine
}

#[test]
fn test_agent_viewport_isolation_never_dirties_or_moves_human_view() {
    let mut engine = setup_test_engine();
    let dirty_before = engine.is_dirty();
    let journal_records_before = engine.journal.records.len();

    let human_ctx = ActorContext::human();
    let human_vp_before = engine.session_viewport_snapshot(&human_ctx);

    let agent_ctx = ActorContext::agent("agent-video-1", "session-agent-1");
    let track_id = engine.document().project.sequence.tracks[0].id;

    // Agent moves playhead and selects clip 1 in agent session viewport
    engine
        .execute_with_context(
            CommandRequest::new("playhead.set", json!({"tick": 750})),
            &agent_ctx,
        )
        .expect("agent playhead.set");

    engine
        .execute_with_context(
            CommandRequest::new("timeline.select", json!({"track_id": track_id, "clip_index": 0})),
            &agent_ctx,
        )
        .expect("agent timeline.select");

    // Human viewport is completely unchanged
    let human_vp_after = engine.session_viewport_snapshot(&human_ctx);
    assert_eq!(human_vp_before, human_vp_after);

    // Agent session viewport snapshot reflects agent's view
    let agent_vp = engine.session_viewport_snapshot(&agent_ctx);
    assert_eq!(agent_vp["playhead_tick"], 750);
    assert_eq!(agent_vp["selected_clips"].as_array().map(|a| a.len()), Some(1));
    assert_eq!(agent_vp["actor"], serde_json::to_value(agent_ctx.actor).unwrap());

    // Document dirty state is unchanged and no journal entry was generated
    assert_eq!(engine.is_dirty(), dirty_before);
    assert_eq!(engine.journal.records.len(), journal_records_before);
}

#[test]
fn test_durable_clip_and_region_locks_and_human_reclaim() {
    let mut engine = setup_test_engine();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let clip1_id = engine.document().project.sequence.tracks[0].clips[0].id;

    let agent1_ctx = ActorContext::agent("agent-1", "session-1");
    let agent2_ctx = ActorContext::agent("agent-2", "session-2");
    let human_ctx = ActorContext::human();

    // Agent 1 locks clip 1
    engine
        .execute_with_context(
            CommandRequest::new("project.lock_clip", json!({"clip_id": clip1_id, "intent": "Color grading"})),
            &agent1_ctx,
        )
        .expect("lock_clip should succeed");

    // Agent 2 attempts to remove clip 1 -> rejected by durable lock
    let err = engine
        .execute_with_context(
            CommandRequest::new("timeline.remove_clip", json!({"track_id": track_id, "clip_index": 0})),
            &agent2_ctx,
        )
        .expect_err("remove_clip should be denied by lock");
    assert!(err.to_string().contains("locked"), "error was: {err}");

    // Agent 2 attempts to acquire lock on already locked clip 1 -> rejected
    let err = engine
        .execute_with_context(
            CommandRequest::new("project.lock_clip", json!({"clip_id": clip1_id, "intent": "Trim"})),
            &agent2_ctx,
        )
        .expect_err("duplicate lock should fail");
    assert!(err.to_string().contains("already locked"), "error was: {err}");

    // Region lock: Agent 1 locks [1200, 1800) on track 0
    engine
        .execute_with_context(
            CommandRequest::new(
                "project.lock_region",
                json!({"track_id": track_id, "start_tick": 1200, "end_tick": 1800, "note": "VFX replacement"}),
            ),
            &agent1_ctx,
        )
        .expect("lock_region should succeed");

    // Human reclaims clip 1 lock with human authority
    let reclaim_res = engine
        .execute_with_context(
            CommandRequest::new("project.lock_reclaim", json!({"lock_id": 1})),
            &human_ctx,
        )
        .expect("human reclaim should succeed");
    assert_eq!(reclaim_res["reclaimed"], true);
    assert_eq!(reclaim_res["revision"], 2);

    // Agent 2 cannot reclaim lock
    let err = engine
        .execute_with_context(
            CommandRequest::new("project.lock_reclaim", json!({"lock_id": 2})),
            &agent2_ctx,
        )
        .expect_err("agent cannot reclaim locks");
    assert!(err.to_string().contains("authority"), "error was: {err}");
}

#[test]
fn test_lock_durability_across_serialization() {
    let mut engine = setup_test_engine();
    let clip1_id = engine.document().project.sequence.tracks[0].clips[0].id;
    let agent_ctx = ActorContext::agent("agent-1", "session-1");

    engine
        .execute_with_context(
            CommandRequest::new("project.lock_clip", json!({"clip_id": clip1_id, "intent": "Grading"})),
            &agent_ctx,
        )
        .expect("lock_clip");

    let doc = engine.document().clone();
    let serialized = serde_json::to_string(&doc).expect("serialize project");
    let deserialized: saturn_core::ProjectDocument =
        serde_json::from_str(&serialized).expect("deserialize project");

    assert_eq!(deserialized.locks.locks.len(), 1);
    let lock = &deserialized.locks.locks[0];
    assert_eq!(lock.scope, LockScope::Clip { clip_id: clip1_id });
    assert_eq!(lock.owner.session_id.as_deref(), Some("session-1"));
    assert_eq!(lock.revision, 1);
}

#[test]
fn test_conflict_safe_selective_undo() {
    let mut engine = setup_test_engine();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let agent_ctx = ActorContext::agent("agent-1", "session-1");

    // Agent 1 performs edit on clip 1 (disables it)
    engine
        .execute_with_context(
            CommandRequest::new(
                "clip.set_enabled",
                json!({"track_id": track_id, "clip_index": 0, "enabled": false}),
            ),
            &agent_ctx,
        )
        .expect("disable clip 0");

    let change_id_1 = engine.journal.records.last().unwrap().change_id;
    assert!(change_id_1 > 0);

    // Agent 1 performs disjoint edit on clip 2 (disables it)
    engine
        .execute_with_context(
            CommandRequest::new(
                "clip.set_enabled",
                json!({"track_id": track_id, "clip_index": 1, "enabled": false}),
            ),
            &agent_ctx,
        )
        .expect("disable clip 1");

    let change_id_2 = engine.journal.records.last().unwrap().change_id;
    assert!(change_id_2 > change_id_1);

    // Undo edit on clip 1 (change_id_1) succeeds because change_id_2 is on disjoint entity (clip 2)
    let undo_res = engine.undo_change(change_id_1, &agent_ctx).expect("disjoint selective undo");
    assert_eq!(undo_res["undone_change_id"], change_id_1);

    // Now make dependent edit: edit clip 2 again (enable it)
    engine
        .execute_with_context(
            CommandRequest::new(
                "clip.set_enabled",
                json!({"track_id": track_id, "clip_index": 1, "enabled": true}),
            ),
            &agent_ctx,
        )
        .expect("enable clip 1 again");

    // Attempting to undo change_id_2 now fails because subsequent change touched clip 2
    let err = engine.undo_change(change_id_2, &agent_ctx).expect_err("dependent undo should conflict");
    assert!(err.to_string().contains("Conflict"), "error was: {err}");
}

#[test]
fn test_stale_branch_merge_rejection() {
    let mut engine = setup_test_engine();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let agent_ctx = ActorContext::agent("agent-branch", "session-branch");

    // Create branch "vfx-cut" from main
    engine
        .execute_with_context(
            CommandRequest::new("project.branch_create", json!({"name": "vfx-cut"})),
            &agent_ctx,
        )
        .expect("project.branch_create");

    // Edit on main branch advances head revision
    let human_ctx = ActorContext::human();
    engine
        .execute_with_context(
            CommandRequest::new(
                "clip.set_enabled",
                json!({"track_id": track_id, "clip_index": 0, "enabled": false}),
            ),
            &human_ctx,
        )
        .expect("edit on main");

    // Attempt to merge "vfx-cut" into main -> refused because target baseline is stale
    let err = engine
        .execute_with_context(
            CommandRequest::new("project.branch_merge", json!({"branch": "vfx-cut", "action": "accept"})),
            &human_ctx,
        )
        .expect_err("merge on stale baseline must fail");

    assert!(err.to_string().to_lowercase().contains("stale"), "error was: {err}");
}

#[test]
fn test_atomic_batch_transaction_rollback() {
    let mut engine = setup_test_engine();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let agent_ctx = ActorContext::agent("agent-batch", "session-batch");

    let doc_before = engine.document().clone();
    let dirty_before = engine.is_dirty();

    // 2-step transaction where step 2 is invalid
    let steps = vec![
        CommandRequest::new(
            "clip.set_enabled",
            json!({"track_id": track_id, "clip_index": 0, "enabled": false}),
        ),
        CommandRequest::new(
            "timeline.remove_clip",
            json!({"track_id": 9999, "clip_index": 0}), // invalid track
        ),
    ];

    let err = engine.execute_transaction(steps, &agent_ctx).expect_err("transaction must fail");
    assert!(!err.to_string().is_empty());

    // Document must be completely restored to pre-transaction state
    assert_eq!(
        serde_json::to_string(engine.document()).unwrap(),
        serde_json::to_string(&doc_before).unwrap()
    );
    assert_eq!(engine.is_dirty(), dirty_before);
}

#[test]
fn test_stop_generation_fencing_and_human_resume() {
    let mut engine = setup_test_engine();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let session_id = "agent-session-fence";
    let agent_ctx = ActorContext::agent("agent-worker", session_id);

    // Initially agent can execute
    engine
        .execute_with_context(
            CommandRequest::new("clip.set_enabled", json!({"track_id": track_id, "clip_index": 0, "enabled": false})),
            &agent_ctx,
        )
        .expect("initial edit succeeds");

    // Human stops the session
    let new_gen = engine.stop_session(session_id);
    assert_eq!(new_gen, 2);
    assert!(engine.is_session_stopped(session_id));

    // Agent attempts another command -> rejected immediately by session stop
    let err = engine
        .execute_with_context(
            CommandRequest::new("clip.set_enabled", json!({"track_id": track_id, "clip_index": 0, "enabled": true})),
            &agent_ctx,
        )
        .expect_err("stopped session must be rejected");
    assert!(err.to_string().contains("stopped"), "error was: {err}");

    // Resume session by human authority
    let resumed_gen = engine.resume_session(session_id, &ActorId::Human).expect("resume session");
    assert_eq!(resumed_gen, 2);
    assert!(!engine.is_session_stopped(session_id));

    // Stale generation request (generation 1) is fenced
    let stale_ctx = ActorContext::agent("agent-worker", session_id).with_generation(1);
    let err = engine
        .execute_with_context(
            CommandRequest::new("clip.set_enabled", json!({"track_id": track_id, "clip_index": 0, "enabled": true})),
            &stale_ctx,
        )
        .expect_err("stale generation must be rejected");
    assert!(err.to_string().to_lowercase().contains("stale"), "error was: {err}");

    // Correct generation 2 succeeds
    let valid_ctx = ActorContext::agent("agent-worker", session_id).with_generation(2);
    engine
        .execute_with_context(
            CommandRequest::new("clip.set_enabled", json!({"track_id": track_id, "clip_index": 0, "enabled": true})),
            &valid_ctx,
        )
        .expect("freshened generation must succeed");
}
