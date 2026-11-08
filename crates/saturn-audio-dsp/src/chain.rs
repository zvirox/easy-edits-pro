// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Host-side plugin chain: load, activate, process, deactivate.
//!
//! The chain consumes interleaved f32 audio. Each render block is
//! de-interleaved into one accumulator buffer per channel, run through every
//! enabled slot's plugin instance in order (slot N's output becomes slot
//! N+1's input), and re-interleaved back into the caller's buffer.
//!
//! Parameter values stored on each slot are re-asserted to the plugin as
//! `ParamValueEvent`s in every block, so a restarted chain converges to the
//! persisted state without stateful flush bookkeeping.

use clack_common::events::event_types::ParamValueEvent;
use clack_common::events::Pckn;
use clack_host::prelude::*;
use clack_host::utils::ClapId;
use saturn_core::project::DspPluginSlot;

use crate::clap_host::{
    activation_config, describe_entry_ids, host_info, load_entry, SaturnHost,
};
use crate::errors::{DspError, DspResult};
use crate::trust::validate_plugin_path;

pub use saturn_core::project::{HOSTED_PLUGIN_FORMATS, MAX_TRACK_SLOTS};

/// Internal chain block size: a chunk the chain itself processes in one
/// process() call per slot. Bounds-kept between the activation contract's
/// min and max so the plugin never sees less than `MIN_BLOCK_FRAMES` or
/// more than `MAX_BLOCK_FRAMES`.
pub const CHAIN_BLOCK_FRAMES: u32 = 512;

/// One loaded, activated, ready-to-process plugin slot.
pub struct ChainSlotExec {
    plugin_id: String,
    // Both are retained deliberately: dropping the entry unloads the
    // .so while the instance may still hold borrowed function pointers.
    #[allow(dead_code, reason = "plugin borrow lifetime anchor")]
    plugin_host_info: HostInfo,
    #[allow(dead_code, reason = "plugin borrow lifetime anchor")]
    plugin_entry: PluginEntry,
    instance: PluginInstance<SaturnHost>,
    /// State machine: None only while Drop deactivates.
    processor: Option<StartedPluginAudioProcessor<SaturnHost>>,
    port_in: AudioPorts,
    port_out: AudioPorts,
    scratch_in: Vec<Vec<f32>>,
    scratch_out: Vec<Vec<f32>>,
    params: Vec<(u64, f64)>,
}

impl Drop for ChainSlotExec {
    fn drop(&mut self) {
        // Ordering is load bearing: the started processor must be stopped
        // and handed back to the instance (deactivate consumes it), or the
        // plugin's activation resources leak.
        if let Some(processor) = self.processor.take() {
            let stopped = processor.stop_processing();
            self.instance.deactivate(stopped);
        }
    }
}

/// A fully materialized plugin chain for one track.
pub struct PluginChain {
    slots: Vec<ChainSlotExec>,
    /// De-interleaved chain-in / chain-out accumulator, indexed by channel.
    accumulator_in: Vec<Vec<f32>>,
    accumulator_out: Vec<Vec<f32>>,
    /// Per-block event scratch (rebuilt, never aliased across blocks).
    param_events: Vec<ParamValueEvent>,
    sample_rate: u32,
    channels: usize,
}

impl PluginChain {
    /// Validates and loads every enabled slot. Disabled slots are skipped
    /// entirely. The chain owns all state it needs to process audio.
    ///
    /// # Errors
    /// Typed [`DspError`]s for untrusted paths, wrong formats, missing
    /// descriptor ids, and failed instantiation/activation.
    pub fn load(
        slots: &[DspPluginSlot],
        sample_rate: u32,
        channels: usize,
    ) -> DspResult<PluginChain> {
        if slots.len() > MAX_TRACK_SLOTS {
            return Err(DspError::SlotInvalid(slots.len()));
        }
        if channels == 0 || channels > 2 {
            return Err(DspError::PluginStartup(
                "chain".to_string(),
                format!("unsupported channel count {channels} (1 or 2 expected)"),
            ));
        }
        if !(8000.0..=192_000.0).contains(&(f64::from(sample_rate))) {
            return Err(DspError::PluginStartup(
                "chain".to_string(),
                format!("unsupported sample rate {sample_rate}"),
            ));
        }

        let mut loaded = Vec::new();
        for slot in slots.iter().filter(|slot| slot.enabled) {
            loaded.push(ChainSlotExec::load(slot, sample_rate, channels)?);
        }

        let mut accumulator_in = Vec::with_capacity(channels);
        let mut accumulator_out = Vec::with_capacity(channels);
        for _ in 0..channels {
            accumulator_in.push(vec![0.0; CHAIN_BLOCK_FRAMES as usize]);
            accumulator_out.push(vec![0.0; CHAIN_BLOCK_FRAMES as usize]);
        }

        Ok(PluginChain {
            slots: loaded,
            accumulator_in,
            accumulator_out,
            param_events: Vec::new(),
            sample_rate,
            channels,
        })
    }

    /// Number of active (instantiated) slots.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Sample rate the chain was activated at.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channel count the chain was activated with.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Maximum frames the chain accepts in one call (the transport layer's
    /// chunk ceiling).
    pub fn max_frames_per_call(&self) -> usize {
        CHAIN_BLOCK_FRAMES as usize * 1024
    }

    /// Processes `block` (interleaved f32, `block.len() == frames * channels`)
    /// through the chain. Empty chains pass the block through untouched.
    ///
    /// Longer inputs are split into [`CHAIN_BLOCK_FRAMES`]-sized internal
    /// blocks; the final partial block is zero-padded for the plugins and
    /// truncated again before it is handed back, so the tail of a render is
    /// not mutated outside its frame range.
    pub fn process_interleaved(&mut self, block: &mut [f32]) -> DspResult<()> {
        if self.slots.is_empty() {
            return Ok(());
        }
        let channels = self.channels;
        debug_assert!(
            block.len() % channels == 0,
            "interleaved block length must be a multiple of the channel count"
        );

        let block_stride = CHAIN_BLOCK_FRAMES as usize * channels;
        let mut offset = 0;
        while offset < block.len() {
            let end = (offset + block_stride).min(block.len());
            let frames = (end - offset) / channels;
            self.process_frames(&mut block[offset..end], frames)?;
            offset = end;
        }
        Ok(())
    }

    /// Processes exactly `frames` frames out of a `chunk` that must have
    /// capacity for `CHAIN_BLOCK_FRAMES` frames worth of interleaved data.
    ///
    /// The chunk is zero-padded on the right during processing and only the
    /// first `frames` frames are written back, so the plugin sees a
    /// full-width block without the caller's tail samples being touched.
    fn process_frames(&mut self, chunk: &mut [f32], frames: usize) -> DspResult<()> {
        let channels = self.channels;
        let _ = CHAIN_BLOCK_FRAMES;

        // 1. De-interleave into the accumulator, zero-pad the tail.
        for (ch, acc) in self.accumulator_in.iter_mut().enumerate() {
            acc.iter_mut().for_each(|slot| *slot = 0.0);
            for frame in 0..frames {
                acc[frame] = chunk[frame * channels + ch];
            }
        }

        // 2. Every active slot processes the full CHAIN_BLOCK_FRAMES block.
        for slot_index in 0..self.slots.len() {
            let events = build_param_events(&mut self.param_events, &self.slots[slot_index].params);
            let mut output_events_mut = OutputEvents::void();

            {
                let slot = &mut self.slots[slot_index];
                let processor = slot.processor.as_mut().expect("processor dropped before chain teardown");
                for (input, source) in slot.scratch_in.iter_mut().zip(&self.accumulator_in) {
                    input.copy_from_slice(source);
                }

                let input_buffers = slot.port_in.with_input_buffers(std::iter::once(
                    AudioPortBuffer {
                        latency: 0,
                        channels: AudioPortBufferType::f32_input_only(
                            slot.scratch_in.iter_mut().map(InputChannel::variable),
                        ),
                    },
                ));
                let mut output_buffers = slot.port_out.with_output_buffers(std::iter::once(
                    AudioPortBuffer {
                        latency: 0,
                        channels: AudioPortBufferType::f32_output_only(
                            slot.scratch_out
                                .iter_mut()
                                .map(|buffer| buffer.as_mut_slice()),
                        ),
                    },
                ));

                processor
                    .process(
                        &input_buffers,
                        &mut output_buffers,
                        &events,
                        &mut output_events_mut,
                        None,
                        None,
                    )
                    .map_err(|err| {
                        DspError::PluginStartup(
                            slot.plugin_id.clone(),
                            format!("process failed: {err}"),
                        )
                    })?;

                // Chain accumulator becomes this slot's output.
                for (sink, produced) in self.accumulator_out.iter_mut().zip(&slot.scratch_out) {
                    sink.copy_from_slice(produced);
                }
            }

            // Hand the output forward as the next slot's input.
            std::mem::swap(&mut self.accumulator_in, &mut self.accumulator_out);
        }

        // 3. Re-interleave the final accumulator output.
        for (ch, acc) in self.accumulator_in.iter().enumerate() {
            for frame in 0..frames {
                chunk[frame * channels + ch] = acc[frame];
            }
        }
        Ok(())
    }
}

/// Builds the per-block `ParamValueEvent` list for a slot. Empty when the
/// slot has no persisted parameter values; an empty `InputEvents` is
/// supplied to the plugin in that case.
fn build_param_events<'a>(
    scratch: &'a mut Vec<ParamValueEvent>,
    params: &[(u64, f64)],
) -> InputEvents<'a> {
    scratch.clear();
    for (param_id, value) in params {
        scratch.push(ParamValueEvent::new(
            0,
            ClapId::new(u32::try_from(*param_id).unwrap_or(u32::MAX)),
            Pckn::match_all(),
            *value,
        ));
    }
    if scratch.is_empty() {
        return InputEvents::empty();
    }
    InputEvents::from_buffer(scratch)
}

impl ChainSlotExec {
    fn load(slot: &DspPluginSlot, sample_rate: u32, channels: usize) -> DspResult<Self> {
        if let Some(first_bad) = slot.params.iter().find(|param| !(0.0..=1.0).contains(&param.value)) {
            return Err(DspError::ParameterInvalid(
                first_bad.param_id,
                format!(
                    "value {} outside the stored range 0.0..=1.0",
                    first_bad.value
                ),
            ));
        }
        if !slot.format.is_hosted() {
            return Err(DspError::FormatNotHosted(slot.format.as_tag().to_string()));
        }
        let path = validate_plugin_path(&slot.path)?;
        if slot.plugin_id.is_empty() {
            return Err(DspError::PluginUnloadable(
                slot.path.clone(),
                "plugin_id must be non-empty".to_string(),
            ));
        }

        let entry = load_entry(&path)?;
        let source_display = path.display().to_string();
        // The descriptor id is checked against what the binary actually
        // exposes, so mismatches fail as typed errors before instantiation.
        let ids = describe_entry_ids(&entry, &source_display)?;
        if !ids.iter().any(|id| id == &slot.plugin_id) {
            return Err(DspError::PluginUnloadable(
                source_display,
                format!(
                    "descriptor id {:?} not found; binary exposes {:?}",
                    slot.plugin_id, ids
                ),
            ));
        }

        let host_info = host_info();
        let plugin_id = std::ffi::CString::new(slot.plugin_id.as_str()).map_err(|_| {
            DspError::PluginUnloadable(source_display, "NUL byte in plugin id".to_string())
        })?;
        let instance = PluginInstance::<SaturnHost>::new(
            |_| crate::clap_host::SaturnShared::new(),
            |_shared| crate::clap_host::SaturnMainThread::new(),
            &entry,
            plugin_id.as_c_str(),
            &host_info,
        )
        .map_err(|err| {
            DspError::PluginStartup(
                slot.plugin_id.clone(),
                format!("instantiation failed: {err}"),
            )
        })?;

        let mut instance = instance;
        let stopped = instance
            .activate(
                |_, _| crate::clap_host::SaturnAudioProcessor::new(),
                activation_config(sample_rate),
            )
            .map_err(|err| {
                DspError::PluginStartup(
                    slot.plugin_id.clone(),
                    format!("activation failed: {err}"),
                )
            })?;

        let processor = stopped.start_processing().map_err(|err| {
            DspError::PluginStartup(slot.plugin_id.clone(), format!("activate: {err}"))
        })?;

        let mut scratch_in = Vec::with_capacity(channels);
        let mut scratch_out = Vec::with_capacity(channels);
        // Port channels must be exactly CHAIN_BLOCK_FRAMES long: the host
        // derives each block's frames_count from the channel-buffer lengths,
        // and the activation contract requires every block to be full-width.
        for _ in 0..channels {
            scratch_in.push(vec![0.0; CHAIN_BLOCK_FRAMES as usize]);
            scratch_out.push(vec![0.0; CHAIN_BLOCK_FRAMES as usize]);
        }

        Ok(Self {
            plugin_id: slot.plugin_id.clone(),
            plugin_host_info: host_info,
            plugin_entry: entry,
            instance,
            processor: Some(processor),
            port_in: AudioPorts::with_capacity(channels, 1),
            port_out: AudioPorts::with_capacity(channels, 1),
            scratch_in,
            scratch_out,
            params: slot
                .params
                .iter()
                .map(|param| (param.param_id, param.value))
                .collect(),
        })
    }
}
