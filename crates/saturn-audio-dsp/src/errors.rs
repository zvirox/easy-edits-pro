// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Typed errors for the Saturn DSP plugin host and its sandbox transport.

use std::path::PathBuf;

/// Every failure a plugin chain, worker, or shared-memory transport can
/// report. Payloads name the blocking condition so command responses and
/// render logs can surface an actionable message without string matching.
#[derive(Debug, thiserror::Error)]
pub enum DspError {
    /// The recorded plugin path does not exist on this machine.
    #[error("plugin path not found: {0}")]
    PluginPathNotFound(PathBuf),
    /// The plugin path resolves outside every trusted plugin root.
    #[error("plugin path is outside the trusted plugin roots: {0}")]
    PluginPathUntrusted(String),
    /// The shared library could not be dlopen-ed / initialized as a CLAP entry,
    /// or no usable descriptor id could be found inside it.
    #[error("failed to load plugin {0}: {1}")]
    PluginUnloadable(String, String),
    /// The plugin's descriptor carries no stable id string.
    #[error("plugin at {0} exposes no descriptor id")]
    PluginIdMissing(String),
    /// Instantiation, activation, or process start failed inside a plugin.
    #[error("plugin startup failed for {0}: {1}")]
    PluginStartup(String, String),
    /// Only CLAP is hosted today; other formats stay schema-valid elsewhere.
    #[error("format not hosted by the Saturn renderer: {0} (hosted formats: clap)")]
    FormatNotHosted(String),
    /// The requested slot index does not exist on the track's chain.
    #[error("plugin slot index {0} is out of range")]
    SlotInvalid(usize),
    /// The worker process failed to spawn, handshake, or died mid-render.
    #[error("plugin sandbox failure: {0}")]
    Sandbox(String),
    /// The shared-memory audio transport failed (map, layout, or size).
    #[error("plugin shared-memory transport failure: {0}")]
    ShmTransport(String),
    /// A parameter id/value passed to a slot is not acceptable to it.
    #[error("invalid plugin parameter {0}: {1}")]
    ParameterInvalid(u64, String),
}

/// Shorthand result type for the DSP host crate.
pub type DspResult<T> = Result<T, DspError>;
