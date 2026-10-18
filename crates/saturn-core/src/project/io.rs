use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

impl ProjectDocument {
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
        let serialized = serde_json::to_vec_pretty(self)
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
        let mut file = File::open(path).map_err(|e| format!("Could not open project: {e}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Could not read project: {e}"))?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Project file is not valid JSON: {e}"))?;
        migrate_project(&mut value)?;
        let document: Self = serde_json::from_value(value)
            .map_err(|e| format!("Project file has an invalid structure: {e}"))?;
        document.validate()?;
        Ok(document)
    }
}
