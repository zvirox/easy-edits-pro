// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for the magnetic storyline pin command (Packet 5.1).

use super::*;
use crate::storyline_pins;
use serde::Deserialize;

#[derive(Deserialize)]
struct PinParams {
    child_track_id: u64,
    child_clip_index: usize,
    parent_track_id: u64,
    parent_clip_index: usize,
}

impl EditorEngine {
    pub(super) fn dispatch_storyline_pin(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "timeline.pin_clip_to_storyline" {
            return None;
        }
        Some((|| {
            let p: PinParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = storyline_pins::pin_clip(
                &mut next,
                p.child_track_id,
                p.child_clip_index,
                p.parent_track_id,
                p.parent_clip_index,
            )
            .map_err(DispatchError::InvalidParameters)?;
            next.validate().map_err(DispatchError::Operation)?;
            self.checkpoint();
            self.document = next;
            Ok(serde_json::to_value(report).map_err(|e| DispatchError::Operation(e.to_string()))?)
        })())
    }
}
