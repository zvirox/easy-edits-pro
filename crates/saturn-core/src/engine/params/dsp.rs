// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct DspSlotParamInput {
    pub(in crate::engine) param_id: u64,
    pub(in crate::engine) value: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct LoadDspPluginParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) format: String,
    pub(in crate::engine) path: String,
    pub(in crate::engine) plugin_id: String,
    pub(in crate::engine) enabled: Option<bool>,
    pub(in crate::engine) params: Option<Vec<DspSlotParamInput>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetDspPluginParamParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) slot_index: usize,
    pub(in crate::engine) param_id: u64,
    pub(in crate::engine) value: f64,
}
