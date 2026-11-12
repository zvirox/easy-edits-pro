// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Session write grants — the enforcement layer for every engine- and
//! wrapper-dispatched filesystem write that carries a caller-supplied path.
//!
//! Closes the security audit's priority item 5 (`docs/security-audit-report.md`):
//! engine-level commands (`file.save`, `interop.export_*`, `shortcuts.export`,
//! `export.presets.export`) and the raw-path Tauri wrappers (`start_export`,
//! `queue_export`, `export_frame`, bridge `render_start`) accepted arbitrary
//! write targets, a silent write-any-file-as-the-user primitive for an MCP
//! agent or compromised renderer following injected text.
//!
//! Policy (planning.md §6f): writes are allowed under the app data dir, the
//! app cache dir, the active project directory, or a session write grant.
//! The UI forwards every native Save/Export dialog pick through
//! `register_write_grant` before the driving command runs — the dialog pick
//! is the consent act, so legitimate flows keep working unchanged. Headless
//! CI harnesses opt in by setting [`BYPASS_ENV`] themselves. Reads
//! (`file.open`, import pickers) are out of scope.

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::path_validation::get_default_allowed_roots;

/// Headless-harness opt-in (set by our own CI acceptance runbooks): every
/// write target is accepted while it is set. A grant API on the bridge would
/// obey injected document text and reopen the original hole; an environment
/// var only the harness process can set does not.
pub const BYPASS_ENV: &str = "SATURN_ALLOW_ANY_WRITE_TARGETS";

/// Session-scoped set of exact, normalized write destinations granted through
/// a UI dialog pick (or tests). Grants survive repeated writes to the same
/// destination (`retry_export`, queued jobs, repeat saves) and never persist
/// across app runs; they are cleared on project switch (`file.open`,
/// `project.new`) and project close.
#[derive(Clone, Default)]
pub struct WriteGrants {
    grants: Arc<Mutex<Vec<PathBuf>>>,
}

impl WriteGrants {
    /// Grant the exact destination for the rest of the session. The path is
    /// normalized with [`normalize_target`] so the grant and a later command
    /// path match even when one side spells it slightly differently.
    pub fn grant(&self, path: &Path) -> Result<(), String> {
        let normalized = normalize_target(path)?;
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| "Write-grant store unavailable".to_string())?;
        if !grants.contains(&normalized) {
            grants.push(normalized);
        }
        Ok(())
    }

    /// Drop every grant (project switch / close).
    pub fn clear(&self) {
        if let Ok(mut grants) = self.grants.lock() {
            grants.clear();
        }
    }

    fn contains(&self, normalized: &Path) -> bool {
        self.grants
            .lock()
            .map(|grants| grants.iter().any(|granted| granted == normalized))
            .unwrap_or(false)
    }
}

/// Resolve `path` to an absolute route with no `..` components: the deepest
/// existing ancestor is canonicalized (resolving symlinks and dot-dots) and
/// the not-yet-existing remainder is appended lexically. Relative paths are
/// rejected — every caller-supplied write destination is either absolute or
/// attacker-controlled noise.
pub fn normalize_target(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Write target must be an absolute path".to_string());
    }
    let raw = PathBuf::from(path);
    if raw.components().any(|component| {
        matches!(component, Component::ParentDir | Component::CurDir)
    }) {
        return Err("Write target must not contain '.' or '..' components".to_string());
    }
    // Walk up to the deepest existing ancestor so a target whose parents are
    // created during the write (save_atomic/create_dir_all) is still anchored
    // to a real directory instead of trusting the caller's spelling.
    let mut ancestors = raw.clone();
    loop {
        match std::fs::canonicalize(&ancestors) {
            Ok(canonical) => {
                if ancestors == raw {
                    return Ok(canonical);
                }
                let suffix = raw
                    .strip_prefix(&ancestors)
                    .map_err(|error| error.to_string())?;
                return Ok(canonical.join(suffix));
            }
            Err(_) => match ancestors.parent() {
                Some(parent) if parent != ancestors => ancestors = parent.to_path_buf(),
                _ => {
                    return Ok(raw);
                }
            },
        }
    }
}

/// The engine-dispatch commands whose single `path` parameter ends up as a
/// raw filesystem write. `execute_command_with_state` consults this before
/// dispatching so the check happens before any state mutation.
pub fn engine_write_target_param(id: &str) -> Option<&'static str> {
    Some(match id {
        "file.save"
        | "interop.export_edl"
        | "interop.export_fcpxml"
        | "interop.export_xmeml_xml"
        | "interop.export_otio"
        | "shortcuts.export"
        | "export.presets.export"
        | "delivery.package" => "path",
        _ => return None,
    })
}

/// Core policy decision, split out so unit tests can exercise it without a
/// Tauri app. `roots` are the prebuilt allowed roots; `granted` answers the
/// session-grant lookup for the normalized target.
fn check_target(
    normalized: &Path,
    roots: &[PathBuf],
    granted: bool,
    bypass: bool,
    raw: &str,
) -> Result<(), String> {
    if bypass {
        return Ok(());
    }
    if roots.iter().any(|root| normalized.starts_with(root)) {
        return Ok(());
    }
    if granted {
        return Ok(());
    }
    Err(format!(
        "Write target '{raw}' is outside the allowed roots (app data, app cache, and the active project directory). Choose the destination in the app's Save/Export dialog, or set {BYPASS_ENV} for headless harnesses."
    ))
}

/// Enforce the write policy for a caller-supplied destination. Returns the
/// normalized path so callers can write the canonical spelling.
pub fn enforce_write_target(
    app: &AppHandle,
    path: &str,
    project_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    let raw = Path::new(path);
    let normalized = normalize_target(raw)?;
    let mut roots = get_default_allowed_roots(app);
    if let Some(dir) = project_dir {
        if let Ok(canonical) = std::fs::canonicalize(dir) {
            roots.push(canonical);
        } else {
            roots.push(dir.to_path_buf());
        }
    }
    let grants = app.state::<WriteGrants>();
    let granted = grants.contains(&normalized);
    check_target(
        &normalized,
        &roots,
        granted,
        std::env::var_os(BYPASS_ENV).is_some(),
        path,
    )?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn roots_for(dir: &Path) -> Vec<PathBuf> {
        vec![dir.to_path_buf()]
    }

    fn grants_with(paths: &[&Path]) -> WriteGrants {
        let store = WriteGrants::default();
        for path in paths {
            store.grant(path).unwrap();
        }
        store
    }

    #[test]
    fn writes_inside_root_pass() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("export").join("out.mp4");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        check_target(&normalize_target(&target).unwrap(), &roots_for(dir.path()), false, false, "x").unwrap();
    }

    #[test]
    fn writes_outside_root_rejected_with_named_error() {
        let inside = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("other.mp4");
        let error = check_target(
            &normalize_target(&target).unwrap(),
            &roots_for(inside.path()),
            false,
            false,
            target.to_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("outside the allowed roots"));
        assert!(error.contains(BYPASS_ENV));
    }

    #[test]
    fn granted_destination_passes() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("granted.mp4");
        fs::create_dir_all(&target.parent().unwrap()).unwrap();
        let target = normalize_target(&target).unwrap();
        let store = grants_with(&[target.as_path()]);
        check_target(&target, &roots_for(dir.path()), store.contains(&target), false, "x").unwrap();
    }

    #[test]
    fn bypass_env_accepts_anything() {
        let target = tempfile::tempdir().unwrap().path().join("night.mp4");
        check_target(&target, &roots_for(Path::new("/nonexistent-root")), false, true, "x").unwrap();
    }

    #[test]
    fn grandparents_of_new_files_normalize_against_disk() {
        // The file does not exist yet, but its existing parent anchors the
        // spelling: a `..`-free target inside /tmp must not resolve identically
        // to a symlinked sibling. Deepest-existing-ancestor walk keeps the
        // symbolic part.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("fresh").join("out.txt");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let normalized = normalize_target(&target).unwrap();
        assert_eq!(normalized, dir.path().join("fresh").join("out.txt"));
        assert!(normalized.is_absolute());
    }

    #[test]
    fn relative_and_dotdot_targets_are_rejected() {
        assert!(normalize_target(Path::new("relative/out.mp4")).is_err());
        let dir = tempfile::tempdir().unwrap();
        let sneaky = dir.path().join("..").join("escape.txt");
        assert!(normalize_target(&sneaky).is_err());
    }

    #[test]
    fn grants_clear_wipes_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("g.txt");
        fs::write(&target, b"x").unwrap();
        let store = grants_with(&[target.as_path()]);
        assert!(store.contains(&target));
        store.clear();
        assert!(!store.contains(&target));
    }

    #[test]
    fn engine_table_covers_exactly_the_audit_surface() {
        for id in [
            "file.save",
            "interop.export_edl",
            "interop.export_fcpxml",
            "interop.export_xmeml_xml",
            "interop.export_otio",
            "shortcuts.export",
            "export.presets.export",
            "delivery.package",
        ] {
            assert_eq!(engine_write_target_param(id), Some("path"), "{id}");
        }
        for id in [
            "file.open",
            "file.save_as",
            "export.presets.import",
            "media.import",
            "delivery.verify",
        ] {
            assert_eq!(engine_write_target_param(id), None, "{id}");
        }
    }

    #[test]
    fn env_var_name_is_stable() {
        assert_eq!(BYPASS_ENV, "SATURN_ALLOW_ANY_WRITE_TARGETS");
    }
}
