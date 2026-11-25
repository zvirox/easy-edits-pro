// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Dispatcher for the bounded fluid morph cut (Packet 5.1).

use super::*;
use crate::fluid_morph;
use serde::Deserialize;

#[derive(Deserialize)]
struct MorphParams {
    track_id: u64,
    cut_at: i64,
    duration: i64,
    measurement: fluid_morph::MeasuredMorph,
    expected_digest: String,
}

impl EditorEngine {
    pub(super) fn dispatch_fluid_morph(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "timeline.fluid_morph_cut" {
            return None;
        }
        Some((|| {
            let p: MorphParams = parse_params(params)?;
            let mut next = self.document.clone();
            let report = fluid_morph::apply_morph(
                &mut next,
                p.track_id,
                Tick(p.cut_at),
                Tick(p.duration),
                &p.measurement,
                &p.expected_digest,
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
