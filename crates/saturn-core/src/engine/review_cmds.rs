// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Engine command handlers for pro review notes & revision rounds (TS4).

use super::*;
use crate::engine::brief_cmds::now_epoch_ms;
use crate::tasksheet::{
    apply_transition, capture_timeline_snapshot, compute_round_diff, ReviewNote, ReviewNoteStatus,
    ReviewRound, RoundDiffReport, RoundTimelineSnapshot, SheetStatus, TaskSheet,
};
use serde_json::{json, Value};

fn note_status_str(status: ReviewNoteStatus) -> &'static str {
    match status {
        ReviewNoteStatus::Open => "open",
        ReviewNoteStatus::Resolved => "resolved",
        ReviewNoteStatus::Dismissed => "dismissed",
    }
}

fn parse_note_status_filter(s: &str) -> Result<ReviewNoteStatus, DispatchError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "open" => Ok(ReviewNoteStatus::Open),
        "resolved" => Ok(ReviewNoteStatus::Resolved),
        "dismissed" => Ok(ReviewNoteStatus::Dismissed),
        other => Err(DispatchError::InvalidParameters(format!(
            "Invalid review note status `{other}`; expected `open`, `resolved`, or `dismissed`"
        ))),
    }
}

fn count_open_notes(sheet: &TaskSheet) -> usize {
    sheet
        .rounds
        .iter()
        .flat_map(|r| r.notes.iter())
        .filter(|n| n.status == ReviewNoteStatus::Open)
        .count()
}

fn latest_prior_snapshot(
    rounds: &[ReviewRound],
    before_round_number: u32,
) -> Option<(u32, &RoundTimelineSnapshot)> {
    rounds
        .iter()
        .rev()
        .find(|r| r.round_number < before_round_number)
        .and_then(|r| {
            r.snapshot_at_close
                .as_ref()
                .or(r.snapshot_at_open.as_ref())
                .map(|snap| (r.round_number, snap))
        })
}

fn transition_sheet_to_in_review(
    sheet: &mut TaskSheet,
    actor_name: &str,
    now: u64,
    reason: Option<String>,
) -> Result<(), DispatchError> {
    match sheet.status {
        SheetStatus::InReview => Ok(()),
        SheetStatus::Draft => {
            apply_transition(sheet, SheetStatus::Ready, actor_name, now, reason.clone())
                .map_err(|e| DispatchError::Operation(e.to_string()))?;
            apply_transition(sheet, SheetStatus::Running, actor_name, now, None)
                .map_err(|e| DispatchError::Operation(e.to_string()))?;
            apply_transition(sheet, SheetStatus::InReview, actor_name, now, reason)
                .map_err(|e| DispatchError::Operation(e.to_string()))
        }
        SheetStatus::Ready | SheetStatus::Planning => {
            apply_transition(sheet, SheetStatus::Running, actor_name, now, None)
                .map_err(|e| DispatchError::Operation(e.to_string()))?;
            apply_transition(sheet, SheetStatus::InReview, actor_name, now, reason)
                .map_err(|e| DispatchError::Operation(e.to_string()))
        }
        SheetStatus::Running | SheetStatus::RevisionRequested | SheetStatus::Stopped => {
            apply_transition(sheet, SheetStatus::InReview, actor_name, now, reason)
                .map_err(|e| DispatchError::Operation(e.to_string()))
        }
        other => Err(DispatchError::Operation(format!(
            "Cannot open review round while task sheet is in `{}` status",
            other.as_str()
        ))),
    }
}

fn open_next_round_on_sheet(
    sheet: &mut TaskSheet,
    project: &crate::project::Project,
    head_revision: u64,
    actor_name: &str,
    now: u64,
    label: Option<String>,
    reason: Option<String>,
) -> Result<u32, DispatchError> {
    if let Some(active) = sheet
        .rounds
        .iter()
        .rev()
        .find(|r| r.completed_at_unix_ms.is_none())
    {
        return Err(DispatchError::Operation(format!(
            "Review round v{} is already open; close it before opening a new round",
            active.round_number
        )));
    }

    let round_number = sheet
        .rounds
        .iter()
        .map(|r| r.round_number)
        .max()
        .unwrap_or(0)
        + 1;
    let snap = capture_timeline_snapshot(project, head_revision);
    let diff_from_prior = latest_prior_snapshot(&sheet.rounds, round_number)
        .map(|(prior_num, prior_snap)| compute_round_diff(prior_num, prior_snap, round_number, &snap));

    transition_sheet_to_in_review(sheet, actor_name, now, reason)?;

    let round_label = label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| format!("v{round_number}"));

    sheet.rounds.push(ReviewRound {
        round_number,
        label: Some(round_label),
        started_at_unix_ms: now,
        completed_at_unix_ms: None,
        opened_by: Some(actor_name.to_string()),
        closed_by: None,
        outcome: None,
        snapshot_at_open: Some(snap),
        snapshot_at_close: None,
        diff_from_prior,
        notes: Vec::new(),
    });

    Ok(round_number)
}

impl EditorEngine {
    pub(super) fn dispatch_review(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "review.round_open" => Some(self.handle_round_open(params)),
            "review.round_close" => Some(self.handle_round_close(params)),
            "review.note_add" => Some(self.handle_note_add(params)),
            "review.note_list" => Some(self.handle_note_list(params)),
            "review.note_resolve" => Some(self.handle_note_resolve(params)),
            _ => None,
        }
    }

    fn handle_round_open(&mut self, params: Value) -> Result<Value, DispatchError> {
        let label = params
            .get("label")
            .and_then(Value::as_str)
            .map(str::to_string);
        let reason = params
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string);

        if let Some(ref l) = label {
            self.check_untrusted_text_in_candidate(l)?;
        }
        if let Some(ref r) = reason {
            self.check_untrusted_text_in_candidate(r)?;
        }

        let actor = self.current_actor();
        let now = now_epoch_ms();
        let mut sheet = self
            .document
            .project
            .task_sheet
            .clone()
            .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

        let round_number = open_next_round_on_sheet(
            &mut sheet,
            &self.document.project,
            self.journal.head_revision,
            actor.name(),
            now,
            label,
            reason,
        )?;

        let unresolved_prior_notes = count_open_notes(&sheet);

        self.checkpoint();
        self.document.project.task_sheet = Some(sheet.clone());

        let opened_round = sheet.rounds.last().cloned();
        let diff_val = opened_round
            .as_ref()
            .and_then(|r| r.diff_from_prior.as_ref())
            .and_then(|d| serde_json::to_value(d).ok());

        Ok(json!({
            "opened": true,
            "round_number": round_number,
            "label": opened_round.as_ref().and_then(|r| r.label.clone()).unwrap_or_else(|| format!("v{round_number}")),
            "status": sheet.status.as_str(),
            "unresolved_prior_notes": unresolved_prior_notes,
            "diff_from_prior": diff_val,
            "round": opened_round,
            "sheet": sheet,
        }))
    }

    fn handle_round_close(&mut self, params: Value) -> Result<Value, DispatchError> {
        let actor = self.current_actor();
        if !actor.is_human() {
            return Err(DispatchError::Operation(
                "PermissionDenied: review.round_close requires human authorization".into(),
            ));
        }

        let outcome_raw = params
            .get("outcome")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "review.round_close requires `outcome` (`revision_requested` or `approved`)"
                        .into(),
                )
            })?;

        let (outcome_str, target_status) = match outcome_raw.trim().to_ascii_lowercase().as_str() {
            "revision_requested" | "revisions_requested" | "request_revisions" => {
                ("revision_requested", SheetStatus::RevisionRequested)
            }
            "approved" | "approve" => ("approved", SheetStatus::Approved),
            other => {
                return Err(DispatchError::InvalidParameters(format!(
                    "Invalid review round outcome `{other}`; expected `revision_requested` or `approved`"
                )));
            }
        };

        let reason = params
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string);
        let requested_round = params
            .get("round_number")
            .and_then(Value::as_u64)
            .map(|n| n as u32);

        let mut sheet = self
            .document
            .project
            .task_sheet
            .clone()
            .ok_or_else(|| DispatchError::Operation("No active task sheet / review round".into()))?;

        let open_notes = count_open_notes(&sheet);
        if target_status == SheetStatus::Approved && open_notes > 0 {
            return Err(DispatchError::Operation(format!(
                "Cannot approve review round: {open_notes} open review note(s) remain unresolved; resolve or dismiss all notes with explanations first"
            )));
        }

        let target_idx = match requested_round {
            Some(rn) => sheet
                .rounds
                .iter()
                .position(|r| r.round_number == rn && r.completed_at_unix_ms.is_none())
                .ok_or_else(|| {
                    DispatchError::Operation(format!("Review round v{rn} is not open"))
                })?,
            None => sheet
                .rounds
                .iter()
                .rposition(|r| r.completed_at_unix_ms.is_none())
                .ok_or_else(|| {
                    DispatchError::Operation("No open review round to close".into())
                })?,
        };

        let now = now_epoch_ms();
        let round_number = sheet.rounds[target_idx].round_number;
        let close_snap = capture_timeline_snapshot(&self.document.project, self.journal.head_revision);

        let diff_report: Option<RoundDiffReport> =
            latest_prior_snapshot(&sheet.rounds[..target_idx], round_number)
                .map(|(prior_num, prior_snap)| {
                    compute_round_diff(prior_num, prior_snap, round_number, &close_snap)
                })
                .or_else(|| {
                    sheet.rounds[target_idx]
                        .snapshot_at_open
                        .as_ref()
                        .map(|open_snap| {
                            compute_round_diff(round_number, open_snap, round_number, &close_snap)
                        })
                });

        if sheet.status != SheetStatus::InReview {
            transition_sheet_to_in_review(&mut sheet, actor.name(), now, None)?;
        }
        apply_transition(&mut sheet, target_status, actor.name(), now, reason)
            .map_err(|e| DispatchError::Operation(e.to_string()))?;

        let round = &mut sheet.rounds[target_idx];
        round.completed_at_unix_ms = Some(now);
        round.closed_by = Some(actor.name().to_string());
        round.outcome = Some(outcome_str.to_string());
        round.snapshot_at_close = Some(close_snap);
        round.diff_from_prior = diff_report;

        let closed_round = sheet.rounds[target_idx].clone();

        self.checkpoint();
        self.document.project.task_sheet = Some(sheet.clone());

        Ok(json!({
            "closed": true,
            "round_number": round_number,
            "outcome": outcome_str,
            "sheet_status": sheet.status.as_str(),
            "open_notes_remaining": count_open_notes(&sheet),
            "diff_from_prior": closed_round.diff_from_prior,
            "round": closed_round,
            "sheet": sheet,
        }))
    }

    fn handle_note_add(&mut self, params: Value) -> Result<Value, DispatchError> {
        let text = params
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "review.note_add requires non-empty `text`".into(),
                )
            })?
            .to_string();

        self.check_untrusted_text_in_candidate(&text)?;

        let target_tick = params.get("target_tick").and_then(Value::as_i64);
        let end_tick = params.get("end_tick").and_then(Value::as_i64);
        let clip_id = params.get("clip_id").and_then(Value::as_u64);
        let track_id = params.get("track_id").and_then(Value::as_u64);
        let requested_round = params
            .get("round_number")
            .and_then(Value::as_u64)
            .map(|n| n as u32);
        let custom_id = params
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        if let Some(end) = end_tick {
            let start = target_tick.ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "review.note_add with `end_tick` also requires `target_tick`".into(),
                )
            })?;
            if end < start {
                return Err(DispatchError::InvalidParameters(format!(
                    "review.note_add `end_tick` ({end}) must be >= `target_tick` ({start})"
                )));
            }
        }

        if let Some(cid) = clip_id {
            let exists = self
                .document
                .project
                .sequence
                .tracks
                .iter()
                .any(|t| t.clips.iter().any(|c| c.id == cid));
            if !exists {
                return Err(DispatchError::Operation(format!(
                    "Clip {cid} not found in active sequence"
                )));
            }
        }

        if let Some(tid) = track_id {
            let exists = self
                .document
                .project
                .sequence
                .tracks
                .iter()
                .any(|t| t.id == tid);
            if !exists {
                return Err(DispatchError::Operation(format!(
                    "Track {tid} not found in active sequence"
                )));
            }
        }

        let actor = self.current_actor();
        let now = now_epoch_ms();
        let mut sheet = self
            .document
            .project
            .task_sheet
            .clone()
            .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

        let target_idx = if let Some(rn) = requested_round {
            let idx = sheet
                .rounds
                .iter()
                .position(|r| r.round_number == rn)
                .ok_or_else(|| DispatchError::Operation(format!("Review round v{rn} not found")))?;
            if sheet.rounds[idx].completed_at_unix_ms.is_some() {
                return Err(DispatchError::Operation(format!(
                    "Review round v{rn} is already closed; open a new round to add notes"
                )));
            }
            idx
        } else if let Some(idx) = sheet
            .rounds
            .iter()
            .rposition(|r| r.completed_at_unix_ms.is_none())
        {
            idx
        } else if actor.is_human() && sheet.status != SheetStatus::Approved && !sheet.status.is_terminal() {
            open_next_round_on_sheet(
                &mut sheet,
                &self.document.project,
                self.journal.head_revision,
                actor.name(),
                now,
                None,
                Some("Opened review round for note creation".into()),
            )?;
            sheet.rounds.len() - 1
        } else {
            return Err(DispatchError::Operation(
                "No open review round; open a round first with `review.round_open`".into(),
            ));
        };

        let round_number = sheet.rounds[target_idx].round_number;
        let note_id = if let Some(cid) = custom_id {
            let duplicate = sheet
                .rounds
                .iter()
                .flat_map(|r| r.notes.iter())
                .any(|n| n.id == cid);
            if duplicate {
                return Err(DispatchError::Operation(format!(
                    "Review note ID `{cid}` already exists"
                )));
            }
            cid
        } else {
            let mut seq = sheet.rounds[target_idx].notes.len() + 1;
            loop {
                let candidate = format!("note_r{round_number}_{seq}");
                let exists = sheet
                    .rounds
                    .iter()
                    .flat_map(|r| r.notes.iter())
                    .any(|n| n.id == candidate);
                if !exists {
                    break candidate;
                }
                seq += 1;
            }
        };

        let note = ReviewNote {
            id: note_id,
            author: actor.name().to_string(),
            created_at_unix_ms: now,
            text,
            target_tick,
            end_tick,
            clip_id,
            track_id,
            status: ReviewNoteStatus::Open,
            resolution: None,
            resolved_by: None,
            resolved_at_unix_ms: None,
        };

        sheet.rounds[target_idx].notes.push(note.clone());

        self.checkpoint();
        self.document.project.task_sheet = Some(sheet.clone());

        Ok(json!({
            "added": true,
            "round_number": round_number,
            "note": note,
            "open_notes_total": count_open_notes(&sheet),
            "sheet": sheet,
        }))
    }

    fn handle_note_list(&mut self, params: Value) -> Result<Value, DispatchError> {
        let round_filter = params
            .get("round_number")
            .and_then(Value::as_u64)
            .map(|n| n as u32);
        let status_filter = params
            .get("status")
            .and_then(Value::as_str)
            .map(parse_note_status_filter)
            .transpose()?;
        let clip_filter = params.get("clip_id").and_then(Value::as_u64);
        let track_filter = params.get("track_id").and_then(Value::as_u64);

        let sheet = self
            .document
            .project
            .task_sheet
            .clone()
            .unwrap_or_else(|| TaskSheet::from_brief(&self.document.project.brief));

        let mut open_total = 0usize;
        let mut resolved_total = 0usize;
        let mut dismissed_total = 0usize;
        let mut notes = Vec::new();
        let mut rounds_summary = Vec::new();

        for round in &sheet.rounds {
            let mut round_open = 0usize;
            for note in &round.notes {
                match note.status {
                    ReviewNoteStatus::Open => {
                        open_total += 1;
                        round_open += 1;
                    }
                    ReviewNoteStatus::Resolved => resolved_total += 1,
                    ReviewNoteStatus::Dismissed => dismissed_total += 1,
                }

                if let Some(rf) = round_filter {
                    if round.round_number != rf {
                        continue;
                    }
                }
                if let Some(sf) = status_filter {
                    if note.status != sf {
                        continue;
                    }
                }
                if let Some(cf) = clip_filter {
                    if note.clip_id != Some(cf) {
                        continue;
                    }
                }
                if let Some(tf) = track_filter {
                    if note.track_id != Some(tf) {
                        continue;
                    }
                }

                let mut item = serde_json::to_value(note)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;
                if let Some(obj) = item.as_object_mut() {
                    obj.insert("round_number".into(), json!(round.round_number));
                    obj.insert(
                        "round_label".into(),
                        json!(round.label.clone().unwrap_or_else(|| format!("v{}", round.round_number))),
                    );
                }
                notes.push(item);
            }

            rounds_summary.push(json!({
                "round_number": round.round_number,
                "label": round.label.clone().unwrap_or_else(|| format!("v{}", round.round_number)),
                "started_at_unix_ms": round.started_at_unix_ms,
                "completed_at_unix_ms": round.completed_at_unix_ms,
                "outcome": round.outcome,
                "note_count": round.notes.len(),
                "open_count": round_open,
                "diff_from_prior": round.diff_from_prior,
            }));
        }

        let active_round = sheet
            .rounds
            .iter()
            .rev()
            .find(|r| r.completed_at_unix_ms.is_none())
            .map(|r| r.round_number);

        Ok(json!({
            "active_round": active_round,
            "sheet_status": sheet.status.as_str(),
            "open_count": open_total,
            "resolved_count": resolved_total,
            "dismissed_count": dismissed_total,
            "notes": notes,
            "rounds": rounds_summary,
        }))
    }

    fn handle_note_resolve(&mut self, params: Value) -> Result<Value, DispatchError> {
        let note_id = params
            .get("note_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "review.note_resolve requires non-empty `note_id`".into(),
                )
            })?
            .to_string();

        let resolution = params
            .get("resolution")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "review.note_resolve requires a non-empty `resolution` explanation; notes cannot be silently dropped"
                        .into(),
                )
            })?
            .to_string();

        self.check_untrusted_text_in_candidate(&resolution)?;

        let target_status = match params.get("status").and_then(Value::as_str) {
            None => ReviewNoteStatus::Resolved,
            Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
                "resolved" => ReviewNoteStatus::Resolved,
                "dismissed" => ReviewNoteStatus::Dismissed,
                other => {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Invalid resolution status `{other}`; expected `resolved` or `dismissed`"
                    )));
                }
            },
        };

        let mut sheet = self
            .document
            .project
            .task_sheet
            .clone()
            .ok_or_else(|| DispatchError::Operation("No active task sheet / review notes".into()))?;

        let actor = self.current_actor();
        let now = now_epoch_ms();

        let mut found: Option<(u32, ReviewNote)> = None;
        for round in &mut sheet.rounds {
            if let Some(note) = round.notes.iter_mut().find(|n| n.id == note_id) {
                note.status = target_status;
                note.resolution = Some(resolution.clone());
                note.resolved_by = Some(actor.name().to_string());
                note.resolved_at_unix_ms = Some(now);
                found = Some((round.round_number, note.clone()));
                break;
            }
        }

        let (round_number, updated_note) = found.ok_or_else(|| {
            DispatchError::Operation(format!("Review note `{note_id}` not found"))
        })?;

        let unresolved_count = count_open_notes(&sheet);

        self.checkpoint();
        self.document.project.task_sheet = Some(sheet.clone());

        Ok(json!({
            "resolved": true,
            "note_id": note_id,
            "round_number": round_number,
            "status": note_status_str(target_status),
            "unresolved_count": unresolved_count,
            "note": updated_note,
            "sheet": sheet,
        }))
    }
}
