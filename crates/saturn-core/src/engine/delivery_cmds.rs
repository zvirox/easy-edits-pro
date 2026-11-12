// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Engine command handlers for TS5 delivery packaging (`delivery.package`, `delivery.verify`).

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use super::*;
use crate::engine::brief_cmds::now_epoch_ms;
use crate::provenance::OriginTracker;
use crate::tasksheet::delivery::{
    assemble_delivery_package, verify_delivery_package, DeliveryPackageOptions, ExportSourceSpec,
};
use crate::tasksheet::{apply_transition, ReviewNoteStatus, SheetStatus};
use crate::verification::VerifyParams;

#[derive(Deserialize)]
struct DeliveryPackageParams {
    path: Option<String>,
    output_dir: Option<String>,
    #[serde(default)]
    exports: Vec<ExportSourceSpec>,
    interchange: Option<Vec<String>>,
    project_file: Option<bool>,
    include_exports: Option<bool>,
    qc_report: Option<bool>,
    change_log: Option<bool>,
    provenance: Option<bool>,
    verify: Option<VerifyParams>,
    #[serde(default)]
    mark_delivered: Option<bool>,
    #[serde(default)]
    require_approved: bool,
    #[serde(default)]
    require_rendered_exports: bool,
}

#[derive(Deserialize)]
struct DeliveryVerifyParams {
    path: Option<String>,
    package_dir: Option<String>,
}

fn validate_path_safety(raw: &str, doc: &ProjectDocument, is_human: bool) -> Result<PathBuf, DispatchError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DispatchError::InvalidParameters(
            "Delivery path cannot be empty".into(),
        ));
    }
    let path = PathBuf::from(trimmed);
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(DispatchError::InvalidParameters(
            "Delivery path must not contain '..' components".into(),
        ));
    }
    if !is_human {
        let tracker = OriginTracker::from_document(doc);
        if let Some(derivation) = tracker.detect_untrusted_derivation(trimmed) {
            return Err(DispatchError::Operation(format!(
                "PermissionDenied: UntrustedOrigin `{trimmed}` appears derived from untrusted {} text (`{}`)",
                derivation.source, derivation.matched_text
            )));
        }
    }
    Ok(path)
}

impl EditorEngine {
    pub(super) fn dispatch_delivery(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "delivery.package" => Some(self.handle_delivery_package(params)),
            "delivery.verify" => Some(self.handle_delivery_verify(params)),
            _ => None,
        }
    }

    fn handle_delivery_package(&mut self, params: Value) -> Result<Value, DispatchError> {
        let parsed: DeliveryPackageParams = parse_params(params)?;
        let raw_path = parsed
            .path
            .or(parsed.output_dir)
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "delivery.package requires `path` (destination directory)".into(),
                )
            })?;

        let ctx = self.current_context.clone().unwrap_or_default();
        let package_dir = validate_path_safety(&raw_path, &self.document, ctx.actor.is_human())?;

        let mut staged_doc = self.document.clone();
        let now = now_epoch_ms();
        let actor_name = ctx.actor.to_string();

        if parsed.require_approved {
            let sheet = staged_doc.project.task_sheet.as_ref().ok_or_else(|| {
                DispatchError::Operation(
                    "ApprovalRequired: delivery.package with `require_approved` needs a task sheet"
                        .into(),
                )
            })?;
            if sheet.status != SheetStatus::Approved {
                return Err(DispatchError::Operation(format!(
                    "ApprovalRequired: Task sheet must be in `approved` state before delivery (currently `{}`)",
                    sheet.status.as_str()
                )));
            }
            let open_notes: usize = sheet
                .rounds
                .iter()
                .flat_map(|r| r.notes.iter())
                .filter(|n| n.status == ReviewNoteStatus::Open)
                .count();
            if open_notes > 0 {
                return Err(DispatchError::Operation(format!(
                    "ApprovalRequired: Cannot deliver with {open_notes} unresolved review note(s)"
                )));
            }
        }

        let mut transitioned_to_delivered = false;
        let should_mark_delivered = match parsed.mark_delivered {
            Some(true) => true,
            Some(false) => false,
            None => staged_doc
                .project
                .task_sheet
                .as_ref()
                .map(|s| s.status == SheetStatus::Approved)
                .unwrap_or(false),
        };

        if should_mark_delivered {
            let sheet = staged_doc.project.task_sheet.as_mut().ok_or_else(|| {
                DispatchError::Operation(
                    "delivery.package cannot mark task sheet delivered without an active task sheet"
                        .into(),
                )
            })?;
            if sheet.status != SheetStatus::Approved {
                return Err(DispatchError::Operation(format!(
                    "InvalidTransition: Task sheet must be `approved` before marking `delivered` (currently `{}`)",
                    sheet.status.as_str()
                )));
            }
            apply_transition(
                sheet,
                SheetStatus::Delivered,
                actor_name,
                now,
                Some("Packaged TS5 delivery bundle".to_string()),
            )
            .map_err(|e| DispatchError::Operation(e.to_string()))?;
            transitioned_to_delivered = true;
        }

        let exports = parsed
            .exports
            .into_iter()
            .map(ExportSourceSpec::into_input)
            .collect();

        let opts = DeliveryPackageOptions {
            package_dir,
            exports,
            interchange_formats: parsed.interchange,
            include_project_file: parsed.project_file,
            include_exports: parsed.include_exports,
            include_qc_report: parsed.qc_report,
            include_change_log: parsed.change_log,
            include_provenance: parsed.provenance.unwrap_or(true),
            verify_params: parsed.verify,
            require_rendered_exports: parsed.require_rendered_exports,
            timestamp_unix_ms: now,
        };

        let built = assemble_delivery_package(&staged_doc, &self.journal, &ctx, &opts)
            .map_err(DispatchError::Operation)?;

        if transitioned_to_delivered {
            self.checkpoint();
            self.document = staged_doc;
        }

        let task_sheet_status = self
            .document
            .project
            .task_sheet
            .as_ref()
            .map(|s| s.status.as_str().to_string());

        Ok(json!({
            "packaged": true,
            "verified": built.verification.valid,
            "package_dir": built.package_dir,
            "manifest_path": built.manifest_path,
            "checksums_blake3_path": built.checksums_blake3_path,
            "checksums_sha256_path": built.checksums_sha256_path,
            "transitioned_to_delivered": transitioned_to_delivered,
            "task_sheet_status": task_sheet_status,
            "manifest": built.manifest,
            "verification": built.verification,
        }))
    }

    fn handle_delivery_verify(&self, params: Value) -> Result<Value, DispatchError> {
        let parsed: DeliveryVerifyParams = parse_params(params)?;
        let raw_path = parsed
            .path
            .or(parsed.package_dir)
            .ok_or_else(|| {
                DispatchError::InvalidParameters(
                    "delivery.verify requires `path` (package directory)".into(),
                )
            })?;

        let ctx = self.current_context.clone().unwrap_or_default();
        let package_dir = validate_path_safety(&raw_path, &self.document, ctx.actor.is_human())?;

        let verification =
            verify_delivery_package(Path::new(&package_dir)).map_err(DispatchError::Operation)?;

        Ok(json!({
            "verified": verification.valid,
            "package_dir": verification.package_dir,
            "checked_files": verification.checked_files,
            "blake3_verified": verification.blake3_verified,
            "sha256_verified": verification.sha256_verified,
            "manifest_checksum_verified": verification.manifest_checksum_verified,
            "verification": verification,
        }))
    }
}
