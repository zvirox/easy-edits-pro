// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration and verification tests for TS3: Plan Approval & Questions Flow.

use saturn_core::{
    actor::ActorContext,
    tasksheet::{QuestionStatus, SheetStatus, TaskSheet},
    CommandRequest, EditorEngine,
};
use serde_json::json;

fn create_engine_with_running_sheet() -> (EditorEngine, ActorContext, ActorContext) {
    let mut engine = EditorEngine::default();
    let human_ctx = ActorContext::human();
    let agent_ctx = ActorContext::agent("agent-video-editor", "session-agent-ts3");

    engine
        .execute_with_context(
            CommandRequest::new("project.new", json!({"name": "TS3 Test Project"})),
            &human_ctx,
        )
        .expect("project.new");

    // Create tasksheet
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.create",
                json!({
                    "id": "ts_orchestration",
                    "title": "Orchestration Work Order",
                    "requester": "human:producer",
                    "deliverables": [
                        {
                            "id": "deliv_1",
                            "name": "Main Reel",
                            "preset": "vertical-9x16",
                            "duration": {"target_s": 45, "tolerance_s": 5}
                        }
                    ]
                }),
            ),
            &human_ctx,
        )
        .expect("tasksheet.create");

    // Submit plan as agent
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({
                    "steps": [
                        {"id": 1, "title": "Ingest & rough cut", "status": "pending"},
                        {"id": 2, "title": "Pacing & music trim", "status": "pending"}
                    ]
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.plan_submit");

    // Approve plan as human
    engine
        .execute_with_context(
            CommandRequest::new("tasksheet.plan_approve", json!({"reason": "Looks good"})),
            &human_ctx,
        )
        .expect("tasksheet.plan_approve");

    (engine, human_ctx, agent_ctx)
}

#[test]
fn test_ts3_question_ask_validation_and_structure() {
    let (mut engine, _, agent_ctx) = create_engine_with_running_sheet();

    // 1. Missing options array (< 2 options)
    let err_less_options = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.question_ask",
            json!({
                "question_id": "q1",
                "question": "Which hook should we use?",
                "options": [
                    {"id": "opt_a", "label": "Hook A"}
                ],
                "default_option_id": "opt_a"
            }),
        ),
        &agent_ctx,
    );
    assert!(err_less_options.is_err());
    let err_msg = format!("{:?}", err_less_options.err().unwrap());
    assert!(err_msg.contains("at least 2 options"));

    // 2. default_option_id not in options
    let err_invalid_default = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.question_ask",
            json!({
                "question_id": "q1",
                "question": "Which hook should we use?",
                "options": [
                    {"id": "opt_a", "label": "Hook A"},
                    {"id": "opt_b", "label": "Hook B"}
                ],
                "default_option_id": "opt_nonexistent"
            }),
        ),
        &agent_ctx,
    );
    assert!(err_invalid_default.is_err());
    let err_msg = format!("{:?}", err_invalid_default.err().unwrap());
    assert!(err_msg.contains("default_option_id `opt_nonexistent` does not match"));

    // 3. Valid question ask
    let res = engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.question_ask",
                json!({
                    "question_id": "q_hook_choice",
                    "question": "Which hook opening do you prefer?",
                    "context": "Both hooks have equal audio quality but different energy",
                    "options": [
                        {"id": "opt_action", "label": "Action Opening", "description": "High tempo", "is_default": true},
                        {"id": "opt_quote", "label": "Key Quote Opening", "description": "Dramatic pause", "is_default": false}
                    ],
                    "default_option_id": "opt_action",
                    "deadline_seconds": 3600
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.question_ask");

    assert_eq!(res["asked"], true);
    assert_eq!(res["status"], "needs_input");
    assert_eq!(res["question_id"], "q_hook_choice");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::NeedsInput);
    assert_eq!(sheet.questions.len(), 1);
    assert_eq!(sheet.questions[0].status, QuestionStatus::Open);
    assert_eq!(sheet.questions[0].default_option_id, "opt_action");
    assert_eq!(sheet.questions[0].deadline_seconds, Some(3600));
}

#[test]
fn test_ts3_question_af5_untrusted_media_text_refusal() {
    let (mut engine, human_ctx, agent_ctx) = create_engine_with_running_sheet();

    // Add media asset with untrusted text in filename / metadata
    engine
        .execute_with_context(
            CommandRequest::new(
                "project.add_media",
                json!({
                    "name": "INJECTED_ADVERSARIAL_PROMPT_PAYLOAD.mp4",
                    "path": "/assets/INJECTED_ADVERSARIAL_PROMPT_PAYLOAD.mp4",
                    "kind": "video"
                }),
            ),
            &human_ctx,
        )
        .expect("add_media");

    // Agent attempts to ask question containing untrusted text
    let agent_ask = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.question_ask",
            json!({
                "question_id": "q_injected",
                "question": "Should we use INJECTED_ADVERSARIAL_PROMPT_PAYLOAD in edit?",
                "options": [
                    {"id": "yes", "label": "Yes"},
                    {"id": "no", "label": "No"}
                ],
                "default_option_id": "no"
            }),
        ),
        &agent_ctx,
    );

    assert!(agent_ask.is_err());
    let err_str = format!("{:?}", agent_ask.err().unwrap());
    assert!(err_str.contains("PermissionDenied"));
    assert!(err_str.contains("untrusted media text"));

    // Human actor is permitted even if text matches
    let human_ask = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.question_ask",
            json!({
                "question_id": "q_human",
                "question": "Human approved INJECTED_ADVERSARIAL_PROMPT_PAYLOAD question?",
                "options": [
                    {"id": "yes", "label": "Yes"},
                    {"id": "no", "label": "No"}
                ],
                "default_option_id": "no"
            }),
        ),
        &human_ctx,
    );
    assert!(human_ask.is_ok());
}

#[test]
fn test_ts3_plan_approval_gates_agent_edits() {
    let mut engine = EditorEngine::default();
    let human_ctx = ActorContext::human();
    let agent_ctx = ActorContext::agent("agent-cutter", "session-agent-cutter");

    engine
        .execute_with_context(
            CommandRequest::new("project.new", json!({"name": "Plan Gate Test"})),
            &human_ctx,
        )
        .expect("project.new");

    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.create",
                json!({
                    "id": "ts_gate",
                    "title": "Gate Work Order",
                    "requester": "human:lead",
                    "deliverables": [
                        {
                            "id": "d1",
                            "name": "Final",
                            "preset": "web-1080p",
                            "duration": {"target_s": 60}
                        }
                    ]
                }),
            ),
            &human_ctx,
        )
        .expect("tasksheet.create");

    // 1. Before submitting plan (sheet is Draft or Ready): agent edits must be blocked
    let premature_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(premature_edit.is_err());
    let err_msg = format!("{:?}", premature_edit.err().unwrap());
    assert!(err_msg.contains("PlanApprovalRequired"));

    // 2. Submit plan -> transitions to AwaitingPlanApproval
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({
                    "steps": [
                        {"id": 1, "title": "Step 1", "status": "pending"}
                    ]
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.plan_submit");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::AwaitingPlanApproval);

    // 3. While in AwaitingPlanApproval, agent edit is blocked
    let paused_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(paused_edit.is_err());
    let err_msg = format!("{:?}", paused_edit.err().unwrap());
    assert!(err_msg.contains("PlanApprovalRequired: Agent edits are paused awaiting human plan approval"));

    // 4. Human rejection -> transitions to Planning
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_reject",
                json!({"reason": "Need more specific steps"}),
            ),
            &human_ctx,
        )
        .expect("tasksheet.plan_reject");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::Planning);

    // Agent edit still blocked in Planning
    let planning_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(planning_edit.is_err());

    // 5. Agent resubmits plan
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({
                    "steps": [
                        {"id": 1, "title": "Revised Step 1", "status": "pending"},
                        {"id": 2, "title": "Revised Step 2", "status": "pending"}
                    ]
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.plan_submit");

    // 6. Human approves -> transitions to Running
    engine
        .execute_with_context(
            CommandRequest::new("tasksheet.plan_approve", json!({"reason": "Approved now"})),
            &human_ctx,
        )
        .expect("tasksheet.plan_approve");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::Running);

    // 7. Agent edit now succeeds!
    let unblocked_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(unblocked_edit.is_ok());
}

#[test]
fn test_ts3_question_pause_and_answer_resumption() {
    let (mut engine, human_ctx, agent_ctx) = create_engine_with_running_sheet();

    // 1. Agent asks question -> transitions to NeedsInput
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.question_ask",
                json!({
                    "question_id": "q_pacing",
                    "question": "Which music pacing do you prefer?",
                    "options": [
                        {"id": "fast", "label": "Fast Pacing", "is_default": true},
                        {"id": "chill", "label": "Chill Pacing", "is_default": false}
                    ],
                    "default_option_id": "fast"
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.question_ask");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::NeedsInput);

    // 2. While in NeedsInput, agent mutating edit is blocked
    let paused_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(paused_edit.is_err());
    let err_msg = format!("{:?}", paused_edit.err().unwrap());
    assert!(err_msg.contains("AgentPaused: Agent edits are paused awaiting human answer to open question(s)"));

    // 3. Read-only commands are still allowed for the agent
    let read_sheet = engine.execute_with_context(
        CommandRequest::new("tasksheet.get", json!({})),
        &agent_ctx,
    );
    assert!(read_sheet.is_ok());

    // 4. Agent cannot answer its own question
    let self_answer = engine.execute_with_context(
        CommandRequest::new(
            "tasksheet.question_answer",
            json!({
                "question_id": "q_pacing",
                "option_id": "fast"
            }),
        ),
        &agent_ctx,
    );
    assert!(self_answer.is_err());
    let err_msg = format!("{:?}", self_answer.err().unwrap());
    assert!(err_msg.contains("PermissionDenied: tasksheet.question_answer requires human authorization"));

    // 5. Human answers the question -> transitions back to Running
    let answer_res = engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.question_answer",
                json!({
                    "question_id": "q_pacing",
                    "option_id": "chill"
                }),
            ),
            &human_ctx,
        )
        .expect("tasksheet.question_answer");

    assert_eq!(answer_res["answered"], true);
    assert_eq!(answer_res["selected_option_id"], "chill");
    assert_eq!(answer_res["sheet_status"], "running");

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::Running);
    assert_eq!(sheet.questions[0].status, QuestionStatus::Answered);
    assert_eq!(
        sheet.questions[0].selected_option_id.as_deref(),
        Some("chill")
    );

    // 6. Agent mutating edit is now resumed and succeeds!
    let resumed_edit = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );
    assert!(resumed_edit.is_ok());
}

#[test]
fn test_ts3_unanswered_question_deadline_safely_pauses() {
    let (mut engine, _, agent_ctx) = create_engine_with_running_sheet();

    // 1. Agent asks question with deadline_seconds: 0 (immediate expiry)
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.question_ask",
                json!({
                    "question_id": "q_deadline_test",
                    "question": "Should we cut this controversial scene?",
                    "options": [
                        {"id": "cut", "label": "Cut Scene", "is_default": true},
                        {"id": "keep", "label": "Keep Scene", "is_default": false}
                    ],
                    "default_option_id": "cut",
                    "deadline_seconds": 0
                }),
            ),
            &agent_ctx,
        )
        .expect("tasksheet.question_ask");

    // 2. Next agent edit triggers deadline evaluation -> question expires, sheet transitions to Stopped
    let attempt = engine.execute_with_context(
        CommandRequest::new("marker.add_chapter", json!({"at": 1000})),
        &agent_ctx,
    );

    assert!(attempt.is_err());
    let err_msg = format!("{:?}", attempt.err().unwrap());
    assert!(err_msg.contains("AgentPaused: Question deadline expired; agent session is paused safely without assuming destructive choices"));

    let sheet = engine.document().project.task_sheet.as_ref().unwrap();
    assert_eq!(sheet.status, SheetStatus::Stopped);
    assert_eq!(sheet.questions[0].status, QuestionStatus::Expired);

    // Direct unit verification of check_deadlines
    let mut test_sheet = TaskSheet::new("ts_unit", "Unit Test", "human:lead");
    test_sheet.questions.push(saturn_core::tasksheet::TaskSheetQuestion {
        id: "q_future".into(),
        question: "Future question?".into(),
        context: None,
        options: vec![
            saturn_core::tasksheet::QuestionOption {
                id: "1".into(),
                label: "Option 1".into(),
                description: None,
                is_default: Some(true),
                thumbnail: None,
            },
            saturn_core::tasksheet::QuestionOption {
                id: "2".into(),
                label: "Option 2".into(),
                description: None,
                is_default: Some(false),
                thumbnail: None,
            },
        ],
        default_option_id: "1".into(),
        deadline_seconds: Some(60),
        asked_at_unix_ms: Some(1_000_000),
        status: QuestionStatus::Open,
        selected_option_id: None,
        answered_by: None,
        answered_at_unix_ms: None,
    });

    // Before deadline (50 seconds elapsed) -> remains open
    let expired_before = test_sheet.check_deadlines(1_050_000);
    assert!(expired_before.is_empty());
    assert_eq!(test_sheet.questions[0].status, QuestionStatus::Open);

    // At or after deadline (60+ seconds elapsed) -> expires
    let expired_after = test_sheet.check_deadlines(1_060_000);
    assert_eq!(expired_after, vec!["q_future"]);
    assert_eq!(test_sheet.questions[0].status, QuestionStatus::Expired);
}
