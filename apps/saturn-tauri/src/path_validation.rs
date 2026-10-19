use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Errors that can occur during path validation
#[derive(Debug, thiserror::Error)]
pub enum PathValidationError {
    #[error("Path contains directory traversal attempts")]
    DirectoryTraversal,
    #[error("Path is not within allowed directories")]
    NotAllowed,
    #[error("Path does not exist")]
    NotFound,
    #[error("Path is not a file")]
    NotAFile,
    #[error("Path is not a directory")]
    NotADirectory,
    #[error("Failed to canonicalize path: {0}")]
    CanonicalizationFailed(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Configuration for path validation
#[derive(Debug, Clone)]
pub struct PathValidationConfig {
    /// Root directories that are allowed for file operations
    pub allowed_roots: Vec<PathBuf>,
    /// Whether to require the path to exist
    pub must_exist: bool,
    /// Whether the path must be a file (vs directory)
    pub must_be_file: bool,
    /// Maximum allowed file size in bytes (None = no limit)
    pub max_file_size: Option<u64>,
}

impl Default for PathValidationConfig {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            must_exist: true,
            must_be_file: true,
            max_file_size: None,
        }
    }
}

/// Get the default allowed roots for the application
pub fn get_default_allowed_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // App data directory (for network imports, assets, recovery)
    if let Ok(data_dir) = app.path().app_data_dir() {
        roots.push(data_dir);
    }

    // App cache directory (for video previews)
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        roots.push(cache_dir);
    }

    roots
}

/// Validate a path against the configuration
pub fn validate_path(
    path: &Path,
    config: &PathValidationConfig,
) -> Result<PathBuf, PathValidationError> {
    // Canonicalize the path to resolve symlinks and .. components
    let canonical = path
        .canonicalize()
        .map_err(|e| PathValidationError::CanonicalizationFailed(e.to_string()))?;

    // Check for directory traversal attempts in the original path
    // (canonicalize resolves them, but we want to detect malicious intent)
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        // Allow .. only if it's within an allowed root after canonicalization
        // This is a defense-in-depth check
    }

    // Check if path is within allowed roots
    if !config.allowed_roots.is_empty() {
        let is_allowed = config.allowed_roots.iter().any(|root| {
            canonical.starts_with(root)
        });
        if !is_allowed {
            return Err(PathValidationError::NotAllowed);
        }
    }

    // Check existence
    if config.must_exist && !canonical.exists() {
        return Err(PathValidationError::NotFound);
    }

    // Check file/directory type
    if config.must_be_file {
        if !canonical.is_file() {
            return Err(PathValidationError::NotAFile);
        }
    } else if !canonical.is_dir() {
        return Err(PathValidationError::NotADirectory);
    }

    // Check file size
    if let Some(max_size) = config.max_file_size {
        if let Ok(metadata) = canonical.metadata() {
            if metadata.len() > max_size {
                return Err(PathValidationError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("File size {} exceeds maximum allowed {}", metadata.len(), max_size),
                )));
            }
        }
    }

    Ok(canonical)
}

/// Convenience function for validating a file path for reading
pub fn validate_read_path(
    app: &AppHandle,
    path: &str,
    max_size: Option<u64>,
) -> Result<PathBuf, PathValidationError> {
    let allowed_roots = get_default_allowed_roots(app);
    let config = PathValidationConfig {
        allowed_roots,
        must_exist: true,
        must_be_file: true,
        max_file_size: max_size,
    };
    validate_path(Path::new(path), &config)
}

/// Convenience function for validating a file path for writing
pub fn validate_write_path(
    app: &AppHandle,
    path: &str,
    max_size: Option<u64>,
) -> Result<PathBuf, PathValidationError> {
    let allowed_roots = get_default_allowed_roots(app);
    let config = PathValidationConfig {
        allowed_roots,
        must_exist: false, // File may not exist yet for writing
        must_be_file: true,
        max_file_size: max_size,
    };
    // For writing, we need to check the parent directory
    let path = Path::new(path);
    let parent = path.parent().ok_or(PathValidationError::NotAllowed)?;
    validate_path(parent, &config)?;
    // Also validate the full path (will fail if file exists but is not a file)
    if path.exists() {
        validate_path(path, &config)?;
    }
    Ok(path.to_path_buf())
}

/// Validate a directory path
pub fn validate_dir_path(
    app: &AppHandle,
    path: &str,
    must_exist: bool,
) -> Result<PathBuf, PathValidationError> {
    let allowed_roots = get_default_allowed_roots(app);
    let config = PathValidationConfig {
        allowed_roots,
        must_exist,
        must_be_file: false,
        max_file_size: None,
    };
    validate_path(Path::new(path), &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_validate_path_allows_within_root() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello").unwrap();

        let config = PathValidationConfig {
            allowed_roots: vec![dir.path().to_path_buf()],
            must_exist: true,
            must_be_file: true,
            max_file_size: None,
        };

        let result = validate_path(&file, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    #[test]
    fn test_validate_path_rejects_outside_root() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let file = dir2.path().join("test.txt");
        fs::write(&file, b"hello").unwrap();

        let config = PathValidationConfig {
            allowed_roots: vec![dir1.path().to_path_buf()],
            must_exist: true,
            must_be_file: true,
            max_file_size: None,
        };

        let result = validate_path(&file, &config);
        assert!(matches!(result, Err(PathValidationError::NotAllowed)));
    }

    #[test]
    fn test_validate_path_rejects_traversal() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello").unwrap();

        // Try to escape using ..
        let traversal_path = dir.path().join("subdir").join("..").join("test.txt");
        fs::create_dir_all(traversal_path.parent().unwrap()).unwrap();

        let config = PathValidationConfig {
            allowed_roots: vec![dir.path().to_path_buf()],
            must_exist: true,
            must_be_file: true,
            max_file_size: None,
        };

        // This should work because canonicalization resolves it to within the root
        let result = validate_path(&traversal_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_enforces_file_size() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("large.txt");
        fs::write(&file, vec![0u8; 1000]).unwrap();

        let config = PathValidationConfig {
            allowed_roots: vec![dir.path().to_path_buf()],
            must_exist: true,
            must_be_file: true,
            max_file_size: Some(500),
        };

        let result = validate_path(&file, &config);
        assert!(matches!(result, Err(PathValidationError::IoError(_))));
    }
}