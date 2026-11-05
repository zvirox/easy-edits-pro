// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Project Templates and Search Bins
//!
//! Provides project template system and smart search bins for media organization
//! similar to the reference editor's project templates and search bins.
//!
//! Each type is defined exactly once, in the module that owns it. This module
//! only declares modules and re-exports their public surface.

pub mod manager;
pub mod search_bin;
pub mod template;

pub use manager::{ProjectManager, TemplateManager};
pub use search_bin::{
    MediaItem, SearchBin, SearchBinManager, SearchBinManager as BinManager, SearchBinMatch,
    SearchBinQuery, SearchBinRule, SearchField, SearchLogic, SearchOperator, presets,
};
pub use template::{
    BinTemplate, ProjectSettings, ProjectTemplate, SearchBinTemplate, SequenceTemplate,
    TemplateCategory, TemplateManifest, TemplateVariable,
};
