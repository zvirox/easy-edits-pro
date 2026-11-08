// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Saturn CLAP plugin host library.
//!
//! This crate owns everything needed to run third-party CLAP plugins on a
//! track's DSP chain: trusted-root path validation, a host-side chain
//! (load → activate → process → deactivate), the `/dev/shm` SPSC ring
//! transport, and the sandbox client driving the dedicated
//! `saturn-plugin-worker` process. The renderer and UI never host plugins
//! in-process; everything runs inside the worker's address space.

pub mod chain;
pub mod clap_host;
pub mod errors;
pub mod sandbox;
pub mod shm_ring;
pub mod trust;

pub use chain::{ChainSlotExec, PluginChain, CHAIN_BLOCK_FRAMES, MAX_TRACK_SLOTS};
pub use clap_host::{SaturnHost, MIN_BLOCK_FRAMES, MAX_BLOCK_FRAMES};
pub use errors::{DspError, DspResult};
pub use sandbox::{SandboxSession, worker_search_error_text};
pub use shm_ring::{RingSide, ShmRings, RING_MAX_COMMAND_FRAMES};
pub use trust::validate_plugin_path;
