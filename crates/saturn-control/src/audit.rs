// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Bounded JSONL audit logging with cryptographic argument hashing (AF9 / AGENT_FIRST_PLAN §2.8).
//!
//! Enforces that no secrets, tokens, or raw sensitive arguments ever appear
//! in plaintext in the log file. Records timestamp, session_id, actor,
//! tool_or_command, argument_hash (SHA-256), tier, duration_ms, and result.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const DEFAULT_MAX_AUDIT_ENTRIES: usize = 2000;

/// Structured audit record written to `audit.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub timestamp: String,
    pub session_id: String,
    pub actor: String,
    pub tool_or_command: String,
    pub argument_hash: String,
    pub tier: String,
    pub duration_ms: u64,
    pub result: String,
}

/// Compute a 64-character SHA-256 hex digest of argument values.
/// Sanitizes any sensitive fields (like tokens/passwords) before hashing,
/// and guarantees no plaintext arguments are logged.
pub fn compute_argument_hash(params: &Value) -> String {
    let sanitized = sanitize_for_hashing(params);
    let canonical = serde_json::to_string(&sanitized).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn sanitize_for_hashing(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let lower = k.to_lowercase();
                if lower.contains("token")
                    || lower.contains("secret")
                    || lower.contains("password")
                    || lower.contains("api_key")
                    || lower.contains("auth")
                {
                    out.insert(k.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    out.insert(k.clone(), sanitize_for_hashing(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sanitize_for_hashing).collect()),
        other => other.clone(),
    }
}

/// Thread-safe bounded audit logger with automatic rotation.
pub struct AuditLogger {
    path: Option<PathBuf>,
    max_entries: usize,
    entry_count: usize,
    file: Option<File>,
}

impl AuditLogger {
    pub fn new(path: Option<PathBuf>, max_entries: usize) -> std::io::Result<Self> {
        let max_entries = if max_entries == 0 {
            DEFAULT_MAX_AUDIT_ENTRIES
        } else {
            max_entries
        };

        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let entry_count = count_lines(&path);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            Ok(Self {
                path: Some(path),
                max_entries,
                entry_count,
                file: Some(file),
            })
        } else {
            Ok(Self {
                path: None,
                max_entries,
                entry_count: 0,
                file: None,
            })
        }
    }

    pub fn log(&mut self, record: &AuditRecord) -> std::io::Result<()> {
        if self.file.is_none() {
            return Ok(());
        }

        if self.entry_count >= self.max_entries {
            self.rotate()?;
        }

        let line = serde_json::to_string(record)?;
        if let Some(ref mut file) = self.file {
            writeln!(file, "{line}")?;
            file.flush()?;
            self.entry_count += 1;
        }
        Ok(())
    }

    pub fn record(
        &mut self,
        session_id: &str,
        actor: &str,
        tool_or_command: &str,
        params: &Value,
        tier: &str,
        duration_ms: u64,
        result: &str,
    ) -> std::io::Result<AuditRecord> {
        let record = AuditRecord {
            timestamp: Utc::now().to_rfc3339(),
            session_id: session_id.to_string(),
            actor: actor.to_string(),
            tool_or_command: tool_or_command.to_string(),
            argument_hash: compute_argument_hash(params),
            tier: tier.to_string(),
            duration_ms,
            result: result.to_string(),
        };
        self.log(&record)?;
        Ok(record)
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        let Some(ref path) = self.path else {
            return Ok(());
        };
        self.file = None;
        let rotated = path.with_extension("jsonl.1");
        if path.exists() {
            let _ = std::fs::remove_file(&rotated);
            std::fs::rename(path, &rotated)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        self.file = Some(file);
        self.entry_count = 0;
        Ok(())
    }
}

fn count_lines(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }
    match File::open(path) {
        Ok(file) => BufReader::new(file).lines().count(),
        Err(_) => 0,
    }
}

pub type SharedAuditLogger = Arc<Mutex<AuditLogger>>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn argument_hash_is_deterministic_and_redacts_secrets() {
        let params1 = json!({
            "command": "timeline.add_clip",
            "token": "secret-super-token-123",
            "duration": 500
        });
        let params2 = json!({
            "command": "timeline.add_clip",
            "token": "different-token-abc",
            "duration": 500
        });

        let hash1 = compute_argument_hash(&params1);
        let hash2 = compute_argument_hash(&params2);

        assert_eq!(hash1.len(), 64);
        assert_eq!(hash1, hash2, "Redacted tokens yield identical argument hash");
    }

    #[test]
    fn audit_logger_writes_structured_jsonl_and_rotates() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        let mut logger = AuditLogger::new(Some(log_path.clone()), 3).unwrap();

        for i in 0..3 {
            logger
                .record(
                    &format!("sess-{i}"),
                    "agent-1",
                    "edit.undo",
                    &json!({"step": i}),
                    "edit",
                    10 + i as u64,
                    "ok",
                )
                .unwrap();
        }

        assert_eq!(logger.entry_count(), 3);
        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(contents.lines().count(), 3);

        // Fourth entry triggers rotation
        logger
            .record(
                "sess-3",
                "agent-1",
                "edit.redo",
                &json!({"step": 3}),
                "edit",
                15,
                "ok",
            )
            .unwrap();

        assert_eq!(logger.entry_count(), 1);
        assert!(log_path.with_extension("jsonl.1").exists());
        let rotated = std::fs::read_to_string(log_path.with_extension("jsonl.1")).unwrap();
        assert_eq!(rotated.lines().count(), 3);

        let fresh = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(fresh.lines().count(), 1);
        assert!(fresh.contains("sess-3"));
    }
}
