// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for object removal settings (Packet 6.1).

use super::*;
use crate::inpaint_object;
use serde::Deserialize;

#[derive(Deserialize)]
struct InpaintParams {
    track_id: u64,
    clip_index: usize,
    region: [f64; 4],
    search_frames: u32,
}

impl EditorEngine {
    pub(super) fn dispatch_inpaint(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "vfx.inpaint_object_removal" {
            return None;
        }
        Some((|| {
            let p: InpaintParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = inpaint_object::configure_inpaint(
                &mut next,
                p.track_id,
                p.clip_index,
                p.region,
                p.search_frames,
            )
            .map_err(DispatchError::InvalidParameters)?;
            next.validate().map_err(DispatchError::Operation)?;
            self.checkpoint();
            self.document = next;
            Ok(serde_json::to_value(report)
                .map_err(|error| DispatchError::Operation(error.to_string()))?)
        })())
    }
}
