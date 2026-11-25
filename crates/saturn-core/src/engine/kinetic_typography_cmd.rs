// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for kinetic typography configuration (Packet 6.2).

use super::*;
use crate::kinetic_typography;
use serde::Deserialize;

#[derive(Deserialize)]
struct KineticParams {
    track_id: u64,
    clip_index: usize,
    words: Vec<kinetic_typography::KineticWord>,
    #[serde(default)]
    faces: Vec<kinetic_typography::KineticFaceBox>,
    #[serde(default)]
    style: kinetic_typography::KineticStyle,
}

impl EditorEngine {
    pub(super) fn dispatch_kinetic_typography(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "graphic.kinetic_typography" {
            return None;
        }
        Some((|| {
            let p: KineticParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = kinetic_typography::configure_kinetic_typography(
                &mut next,
                p.track_id,
                p.clip_index,
                p.words,
                p.faces,
                p.style,
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
