// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Headless project document lifecycle management for daemon sessions (AF9).

use std::fs;
use std::path::{Path, PathBuf};

use saturn_core::engine::EditorEngine;
use saturn_core::project::ProjectDocument;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::provenance::{ProvenanceError, validate_path_containment};
use crate::session::DaemonSession;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum ProjectError {
    #[error("ProjectNotOpen: no project is currently open in this session")]
    ProjectNotOpen,
    #[error("ProjectAlreadyOpen: a project is already open in this session; close it first")]
    ProjectAlreadyOpen,
    #[error("IoError: {message}")]
    IoError { message: String },
    #[error("SerializationError: {message}")]
    SerializationError { message: String },
    #[error("ProvenanceError: {0}")]
    Provenance(#[from] ProvenanceError),
}

impl ProjectError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::ProjectNotOpen => "ProjectNotOpen",
            Self::ProjectAlreadyOpen => "ProjectAlreadyOpen",
            Self::IoError { .. } => "IoError",
            Self::SerializationError { .. } => "SerializationError",
            Self::Provenance(p) => p.error_code(),
        }
    }
}

pub fn project_open(
    session: &mut DaemonSession,
    path_opt: Option<&str>,
    project_root: Option<&Path>,
) -> Result<Value, ProjectError> {
    if session.engine.is_some() {
        return Err(ProjectError::ProjectAlreadyOpen);
    }

    let effective_root = project_root.or(session.project_root.as_deref());

    let (engine, final_path) = match path_opt {
        Some(p) if !p.trim().is_empty() => {
            let candidate = PathBuf::from(p.trim());
            let contained = validate_path_containment(&candidate, effective_root)?;
            let doc = if contained.exists() {
                let content = fs::read_to_string(&contained).map_err(|e| ProjectError::IoError {
                    message: format!("failed to read project file {}: {e}", contained.display()),
                })?;
                serde_json::from_str::<ProjectDocument>(&content).map_err(|e| {
                    ProjectError::SerializationError {
                        message: format!("invalid project document: {e}"),
                    }
                })?
            } else {
                ProjectDocument::default()
            };
            let mut eng = EditorEngine::default();
            eng.restore_recovery(doc, Some(contained.clone())).map_err(|e| ProjectError::IoError {
                message: e.to_string(),
            })?;
            eng.mark_saved();
            (eng, Some(contained))
        }
        _ => {
            let mut eng = EditorEngine::default();
            eng.mark_saved();
            (eng, None)
        }
    };

    let doc = engine.document();
    let name = doc.project.name.clone();
    let duration = doc.project.sequence.duration().0;
    let tracks_count = doc.project.sequence.tracks.len();
    let clips_count = doc
        .project
        .sequence
        .tracks
        .iter()
        .map(|t| t.clips.len())
        .sum::<usize>();

    session.engine = Some(engine);

    Ok(json!({
        "opened": true,
        "session_id": session.id,
        "path": final_path.map(|p| p.display().to_string()),
        "name": name,
        "duration_ticks": duration,
        "tracks_count": tracks_count,
        "clips_count": clips_count,
        "is_dirty": false,
    }))
}

pub fn project_close(session: &mut DaemonSession) -> Result<Value, ProjectError> {
    if session.engine.is_none() {
        return Err(ProjectError::ProjectNotOpen);
    }

    session.cleanup();
    session.engine = None;

    Ok(json!({
        "closed": true,
        "session_id": session.id,
    }))
}

pub fn project_status(session: &DaemonSession) -> Value {
    match session.engine.as_ref() {
        None => json!({
            "is_open": false,
            "session_id": session.id,
            "stopped": session.stopped,
            "generation": session.generation,
        }),
        Some(engine) => {
            let doc = engine.document();
            json!({
                "is_open": true,
                "session_id": session.id,
                "path": engine.current_path().map(|p| p.display().to_string()),
                "name": doc.project.name,
                "is_dirty": engine.is_dirty(),
                "duration_ticks": doc.project.sequence.duration().0,
                "tracks_count": doc.project.sequence.tracks.len(),
                "clips_count": doc.project.sequence.tracks.iter().map(|t| t.clips.len()).sum::<usize>(),
                "active_branch": engine.current_branch(),
                "stopped": session.stopped,
                "generation": session.generation,
            })
        }
    }
}

pub fn project_save(
    session: &mut DaemonSession,
    path_opt: Option<&str>,
    project_root: Option<&Path>,
) -> Result<Value, ProjectError> {
    let engine = session.engine.as_mut().ok_or(ProjectError::ProjectNotOpen)?;

    let target_path_str = match path_opt {
        Some(p) if !p.trim().is_empty() => p.trim(),
        _ => match engine.current_path() {
            Some(p) => p.to_str().unwrap_or_default(),
            None => {
                return Err(ProjectError::IoError {
                    message: "Save requires a project path. Use --path or specify path in params."
                        .into(),
                });
            }
        },
    };

    let candidate = PathBuf::from(target_path_str);
    let effective_root = project_root.or(session.project_root.as_deref());
    let contained = validate_path_containment(&candidate, effective_root)?;

    if let Some(parent) = contained.parent() {
        fs::create_dir_all(parent).map_err(|e| ProjectError::IoError {
            message: format!("failed to create directories for {}: {e}", contained.display()),
        })?;
    }

    let serialized = serde_json::to_string_pretty(engine.document()).map_err(|e| {
        ProjectError::SerializationError {
            message: e.to_string(),
        }
    })?;

    fs::write(&contained, serialized).map_err(|e| ProjectError::IoError {
        message: format!("failed to write project file {}: {e}", contained.display()),
    })?;

    engine.mark_saved();

    Ok(json!({
        "saved": true,
        "session_id": session.id,
        "path": contained.display().to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn headless_project_open_status_save_close_lifecycle() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let proj_path = root.join("test.eep");

        let mut session = DaemonSession::new("lifecycle-test", "agent-1", Some(root.to_path_buf()), 60, None);

        // 1. Initial status: not open
        let st0 = project_status(&session);
        assert_eq!(st0["is_open"], false);

        // 2. Open project
        let open_res = project_open(&mut session, Some(proj_path.to_str().unwrap()), Some(root)).unwrap();
        assert_eq!(open_res["opened"], true);

        // 3. Status is open
        let st1 = project_status(&session);
        assert_eq!(st1["is_open"], true);
        assert_eq!(st1["tracks_count"].as_u64().unwrap(), 2);

        // 4. Save project
        let save_res = project_save(&mut session, Some(proj_path.to_str().unwrap()), Some(root)).unwrap();
        assert_eq!(save_res["saved"], true);
        assert!(proj_path.exists());

        // 5. Close project
        let close_res = project_close(&mut session).unwrap();
        assert_eq!(close_res["closed"], true);

        // 6. Status is closed again
        let st2 = project_status(&session);
        assert_eq!(st2["is_open"], false);
    }
}
