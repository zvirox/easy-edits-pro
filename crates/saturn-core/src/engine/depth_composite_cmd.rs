// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for depth compositor configuration (Packet 6.1).

use super::*;
use crate::depth_composite;
use serde::Deserialize;

#[derive(Deserialize)]
struct DepthCompositeParams {
    track_id: u64,
    clip_index: usize,
    mode: depth_composite::DepthCompositeMode,
    threshold: f32,
    #[serde(default)]
    blur_radius: u32,
}

impl EditorEngine {
    pub(super) fn dispatch_depth_composite(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "vfx.depth_map_compositor" {
            return None;
        }
        Some((|| {
            let p: DepthCompositeParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = depth_composite::configure_depth_composite(
                &mut next,
                p.track_id,
                p.clip_index,
                p.mode,
                p.threshold,
                p.blur_radius,
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
