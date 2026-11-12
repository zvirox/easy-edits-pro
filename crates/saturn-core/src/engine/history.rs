// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

impl EditorEngine {
    pub(super) fn checkpoint(&mut self) {
        if self.gesture_depth > 0 {
            if self.gesture_checkpointed {
                return;
            }
            self.gesture_checkpointed = true;
        }
        self.undo.push(self.document.clone());
        let label = self
            .pending_label
            .take()
            .unwrap_or_else(|| "unknown".into());
        self.undo_labels.push(label);
        if self.undo.len() > 100 {
            self.undo.remove(0);
            self.undo_labels.remove(0);
        }
        self.redo.clear();
        self.redo_labels.clear();
    }

    pub(super) fn dispatch_history(
        &mut self,
        id: &str,
        _params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "edit.begin_gesture" => Some((|| {
                self.begin_gesture();
                Ok(json!({"gesture": true, "depth": self.gesture_depth}))
            })()),
            "edit.end_gesture" => Some((|| {
                self.end_gesture();
                Ok(json!({"gesture": self.in_gesture(), "depth": self.gesture_depth}))
            })()),
            "edit.undo" => Some((|| {
                let previous = self.undo.pop().expect("availability checked");
                let label = self.undo_labels.pop().unwrap_or_default();
                self.redo.push(self.document.clone());
                self.redo_labels.push(label);
                self.document = previous;
                Ok(json!({"undone": true}))
            })()),
            "edit.redo" => Some((|| {
                let next = self.redo.pop().expect("availability checked");
                let label = self.redo_labels.pop().unwrap_or_default();
                self.undo.push(self.document.clone());
                self.undo_labels.push(label);
                self.document = next;
                Ok(json!({"redone": true}))
            })()),
            // Query: the undo/redo stacks with entry labels, newest first.
            "history.list" => Some((|| {
                let undo: Vec<Value> = self
                    .undo_labels
                    .iter()
                    .rev()
                    .map(|label| json!({"label": label}))
                    .collect();
                let redo: Vec<Value> = self
                    .redo_labels
                    .iter()
                    .rev()
                    .map(|label| json!({"label": label}))
                    .collect();
                Ok(json!({
                    "undo": undo,
                    "redo": redo,
                    "current": self.document.project.name,
                }))
            })()),
            "history.undo_change" => Some((|| {
                let change_id = _params.get("change_id").and_then(Value::as_u64).ok_or_else(|| {
                    DispatchError::InvalidParameters("history.undo_change requires `change_id`".into())
                })?;
                let ctx = self.current_context().cloned().unwrap_or_default();
                self.undo_change(change_id, &ctx)
            })()),
            _ => None,
        }
    }

    pub fn undo_change(&mut self, change_id: u64, ctx: &ActorContext) -> Result<Value, DispatchError> {
        self.check_actor_context(ctx)?;
        let record = self
            .journal
            .check_undo_allowed(change_id)
            .map_err(DispatchError::Operation)?
            .clone();

        for entity_id in &record.affected_entity_ids {
            self.document
                .locks
                .check_clip_mutation(*entity_id, &ctx.actor)
                .map_err(DispatchError::Operation)?;
        }

        let inverse_val = record.inverse_state.ok_or_else(|| {
            DispatchError::Operation(format!("Change {change_id} has no inverse state"))
        })?;

        let restored_doc: ProjectDocument = serde_json::from_value(inverse_val)
            .map_err(|e| DispatchError::Operation(format!("Failed to deserialize inverse state: {e}")))?;

        self.checkpoint();
        let active_branch = self.active_branch_name();
        if record.branch != active_branch {
            if let Some(branch_rec) = self
                .document
                .branches
                .iter_mut()
                .find(|b| b.name == record.branch)
            {
                let mut parked: crate::project::Project =
                    serde_json::from_value(branch_rec.state.clone()).map_err(|e| {
                        DispatchError::Operation(format!("Failed to read branch state: {e}"))
                    })?;
                if !Self::apply_selective_clip_restore(
                    &mut parked,
                    &restored_doc.project,
                    &record.affected_entity_ids,
                ) {
                    parked = restored_doc.project;
                }
                branch_rec.state = serde_json::to_value(&parked).map_err(|e| {
                    DispatchError::Operation(format!("Failed to encode branch state: {e}"))
                })?;
            } else {
                return Err(DispatchError::Operation(format!(
                    "Cannot undo change {change_id}: branch '{}' is no longer present",
                    record.branch
                )));
            }
        } else if !Self::apply_selective_clip_restore(
            &mut self.document.project,
            &restored_doc.project,
            &record.affected_entity_ids,
        ) {
            self.document = restored_doc;
        }

        let undo_change_id = self.journal.record_change(
            0,
            ctx.actor.clone(),
            &ctx.session_id,
            &record.branch,
            format!("undo:{}", record.command_id),
            record.affected_entity_ids.clone(),
            None,
        );

        Ok(json!({
            "undone_change_id": change_id,
            "undo_change_id": undo_change_id,
            "command_id": record.command_id,
            "affected_entities": record.affected_entity_ids,
        }))
    }

    /// Compare clip snapshots before and after a mutating command so newly added,
    /// removed, or edited clips are tracked for conflict-safe selective undo.
    pub(super) fn collect_changed_entities(
        &self,
        mut affected: Vec<u64>,
        before_doc: Option<&ProjectDocument>,
    ) -> Vec<u64> {
        let Some(before) = before_doc else {
            return affected;
        };

        use std::collections::HashMap;
        let mut before_clips: HashMap<u64, Value> = HashMap::new();
        for track in &before.project.sequence.tracks {
            for clip in &track.clips {
                if let Ok(val) = serde_json::to_value(clip) {
                    before_clips.insert(clip.id, val);
                }
            }
        }

        let mut after_clips: HashMap<u64, Value> = HashMap::new();
        for track in &self.document.project.sequence.tracks {
            for clip in &track.clips {
                if let Ok(val) = serde_json::to_value(clip) {
                    after_clips.insert(clip.id, val);
                }
            }
        }

        for (cid, after_val) in &after_clips {
            let changed = match before_clips.get(cid) {
                Some(before_val) => before_val != after_val,
                None => true,
            };
            if changed && !affected.contains(cid) {
                affected.push(*cid);
            }
        }
        for cid in before_clips.keys() {
            if !after_clips.contains_key(cid) && !affected.contains(cid) {
                affected.push(*cid);
            }
        }
        affected
    }

    pub(super) fn apply_selective_clip_restore(
        target: &mut crate::project::Project,
        restored: &crate::project::Project,
        affected_entity_ids: &[u64],
    ) -> bool {
        if affected_entity_ids.is_empty() {
            return false;
        }
        let target_has_any = target
            .sequence
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .any(|c| affected_entity_ids.contains(&c.id));
        let restored_has_any = restored
            .sequence
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .any(|c| affected_entity_ids.contains(&c.id));
        if !target_has_any && !restored_has_any {
            return false;
        }

        for track in &mut target.sequence.tracks {
            track.clips.retain(|c| !affected_entity_ids.contains(&c.id));
        }
        for rtrack in &restored.sequence.tracks {
            let clips_to_restore: Vec<_> = rtrack
                .clips
                .iter()
                .filter(|c| affected_entity_ids.contains(&c.id))
                .cloned()
                .collect();
            if clips_to_restore.is_empty() {
                continue;
            }
            if let Some(ttrack) = target.sequence.tracks.iter_mut().find(|t| t.id == rtrack.id) {
                ttrack.clips.extend(clips_to_restore);
                ttrack.clips.sort_by_key(|c| c.timeline_start.0);
            } else {
                let mut cloned_track = rtrack.clone();
                cloned_track.clips = clips_to_restore;
                cloned_track.clips.sort_by_key(|c| c.timeline_start.0);
                target.sequence.tracks.push(cloned_track);
            }
        }
        true
    }

    pub(super) fn handle_review_list_changes(&self, params: Value) -> Result<Value, DispatchError> {
        let branch_filter = params
            .get("branch")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let actor_filter = params
            .get("actor")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let session_filter = params
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let parked_projects: std::collections::HashMap<&str, crate::project::Project> = self
            .document
            .branches
            .iter()
            .filter_map(|b| {
                serde_json::from_value::<crate::project::Project>(b.state.clone())
                    .ok()
                    .map(|proj| (b.name.as_str(), proj))
            })
            .collect();

        let active_branch = self.active_branch_name();
        let records = &self.journal.records;

        let mut changes = Vec::new();
        for (idx, record) in records.iter().enumerate() {
            if matches!(
                record.command_id.as_str(),
                "project.branch_switch" | "project.branch_create"
            ) {
                continue;
            }
            if let Some(ref bf) = branch_filter {
                if &record.branch != bf {
                    continue;
                }
            }
            if let Some(ref sf) = session_filter {
                if &record.session_id != sf {
                    continue;
                }
            }
            if let Some(ref af) = actor_filter {
                let display = record.actor.to_string();
                let name = record.actor.name();
                if &display != af && name != af {
                    continue;
                }
            }

            let tier = crate::find_command(&record.command_id)
                .map(|c| c.tier.as_str())
                .unwrap_or("edit");

            let branch_project = if record.branch == active_branch {
                Some(&self.document.project)
            } else {
                parked_projects.get(record.branch.as_str())
            };
            let inverse_project = record
                .inverse_state
                .as_ref()
                .and_then(|v| serde_json::from_value::<crate::project::ProjectDocument>(v.clone()).ok())
                .map(|d| d.project);

            let tick_range = compute_change_tick_range(
                &record.affected_entity_ids,
                branch_project,
                inverse_project.as_ref(),
            );

            let undone = records[idx + 1..].iter().any(|later| {
                later.branch == record.branch
                    && later.command_id.starts_with("undo:")
                    && record
                        .affected_entity_ids
                        .iter()
                        .any(|eid| later.affected_entity_ids.contains(eid))
            });

            let conflict_err = self.journal.check_undo_allowed(record.change_id).err();
            let conflict = !undone && conflict_err.is_some();
            let conflict_reason = if undone { None } else { conflict_err };

            changes.push(json!({
                "change_id": record.change_id,
                "transaction_id": record.transaction_id,
                "actor": record.actor.to_string(),
                "actor_name": record.actor.name(),
                "is_human": record.actor.is_human(),
                "session_id": record.session_id,
                "timestamp_unix_ms": record.timestamp_unix_ms,
                "branch": record.branch,
                "command_id": record.command_id,
                "tier": tier,
                "tick_range": [tick_range.0, tick_range.1],
                "affected_entity_ids": record.affected_entity_ids,
                "rationale": record.rationale,
                "confidence": record.confidence,
                "conflict": conflict,
                "conflict_reason": conflict_reason,
                "undone": undone,
                "correction_of": record.correction_of,
            }));
        }

        changes.reverse();
        Ok(json!({
            "branch": branch_filter.unwrap_or(active_branch),
            "changes": changes,
            "count": changes.len(),
        }))
    }
}

fn compute_change_tick_range(
    entity_ids: &[u64],
    live_project: Option<&crate::project::Project>,
    inverse_project: Option<&crate::project::Project>,
) -> (i64, i64) {
    if entity_ids.is_empty() {
        return (0, 0);
    }
    let mut min_tick: Option<i64> = None;
    let mut max_tick: Option<i64> = None;

    for proj in [live_project, inverse_project].into_iter().flatten() {
        for track in &proj.sequence.tracks {
            for clip in &track.clips {
                if entity_ids.contains(&clip.id) {
                    let start = clip.timeline_start.0;
                    let end = start.saturating_add(clip.duration.0);
                    min_tick = Some(min_tick.map_or(start, |m| m.min(start)));
                    max_tick = Some(max_tick.map_or(end, |m| m.max(end)));
                }
            }
        }
        if min_tick.is_some() {
            break;
        }
    }

    match (min_tick, max_tick) {
        (Some(s), Some(e)) => (s, e),
        _ => (0, 0),
    }
}
