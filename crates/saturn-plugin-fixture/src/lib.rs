// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Reference CLAP plugin used to drive Saturn's DSP host tests.
//!
//! Contract (stability matters to the parity gate):
//! * Plugin id: `org.eep.fixture.gain`
//! * Exactly one parameter, id 0, range `0.0..=1.0`.
//! * Playback gain applied to every sample: `value * 2.0`
//!   (0.0 mutes, 0.5 is unity, 1.0 doubles the signal / +6.02 dB).

use clack_common::events::event_types::ParamValueEvent;
use clack_common::plugin::PluginDescriptor;
use clack_extensions::params::{
    ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginAudioProcessorParams,
    PluginMainThreadParams, PluginParams,
};
use clack_plugin::entry::{DefaultPluginFactory, SinglePluginEntry};
use clack_plugin::plugin::{Plugin, PluginAudioProcessor};
use clack_plugin::prelude::*;
use clack_plugin::process::{Audio, Process, ProcessStatus};
use std::fmt::Write as _;

pub struct SaturnFixtureGain;

pub struct FixtureMainThread {
    value: f64,
}

pub struct FixtureAudioProcessor {
    gain: f32,
}

impl<'a> PluginMainThread<'a, ()> for FixtureMainThread {
    fn on_main_thread(&self) {}
}

// Plugin-parameter extension on the main thread.
impl PluginMainThreadParams for FixtureMainThread {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, writer: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }
        writer.set(&ParamInfo {
            id: clack_common::utils::ClapId::new(0),
            flags: ParamInfoFlags::IS_AUTOMATABLE,
            cookie: clack_common::utils::Cookie::empty(),
            name: b"Gain",
            module: b"",
            min_value: 0.0,
            max_value: 1.0,
            default_value: 0.5,
        });
    }

    fn get_value(&self, param_id: ClapId) -> Option<f64> {
        (param_id.get() == 0).then_some(self.value)
    }

    fn value_to_text(
        &self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        if param_id.get() != 0 {
            return Err(core::fmt::Error);
        }
        write!(writer, "{:.2} (x{:.2})", value, value * 2.0)
    }

    fn text_to_value(&self, param_id: ClapId, text: &std::ffi::CStr) -> Option<f64> {
        if param_id.get() != 0 {
            return None;
        }
        text.to_string_lossy().trim().parse::<f64>().ok()
    }

    fn flush(
        &self,
        _input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
    }
}

// Audio-thread parameter flush (plugin is not processing here).
impl PluginAudioProcessorParams for FixtureAudioProcessor {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        for event in input_parameter_changes {
            if let Some(param) = event.as_event::<ParamValueEvent>() {
                if param.param_id().map(|id| id.get()) == Some(0) {
                    self.gain = (param.value() * 2.0) as f32;
                }
            }
        }
    }
}

impl<'a> PluginAudioProcessor<'a, (), FixtureMainThread> for FixtureAudioProcessor {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        main_thread: &FixtureMainThread,
        _shared: &'a (),
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(FixtureAudioProcessor {
            gain: (main_thread.value * 2.0) as f32,
        })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: clack_plugin::process::Events,
    ) -> Result<ProcessStatus, PluginError> {
        for event in events.input {
            if let Some(param) = event.as_event::<ParamValueEvent>() {
                if param.param_id().map(|id| id.get()) == Some(0) {
                    self.gain = (param.value() * 2.0) as f32;
                }
            }
        }

        for mut port_pair in &mut audio {
            let Ok(channel_pairs) = port_pair.channels() else {
                continue;
            };
            let Some(channel_pairs) = channel_pairs.into_f32() else {
                continue;
            };
            for channel_pair in channel_pairs {
                match channel_pair {
                    ChannelPair::InputOnly(_) => {}
                    ChannelPair::OutputOnly(buf) => buf.fill(0.0),
                    ChannelPair::InputOutput(input, output) => {
                        for (input, output) in input.iter().zip(output) {
                            *output = input * self.gain;
                        }
                    }
                    ChannelPair::InPlace(buf) => {
                        for sample in buf {
                            *sample *= self.gain;
                        }
                    }
                }
            }
        }

        Ok(ProcessStatus::Continue)
    }
}

impl Plugin for SaturnFixtureGain {
    type AudioProcessor<'a> = FixtureAudioProcessor;
    type Shared<'a> = ();
    type MainThread<'a> = FixtureMainThread;

    fn declare_extensions(
        builder: &mut clack_plugin::extensions::PluginExtensions<Self>,
        _shared: Option<&Self::Shared<'_>>,
    ) {
        builder.register::<PluginParams>();
    }
}

impl DefaultPluginFactory for SaturnFixtureGain {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.eep.fixture.gain", "Saturn Fixture Gain")
            .with_version("1.0.0")
            .with_vendor("Saturn")
    }

    fn new_shared<'a>(
        _host: HostSharedHandle<'a>,
    ) -> Result<Self::Shared<'a>, PluginError> {
        Ok(())
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        _shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(FixtureMainThread { value: 0.5 })
    }
}

clack_export_entry!(SinglePluginEntry::<SaturnFixtureGain>);
