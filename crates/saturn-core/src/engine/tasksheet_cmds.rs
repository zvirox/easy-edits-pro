// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::provenance::OriginTracker;
use crate::tasksheet::{
    apply_transition, DurationConstraint, SheetAcceptance, SheetBudget, SheetDeliverable,
    SheetHandoff, SheetInput, SheetMustNot, SheetStatus, SheetStepStatus, SheetStyle, TaskSheet,
    TASKSHEET_SCHEMA_VERSION,
};
use serde_json::{json, Value};

impl EditorEngine {
    pub(super) fn check_untrusted_text_in_candidate(
        &self,
        candidate: &str,
    ) -> Result<(), DispatchError> {
        let tracker = OriginTracker::from_document(&self.document);
        if let Some(derivation) = tracker.detect_untrusted_derivation(candidate) {
            let actor = self.current_actor();
            if !actor.is_human() {
                return Err(DispatchError::Operation(format!(
                    "PermissionDenied: Task sheet mutation refused: text `{candidate}` originates from untrusted media text (matched '{}')",
                    derivation.matched_text
                )));
            }
        }
        Ok(())
    }

    pub(super) fn dispatch_tasksheet(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "tasksheet.create" => Some((|| {
                let id_val = params
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("ts_default");
                let title_val = params
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("New Task Sheet");
                let requester_val = params
                    .get("requester")
                    .and_then(Value::as_str)
                    .unwrap_or("human:user");

                self.check_untrusted_text_in_candidate(title_val)?;

                let mut sheet: TaskSheet = serde_json::from_value(params.clone())
                    .unwrap_or_else(|_| TaskSheet::new(id_val, title_val, requester_val));
                sheet.schema = TASKSHEET_SCHEMA_VERSION.to_string();

                if let Some(inputs) = params.get("inputs").and_then(Value::as_array) {
                    sheet.inputs =
                        serde_json::from_value::<Vec<SheetInput>>(Value::Array(inputs.clone()))
                            .unwrap_or_default();
                }
                if let Some(handoff) = params.get("handoff") {
                    if let Ok(h) = serde_json::from_value::<SheetHandoff>(handoff.clone()) {
                        sheet.handoff = Some(h);
                    }
                }
                if let Some(delivs) = params.get("deliverables").and_then(Value::as_array) {
                    sheet.deliverables =
                        serde_json::from_value(Value::Array(delivs.clone())).unwrap_or_default();
                } else if sheet.deliverables.is_empty() {
                    sheet.deliverables = vec![SheetDeliverable {
                        id: "deliv_main".into(),
                        name: "Main Deliverable".into(),
                        preset: "web-1080p".into(),
                        duration: Some(DurationConstraint {
                            target_s: 60.0,
                            tolerance_s: Some(10.0),
                        }),
                        loudness_lufs: Some(-14.0),
                        true_peak_dbtp: Some(-1.0),
                        captions: Some(true),
                        must_include: Vec::new(),
                    }];
                }

                sheet.validate().map_err(DispatchError::Operation)?;

                self.checkpoint();
                let before_doc = self.document.clone();

                self.document.project.brief = sheet.to_brief();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.create",
                    vec![],
                    Some(before_doc),
                    Some(format!("Created task sheet {}", sheet.id)),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "created": true,
                    "sheet": val,
                }))
            })()),

            "tasksheet.update" => Some((|| {
                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                if let Some(title) = params.get("title").and_then(Value::as_str) {
                    self.check_untrusted_text_in_candidate(title)?;
                    sheet.title = title.to_string();
                }
                if let Some(inputs) = params.get("inputs") {
                    if let Ok(parsed) = serde_json::from_value::<Vec<SheetInput>>(inputs.clone()) {
                        sheet.inputs = parsed;
                    }
                }
                if let Some(delivs) = params.get("deliverables") {
                    if let Ok(parsed) =
                        serde_json::from_value::<Vec<SheetDeliverable>>(delivs.clone())
                    {
                        for d in &parsed {
                            self.check_untrusted_text_in_candidate(&d.name)?;
                        }
                        sheet.deliverables = parsed;
                    }
                }
                if let Some(style) = params.get("style") {
                    if let Ok(parsed) = serde_json::from_value::<SheetStyle>(style.clone()) {
                        if let Some(notes) = &parsed.notes {
                            self.check_untrusted_text_in_candidate(notes)?;
                        }
                        sheet.style = Some(parsed);
                    }
                }
                if let Some(must_not) = params.get("must_not") {
                    if let Ok(parsed) = serde_json::from_value::<SheetMustNot>(must_not.clone()) {
                        sheet.must_not = Some(parsed);
                    }
                }
                if let Some(acceptance) = params.get("acceptance") {
                    if let Ok(parsed) =
                        serde_json::from_value::<SheetAcceptance>(acceptance.clone())
                    {
                        sheet.acceptance = Some(parsed);
                    }
                }
                if let Some(budget) = params.get("budget") {
                    if let Ok(parsed) = serde_json::from_value::<SheetBudget>(budget.clone()) {
                        sheet.budget = Some(parsed);
                    }
                }
                if let Some(escalate) = params.get("escalate_when").and_then(Value::as_array) {
                    sheet.escalate_when = escalate
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                }
                if let Some(handoff) = params.get("handoff") {
                    if let Ok(parsed) = serde_json::from_value::<SheetHandoff>(handoff.clone()) {
                        sheet.handoff = Some(parsed);
                    }
                }

                sheet.validate().map_err(DispatchError::Operation)?;

                self.checkpoint();
                let before_doc = self.document.clone();

                self.document.project.brief = sheet.to_brief();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.update",
                    vec![],
                    Some(before_doc),
                    Some(format!("Updated task sheet {}", sheet.id)),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "updated": true,
                    "sheet": val,
                }))
            })()),

            "tasksheet.validate" => Some((|| {
                let sheet = if let Some(sheet_val) = params.get("sheet") {
                    serde_json::from_value::<TaskSheet>(sheet_val.clone()).map_err(|e| {
                        DispatchError::Operation(format!("Invalid task sheet format: {e}"))
                    })?
                } else if let Some(ref current) = self.document.project.task_sheet {
                    current.clone()
                } else {
                    TaskSheet::from_brief(&self.document.project.brief)
                };

                match sheet.validate() {
                    Ok(()) => Ok(json!({
                        "valid": true,
                        "status": sheet.status.as_str(),
                        "deliverables_count": sheet.deliverables.len(),
                    })),
                    Err(e) => Ok(json!({
                        "valid": false,
                        "error": e,
                    })),
                }
            })()),

            "tasksheet.submit" => Some((|| {
                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                let actor = self.current_actor();
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                apply_transition(
                    &mut sheet,
                    SheetStatus::Ready,
                    actor.name(),
                    crate::engine::brief_cmds::now_epoch_ms(),
                    reason,
                )
                .map_err(|e| DispatchError::Operation(e.to_string()))?;

                self.checkpoint();
                let before_doc = self.document.clone();

                self.document.project.task_sheet = Some(sheet.clone());
                self.document.project.brief = sheet.to_brief();

                self.record_mutating_change(
                    "tasksheet.submit",
                    vec![],
                    Some(before_doc),
                    Some(format!("Submitted task sheet {}", sheet.id)),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "submitted": true,
                    "status": "ready",
                    "sheet": val,
                }))
            })()),

            "tasksheet.get" => Some((|| {
                let sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));
                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "sheet": val,
                    "status": sheet.status.as_str(),
                }))
            })()),

            "tasksheet.run_report" => Some((|| {
                let mut sheet = self
                    .document
                    .project
                    .task_sheet
                    .clone()
                    .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

                if let Some(completed_id) = params.get("completed_step_id").and_then(Value::as_u64)
                {
                    if let Some(ref mut plan) = sheet.plan {
                        for step in &mut plan.steps {
                            if step.id as u64 == completed_id {
                                step.status = SheetStepStatus::Done;
                            }
                        }
                    }
                }

                if let Some(current_id) = params.get("current_step_id").and_then(Value::as_u64) {
                    if let Some(ref mut plan) = sheet.plan {
                        for step in &mut plan.steps {
                            if step.id as u64 == current_id {
                                step.status = SheetStepStatus::Current;
                            }
                        }
                    }
                }

                self.checkpoint();
                let before_doc = self.document.clone();
                self.document.project.task_sheet = Some(sheet.clone());

                self.record_mutating_change(
                    "tasksheet.run_report",
                    vec![],
                    Some(before_doc),
                    Some("Updated run report".into()),
                    None,
                );

                let val = serde_json::to_value(&sheet)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                Ok(json!({
                    "reported": true,
                    "sheet": val,
                }))
            })()),

            _ => None,
        }
    }
}
