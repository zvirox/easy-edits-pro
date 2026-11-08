// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::{
    audio_edit::snapshot_digest,
    match_cut::{MatchCutParams, plan, validate_choice},
};

impl EditorEngine {
    pub(super) fn dispatch_match_cut(
        &mut self,
        id: &str,
        mut params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "edit.match_cut_align" {
            return None;
        }
        Some((|| {
            let invalid = DispatchError::InvalidParameters;
            let object = params
                .as_object_mut()
                .ok_or_else(|| invalid("Expected match-cut parameters".into()))?;
            let expected = object.remove("expected_digest").ok_or_else(|| {
                invalid(
                    "Prepare the alignment with edit_match_cut_align or the desktop menu".into(),
                )
            })?;
            if expected.as_str()
                != Some(&snapshot_digest(&self.document).map_err(DispatchError::Operation)?)
            {
                return Err(invalid(
                    "Project changed during match-cut analysis; rerun alignment".into(),
                ));
            }
            let source_in: Tick = serde_json::from_value(
                object
                    .remove("matched_source_in")
                    .ok_or_else(|| invalid("A measured source position is required".into()))?,
            )
            .map_err(|e| invalid(e.to_string()))?;
            let p: MatchCutParams = parse_params(params)?;
            let plan = plan(&self.document, &p).map_err(invalid)?;
            validate_choice(&plan, p.mode, source_in).map_err(invalid)?;
            let old = plan.right.source_in;
            let index = plan.track_index;
            if old != source_in {
                let mut next = self.document.clone();
                next.project.sequence.tracks[index].clips[p.right_clip_index].source_in = source_in;
                next.validate().map_err(DispatchError::Operation)?;
                self.checkpoint();
                self.document = next;
            }
            Ok(json!({"changed": old != source_in, "source_in": source_in,
                "previous_source_in": old, "slip_ticks": source_in.0 - old.0, "mode": p.mode}))
        })())
    }
}
