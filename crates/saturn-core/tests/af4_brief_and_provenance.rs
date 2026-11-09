// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

use saturn_core::{
    brief::{BriefSource, ConfidenceLevel},
    ActorContext, ActorId, CommandRequest, EditorEngine, ProjectDocument,
};
use serde_json::json;

fn setup_test_engine() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "AF4 Test Project"}),
        ))
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
fn test_brief_set_and_get_by_human_authority() {
    let mut engine = setup_test_engine();
    let human_ctx = ActorContext::human();

    let set_res = engine
        .execute_with_context(
            CommandRequest::new(
                "project.brief_set",
                json!({
                    "goal": "Highlight reel for social media",
                    "platform": "tiktok",
                    "target_duration_ticks": 3000,
                    "duration_tolerance_ticks": 120,
                    "must_keep_clip_ids": [1, 2],
                    "banned_content": ["watermark", "silent_gap"],
                    "style_notes": "Fast pacing, punchy cuts",
                    "loudness_target_lufs": -14.0,
                    "source": "user_prompt",
                }),
            ),
            &human_ctx,
        )
        .expect("project.brief_set should succeed for human");

    assert_eq!(set_res["updated"], true);
    assert_eq!(
        set_res["brief"]["goal"],
        "Highlight reel for social media"
    );

    let get_res = engine
        .execute_with_context(
            CommandRequest::new("project.brief_get", json!({})),
            &human_ctx,
        )
        .expect("project.brief_get should succeed");

    assert_eq!(get_res["goal"], "Highlight reel for social media");
    assert_eq!(get_res["platform"], "tiktok");
    assert_eq!(get_res["target_duration_ticks"], 3000);
    assert_eq!(get_res["duration_tolerance_ticks"], 120);
    assert_eq!(get_res["must_keep_clip_ids"], json!([1, 2]));
    assert_eq!(
        get_res["banned_content"],
        json!(["watermark", "silent_gap"])
    );
    assert_eq!(get_res["style_notes"], "Fast pacing, punchy cuts");
    assert_eq!(get_res["loudness_target_lufs"], -14.0);
    assert_eq!(get_res["source"], "user_prompt");
    assert_eq!(get_res["updated_by"]["kind"], "human");
}

#[test]
fn test_brief_set_by_agent_is_denied() {
    let mut engine = setup_test_engine();
    let agent_ctx = ActorContext::agent("agent-rough-cut", "session-agent-1");

    let err = engine
        .execute_with_context(
            CommandRequest::new(
                "project.brief_set",
                json!({
                    "goal": "Agent attempting to rewrite brief",
                }),
            ),
            &agent_ctx,
        )
        .expect_err("project.brief_set must fail for agent");

    let err_str = err.to_string();
    assert!(
        err_str.contains("PermissionDenied")
            && err_str.contains("human authorization"),
        "Unexpected error: {err_str}"
    );

    // Verify brief was not modified
    assert_eq!(engine.document().project.brief.goal, "");
}

#[test]
fn test_brief_persists_across_save_and_reopen() {
    let mut engine = setup_test_engine();
    let human_ctx = ActorContext::human();

    engine
        .execute_with_context(
            CommandRequest::new(
                "project.brief_set",
                json!({
                    "goal": "Preserved across save",
                    "platform": "youtube_shorts",
                    "target_duration_ticks": 1800,
                    "style_notes": "Cinematic colors",
                    "loudness_target_lufs": -16.0,
                    "source": "preset",
                }),
            ),
            &human_ctx,
        )
        .expect("brief_set");

    let snapshot = serde_json::to_string(&engine.document()).expect("serialize");
    let restored: ProjectDocument = serde_json::from_str(&snapshot).expect("deserialize");

    assert_eq!(restored.project.brief.goal, "Preserved across save");
    assert_eq!(
        restored.project.brief.platform.as_deref(),
        Some("youtube_shorts")
    );
    assert_eq!(restored.project.brief.target_duration_ticks, Some(1800));
    assert_eq!(restored.project.brief.style_notes, "Cinematic colors");
    assert_eq!(restored.project.brief.loudness_target_lufs, Some(-16.0));
    assert_eq!(restored.project.brief.source, BriefSource::Preset);
    assert_eq!(restored.project.brief.updated_by, ActorId::Human);
}

#[test]
fn test_agent_edit_records_rationale_confidence_and_clip_agent_note() {
    let mut engine = setup_test_engine();
    let agent_ctx = ActorContext::agent("agent-video-1", "session-agent-1");
    let track_id = engine.document().project.sequence.tracks[0].id;
    let clip_id = engine.document().project.sequence.tracks[0].clips[0].id;

    let req = CommandRequest::new(
        "clip.set_volume_db",
        json!({
            "track_id": track_id,
            "clip_index": 0,
            "volume_db": -6.0,
        }),
    )
    .with_rationale("Reduced volume to avoid clipping under dialogue")
    .with_confidence(ConfidenceLevel::High);

    engine
        .execute_with_context(req, &agent_ctx)
        .expect("agent clip.set_volume");

    // 1. Verify change journal has metadata
    let last_record = engine
        .journal
        .records
        .last()
        .expect("journal record exists");
    assert_eq!(
        last_record.actor,
        ActorId::agent("agent-video-1")
    );
    assert_eq!(
        last_record.rationale.as_deref(),
        Some("Reduced volume to avoid clipping under dialogue")
    );
    assert_eq!(last_record.confidence, Some(ConfidenceLevel::High));
    assert_eq!(last_record.correction_of, None);

    // 2. Verify clip has active agent note with provenance
    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert_eq!(clip.id, clip_id);
    let note = clip.agent_note.as_ref().expect("clip agent note");
    assert_eq!(
        note.rationale,
        "Reduced volume to avoid clipping under dialogue"
    );
    assert_eq!(note.confidence, ConfidenceLevel::High);
    assert_eq!(note.actor, ActorId::agent("agent-video-1"));
    assert_eq!(note.session_id, "session-agent-1");
    assert_eq!(note.active, true);
    assert_eq!(note.change_id, last_record.change_id);
    assert!(note.correction.is_none());
}

#[test]
fn test_rationale_character_limit_exceeded_fails() {
    let mut engine = setup_test_engine();
    let agent_ctx = ActorContext::agent("agent-video-1", "session-agent-1");
    let track_id = engine.document().project.sequence.tracks[0].id;

    let long_rationale = "x".repeat(201);
    let req = CommandRequest::new(
        "clip.set_volume_db",
        json!({
            "track_id": track_id,
            "clip_index": 0,
            "volume_db": -3.0,
        }),
    )
    .with_rationale(long_rationale);

    let err = engine
        .execute_with_context(req, &agent_ctx)
        .expect_err("long rationale must fail");

    assert!(
        err.to_string().contains("Rationale exceeds 200 characters"),
        "Unexpected error message: {err}"
    );
}

#[test]
fn test_human_modification_clears_active_and_records_human_correction() {
    let mut engine = setup_test_engine();
    let agent_ctx = ActorContext::agent("agent-video-1", "session-agent-1");
    let human_ctx = ActorContext::human();
    let track_id = engine.document().project.sequence.tracks[0].id;
    let clip_id = engine.document().project.sequence.tracks[0].clips[0].id;

    // Step 1: Agent modifies clip
    let agent_req = CommandRequest::new(
        "clip.set_volume_db",
        json!({
            "track_id": track_id,
            "clip_index": 0,
            "volume_db": -8.0,
        }),
    )
    .with_rationale("Lowered background track gain")
    .with_confidence(ConfidenceLevel::Medium);

    engine
        .execute_with_context(agent_req, &agent_ctx)
        .expect("agent edit");

    let agent_change_id = engine.journal.records.last().unwrap().change_id;

    // Step 2: Human modifies the same clip
    let human_req = CommandRequest::new(
        "clip.set_volume_db",
        json!({
            "track_id": track_id,
            "clip_index": 0,
            "volume_db": -4.0,
        }),
    );

    engine
        .execute_with_context(human_req, &human_ctx)
        .expect("human edit");

    let human_record = engine.journal.records.last().unwrap();
    assert_eq!(human_record.actor, ActorId::Human);
    assert_eq!(human_record.correction_of, Some(agent_change_id));

    // Step 3: Inspect clip agent note
    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    assert_eq!(clip.id, clip_id);
    let note = clip.agent_note.as_ref().expect("clip agent note persists");

    // Active marker is cleared
    assert_eq!(note.active, false);
    // Original agent provenance is preserved
    assert_eq!(note.actor, ActorId::agent("agent-video-1"));
    assert_eq!(note.rationale, "Lowered background track gain");
    assert_eq!(note.confidence, ConfidenceLevel::Medium);
    assert_eq!(note.change_id, agent_change_id);

    // Human correction is recorded
    let correction = note.correction.as_ref().expect("human correction recorded");
    assert_eq!(correction.corrected_by, ActorId::Human);
    assert!(correction.corrected_at_unix_ms > 0);
    assert_eq!(correction.note, "Human modification");
}

#[test]
fn test_branch_merge_preserves_brief_and_agent_notes() {
    let mut engine = setup_test_engine();
    let human_ctx = ActorContext::human();
    let agent_ctx = ActorContext::agent("agent-editor", "session-agent-2");
    let track_id = engine.document().project.sequence.tracks[0].id;

    // Set brief
    engine
        .execute_with_context(
            CommandRequest::new(
                "project.brief_set",
                json!({
                    "goal": "Branch preservation test",
                    "platform": "instagram_reels",
                }),
            ),
            &human_ctx,
        )
        .expect("brief_set");

    // Agent modifies clip
    engine
        .execute_with_context(
            CommandRequest::new(
                "clip.set_volume_db",
                json!({
                    "track_id": track_id,
                    "clip_index": 0,
                    "volume_db": -2.0,
                }),
            )
            .with_rationale("Normalized loudness")
            .with_confidence(ConfidenceLevel::High),
            &agent_ctx,
        )
        .expect("clip edit");

    // Create branch
    engine
        .execute(CommandRequest::new(
            "project.branch_create",
            json!({"name": "feat-experiment", "note": "Testing branch merge"}),
        ))
        .expect("branch_create");

    // Merge branch into main
    let merge_res = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "feat-experiment", "action": "accept", "force": true}),
        ))
        .expect("branch_merge");

    assert_eq!(merge_res["merged"], "accept");

    // Verify brief and clip agent note remain intact
    assert_eq!(
        engine.document().project.brief.goal,
        "Branch preservation test"
    );
    assert_eq!(
        engine.document().project.brief.platform.as_deref(),
        Some("instagram_reels")
    );

    let clip = &engine.document().project.sequence.tracks[0].clips[0];
    let note = clip.agent_note.as_ref().expect("agent note preserved");
    assert_eq!(note.rationale, "Normalized loudness");
    assert_eq!(note.confidence, ConfidenceLevel::High);
    assert_eq!(note.active, true);
}

#[test]
fn test_atomic_transaction_with_provenance() {
    let mut engine = setup_test_engine();
    let agent_ctx = ActorContext::agent("agent-batch", "session-batch-1");
    let track_id = engine.document().project.sequence.tracks[0].id;

    let steps = vec![
        CommandRequest::new(
            "clip.set_volume_db",
            json!({
                "track_id": track_id,
                "clip_index": 0,
                "volume_db": -5.0,
            }),
        )
        .with_rationale("Step 1 volume adjustment")
        .with_confidence(ConfidenceLevel::High),
        CommandRequest::new(
            "clip.set_volume_db",
            json!({
                "track_id": track_id,
                "clip_index": 1,
                "volume_db": -7.0,
            }),
        )
        .with_rationale("Step 2 volume adjustment")
        .with_confidence(ConfidenceLevel::Medium),
    ];

    let tx_res = engine
        .execute_transaction(steps, &agent_ctx)
        .expect("execute_transaction");

    assert_eq!(tx_res["executed"], 2);

    let records = &engine.journal.records;
    let rec1 = records.iter().find(|r| r.rationale.as_deref() == Some("Step 1 volume adjustment")).expect("rec 1");
    let rec2 = records.iter().find(|r| r.rationale.as_deref() == Some("Step 2 volume adjustment")).expect("rec 2");

    assert_eq!(rec1.confidence, Some(ConfidenceLevel::High));
    assert_eq!(rec2.confidence, Some(ConfidenceLevel::Medium));
}
