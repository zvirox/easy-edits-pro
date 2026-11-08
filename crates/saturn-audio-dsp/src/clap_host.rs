// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Saturn's host-side callback handlers for plugin instances.
//!
//! The handlers are deliberately inert: plugins are hosted inside the
//! dedicated `saturn-plugin-worker` sandbox process, so no plugin callback
//! may reach the renderer or any UI surface.

use clack_host::prelude::*;
use crate::errors::DspError;

/// Metadata presented to plugins. Registering this string is honest for the
/// renderer path, which is where plugin chains execute (through the worker).
pub const HOST_NAME: &str = "Easy Edits Pro";
pub const HOST_VENDOR: &str = "Easy Edits Pro authors";
pub const HOST_URL: &str = "";
pub const HOST_VERSION: &str = "0.1.0";

/// The `[thread-safe]` handler. Implements nothing beyond no-op request
/// callbacks: plugins may not influence scheduling.
pub struct SaturnShared<'a> {
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> SaturnShared<'a> {
    pub(crate) fn new() -> Self {
        Self {
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a> SharedHandler<'a> for SaturnShared<'a> {
    fn request_restart(&self) {
        // Deliberately ignored: the worker keeps a fixed schedule, and a
        // plugin cannot restart it.
    }

    fn request_process(&self) {
        // Deliberately ignored: the worker's schedule never changes on a
        // plugin's request.
    }

    fn request_callback(&self) {
        // Deliberately ignored: the worker has no plugin-callback loop.
    }
}

/// The `[main-thread]` handler. The worker only uses the main thread for
/// setup and teardown, so there is nothing for the plugin to call back into.
#[derive(Default)]
pub struct SaturnMainThread<'a> {
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> SaturnMainThread<'a> {
    pub(crate) fn new() -> Self {
        Self {
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a> MainThreadHandler<'a> for SaturnMainThread<'a> {}

/// The `[audio-thread]` handler. We run callbacks synchronously, so there
/// is no cross-thread state to share with the plugin.
#[derive(Default)]
pub struct SaturnAudioProcessor<'a> {
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> SaturnAudioProcessor<'a> {
    pub(crate) fn new() -> Self {
        Self {
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a> AudioProcessorHandler<'a> for SaturnAudioProcessor<'a> {}

/// The host-handler marker type used when instantiating a plugin.
pub struct SaturnHost;

impl HostHandlers for SaturnHost {
    type Shared<'a> = SaturnShared<'a>;
    type MainThread<'a> = SaturnMainThread<'a>;
    type AudioProcessor<'a> = SaturnAudioProcessor<'a>;
}

/// Builds the [`HostInfo`] passed to the plugin when it is instantiated.
pub fn host_info() -> HostInfo {
    HostInfo::new(HOST_NAME, HOST_VENDOR, HOST_URL, HOST_VERSION)
        .expect("host info strings contain no NUL bytes")
}

/// Fixed worker activation configuration: after activation the plugin is
/// guaranteed to see no smaller than `MIN_BLOCK_FRAMES` frames and no larger
/// than `MAX_BLOCK_FRAMES` frames per block. The worker enforces this by
/// splitting into blocks of `MAX_BLOCK_FRAMES` and padding the final block.
pub const MIN_BLOCK_FRAMES: u32 = 32;
pub const MAX_BLOCK_FRAMES: u32 = 2048;

/// The activation configuration used for every plugin in a chain.
pub fn activation_config(sample_rate: u32) -> PluginAudioConfiguration {
    PluginAudioConfiguration {
        sample_rate: f64::from(sample_rate),
        min_frames_count: MIN_BLOCK_FRAMES,
        max_frames_count: MAX_BLOCK_FRAMES,
    }
}

/// Iterates every descriptor id exposed by `entry`, for typed errors that
/// tell the user what the binary actually contains.
pub fn describe_entry_ids(
    entry: &PluginEntry,
    source_display: &str,
) -> Result<Vec<String>, DspError> {
    let Some(factory) = entry.get_plugin_factory() else {
        return Err(DspError::PluginUnloadable(
            source_display.to_string(),
            "entry exposes no plugin factory".to_string(),
        ));
    };

    let mut ids = Vec::new();
    for descriptor in factory.plugin_descriptors() {
        if let Some(id) = descriptor.id() {
            ids.push(id.to_string_lossy().into_owned());
        }
    }
    if ids.is_empty() {
        return Err(DspError::PluginIdMissing(source_display.to_string()));
    }
    Ok(ids)
}

/// The clack path we take when loading a shared library entry.
///
/// Exposed because the worker repeats the same load after trust validation.
pub fn load_entry(path: &std::path::Path) -> Result<PluginEntry, DspError> {
    // SAFETY: dlopen-ing a shared library is inherently unsafe; the caller
    // must have validated the path through the trusted-roots check.
    let entry = unsafe {
        PluginEntry::load(path).map_err(|err| {
            DspError::PluginUnloadable(path.display().to_string(), err.to_string())
        })?
    };
    Ok(entry)
}
