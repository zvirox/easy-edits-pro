// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::actor::ActorId;
use crate::brief::{BriefSource, ClipAgentNote};
use serde_json::{json, Value};

pub(super) fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl EditorEngine {
    pub(super) fn set_clip_agent_note(&mut self, clip_id: u64, note: ClipAgentNote) {
        for track in &mut self.document.project.sequence.tracks {
            for clip in &mut track.clips {
                if clip.id == clip_id {
                    clip.agent_note = Some(note.clone());
                }
            }
        }
    }

    pub(super) fn record_human_correction_on_clip(
        &mut self,
        clip_id: u64,
        human: ActorId,
    ) -> Option<u64> {
        let mut prev_change_id = None;
        for track in &mut self.document.project.sequence.tracks {
            for clip in &mut track.clips {
                if clip.id == clip_id {
                    if let Some(ref mut note) = clip.agent_note {
                        if note.active {
                            prev_change_id = Some(note.change_id);
                            note.record_correction(human.clone(), now_epoch_ms(), "Human modification");
                        }
                    }
                }
            }
        }
        prev_change_id
    }

    pub(super) fn record_mutating_change(
        &mut self,
        command_id: &str,
        affected: Vec<u64>,
        before_doc: Option<ProjectDocument>,
        rationale: Option<String>,
        confidence: Option<crate::brief::ConfidenceLevel>,
    ) {
        let mut correction_of = None;
        let current_actor = self.current_actor();

        if !affected.is_empty() {
            if !current_actor.is_human() {
                let note = ClipAgentNote {
                    rationale: rationale.clone().unwrap_or_default(),
                    confidence: confidence.unwrap_or(crate::brief::ConfidenceLevel::Medium),
                    actor: current_actor.clone(),
                    session_id: self.current_session_id(),
                    created_at_unix_ms: now_epoch_ms(),
                    change_id: self.journal.next_change_id,
                    active: true,
                    correction: None,
                };
                for cid in &affected {
                    self.set_clip_agent_note(*cid, note.clone());
                }
            } else {
                for cid in &affected {
                    if let Some(prev) =
                        self.record_human_correction_on_clip(*cid, current_actor.clone())
                    {
                        correction_of = Some(prev);
                    }
                }
            }
        }

        if let Some(before) = before_doc {
            let inverse = serde_json::to_value(&before).ok();
            self.journal.record_change_with_metadata(
                0,
                current_actor,
                self.current_session_id(),
                self.current_branch(),
                command_id,
                affected,
                inverse,
                rationale,
                confidence,
                correction_of,
            );
        }
    }

    pub(super) fn dispatch_brief(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.brief_get" => Some((|| {
                serde_json::to_value(&self.document.project.brief)
                    .map_err(|e| DispatchError::Operation(e.to_string()))
            })()),
            "project.brief_set" => Some((|| {
                let actor = self.current_actor();
                if !actor.is_human() {
                    return Err(DispatchError::Operation(
                        "PermissionDenied: project.brief_set requires human authorization".into(),
                    ));
                }

                self.checkpoint();

                let brief = &mut self.document.project.brief;
                if let Some(goal) = params.get("goal").and_then(Value::as_str) {
                    brief.goal = goal.to_string();
                }
                if let Some(platform) = params.get("platform") {
                    brief.platform = platform.as_str().map(str::to_string);
                }
                if let Some(target_dur) = params.get("target_duration_ticks") {
                    brief.target_duration_ticks = target_dur.as_u64();
                }
                if let Some(tol) = params.get("duration_tolerance_ticks") {
                    brief.duration_tolerance_ticks = tol.as_u64();
                }
                if let Some(must_keep) = params.get("must_keep_clip_ids").and_then(Value::as_array)
                {
                    brief.must_keep_clip_ids =
                        must_keep.iter().filter_map(Value::as_u64).collect();
                }
                if let Some(banned) = params.get("banned_content").and_then(Value::as_array) {
                    brief.banned_content = banned
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                }
                if let Some(style) = params.get("style_notes").and_then(Value::as_str) {
                    brief.style_notes = style.to_string();
                }
                if let Some(loudness) = params.get("loudness_target_lufs") {
                    brief.loudness_target_lufs = loudness.as_f64();
                }
                if let Some(src) = params.get("source").and_then(Value::as_str) {
                    brief.source = match src {
                        "imported_excerpt" => BriefSource::ImportedExcerpt,
                        "preset" => BriefSource::Preset,
                        _ => BriefSource::UserPrompt,
                    };
                }

                brief.updated_at_unix_ms = now_epoch_ms();
                brief.updated_by = actor;

                let brief_val = serde_json::to_value(&*brief)
                    .map_err(|e| DispatchError::Operation(e.to_string()))?;

                Ok(json!({
                    "updated": true,
                    "brief": brief_val,
                }))
            })()),
            _ => None,
        }
    }
}
