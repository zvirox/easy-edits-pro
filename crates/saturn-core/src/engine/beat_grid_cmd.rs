// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::{
    audio_edit::snapshot_digest,
    beat_grid::{BeatGridParams, MeasuredBeatGrid, quantized_clips},
};

impl EditorEngine {
    pub(super) fn dispatch_beat_grid(
        &mut self,
        id: &str,
        mut params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "timeline.beat_grid_quantize" {
            return None;
        }
        Some((|| {
            let invalid = DispatchError::InvalidParameters;
            let object = params
                .as_object_mut()
                .ok_or_else(|| invalid("Expected beat-grid parameters".into()))?;
            let expected = object.remove("expected_digest").ok_or_else(|| {
                invalid("Prepare with timeline_beat_grid_quantize or the desktop menu".into())
            })?;
            if expected.as_str()
                != Some(&snapshot_digest(&self.document).map_err(DispatchError::Operation)?)
            {
                return Err(invalid(
                    "Project changed during beat analysis; rerun quantization".into(),
                ));
            }
            let grid: MeasuredBeatGrid = serde_json::from_value(
                object
                    .remove("grid")
                    .ok_or_else(|| invalid("A measured beat grid is required".into()))?,
            )
            .map_err(|e| invalid(e.to_string()))?;
            let p: BeatGridParams = parse_params(params)?;
            let (index, clips, cuts) =
                quantized_clips(&self.document, &p, &grid).map_err(invalid)?;
            let changed = cuts.iter().filter(|(a, b)| a != b).count();
            if changed > 0 {
                let mut next = self.document.clone();
                next.project.sequence.tracks[index].clips = clips;
                next.validate().map_err(DispatchError::Operation)?;
                self.checkpoint();
                self.document = next;
            }
            Ok(
                json!({"changed_cuts":changed,"cuts":cuts.iter().map(|(a,b)|json!({"previous_tick":a,"tick":b,"delta_ticks":b.0-a.0})).collect::<Vec<_>>(),"subdivision":p.subdivision,"strength":p.strength}),
            )
        })())
    }
}
