// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Trusted plugin root configuration and path validation.
//!
//! A plugin library is only loadable when it resolves to a real file that
//! lives under one of the canonical trusted roots:
//!
//! * `$HOME/.clap` and the `$XDG_DATA_HOME`-resolved `clap` directory
//! * `/usr/lib/clap`
//! * `/usr/local/lib/clap`
//! * `SATURN_PLUGIN_ROOT` entries (colon-separated, opt-in; the operator is
//!   explicitly attesting those directories hold audited plugins)
//!
//! This module lives in `saturn-core` so both the command/validation layer
//! and the plugin host crate (`saturn-audio-dsp`) enforce exactly one trust
//! policy.

use std::path::PathBuf;

/// Canonical plugin-library extensions accepted by the host.
pub const PLUGIN_SUFFIXES: [&str; 2] = [".clap", ".so"];

/// Hard ceiling on the number of plugin slots per track's DSP chain.
/// Both the command layer and the plugin host enforce this cap.
pub const MAX_TRACK_SLOTS: usize = 64;

/// Plugin formats the Saturn render engine can host today; the remainder
/// of the schema's format tags stay valid for project round-tripping but
/// can never be instantiated.
pub const HOSTED_PLUGIN_FORMATS: [&str; 1] = ["clap"];

/// Why a plugin path was rejected by the trust policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DspPathError {
    /// The recorded plugin path does not exist (or is not a file).
    NotFound(String),
    /// The path is absolute-but-untrusted (wrong suffix, outside every
    /// trusted root, or relative).
    Untrusted(String),
}

impl std::fmt::Display for DspPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DspPathError::NotFound(path) => write!(f, "plugin path not found: {path}"),
            DspPathError::Untrusted(reason) => write!(f, "{reason}"),
        }
    }
}

/// Returns the default scan roots plus any operator-opt-in
/// `SATURN_PLUGIN_ROOT` entries.
pub fn trusted_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(&home).join(".clap"));
        let xdg = std::env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty());
        match xdg {
            Some(xdg) => roots.push(PathBuf::from(xdg).join("clap")),
            None => roots.push(PathBuf::from(home).join(".local/share/clap")),
        }
    }

    roots.push(PathBuf::from("/usr/lib/clap"));
    roots.push(PathBuf::from("/usr/local/lib/clap"));

    if let Ok(extra) = std::env::var("SATURN_PLUGIN_ROOT") {
        for entry in extra.split(':') {
            if !entry.is_empty() {
                roots.push(PathBuf::from(entry));
            }
        }
    }

    roots
}

/// Canonicalizes `path` and checks:
///
/// 1. the resolved file exists,
/// 2. it has an accepted plugin-library suffix,
/// 3. it canonically sits inside a trusted root.
///
/// Returns the canonicalized path. Parent/traversal tricks (`..`,
/// symlinked outside a root) fail the root containment check because
/// `canonicalize` resolves them before containment is compared.
pub fn validate_plugin_path(path: &str) -> Result<PathBuf, DspPathError> {
    let candidate = PathBuf::from(path);
    if !candidate.is_absolute() {
        return Err(DspPathError::Untrusted(format!(
            "plugin path is outside the trusted plugin roots: {path} (only absolute plugin paths are accepted)"
        )));
    }

    let ends_known = PLUGIN_SUFFIXES
        .iter()
        .any(|suffix| candidate.to_str().is_some_and(|p| p.ends_with(suffix)));
    if !ends_known {
        return Err(DspPathError::Untrusted(format!(
            "plugin path is outside the trusted plugin roots: {path} (plugin libraries must carry a known suffix: {})",
            PLUGIN_SUFFIXES.join(", ")
        )));
    }

    let canonical =
        std::fs::canonicalize(&candidate).map_err(|_| DspPathError::NotFound(path.to_string()))?;
    if !canonical.is_file() {
        return Err(DspPathError::NotFound(
            canonical.display().to_string(),
        ));
    }

    for root in trusted_roots() {
        let Ok(root_canonical) = std::fs::canonicalize(&root) else {
            continue;
        };
        if canonical.starts_with(&root_canonical) {
            return Ok(canonical);
        }
    }

    let roots_hint = trusted_roots()
        .iter()
        .map(|root| root.display().to_string())
        .chain(
            std::env::var("SATURN_PLUGIN_ROOT")
                .ok()
                .map(|extra| format!("+ SATURN_PLUGIN_ROOT:{extra}")),
        )
        .collect::<Vec<_>>()
        .join(", ");
    Err(DspPathError::Untrusted(format!(
        "plugin path is outside the trusted plugin roots: {path} (trusted roots checked: {roots_hint})"
    )))
}

/// Test-only accessor: the engine's DSP command tests must serialize their
/// env swaps with the trust tests through this same gate.
#[cfg(test)]
pub(crate) fn dsp_trust_gate() -> std::sync::MutexGuard<'static, ()> {
    gate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_paths() {
        let _guard = gate();
        set_root(None);
        assert!(matches!(
            validate_plugin_path("plugins/gain.so"),
            Err(DspPathError::Untrusted(_))
        ));
        set_root(None);
    }

    #[test]
    fn rejects_wrong_suffix_even_inside_root() {
        let _guard = gate();
        let root = std::env::temp_dir().join("eep-dsp-trust-test");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("evil.sh");
        std::fs::write(&file, b"").unwrap();
        let canonical = std::fs::canonicalize(&file).unwrap();
        set_root(Some(root.to_str().unwrap().to_string()));
        assert!(matches!(
            validate_plugin_path(canonical.to_str().unwrap()),
            Err(DspPathError::Untrusted(_))
        ));
        set_root(None);
    }

    #[test]
    fn accepts_existing_file_under_opt_in_root() {
        let _guard = gate();
        let root = std::env::temp_dir().join("eep-dsp-trust-test-ok");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("gain.so");
        std::fs::write(&file, b"").unwrap();
        let canonical = std::fs::canonicalize(&file).unwrap();
        set_root(Some(root.to_str().unwrap().to_string()));
        let accepted = validate_plugin_path(canonical.to_str().unwrap()).unwrap();
        assert_eq!(accepted, canonical);
        set_root(None);
    }

    #[test]
    fn rejects_path_outside_root() {
        let _guard = gate();
        let file = std::env::temp_dir().join("eep-dsp-trust-outside.so");
        std::fs::write(&file, b"").unwrap();
        let canonical = std::fs::canonicalize(&file).unwrap();
        // A root that does not contain the file; /tmp itself is never a
        // trusted plugin root for this policy.
        set_root(Some(
            std::env::temp_dir()
                .join("eep-dsp-trust-nowhere")
                .to_str()
                .unwrap()
                .to_string(),
        ));
        assert!(matches!(
            validate_plugin_path(canonical.to_str().unwrap()),
            Err(DspPathError::Untrusted(_))
        ));
        // Restore the shared default so sibling suites start clean.
        set_root(None);
    }
}

// Test-only env gate shared by dsp_trust tests and the engine's dsp.*
// command tests: swapping SATURN_PLUGIN_ROOT mutates process-global state,
// and every such test must hold this gate for its whole body.
#[cfg(test)]
static TRUST_ENV_GATE: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

#[cfg(test)]
fn gate() -> std::sync::MutexGuard<'static, ()> {
    TRUST_ENV_GATE
        .get_or_init(std::sync::Mutex::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Requires the caller to hold [`gate()`]; used by every env-mutating test.
#[cfg(test)]
fn set_root(value: Option<String>) {
    // SAFETY: set_var/remove_var are unsafe because of cross-thread env
    // mutation; every env-mutating test here (and in the engine's dsp test
    // module) holds the process-wide gate for its whole body, which
    // serializes all of them.
    unsafe {
        match value {
            Some(v) => std::env::set_var("SATURN_PLUGIN_ROOT", v),
            None => std::env::remove_var("SATURN_PLUGIN_ROOT"),
        }
    }
}
