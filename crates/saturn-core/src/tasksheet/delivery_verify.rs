// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Cryptographic checksum calculation and fail-closed verification for TS5 packages.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use super::delivery_report::{
    DeliveryManifest, PackageVerificationResult, ARTIFACT_SUBDIRS,
    DELIVERY_MANIFEST_SCHEMA_VERSION,
};

fn to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Computes lowercase hex BLAKE3 digest for a byte slice.
pub fn compute_blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Computes lowercase hex SHA-256 digest for a byte slice.
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    to_lower_hex(&Sha256::digest(bytes))
}

pub(crate) fn validate_safe_relative_entry_path(rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if rel.is_empty()
        || rel.contains('\\')
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("Unsafe manifest entry path `{rel}`"));
    }
    Ok(p.to_path_buf())
}

fn parse_checksum_file(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read checksum file `{}`: {e}", path.display()))?;
    let mut map = BTreeMap::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let (digest, rel) = trimmed.split_once("  ").ok_or_else(|| {
            format!(
                "Malformed checksum line {} in `{}`",
                idx + 1,
                path.display()
            )
        })?;
        validate_safe_relative_entry_path(rel)?;
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid hex digest on line {} in `{}`",
                idx + 1,
                path.display()
            ));
        }
        if map.insert(rel.to_string(), digest.to_ascii_lowercase()).is_some() {
            return Err(format!(
                "Duplicate checksum entry `{rel}` in `{}`",
                path.display()
            ));
        }
    }
    Ok(map)
}

fn load_checksum_map(
    path: &Path,
    missing_msg: &str,
    errors: &mut Vec<String>,
) -> (bool, Option<BTreeMap<String, String>>) {
    if !path.is_file() {
        errors.push(missing_msg.to_string());
        return (false, None);
    }
    match parse_checksum_file(path) {
        Ok(map) => (true, Some(map)),
        Err(e) => {
            errors.push(e);
            (true, None)
        }
    }
}

fn verify_entry_digest(
    algorithm: &str,
    checksum_filename: &str,
    rel_path: &str,
    actual_digest: &str,
    manifest_digest: Option<&str>,
    checksum_map: Option<&BTreeMap<String, String>>,
    errors: &mut Vec<String>,
) -> bool {
    let mut ok = true;
    if let Some(expected) = manifest_digest {
        if actual_digest != expected.to_ascii_lowercase() {
            errors.push(format!(
                "{algorithm} mismatch for `{rel_path}` (expected {expected}, got {actual_digest})"
            ));
            ok = false;
        }
    }
    if let Some(map) = checksum_map {
        match map.get(rel_path) {
            Some(sum) if sum == actual_digest => {}
            Some(sum) => {
                errors.push(format!(
                    "{checksum_filename} mismatch for `{rel_path}` (expected {sum}, got {actual_digest})"
                ));
                ok = false;
            }
            None => {
                errors.push(format!("{checksum_filename} missing entry `{rel_path}`"));
                ok = false;
            }
        }
    } else {
        ok = false;
    }
    ok
}

fn collect_relative_files_in_subdir(
    package_dir: &Path,
    subdir: &str,
    out: &mut BTreeSet<String>,
) -> Result<(), String> {
    let dir = package_dir.join(subdir);
    if !dir.exists() {
        return Ok(());
    }
    if !dir.is_dir() {
        return Err(format!(
            "Expected `{subdir}` inside package to be a directory"
        ));
    }
    let mut stack = vec![dir];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)
            .map_err(|e| format!("Failed scanning `{}`: {e}", current.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Directory entry error: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(package_dir)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel);
            }
        }
    }
    Ok(())
}

/// Inspects a delivery package directory and returns detailed integrity verification findings.
pub fn inspect_delivery_package_integrity(
    package_dir: &Path,
) -> Result<PackageVerificationResult, String> {
    if !package_dir.is_dir() {
        return Err(format!(
            "Delivery package directory `{}` does not exist",
            package_dir.display()
        ));
    }

    let manifest_path = package_dir.join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|e| format!("Cannot read `{}`: {e}", manifest_path.display()))?;
    let manifest: DeliveryManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Invalid `{}`: {e}", manifest_path.display()))?;

    let mut errors = Vec::new();
    if manifest.schema != DELIVERY_MANIFEST_SCHEMA_VERSION {
        errors.push(format!(
            "Unexpected manifest schema `{}`; expected `{DELIVERY_MANIFEST_SCHEMA_VERSION}`",
            manifest.schema
        ));
    }
    if manifest.files.is_empty() {
        errors.push("Manifest contains zero packaged files".to_string());
    }

    let (checksum_file_blake3_present, b3_map) = load_checksum_map(
        &package_dir.join("CHECKSUMS.blake3"),
        "Missing CHECKSUMS.blake3 file",
        &mut errors,
    );
    let (checksum_file_sha256_present, s256_map) = load_checksum_map(
        &package_dir.join("SHA256SUMS"),
        "Missing SHA256SUMS file",
        &mut errors,
    );

    let manifest_b3_ok = verify_entry_digest(
        "BLAKE3",
        "CHECKSUMS.blake3",
        "manifest.json",
        &compute_blake3_hex(&manifest_bytes),
        None,
        b3_map.as_ref(),
        &mut errors,
    );
    let manifest_s256_ok = verify_entry_digest(
        "SHA-256",
        "SHA256SUMS",
        "manifest.json",
        &compute_sha256_hex(&manifest_bytes),
        None,
        s256_map.as_ref(),
        &mut errors,
    );
    let manifest_checksum_verified = manifest_b3_ok && manifest_s256_ok;

    let mut blake3_verified = checksum_file_blake3_present && manifest_checksum_verified;
    let mut sha256_verified = checksum_file_sha256_present && manifest_checksum_verified;
    let mut expected_paths = BTreeSet::new();
    let mut checked_files = 0usize;

    for entry in &manifest.files {
        let rel_buf = match validate_safe_relative_entry_path(&entry.relative_path) {
            Ok(p) => p,
            Err(e) => {
                errors.push(e);
                blake3_verified = false;
                sha256_verified = false;
                continue;
            }
        };
        if !expected_paths.insert(entry.relative_path.clone()) {
            errors.push(format!("Duplicate manifest entry `{}`", entry.relative_path));
        }
        let file_path = package_dir.join(rel_buf);
        let bytes = match fs::read(&file_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "Missing or unreadable artifact `{}`: {e}",
                    entry.relative_path
                ));
                blake3_verified = false;
                sha256_verified = false;
                continue;
            }
        };
        checked_files += 1;

        if bytes.len() as u64 != entry.size_bytes {
            errors.push(format!(
                "Size mismatch for `{}` (expected {} bytes, got {} bytes)",
                entry.relative_path,
                entry.size_bytes,
                bytes.len()
            ));
            blake3_verified = false;
            sha256_verified = false;
        }

        if !verify_entry_digest(
            "BLAKE3",
            "CHECKSUMS.blake3",
            &entry.relative_path,
            &compute_blake3_hex(&bytes),
            Some(&entry.blake3),
            b3_map.as_ref(),
            &mut errors,
        ) {
            blake3_verified = false;
        }
        if !verify_entry_digest(
            "SHA-256",
            "SHA256SUMS",
            &entry.relative_path,
            &compute_sha256_hex(&bytes),
            Some(&entry.sha256),
            s256_map.as_ref(),
            &mut errors,
        ) {
            sha256_verified = false;
        }
    }

    let mut disk_subdir_files = BTreeSet::new();
    for subdir in ARTIFACT_SUBDIRS {
        if let Err(e) = collect_relative_files_in_subdir(package_dir, subdir, &mut disk_subdir_files)
        {
            errors.push(e);
        }
    }
    for extra in disk_subdir_files.difference(&expected_paths) {
        errors.push(format!("Unlisted file `{extra}` found in package"));
        blake3_verified = false;
        sha256_verified = false;
    }

    Ok(PackageVerificationResult {
        valid: errors.is_empty(),
        package_dir: package_dir.to_string_lossy().into_owned(),
        manifest_schema: manifest.schema,
        checked_files,
        blake3_verified,
        sha256_verified,
        manifest_checksum_verified,
        checksum_file_blake3_present,
        checksum_file_sha256_present,
        errors,
    })
}

/// Fail-closed checksum verification of a TS5 delivery package.
pub fn verify_delivery_package(package_dir: &Path) -> Result<PackageVerificationResult, String> {
    let report = inspect_delivery_package_integrity(package_dir)?;
    if !report.valid {
        return Err(format!(
            "Delivery package checksum verification failed: {}",
            report.errors.join("; ")
        ));
    }
    Ok(report)
}
