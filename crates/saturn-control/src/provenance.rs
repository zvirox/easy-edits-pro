// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Provenance validation and project-root containment for daemon sessions (AF5 / AF9).

use std::path::{Path, PathBuf};

use saturn_core::catalogue::Tier;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum ProvenanceError {
    #[error("PathOutsideProjectRoot: path '{path}' escapes project root '{root}'")]
    PathOutsideProjectRoot { path: String, root: String },
    #[error("UntrustedMediaOrigin: untrusted media text payload rejected for privileged tier '{tier}': {reason}")]
    UntrustedMediaOrigin { tier: String, reason: String },
}

impl ProvenanceError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::PathOutsideProjectRoot { .. } => "PathOutsideProjectRoot",
            Self::UntrustedMediaOrigin { .. } => "UntrustedMediaOrigin",
        }
    }
}

/// Validate that a requested path stays strictly within the allowed `project_root`.
pub fn validate_path_containment(
    candidate_path: &Path,
    project_root: Option<&Path>,
) -> Result<PathBuf, ProvenanceError> {
    let root = match project_root {
        Some(r) => r,
        None => return Ok(candidate_path.to_path_buf()),
    };

    let canonical_root = match root.canonicalize() {
        Ok(c) => c,
        Err(_) => root.to_path_buf(),
    };

    // Normalize candidate relative to root if relative
    let resolved = if candidate_path.is_relative() {
        canonical_root.join(candidate_path)
    } else {
        candidate_path.to_path_buf()
    };

    // Check canonicalization if path exists, or normalized lexical path if not yet created
    let normalized = match resolved.canonicalize() {
        Ok(c) => c,
        Err(_) => normalize_lexical(&resolved),
    };

    if !normalized.starts_with(&canonical_root) {
        return Err(ProvenanceError::PathOutsideProjectRoot {
            path: candidate_path.display().to_string(),
            root: canonical_root.display().to_string(),
        });
    }

    Ok(normalized)
}

/// Pure lexical path normalization without disk access.
fn normalize_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = components.last() {
                    components.pop();
                } else {
                    components.push(comp);
                }
            }
            _ => components.push(comp),
        }
    }
    components.into_iter().collect()
}

/// Inspect parameters for untrusted media text payloads (AF5) and fence privileged actions.
pub fn validate_untrusted_media_text(params: &Value, tier: Tier) -> Result<(), ProvenanceError> {
    if matches!(tier, Tier::Destructive | Tier::Filesystem) {
        if let Some(reason) = find_untrusted_marker(params) {
            return Err(ProvenanceError::UntrustedMediaOrigin {
                tier: tier.as_str().to_string(),
                reason,
            });
        }
    }
    Ok(())
}

fn find_untrusted_marker(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(untrusted) = map.get("untrusted").and_then(Value::as_bool) {
                if untrusted {
                    let source = map
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or("media text");
                    return Some(format!("payload explicitly tagged untrusted from source '{source}'"));
                }
            }
            for v in map.values() {
                if let Some(reason) = find_untrusted_marker(v) {
                    return Some(reason);
                }
            }
            None
        }
        Value::Array(arr) => {
            for v in arr {
                if let Some(reason) = find_untrusted_marker(v) {
                    return Some(reason);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn path_containment_permits_inner_paths_and_rejects_escape() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let inner = root.join("project/file.eep");

        assert!(validate_path_containment(&inner, Some(root)).is_ok());

        let escape = root.join("../../../etc/passwd");
        assert!(validate_path_containment(&escape, Some(root)).is_err());

        let root_str = root.canonicalize().unwrap().display().to_string();
        let err = validate_path_containment(Path::new("/tmp/outside.eep"), Some(root)).unwrap_err();
        assert_eq!(err.error_code(), "PathOutsideProjectRoot");
        match err {
            ProvenanceError::PathOutsideProjectRoot { path, root: r } => {
                assert_eq!(path, "/tmp/outside.eep");
                assert_eq!(r, root_str);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn untrusted_media_payload_rejected_for_filesystem_tier() {
        let payload = json!({
            "target": "export_dir",
            "metadata": {
                "text": "rm -rf /",
                "untrusted": true,
                "source": "transcript"
            }
        });

        let err = validate_untrusted_media_text(&payload, Tier::Filesystem).unwrap_err();
        assert_eq!(err.error_code(), "UntrustedMediaOrigin");

        // Allowed for read tier
        assert!(validate_untrusted_media_text(&payload, Tier::Read).is_ok());
    }
}
