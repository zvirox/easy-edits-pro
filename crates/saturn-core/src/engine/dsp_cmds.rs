// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

use crate::project::{
    validate_plugin_path, DspPluginFormat, DspPluginSlot, DspPluginSlotParam, HOSTED_PLUGIN_FORMATS,
    MAX_TRACK_SLOTS,
};

impl EditorEngine {
    /// Track DSP plugin-chain commands. The records mutated here are the
    /// persisted source of truth; the render engine re-derives the running
    /// plugin chain from them every render, so no runtime state is forked.
    pub(super) fn dispatch_dsp(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "dsp.load_plugin" => Some((|| {
                let params: LoadDspPluginParams = parse_params(params)?;

                let format = DspPluginFormat::from_tag(&params.format);
                if !format.is_hosted() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "plugin format `{}` is not hosted (hosted formats: {}) — \
                         the slot is not stored; use format \"clap\"",
                        params.format,
                        HOSTED_PLUGIN_FORMATS.join(", ")
                    )));
                }
                if params.plugin_id.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "plugin_id must be a non-empty stable descriptor id \
                         (e.g. \"org.eep.fixture.gain\")"
                            .into(),
                    ));
                }
                if let Err(err) = validate_plugin_path(&params.path) {
                    return Err(DispatchError::InvalidParameters(err.to_string()));
                }
                if let Some(slot_params) = &params.params {
                    for slot_param in slot_params {
                        if !slot_param.value.is_finite()
                            || !(0.0..=1.0).contains(&slot_param.value)
                        {
                            return Err(DispatchError::InvalidParameters(format!(
                                "parameter {} value {} is outside the stored plain range 0.0..=1.0",
                                slot_param.param_id, slot_param.value
                            )));
                        }
                    }
                }

                let track_index = find_track_index(&self.document, params.track_id)?;
                let chain_len =
                    self.document.project.sequence.tracks[track_index].dsp_plugin_chain.len();
                if chain_len >= MAX_TRACK_SLOTS {
                    return Err(DispatchError::InvalidParameters(format!(
                        "track already carries {chain_len} plugin slots (limit {MAX_TRACK_SLOTS})"
                    )));
                }

                self.checkpoint();
                let slot = DspPluginSlot {
                    format,
                    path: params.path,
                    plugin_id: params.plugin_id,
                    enabled: params.enabled.unwrap_or(true),
                    params: params
                        .params
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| DspPluginSlotParam {
                            param_id: p.param_id,
                            value: p.value,
                        })
                        .collect(),
                };
                let slot_index = chain_len;
                self.document.project.sequence.tracks[track_index]
                    .dsp_plugin_chain
                    .push(slot);
                Ok(json!({
                    "added": true,
                    "slot_index": slot_index,
                    "hosted": format.is_hosted(),
                }))
            })()),
            "dsp.set_plugin_param" => Some((|| {
                let params: SetDspPluginParamParams = parse_params(params)?;
                if !params.value.is_finite() || !(0.0..=1.0).contains(&params.value) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "parameter value {} is outside the stored plain range 0.0..=1.0",
                        params.value
                    )));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                {
                    let chain = &self.document.project.sequence.tracks[track_index]
                        .dsp_plugin_chain;
                    let slot_count = chain.len();
                    let in_range = chain.get(params.slot_index).is_some();
                    if !in_range {
                        return Err(DispatchError::InvalidParameters(format!(
                            "plugin slot index {} is out of range (track carries {slot_count} slots)",
                            params.slot_index
                        )));
                    }
                }

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                let slot = &mut track.dsp_plugin_chain[params.slot_index];
                let value = params.value;
                if let Some(existing) = slot
                    .params
                    .iter_mut()
                    .find(|p| p.param_id == params.param_id)
                {
                    existing.value = value;
                } else {
                    slot.params.push(DspPluginSlotParam {
                        param_id: params.param_id,
                        value,
                    });
                }
                Ok(json!({
                    "updated": true,
                    "slot_index": params.slot_index,
                    "param_id": params.param_id,
                    "value": value,
                }))
            })()),
            _ => None,
        }
    }
}
