// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Trusted plugin root validation, delegated to the single policy
//! implementation in `saturn-core::project::dsp_trust` and mapped onto the
//! typed [`DspError`]s.

use std::path::PathBuf;

pub use saturn_core::project::{trusted_roots, PLUGIN_SUFFIXES};

use crate::errors::{DspError, DspResult};

/// Validates a plugin path under the shared trust policy and returns its
/// canonicalized form. Typed errors:
///
/// * [`DspError::PluginPathNotFound`] — missing file
/// * [`DspError::PluginPathUntrusted`] — succeeded-but-untrusted
pub fn validate_plugin_path(path: &str) -> DspResult<PathBuf> {
    match saturn_core::project::validate_plugin_path(path) {
        Ok(canonical) => Ok(canonical),
        Err(saturn_core::project::DspPathError::NotFound(display)) => {
            Err(DspError::PluginPathNotFound(PathBuf::from(display)))
        }
        Err(saturn_core::project::DspPathError::Untrusted(reason)) => {
            Err(DspError::PluginPathUntrusted(reason))
        }
    }
}
