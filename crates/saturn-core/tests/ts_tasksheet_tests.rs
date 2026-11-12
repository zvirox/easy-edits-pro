// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration and conformance tests for Task Sheet v1 (TS0/TS1) and CET v0.1.

use saturn_core::{
    actor::ActorContext,
    brief::ProjectBrief,
    tasksheet::{
        apply_transition, can_transition, parse_cet_jsonl, CetEventType, CetGenerator,
        DurationConstraint, SheetDeliverable, SheetStatus, SheetStepStatus, TaskSheet,
        TaskSheetError, TASKSHEET_SCHEMA_VERSION,
    },
    CommandRequest, EditorEngine,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn test_ts0_fixture_templates_validate() {
    let templates_dir = repo_root().join("docs/templates/tasksheets");
    let files = [
        "podcast-social-clips.json",
        "talking-head-tutorial.json",
        "webinar-recap.json",
    ];

    for file in &files {
        let path = templates_dir.join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed reading {}: {e}", path.display()));
        let sheet: TaskSheet = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed deserializing {}: {e}", path.display()));

        assert_eq!(sheet.schema, TASKSHEET_SCHEMA_VERSION);
        sheet
            .validate()
            .unwrap_or_else(|e| panic!("Validation failed for {}: {e}", path.display()));
        assert!(!sheet.deliverables.is_empty());
    }
}

#[test]
fn test_ts0_invalid_structures_rejected() {
    let mut sheet = TaskSheet::new("valid_id", "Valid Title", "human:user");
    sheet.deliverables = vec![SheetDeliverable {
        id: "d1".into(),
        name: "Clip".into(),
        preset: "web-1080p".into(),
        duration: Some(DurationConstraint {
            target_s: 60.0,
            tolerance_s: Some(5.0),
        }),
        loudness_lufs: Some(-14.0),
        true_peak_dbtp: Some(-1.0),
        captions: Some(true),
        must_include: Vec::new(),
    }];

    assert!(sheet.validate().is_ok());

    // 1. Invalid schema version
    sheet.schema = "tasksheet/2".into();
    assert!(sheet.validate().is_err());
    sheet.schema = TASKSHEET_SCHEMA_VERSION.into();

    // 2. Empty title
    sheet.title = "   ".into();
    assert!(sheet.validate().is_err());
    sheet.title = "Valid Title".into();

    // 3. Invalid characters in id
    sheet.id = "bad id with spaces".into();
    assert!(sheet.validate().is_err());
    sheet.id = "valid_id".into();

    // 4. Invalid requester pattern
    sheet.requester = "anonymous:nobody".into();
    assert!(sheet.validate().is_err());
    sheet.requester = "agent:auto".into();
    assert!(sheet.validate().is_ok());

    // 5. Zero deliverables
    sheet.deliverables.clear();
    assert!(sheet.validate().is_err());
    sheet.deliverables.push(SheetDeliverable {
        id: "d1".into(),
        name: "Clip".into(),
        preset: "web-1080p".into(),
        duration: Some(DurationConstraint {
            target_s: -10.0,
            tolerance_s: None,
        }),
        loudness_lufs: None,
        true_peak_dbtp: None,
        captions: None,
        must_include: Vec::new(),
    });
    // Negative duration
    assert!(sheet.validate().is_err());
}

#[test]
fn test_ts0_state_machine_transitions_exhaustive() {
    let mut sheet = TaskSheet::new("ts_workflow", "Workflow Sheet", "human:producer");
    sheet.deliverables = vec![SheetDeliverable {
        id: "d1".into(),
        name: "Main".into(),
        preset: "web-1080p".into(),
        duration: Some(DurationConstraint {
            target_s: 30.0,
            tolerance_s: None,
        }),
        loudness_lufs: None,
        true_peak_dbtp: None,
        captions: None,
        must_include: Vec::new(),
    }];

    assert_eq!(sheet.status, SheetStatus::Draft);

    // Draft -> Delivered must be rejected
    assert!(!can_transition(SheetStatus::Draft, SheetStatus::Delivered));
    assert!(
        apply_transition(&mut sheet, SheetStatus::Delivered, "human:user", 1000, None).is_err()
    );

    // Draft -> Ready
    apply_transition(
        &mut sheet,
        SheetStatus::Ready,
        "human:user",
        1000,
        Some("Ready for agent".into()),
    )
    .expect("Draft -> Ready");
    assert_eq!(sheet.status, SheetStatus::Ready);
    assert_eq!(sheet.transitions.len(), 1);

    // Ready -> Planning
    apply_transition(&mut sheet, SheetStatus::Planning, "agent:bot", 2000, None)
        .expect("Ready -> Planning");
    assert_eq!(sheet.status, SheetStatus::Planning);

    // Planning -> AwaitingPlanApproval
    apply_transition(
        &mut sheet,
        SheetStatus::AwaitingPlanApproval,
        "agent:bot",
        3000,
        Some("Plan posted".into()),
    )
    .expect("Planning -> AwaitingPlanApproval");
    assert_eq!(sheet.status, SheetStatus::AwaitingPlanApproval);

    // Agent cannot self-approve plan
    let agent_approval = apply_transition(
        &mut sheet,
        SheetStatus::Running,
        "agent:bot",
        4000,
        Some("Self approval".into()),
    );
    assert!(matches!(
        agent_approval,
        Err(TaskSheetError::Unauthorized(_))
    ));

    // Human approves plan: AwaitingPlanApproval -> Running
    apply_transition(
        &mut sheet,
        SheetStatus::Running,
        "human:approver",
        4000,
        Some("Plan approved".into()),
    )
    .expect("Human plan approval");
    assert_eq!(sheet.status, SheetStatus::Running);

    // Running -> NeedsInput (agent question)
    apply_transition(
        &mut sheet,
        SheetStatus::NeedsInput,
        "agent:bot",
        5000,
        Some("Need music choice".into()),
    )
    .expect("Running -> NeedsInput");
    assert_eq!(sheet.status, SheetStatus::NeedsInput);

    // NeedsInput -> Running (human answered)
    apply_transition(
        &mut sheet,
        SheetStatus::Running,
        "human:user",
        6000,
        Some("Use ambient track".into()),
    )
    .expect("NeedsInput -> Running");

    // Running -> InReview
    apply_transition(
        &mut sheet,
        SheetStatus::InReview,
        "agent:bot",
        7000,
        Some("Round 1 ready for review".into()),
    )
    .expect("Running -> InReview");

    // InReview -> RevisionRequested
    apply_transition(
        &mut sheet,
        SheetStatus::RevisionRequested,
        "human:reviewer",
        8000,
        Some("Shorten hook clip".into()),
    )
    .expect("InReview -> RevisionRequested");

    // RevisionRequested -> Running
    apply_transition(
        &mut sheet,
        SheetStatus::Running,
        "agent:bot",
        9000,
        Some("Addressing notes for round 2".into()),
    )
    .expect("RevisionRequested -> Running");

    // Running -> InReview -> Approved -> Delivered
    apply_transition(&mut sheet, SheetStatus::InReview, "agent:bot", 10000, None).unwrap();
    apply_transition(
        &mut sheet,
        SheetStatus::Approved,
        "human:reviewer",
        11000,
        Some("Looks great".into()),
    )
    .unwrap();
    apply_transition(
        &mut sheet,
        SheetStatus::Delivered,
        "agent:packager",
        12000,
        Some("Delivery package generated".into()),
    )
    .unwrap();
    assert_eq!(sheet.status, SheetStatus::Delivered);
    assert!(sheet.status.is_terminal());

    // Delivered cannot transition anywhere
    assert!(!can_transition(
        SheetStatus::Delivered,
        SheetStatus::Running
    ));
    assert!(apply_transition(&mut sheet, SheetStatus::Running, "human:user", 13000, None).is_err());
}

#[test]
fn test_ts0_cet_parser_and_sample_track() {
    let sample_path = repo_root().join("docs/templates/cet/sample.cet.jsonl");
    let content = fs::read_to_string(&sample_path)
        .unwrap_or_else(|e| panic!("Failed reading {}: {e}", sample_path.display()));

    let (header, events) = parse_cet_jsonl(&content).expect("Valid CET parse");
    assert_eq!(header.cet, "0.1");
    assert_eq!(header.r#type, "header");
    assert_eq!(header.video.file, "tutorial_sample.mkv");
    assert_eq!(header.video.fps, 60.0);
    assert_eq!(header.privacy.keys, "classes_only");
    assert_eq!(header.privacy.text, "off");

    assert_eq!(events.len(), 9);
    assert_eq!(events[0].r#type, CetEventType::Cursor);
    assert_eq!(events[2].r#type, CetEventType::Window);
    assert_eq!(events[2].app.as_deref(), Some("alacritty"));
    assert_eq!(events[5].r#type, CetEventType::ZoomHint);
    assert_eq!(events[7].r#type, CetEventType::Key);
    assert_eq!(events[7].class.as_deref(), Some("letter"));
    assert_eq!(events[8].r#type, CetEventType::Mark);
    assert_eq!(events[8].kind.as_deref(), Some("chapter"));
}

#[test]
fn test_ts0_cet_synthetic_generator_and_privacy() {
    let mut generator = CetGenerator::new("demo.mkv", 60.0, 1920, 1080);

    // Class-only by default: key name is suppressed
    generator.emit_cursor(100.0, 200.0, "arrow");
    generator.advance_ms(16);
    generator.emit_click(500.0, 400.0, "left");
    generator.advance_ms(100);
    generator.emit_zoom_hint(400.0, 300.0, 300.0, 200.0, "click_focus");
    generator.advance_ms(50);
    generator.emit_key("letter", &["ctrl"], Some("s"));

    let jsonl = generator.to_jsonl();
    let (header, events) = parse_cet_jsonl(&jsonl).expect("parse synthetic CET");
    assert_eq!(header.privacy.keys, "classes_only");

    // Key event should have suppressed key name by default
    let key_ev = events
        .iter()
        .find(|e| e.r#type == CetEventType::Key)
        .expect("key event");
    assert_eq!(key_ev.class.as_deref(), Some("letter"));
    assert!(key_ev.name.is_none());

    // With named_shortcuts opted in: key name is retained
    let mut generator_opt_in = CetGenerator::new("tutorial.mkv", 60.0, 1920, 1080);
    generator_opt_in.set_named_shortcuts_opt_in(true);
    generator_opt_in.emit_key("letter", &["ctrl"], Some("s"));

    let (header_opt_in, events_opt_in) =
        parse_cet_jsonl(&generator_opt_in.to_jsonl()).expect("parse opt-in CET");
    assert_eq!(header_opt_in.privacy.keys, "named_shortcuts");
    let key_opt_in = events_opt_in
        .iter()
        .find(|e| e.r#type == CetEventType::Key)
        .expect("key event");
    assert_eq!(key_opt_in.name.as_deref(), Some("s"));
}

#[test]
fn test_ts1_lossless_project_brief_migration() {
    let mut brief = ProjectBrief::default();
    brief.goal = "Podcast social clips from interview".into();
    brief.platform = Some("vertical-9x16".into());
    brief.target_duration_ticks = Some(45_000_000); // 45 seconds
    brief.duration_tolerance_ticks = Some(5_000_000); // 5 seconds
    brief.must_keep_clip_ids = vec![101, 102];
    brief.banned_content = vec!["dead_air_over_2s".into(), "profanity".into()];
    brief.style_notes = "Fast paced, kinetic subtitles".into();
    brief.loudness_target_lufs = Some(-14.0);

    let sheet = TaskSheet::from_brief(&brief);
    assert_eq!(sheet.title, "Podcast social clips from interview");
    assert_eq!(sheet.status, SheetStatus::Ready);
    assert_eq!(sheet.deliverables.len(), 1);

    let deliv = &sheet.deliverables[0];
    assert_eq!(deliv.preset, "vertical-9x16");
    assert_eq!(deliv.duration.unwrap().target_s, 45.0);
    assert_eq!(deliv.duration.unwrap().tolerance_s, Some(5.0));
    assert_eq!(deliv.loudness_lufs, Some(-14.0));
    assert_eq!(deliv.must_include, vec!["clip:101", "clip:102"]);

    assert_eq!(
        sheet.must_not.as_ref().unwrap().content,
        vec!["dead_air_over_2s", "profanity"]
    );
    assert_eq!(
        sheet.style.as_ref().unwrap().notes.as_deref(),
        Some("Fast paced, kinetic subtitles")
    );

    // Project back to ProjectBrief
    let restored_brief = sheet.to_brief();
    assert_eq!(restored_brief.goal, brief.goal);
    assert_eq!(restored_brief.platform, brief.platform);
    assert_eq!(
        restored_brief.target_duration_ticks,
        brief.target_duration_ticks
    );
    assert_eq!(
        restored_brief.duration_tolerance_ticks,
        brief.duration_tolerance_ticks
    );
    assert_eq!(restored_brief.must_keep_clip_ids, brief.must_keep_clip_ids);
    assert_eq!(restored_brief.banned_content, brief.banned_content);
    assert_eq!(restored_brief.style_notes, brief.style_notes);
    assert_eq!(
        restored_brief.loudness_target_lufs,
        brief.loudness_target_lufs
    );
}

#[test]
fn test_ts1_tasksheet_commands_workflow() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "TS1 Command Test"}),
        ))
        .expect("project.new failed");

    // 1. tasksheet.create
    let create_res = engine
        .execute(CommandRequest::new(
            "tasksheet.create",
            json!({
                "id": "ts_unit_test",
                "title": "Tutorial Work Order",
                "requester": "human:instructor",
                "deliverables": [
                    {
                        "id": "deliv_1",
                        "name": "Main Video",
                        "preset": "web-1080p",
                        "duration": {"target_s": 120, "tolerance_s": 10},
                        "loudness_lufs": -14,
                        "true_peak_dbtp": -1,
                        "captions": true
                    }
                ]
            }),
        ))
        .expect("tasksheet.create");
    assert_eq!(create_res["created"], true);
    assert_eq!(create_res["sheet"]["id"], "ts_unit_test");

    // 2. tasksheet.get
    let get_res = engine
        .execute(CommandRequest::new("tasksheet.get", json!({})))
        .expect("tasksheet.get");
    assert_eq!(get_res["sheet"]["id"], "ts_unit_test");
    assert_eq!(get_res["status"], "draft");

    // 3. tasksheet.validate
    let val_res = engine
        .execute(CommandRequest::new("tasksheet.validate", json!({})))
        .expect("tasksheet.validate");
    assert_eq!(val_res["valid"], true);

    // 4. tasksheet.update
    let update_res = engine
        .execute(CommandRequest::new(
            "tasksheet.update",
            json!({
                "title": "Updated Tutorial Work Order",
                "style": {"notes": "Snappy pacing"}
            }),
        ))
        .expect("tasksheet.update");
    assert_eq!(update_res["updated"], true);
    assert_eq!(update_res["sheet"]["title"], "Updated Tutorial Work Order");

    // 5. tasksheet.submit (draft -> ready)
    let submit_res = engine
        .execute(CommandRequest::new(
            "tasksheet.submit",
            json!({"reason": "Form completed by human"}),
        ))
        .expect("tasksheet.submit");
    assert_eq!(submit_res["status"], "ready");

    // Verify ProjectBrief was kept in sync
    assert_eq!(
        engine.document().project.brief.goal,
        "Updated Tutorial Work Order"
    );
}

#[test]
fn test_ts1_plan_submission_and_approval_gate() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "TS1 Plan Approval Test"}),
        ))
        .expect("project.new");

    engine
        .execute(CommandRequest::new(
            "tasksheet.create",
            json!({
                "id": "ts_plan_test",
                "title": "Plan Gate Test",
                "requester": "human:lead"
            }),
        ))
        .expect("tasksheet.create");

    // Agent submits plan
    let agent_ctx = ActorContext::agent("agent_editor", "sess_agent_1");
    let plan_res = engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({
                    "steps": [
                        {"id": 1, "title": "Cut dead air", "status": "pending"},
                        {"id": 2, "title": "Add captions", "status": "pending"}
                    ]
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.plan_submit");
    assert_eq!(plan_res["status"], "awaiting_plan_approval");

    // Agent attempts to self-approve: must fail
    let agent_approve_err = engine.execute_with_context(
        CommandRequest::new("tasksheet.plan_approve", json!({})),
        &agent_ctx,
    );
    assert!(agent_approve_err.is_err());
    assert!(format!("{:?}", agent_approve_err.unwrap_err())
        .contains("tasksheet.plan_approve requires human authorization"));

    // Human approves plan
    let human_ctx = ActorContext::human();
    let approve_res = engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_approve",
                json!({"reason": "Plan looks solid"}),
            ),
            &human_ctx,
        )
        .expect("tasksheet.plan_approve");
    assert_eq!(approve_res["status"], "running");

    // Agent reports progress
    let report_res = engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.run_report",
                json!({
                    "completed_step_id": 1,
                    "current_step_id": 2,
                    "wall_clock_min_spent": 5.5
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.run_report");
    assert_eq!(report_res["reported"], true);

    let sheet = engine
        .document()
        .project
        .task_sheet
        .as_ref()
        .expect("task sheet present");
    let steps = &sheet.plan.as_ref().unwrap().steps;
    assert_eq!(steps[0].status, SheetStepStatus::Done);
    assert_eq!(steps[1].status, SheetStepStatus::Current);
}

#[test]
fn test_ts1_af5_untrusted_media_text_refusal() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "TS1 Untrusted Test"}),
        ))
        .expect("project.new");

    // Add media asset with untrusted text in filename
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({
                "name": "INJECTED_ADVERSARIAL_PROMPT_PAYLOAD.mp4",
                "path": "/assets/INJECTED_ADVERSARIAL_PROMPT_PAYLOAD.mp4",
                "kind": "video"
            }),
        ))
        .expect("add_media");

    let agent_ctx = ActorContext::agent("assistant", "sess_assistant_1");

    // Agent attempts to create task sheet with untrusted text in title
    let err = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.create",
            json!({
                "id": "ts_tainted",
                "title": "Use INJECTED_ADVERSARIAL_PROMPT_PAYLOAD for editing",
                "requester": "agent:assistant"
            }),
        ),
        &agent_ctx,
    );
    assert!(err.is_err());
    let err_msg = format!("{:?}", err.unwrap_err());
    assert!(err_msg.contains("PermissionDenied"));
    assert!(err_msg.contains("originates from untrusted media text"));

    // Human actor can set it
    let human_ctx = ActorContext::human();
    let ok = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.create",
            json!({
                "id": "ts_approved_human",
                "title": "Human override with INJECTED_ADVERSARIAL_PROMPT_PAYLOAD",
                "requester": "human:user"
            }),
        ),
        &human_ctx,
    );
    assert!(ok.is_ok());
}

#[test]
fn test_ts1_project_save_and_reopen_preserves_task_sheet() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.new",
            json!({"name": "Reopen Task Sheet Test"}),
        ))
        .expect("project.new");

    engine
        .execute(CommandRequest::new(
            "tasksheet.create",
            json!({
                "id": "ts_reopen_check",
                "title": "Persistence Test Sheet",
                "requester": "human:admin",
                "deliverables": [
                    {
                        "id": "d1",
                        "name": "Social Reel",
                        "preset": "vertical-9x16",
                        "duration": {"target_s": 50},
                        "loudness_lufs": -14
                    }
                ]
            }),
        ))
        .expect("tasksheet.create");

    // Save to tempfile and reopen via file.open
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("reopen_test.easy-edits-pro.project");
    engine.document().save_atomic(&path).expect("save_atomic");

    // Reopen in a new engine
    let mut reopened_engine = EditorEngine::default();
    reopened_engine
        .execute(CommandRequest::new(
            "file.open",
            json!({"path": path.to_str().unwrap()}),
        ))
        .expect("file.open");

    let loaded_sheet = reopened_engine
        .document()
        .project
        .task_sheet
        .as_ref()
        .expect("reopened document has task_sheet");
    assert_eq!(loaded_sheet.id, "ts_reopen_check");
    assert_eq!(loaded_sheet.title, "Persistence Test Sheet");
    assert_eq!(loaded_sheet.deliverables[0].preset, "vertical-9x16");
}
