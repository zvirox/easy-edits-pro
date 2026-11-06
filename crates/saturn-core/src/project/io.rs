// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

impl ProjectDocument {
    /// Schema v15: media that lives inside the project folder is stored with
    /// a path relative to the project file, so the whole folder can move.
    /// Media outside the project folder keeps its absolute path.
    fn for_saving(&self, project_dir: &Path) -> Self {
        let mut out = self.clone();
        for media in &mut out.project.media {
            relativize(&mut media.path, project_dir);
            if let Some(proxy) = &mut media.proxy_path {
                relativize(proxy, project_dir);
            }
        }
        out
    }

    /// In-memory documents always carry resolved paths: relative entries from
    /// the file are joined against the project file's folder. A moved folder
    /// therefore re-resolves media that moved with it.
    fn resolve_saved_paths(&mut self, project_dir: &Path) {
        for media in &mut self.project.media {
            absolutize(&mut media.path, project_dir);
            if let Some(proxy) = &mut media.proxy_path {
                absolutize(proxy, project_dir);
            }
        }
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("Could not create project folder: {e}"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| "Project path has no file name".to_string())?;
        let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            suffix
        ));
        let serialized = serde_json::to_vec_pretty(&self.for_saving(parent))
            .map_err(|e| format!("Could not serialize project: {e}"))?;

        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|e| format!("Could not create temporary project file: {e}"))?;
            file.write_all(&serialized)
                .map_err(|e| format!("Could not write project file: {e}"))?;
            file.write_all(b"\n")
                .map_err(|e| format!("Could not finish project file: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("Could not flush project file: {e}"))?;
            drop(file);
            fs::rename(&temporary_path, path)
                .map_err(|e| format!("Could not replace project file: {e}"))?;
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        // A project file is structured JSON; anything beyond this bound is
        // not a project - refuse it before reading it into memory.
        const MAX_PROJECT_BYTES: u64 = 256 * 1024 * 1024;
        let mut file = File::open(path).map_err(|e| format!("Could not open project: {e}"))?;
        if let Ok(metadata) = file.metadata() {
            if metadata.len() > MAX_PROJECT_BYTES {
                return Err(format!(
                    "Project file is too large: {} bytes (maximum is {MAX_PROJECT_BYTES})",
                    metadata.len()
                ));
            }
        }
        let mut bytes = Vec::new();
        // take() bounds the read even if the file grows between the stat and
        // the read.
        (&mut file)
            .take(MAX_PROJECT_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Could not read project: {e}"))?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Project file is not valid JSON: {e}"))?;
        migrate_project(&mut value)?;
        let mut document: Self = serde_json::from_value(value)
            .map_err(|e| format!("Project file has an invalid structure: {e}"))?;
        document.resolve_saved_paths(path.parent().unwrap_or_else(|| Path::new(".")));
        document.validate()?;
        Ok(document)
    }
}

/// Store an absolute path under `project_dir` as a relative path. Any path
/// that is not under `project_dir` is written through unchanged.
fn relativize(path: &mut PathBuf, project_dir: &Path) {
    if path.is_absolute() {
        if let Ok(relative) = path.strip_prefix(project_dir) {
            // The prefix match guarantees the remainder can never contain `..`.
            if !relative.as_os_str().is_empty() {
                *path = relative.to_path_buf();
            }
        }
    }
}

/// Join a relative stored path against the project folder on load.
fn absolutize(path: &mut PathBuf, project_dir: &Path) {
    if !path.is_absolute() && !path.as_os_str().is_empty() {
        let joined = project_dir.join(&*path);
        *path = joined;
    }
}
