// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Delivery packaging and staging orchestration (TS5).
//!
//! Assembles self-contained handoff bundles containing project files, staged
//! exports, interchange exports (`FCPXML`, `OTIO`, EDL, XMEML), QC reports,
//! attributed change logs, provenance metadata, and cryptographic manifests.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use crate::actor::ActorContext;
use crate::interop::{export_cmx3600_edl, export_fcpxml, export_otio, export_xmeml_xml};
use crate::journal::ChangeJournal;
use crate::project::ProjectDocument;
use crate::verification::{run_verification, VerifyReport};

use super::types::SheetHandoff;

pub use super::delivery_report::{
    build_attributed_changelog, build_delivery_provenance, format_changelog_markdown,
    AttributedChangeLog, ClipAttributionEntry, DeliveryBuildOutput, DeliveryExportInput,
    DeliveryManifest, DeliveryPackageOptions, DeliveryProvenance, DeliveryQcSummary,
    ExportSourceSpec, PackageVerificationResult, PackagedFileEntry, ProvenanceMediaAsset,
    DELIVERY_MANIFEST_SCHEMA_VERSION,
};
pub use super::delivery_verify::{
    compute_blake3_hex, compute_sha256_hex, inspect_delivery_package_integrity,
    verify_delivery_package,
};

use super::delivery_verify::validate_safe_relative_entry_path;

/// Sanitizes a name into a filesystem-safe stem with fallback `"project"`.
pub fn sanitize_file_stem(raw: &str) -> String {
    let mut stem = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch);
            prev_dash = false;
        } else if (ch == '-' || ch == '_' || ch == ' ') && !prev_dash && !stem.is_empty() {
            stem.push(if ch == ' ' { '-' } else { ch });
            prev_dash = true;
        }
    }
    let trimmed = stem.trim_matches(['-', '_']);
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_safe_package_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Delivery package path cannot be empty".into());
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Delivery package path must not contain '..' components".into());
    }
    if path.exists() && !path.is_dir() {
        return Err(format!(
            "Delivery destination `{}` exists and is not a directory",
            path.display()
        ));
    }
    Ok(())
}

struct StagedArtifact {
    relative_path: String,
    role: String,
    format: String,
    bytes: Vec<u8>,
    source_path: Option<String>,
    deliverable_id: Option<String>,
    label: Option<String>,
}

fn resolve_handoff_flags(
    handoff: Option<&SheetHandoff>,
    opts: &DeliveryPackageOptions,
) -> (bool, bool, bool, bool, Vec<String>) {
    let include_project = opts
        .include_project_file
        .or_else(|| handoff.and_then(|h| h.project_file))
        .unwrap_or(true);
    let include_exports = opts
        .include_exports
        .or_else(|| handoff.and_then(|h| h.exports))
        .unwrap_or(true);
    let include_qc = opts
        .include_qc_report
        .or_else(|| handoff.and_then(|h| h.qc_report))
        .unwrap_or(true);
    let include_changelog = opts
        .include_change_log
        .or_else(|| handoff.and_then(|h| h.change_log))
        .unwrap_or(true);

    let formats = if let Some(ref requested) = opts.interchange_formats {
        requested.clone()
    } else if let Some(h) = handoff {
        if !h.interchange.is_empty() {
            h.interchange.clone()
        } else {
            vec!["fcpxml".to_string(), "otio".to_string()]
        }
    } else {
        vec!["fcpxml".to_string(), "otio".to_string()]
    };

    (
        include_project,
        include_exports,
        include_qc,
        include_changelog,
        formats,
    )
}

fn generate_interchange_bytes(
    document: &ProjectDocument,
    format_raw: &str,
    stem: &str,
) -> Result<(String, String, Vec<u8>), String> {
    let normalized = format_raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "fcpxml" => {
            let content = export_fcpxml(document)?;
            Ok((
                format!("interchange/{stem}.fcpxml"),
                "fcpxml".to_string(),
                content.into_bytes(),
            ))
        }
        "otio" => {
            let content = export_otio(document)?;
            Ok((
                format!("interchange/{stem}.otio"),
                "otio".to_string(),
                content.into_bytes(),
            ))
        }
        "edl" | "cmx3600" => {
            let content = export_cmx3600_edl(document)?;
            Ok((
                format!("interchange/{stem}.edl"),
                "edl".to_string(),
                content.into_bytes(),
            ))
        }
        "xmeml" | "xml" => {
            let content = export_xmeml_xml(document)?;
            Ok((
                format!("interchange/{stem}.xml"),
                "xmeml".to_string(),
                content.into_bytes(),
            ))
        }
        other => Err(format!(
            "Unsupported interchange format `{other}`; expected `fcpxml`, `otio`, `edl`, or `xmeml`"
        )),
    }
}

fn unique_export_relative_path(
    src: &Path,
    used_names: &mut BTreeSet<String>,
) -> Result<String, String> {
    let file_name = src
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid export filename `{}`", src.display()))?;
    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".."
    {
        return Err(format!("Unsafe export filename `{file_name}`"));
    }

    let candidate = format!("exports/{file_name}");
    if used_names.insert(candidate.clone()) {
        return Ok(candidate);
    }

    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = src.extension().and_then(|e| e.to_str());
    for idx in 1..10_000 {
        let renamed = match ext {
            Some(e) if !e.is_empty() => format!("exports/{stem}-{idx}.{e}"),
            _ => format!("exports/{stem}-{idx}"),
        };
        if used_names.insert(renamed.clone()) {
            return Ok(renamed);
        }
    }
    Err(format!("Too many duplicate filenames for `{file_name}`"))
}

fn stage_export_artifacts(
    document: &ProjectDocument,
    opts: &DeliveryPackageOptions,
    staged: &mut Vec<StagedArtifact>,
) -> Result<usize, String> {
    if opts.require_rendered_exports && opts.exports.is_empty() {
        return Err("Delivery package requires at least one rendered export file".into());
    }

    let mut used_export_paths = BTreeSet::new();
    let mut staged_count = 0usize;
    let mut staged_meta = Vec::new();

    for exp in &opts.exports {
        let src = Path::new(&exp.path);
        if src.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(format!(
                "Export path `{}` must not contain '..' components",
                exp.path
            ));
        }
        let meta = fs::metadata(src)
            .map_err(|e| format!("Cannot read export file `{}`: {e}", src.display()))?;
        if !meta.is_file() {
            return Err(format!(
                "Export source `{}` is not a regular file",
                src.display()
            ));
        }
        let bytes = fs::read(src)
            .map_err(|e| format!("Failed reading export file `{}`: {e}", src.display()))?;
        let rel = unique_export_relative_path(src, &mut used_export_paths)?;
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_ascii_lowercase();

        staged_meta.push(serde_json::json!({
            "relative_path": rel,
            "source_path": exp.path,
            "deliverable_id": exp.deliverable_id,
            "label": exp.label,
            "size_bytes": bytes.len(),
        }));
        staged.push(StagedArtifact {
            relative_path: rel,
            role: "export".to_string(),
            format: ext,
            bytes,
            source_path: Some(exp.path.clone()),
            deliverable_id: exp.deliverable_id.clone(),
            label: exp.label.clone(),
        });
        staged_count += 1;
    }

    let deliverables = document
        .project
        .task_sheet
        .as_ref()
        .map(|s| s.deliverables.clone())
        .unwrap_or_default();

    let deliverables_doc = serde_json::json!({
        "schema": "delivery-exports/1",
        "project_name": document.project.name,
        "settings": {
            "width": document.project.settings.width,
            "height": document.project.settings.height,
            "frame_rate": document.project.settings.frame_rate,
            "audio_sample_rate": document.project.settings.audio_sample_rate,
        },
        "deliverables": deliverables,
        "staged_exports": staged_meta,
    });
    let deliverables_bytes = serde_json::to_vec_pretty(&deliverables_doc)
        .map_err(|e| format!("Failed to serialize deliverables spec: {e}"))?;
    staged.push(StagedArtifact {
        relative_path: "exports/deliverables.json".to_string(),
        role: "deliverables_spec".to_string(),
        format: "json".to_string(),
        bytes: deliverables_bytes,
        source_path: None,
        deliverable_id: None,
        label: Some("Deliverable Specifications".to_string()),
    });

    Ok(staged_count)
}

/// Assembles, writes, hashes (BLAKE3 + SHA-256), and verifies a TS5 delivery package.
pub fn assemble_delivery_package(
    document: &ProjectDocument,
    journal: &ChangeJournal,
    actor: &ActorContext,
    opts: &DeliveryPackageOptions,
) -> Result<DeliveryBuildOutput, String> {
    validate_safe_package_path(&opts.package_dir)?;

    let stem = sanitize_file_stem(&document.project.name);
    let handoff = document
        .project
        .task_sheet
        .as_ref()
        .and_then(|s| s.handoff.as_ref());
    let (include_project, include_exports, include_qc, include_changelog, interchange_formats) =
        resolve_handoff_flags(handoff, opts);

    let mut staged: Vec<StagedArtifact> = Vec::new();
    let mut staged_exports_count = 0usize;

    if include_exports {
        staged_exports_count = stage_export_artifacts(document, opts, &mut staged)?;
    }

    if include_project {
        let project_bytes = serde_json::to_vec_pretty(document)
            .map_err(|e| format!("Failed to serialize project document: {e}"))?;
        staged.push(StagedArtifact {
            relative_path: format!("project/{stem}.saturn"),
            role: "project_file".to_string(),
            format: "saturn".to_string(),
            bytes: project_bytes,
            source_path: None,
            deliverable_id: None,
            label: Some(document.project.name.clone()),
        });
    }

    let mut seen_interchange = BTreeSet::new();
    let mut emitted_interchange = Vec::new();
    for fmt in &interchange_formats {
        let (rel, fmt_name, bytes) = generate_interchange_bytes(document, fmt, &stem)?;
        if seen_interchange.insert(rel.clone()) {
            emitted_interchange.push(fmt_name.clone());
            staged.push(StagedArtifact {
                relative_path: rel,
                role: "interchange".to_string(),
                format: fmt_name,
                bytes,
                source_path: None,
                deliverable_id: None,
                label: None,
            });
        }
    }

    let mut qc_summary = None;
    if include_qc {
        let verify_params = opts.verify_params.clone().unwrap_or_default();
        let report: VerifyReport = run_verification(document, &verify_params)?;
        qc_summary = Some(DeliveryQcSummary {
            preset: report.preset.as_str().to_string(),
            total_checks: report.summary.total_checks,
            passed: report.summary.passed,
            warned: report.summary.warned,
            failed: report.summary.failed,
            unavailable: report.summary.unavailable,
        });
        let report_bytes = serde_json::to_vec_pretty(&report)
            .map_err(|e| format!("Failed to serialize verify.run report: {e}"))?;
        staged.push(StagedArtifact {
            relative_path: "qc/verify-report.json".to_string(),
            role: "qc_report".to_string(),
            format: "json".to_string(),
            bytes: report_bytes,
            source_path: None,
            deliverable_id: None,
            label: Some("Automated QC Verification Report".to_string()),
        });
    }

    let mut changelog_entries_count = journal.records.len();
    if include_changelog {
        let changelog = build_attributed_changelog(document, journal, opts.timestamp_unix_ms);
        changelog_entries_count = changelog.records.len();
        let changelog_json = serde_json::to_vec_pretty(&changelog)
            .map_err(|e| format!("Failed to serialize change log: {e}"))?;
        let changelog_md = format_changelog_markdown(&changelog).into_bytes();
        staged.push(StagedArtifact {
            relative_path: "changelog/changelog.json".to_string(),
            role: "change_log".to_string(),
            format: "json".to_string(),
            bytes: changelog_json,
            source_path: None,
            deliverable_id: None,
            label: Some("Attributed Change Log JSON".to_string()),
        });
        staged.push(StagedArtifact {
            relative_path: "changelog/CHANGELOG.md".to_string(),
            role: "change_log".to_string(),
            format: "markdown".to_string(),
            bytes: changelog_md,
            source_path: None,
            deliverable_id: None,
            label: Some("Attributed Change Log Markdown".to_string()),
        });
    }

    if opts.include_provenance {
        let provenance =
            build_delivery_provenance(document, journal, actor, opts.timestamp_unix_ms);
        let provenance_bytes = serde_json::to_vec_pretty(&provenance)
            .map_err(|e| format!("Failed to serialize provenance metadata: {e}"))?;
        staged.push(StagedArtifact {
            relative_path: "provenance/provenance.json".to_string(),
            role: "provenance".to_string(),
            format: "json".to_string(),
            bytes: provenance_bytes,
            source_path: None,
            deliverable_id: None,
            label: Some("Input, Media, and Edit Provenance".to_string()),
        });
    }

    if staged.is_empty() {
        return Err("Delivery package produced no artifacts to package".into());
    }

    staged.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    fs::create_dir_all(&opts.package_dir).map_err(|e| {
        format!(
            "Failed to create delivery package directory `{}`: {e}",
            opts.package_dir.display()
        )
    })?;

    let mut manifest_entries = Vec::with_capacity(staged.len());
    let mut blake3_lines = Vec::with_capacity(staged.len() + 1);
    let mut sha256_lines = Vec::with_capacity(staged.len() + 1);

    for item in staged {
        let rel_buf = validate_safe_relative_entry_path(&item.relative_path)?;
        let dst = opts.package_dir.join(rel_buf);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create delivery subdirectory `{}`: {e}",
                    parent.display()
                )
            })?;
        }
        fs::write(&dst, &item.bytes).map_err(|e| {
            format!("Failed writing delivery artifact `{}`: {e}", dst.display())
        })?;

        let b3 = compute_blake3_hex(&item.bytes);
        let s256 = compute_sha256_hex(&item.bytes);
        blake3_lines.push(format!("{b3}  {}", item.relative_path));
        sha256_lines.push(format!("{s256}  {}", item.relative_path));

        manifest_entries.push(PackagedFileEntry {
            relative_path: item.relative_path,
            role: item.role,
            format: item.format,
            size_bytes: item.bytes.len() as u64,
            blake3: b3,
            sha256: s256,
            source_path: item.source_path,
            deliverable_id: item.deliverable_id,
            label: item.label,
        });
    }

    let (task_sheet_id, task_sheet_status, deliverables, review_rounds_count) =
        match &document.project.task_sheet {
            Some(sheet) => (
                Some(sheet.id.clone()),
                Some(sheet.status.as_str().to_string()),
                sheet.deliverables.clone(),
                sheet.rounds.len(),
            ),
            None => (None, None, Vec::new(), 0),
        };

    let manifest = DeliveryManifest {
        schema: DELIVERY_MANIFEST_SCHEMA_VERSION.to_string(),
        created_at_unix_ms: opts.timestamp_unix_ms,
        created_by: actor.actor.to_string(),
        session_id: actor.session_id.clone(),
        project_name: document.project.name.clone(),
        project_format: document.format.clone(),
        project_schema_version: document.schema_version,
        task_sheet_id,
        task_sheet_status,
        hash_algorithms: vec!["blake3".to_string(), "sha256".to_string()],
        qc_summary,
        deliverables,
        staged_exports_count,
        interchange_formats: emitted_interchange,
        changelog_entries_count,
        review_rounds_count,
        files: manifest_entries,
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize delivery manifest: {e}"))?;
    let manifest_path = opts.package_dir.join("manifest.json");
    fs::write(&manifest_path, &manifest_bytes)
        .map_err(|e| format!("Failed writing `{}`: {e}", manifest_path.display()))?;

    let manifest_b3 = compute_blake3_hex(&manifest_bytes);
    let manifest_s256 = compute_sha256_hex(&manifest_bytes);
    blake3_lines.push(format!("{manifest_b3}  manifest.json"));
    sha256_lines.push(format!("{manifest_s256}  manifest.json"));
    blake3_lines.sort();
    sha256_lines.sort();

    let checksums_blake3_path = opts.package_dir.join("CHECKSUMS.blake3");
    let checksums_sha256_path = opts.package_dir.join("SHA256SUMS");
    fs::write(&checksums_blake3_path, format!("{}\n", blake3_lines.join("\n")))
        .map_err(|e| format!("Failed writing `{}`: {e}", checksums_blake3_path.display()))?;
    fs::write(&checksums_sha256_path, format!("{}\n", sha256_lines.join("\n")))
        .map_err(|e| format!("Failed writing `{}`: {e}", checksums_sha256_path.display()))?;

    let verification = verify_delivery_package(&opts.package_dir)?;

    Ok(DeliveryBuildOutput {
        package_dir: opts.package_dir.to_string_lossy().into_owned(),
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        checksums_blake3_path: checksums_blake3_path.to_string_lossy().into_owned(),
        checksums_sha256_path: checksums_sha256_path.to_string_lossy().into_owned(),
        manifest,
        verification,
    })
}
