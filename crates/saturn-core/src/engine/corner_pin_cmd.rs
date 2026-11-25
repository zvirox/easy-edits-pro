// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for planar corner pins (Packet 6.1).

use super::*;
use crate::corner_pin;
use serde::Deserialize;

#[derive(Deserialize)]
struct CornerPinParams {
    track_id: u64,
    clip_index: usize,
    corners: [[f64; 2]; 4],
    #[serde(default)]
    motion_blur: bool,
}

impl EditorEngine {
    pub(super) fn dispatch_corner_pin(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "vfx.planar_corner_pin" {
            return None;
        }
        Some((|| {
            let p: CornerPinParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = corner_pin::pin_corners(
                &mut next,
                p.track_id,
                p.clip_index,
                p.corners,
                p.motion_blur,
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
