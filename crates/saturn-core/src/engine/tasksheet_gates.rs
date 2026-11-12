// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Task Sheet plan approval, questions, and agent execution gatekeeping (TS1/TS3).

use super::*;
use crate::tasksheet::{
    apply_transition, QuestionOption, QuestionStatus, SheetPlan, SheetStatus, SheetStep,
    TaskSheet, TaskSheetQuestion,
};
use serde_json::{json, Value};

impl EditorEngine {
    pub(super) fn dispatch_tasksheet_gates(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "tasksheet.plan_submit" => Some((|| {
                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let steps_val = params.get("steps").ok_or_else(|| {
                    DispatchError::Operation("tasksheet.plan_submit requires `steps` array".into())
                })?;
                let steps: Vec<SheetStep> = serde_json::from_value(steps_val.clone())
                    .map_err(|e| DispatchError::Operation(format!("Invalid plan steps: {e}")))?;

                for step in &steps {
                    self.check_untrusted_text_in_candidate(&step.title)?;
                    if let Some(desc) = &step.description {
                        self.check_untrusted_text_in_candidate(desc)?;
                    }
                }

                let actor = self.current_actor();
                let now = crate::engine::brief_cmds::now_epoch_ms();

                if sheet.status == SheetStatus::Draft {
                    apply_transition(&mut sheet, SheetStatus::Ready, actor.name(), now, None)
                        .map_err(|e| DispatchError::Operation(e.to_string()))?;
                }
                if sheet.status == SheetStatus::Ready {
                    apply_transition(&mut sheet, SheetStatus::Planning, actor.name(), now, None)
                        .map_err(|e| DispatchError::Operation(e.to_string()))?;
                }

                apply_transition(
                    &mut sheet,
                    SheetStatus::AwaitingPlanApproval,
                    actor.name(),
                    now,
                    Some("Submitted plan for human approval".into()),
                )
                .map_err(|e| DispatchError::Operation(e.to_string()))?;

                sheet.plan = Some(SheetPlan {
                    steps,
                    submitted_by: Some(actor.name().to_string()),
                    submitted_at_unix_ms: Some(now),
                    approved_by: None,
                    approved_at_unix_ms: None,
                    rejection_reason: None,
                });

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.plan_submit",
                    vec![],
                    Some(before_doc),
                    Some("Submitted plan".into()),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "plan_submitted": true,
                    "status": "awaiting_plan_approval",
                    "sheet": val,
                }))
            })()),

            "tasksheet.plan_approve" => Some((|| {
                let actor = self.current_actor();
                if !actor.is_human() {
                    return Err(DispatchError::Operation(
                        "PermissionDenied: tasksheet.plan_approve requires human authorization"
                            .into(),
                    ));
                }

                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let now = crate::engine::brief_cmds::now_epoch_ms();
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                apply_transition(
                    &mut sheet,
                    SheetStatus::Running,
                    "human:approver",
                    now,
                    reason,
                )
                .map_err(|e| DispatchError::Operation(e.to_string()))?;

                if let Some(ref mut plan) = sheet.plan {
                    plan.approved_by = Some(actor.name().to_string());
                    plan.approved_at_unix_ms = Some(now);
                    plan.rejection_reason = None;
                }

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.plan_approve",
                    vec![],
                    Some(before_doc),
                    Some("Approved task sheet plan".into()),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "approved": true,
                    "status": "running",
                    "sheet": val,
                }))
            })()),

            "tasksheet.plan_reject" => Some((|| {
                let actor = self.current_actor();
                if !actor.is_human() {
                    return Err(DispatchError::Operation(
                        "PermissionDenied: tasksheet.plan_reject requires human authorization"
                            .into(),
                    ));
                }

                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation("tasksheet.plan_reject requires `reason`".into())
                    })?;

                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let now = crate::engine::brief_cmds::now_epoch_ms();

                apply_transition(
                    &mut sheet,
                    SheetStatus::Planning,
                    "human:reviewer",
                    now,
                    Some(reason.to_string()),
                )
                .map_err(|e| DispatchError::Operation(e.to_string()))?;

                if let Some(ref mut plan) = sheet.plan {
                    plan.rejection_reason = Some(reason.to_string());
                }

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.plan_reject",
                    vec![],
                    Some(before_doc),
                    Some(format!("Rejected plan: {reason}")),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "rejected": true,
                    "status": "planning",
                    "reason": reason,
                    "sheet": val,
                }))
            })()),

            "tasksheet.question_ask" => Some((|| {
                let question_id = params
                    .get("question_id")
                    .or_else(|| params.get("id"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "tasksheet.question_ask requires `question_id`".into(),
                        )
                    })?;
                let question = params
                    .get("question")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation("tasksheet.question_ask requires `question`".into())
                    })?;
                let context = params.get("context").and_then(Value::as_str);
                let default_option_id = params
                    .get("default_option_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "tasksheet.question_ask requires `default_option_id`".into(),
                        )
                    })?;
                let deadline_seconds = params.get("deadline_seconds").and_then(Value::as_u64);

                let options_val = params.get("options").ok_or_else(|| {
                    DispatchError::Operation(
                        "tasksheet.question_ask requires `options` array".into(),
                    )
                })?;
                let options: Vec<QuestionOption> = serde_json::from_value(options_val.clone())
                    .map_err(|e| {
                        DispatchError::Operation(format!("Invalid options array: {e}"))
                    })?;

                if question_id.trim().is_empty() {
                    return Err(DispatchError::Operation(
                        "Question ID cannot be empty".into(),
                    ));
                }
                if question.trim().is_empty() {
                    return Err(DispatchError::Operation(
                        "Question text cannot be empty".into(),
                    ));
                }
                if options.len() < 2 {
                    return Err(DispatchError::Operation(format!(
                        "Questions must offer at least 2 options (found {})",
                        options.len()
                    )));
                }
                if !options.iter().any(|o| o.id == default_option_id) {
                    return Err(DispatchError::Operation(format!(
                        "default_option_id `{default_option_id}` does not match any option ID"
                    )));
                }

                // AF5 Untrusted text check
                self.check_untrusted_text_in_candidate(question)?;
                if let Some(ctx_text) = context {
                    self.check_untrusted_text_in_candidate(ctx_text)?;
                }
                for opt in &options {
                    self.check_untrusted_text_in_candidate(&opt.label)?;
                    if let Some(desc) = &opt.description {
                        self.check_untrusted_text_in_candidate(desc)?;
                    }
                }

                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let actor = self.current_actor();
                let now = crate::engine::brief_cmds::now_epoch_ms();

                if sheet.status != SheetStatus::NeedsInput {
                    apply_transition(
                        &mut sheet,
                        SheetStatus::NeedsInput,
                        actor.name(),
                        now,
                        Some(format!("Agent asked question: {question}")),
                    )
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                }

                let new_question = TaskSheetQuestion {
                    id: question_id.to_string(),
                    question: question.to_string(),
                    context: context.map(str::to_string),
                    options,
                    default_option_id: default_option_id.to_string(),
                    deadline_seconds,
                    asked_at_unix_ms: Some(now),
                    status: QuestionStatus::Open,
                    selected_option_id: None,
                    answered_by: None,
                    answered_at_unix_ms: None,
                };

                if let Some(idx) = sheet.questions.iter().position(|q| q.id == question_id) {
                    sheet.questions[idx] = new_question;
                } else {
                    sheet.questions.push(new_question);
                }

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.question_ask",
                    vec![],
                    Some(before_doc),
                    Some(format!("Asked question `{question_id}`: {question}")),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "asked": true,
                    "question_id": question_id,
                    "status": "needs_input",
                    "sheet": val,
                }))
            })()),

            "tasksheet.question_answer" => Some((|| {
                let actor = self.current_actor();
                if !actor.is_human() {
                    return Err(DispatchError::Operation(
                        "PermissionDenied: tasksheet.question_answer requires human authorization"
                            .into(),
                    ));
                }

                let question_id = params
                    .get("question_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "tasksheet.question_answer requires `question_id`".into(),
                        )
                    })?;
                let option_id = params
                    .get("option_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "tasksheet.question_answer requires `option_id`".into(),
                        )
                    })?;

                self.check_untrusted_text_in_candidate(option_id)?;

                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let now = crate::engine::brief_cmds::now_epoch_ms();

                let q = sheet
                    .questions
                    .iter_mut()
                    .find(|q| q.id == question_id)
                    .ok_or_else(|| {
                        DispatchError::Operation(format!("Question `{question_id}` not found"))
                    })?;

                if q.status != QuestionStatus::Open {
                    return Err(DispatchError::Operation(format!(
                        "Question `{question_id}` is not open (status: {:?})",
                        q.status
                    )));
                }

                if !q.options.iter().any(|opt| opt.id == option_id) {
                    return Err(DispatchError::Operation(format!(
                        "Option `{option_id}` is not a valid choice for question `{question_id}`"
                    )));
                }

                q.status = QuestionStatus::Answered;
                q.selected_option_id = Some(option_id.to_string());
                q.answered_by = Some(actor.name().to_string());
                q.answered_at_unix_ms = Some(now);

                let resume = !sheet.has_open_questions();
                if resume && sheet.status == SheetStatus::NeedsInput {
                    apply_transition(
                        &mut sheet,
                        SheetStatus::Running,
                        actor.name(),
                        now,
                        Some(format!(
                            "Resolved question `{question_id}` with `{option_id}`"
                        )),
                    )
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                }

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.question_answer",
                    vec![],
                    Some(before_doc),
                    Some(format!(
                        "Answered question `{question_id}` with `{option_id}`"
                    )),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "answered": true,
                    "question_id": question_id,
                    "selected_option_id": option_id,
                    "sheet_status": sheet.status.as_str(),
                    "sheet": val,
                }))
            })()),

            _ => None,
        }
    }

    pub(crate) fn check_agent_tasksheet_gates(
        &mut self,
        request: &CommandRequest,
        ctx: &ActorContext,
    ) -> Result<(), DispatchError> {
        if ctx.actor.is_human() {
            return Ok(());
        }

        let command = match find_command(&request.id) {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        if !command.mutating {
            return Ok(());
        }

        // Human authorization required commands
        if request.id == "tasksheet.question_answer" {
            return Err(DispatchError::Operation(
                "PermissionDenied: tasksheet.question_answer requires human authorization".into(),
            ));
        }
        if request.id == "tasksheet.plan_approve" {
            return Err(DispatchError::Operation(
                "PermissionDenied: tasksheet.plan_approve requires human authorization".into(),
            ));
        }
        if request.id == "tasksheet.plan_reject" {
            return Err(DispatchError::Operation(
                "PermissionDenied: tasksheet.plan_reject requires human authorization".into(),
            ));
        }
        if request.id == "review.round_close" {
            return Err(DispatchError::Operation(
                "PermissionDenied: review.round_close requires human authorization".into(),
            ));
        }

        // Safe inspection and control commands allowed for agents
        if request.id == "tasksheet.plan_submit"
            || request.id == "tasksheet.question_ask"
            || request.id == "tasksheet.run_report"
            || request.id == "tasksheet.validate"
            || request.id == "tasksheet.get"
            || request.id.starts_with("agent.viewport_")
        {
            return Ok(());
        }

        let Some(ref mut sheet) = self.document.project.task_sheet else {
            return Ok(());
        };

        let now = crate::engine::brief_cmds::now_epoch_ms();
        let expired = sheet.check_deadlines(now);
        if !expired.is_empty() && sheet.status != SheetStatus::Stopped {
            let _ = apply_transition(
                sheet,
                SheetStatus::Stopped,
                "system:deadline_monitor",
                now,
                Some("Unanswered question deadline expired; paused safely".into()),
            );
        }

        if sheet.has_expired_questions() {
            return Err(DispatchError::Operation(
                "AgentPaused: Question deadline expired; agent session is paused safely without assuming destructive choices".into(),
            ));
        }

        if sheet.status == SheetStatus::AwaitingPlanApproval {
            return Err(DispatchError::Operation(
                "PlanApprovalRequired: Agent edits are paused awaiting human plan approval".into(),
            ));
        }

        if sheet.status == SheetStatus::NeedsInput || sheet.has_open_questions() {
            return Err(DispatchError::Operation(
                "AgentPaused: Agent edits are paused awaiting human answer to open question(s)"
                    .into(),
            ));
        }

        if sheet.status == SheetStatus::Stopped {
            return Err(DispatchError::Operation(
                "AgentPaused: Agent session is stopped".into(),
            ));
        }

        if sheet.status == SheetStatus::Draft
            || sheet.status == SheetStatus::Ready
            || sheet.status == SheetStatus::Planning
        {
            return Err(DispatchError::Operation(
                "PlanApprovalRequired: Plan approval required before agent can execute edits".into(),
            ));
        }

        if sheet.status != SheetStatus::Running && sheet.status != SheetStatus::RevisionRequested {
            return Err(DispatchError::Operation(format!(
                "AgentPaused: Agent edits not permitted while task sheet is in `{}` state",
                sheet.status.as_str()
            )));
        }

        Ok(())
    }
}
