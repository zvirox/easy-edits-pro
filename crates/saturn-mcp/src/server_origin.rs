// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Server-owned untrusted media provenance wrapping and origin validation (AF5).
//!
//! Enforces that media-derived text is wrapped with explicit provenance,
//! and that destructive/filesystem arguments derived from untrusted content
//! require explicit human approval.

use std::path::PathBuf;
use saturn_core::catalogue::Tier;
use saturn_core::provenance::UntrustedText;
use saturn_core::{CommandSpec, ProjectDocument};
use serde_json::Value;

use super::EditorMcp;

impl EditorMcp {
    /// Synchronize the server-owned origin tracker with all untrusted items in the current document.
    pub async fn sync_origin_tracker(&self, doc: &ProjectDocument) {
        let mut tracker = self.origin_tracker.write().await;
        tracker.index_document(doc);
    }

    /// Record an untrusted media text item in the server-owned origin tracker.
    #[allow(dead_code)]
    pub async fn record_untrusted_text(&self, item: UntrustedText) {
        let mut tracker = self.origin_tracker.write().await;
        tracker.record(item);
    }

    /// Determine allowed filesystem roots for the current session.
    pub async fn allowed_filesystem_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        roots.push(std::env::temp_dir());
        if let Ok(Some(proj_path)) = self.headless_project_path().await {
            if let Some(parent) = proj_path.parent() {
                roots.push(parent.to_path_buf());
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        roots
    }

    /// Validate command arguments for origin-aware security (AF5).
    ///
    /// If the command operates on Tier::Filesystem or Tier::Destructive:
    /// - Checks whether any string argument is derived from untrusted media text.
    /// - Checks whether any argument has an unknown origin outside allowed roots.
    /// Fails with `PermissionDenied` unless explicitly approved by a human.
    pub async fn validate_command_origin(
        &self,
        id: &str,
        params: &Value,
        spec: &CommandSpec,
    ) -> Result<(), String> {
        let is_filesystem_or_destructive = spec.tiers.contains(&Tier::Filesystem)
            || spec.tiers.contains(&Tier::Destructive)
            || id.starts_with("file.")
            || id.starts_with("interop.export")
            || id.starts_with("render_")
            || id.contains("delete");

        if !is_filesystem_or_destructive {
            return Ok(());
        }

        let allowed_roots = self.allowed_filesystem_roots().await;
        let tracker = self.origin_tracker.read().await;

        let mut string_args = Vec::new();
        collect_string_args(params, "", &mut string_args);

        for (arg_name, arg_val) in string_args {
            if let Err(err) = tracker.validate_argument(
                &arg_name,
                &arg_val,
                true,
                &allowed_roots,
            ) {
                return Err(err.to_string());
            }
        }

        Ok(())
    }

    /// Validate dedicated tool arguments for origin-aware security (AF5).
    pub async fn validate_tool_origin(
        &self,
        tool_name: &str,
        arguments: &Value,
    ) -> Result<(), String> {
        let is_fs = tool_name.starts_with("render_")
            || tool_name == "vision_extract_frame_sheet"
            || tool_name == "audio_process";

        if !is_fs {
            return Ok(());
        }

        let allowed_roots = self.allowed_filesystem_roots().await;
        let tracker = self.origin_tracker.read().await;

        let mut string_args = Vec::new();
        collect_string_args(arguments, "", &mut string_args);

        for (arg_name, arg_val) in string_args {
            if let Err(err) = tracker.validate_argument(
                &arg_name,
                &arg_val,
                true,
                &allowed_roots,
            ) {
                return Err(err.to_string());
            }
        }

        Ok(())
    }

    /// Wrap media search output with explicit untrusted provenance (AF5).
    pub async fn wrap_media_search_output(&self, raw: Value) -> Value {
        let mut tracker = self.origin_tracker.write().await;
        tracker.wrap_media_search(raw)
    }

    /// Wrap media preview output with explicit provenance (AF5).
    pub async fn wrap_media_preview_output(&self, raw: Value) -> Value {
        let mut tracker = self.origin_tracker.write().await;
        tracker.wrap_media_preview(raw)
    }

    /// Wrap sequence inspect output with explicit provenance (AF5).
    pub async fn wrap_sequence_inspect_output(&self, raw: Value) -> Value {
        let mut tracker = self.origin_tracker.write().await;
        tracker.wrap_sequence_inspect(raw)
    }

    /// Wrap semantic query output with explicit provenance (AF5).
    pub async fn wrap_semantic_query_output(&self, raw: Value) -> Value {
        let mut tracker = self.origin_tracker.write().await;
        tracker.wrap_semantic_query(raw)
    }
}

/// Helper to recursively collect string arguments from JSON params.
fn collect_string_args(val: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match val {
        Value::String(s) => {
            if !s.trim().is_empty() {
                out.push((prefix.to_string(), s.clone()));
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                collect_string_args(v, &new_prefix, out);
            }
        }
        Value::Array(arr) => {
            for (idx, v) in arr.iter().enumerate() {
                let new_prefix = format!("{prefix}[{idx}]");
                collect_string_args(v, &new_prefix, out);
            }
        }
        _ => {}
    }
}
