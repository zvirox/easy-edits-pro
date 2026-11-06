// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Timeline branch commands (AI-native plan Packet 0.2).
//!
//! Branches are copy-on-write snapshots of the whole `Project` edit state.
//! Merge v1 is whole-branch accept/reject only (plan review §4.11); there is
//! no range merging and no conflict logic yet — that gate stays closed until
//! a written conflict rule exists.

use super::*;
use crate::branch::{diff_projects, BranchDiffReport, TimelineBranch, MAIN_BRANCH};
use serde_json::{json, Value};

/// Wall-clock epoch ms, used only for branch creation stamps.
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

impl EditorEngine {
    /// The branch record backing the live state. The active viewport branch
    /// name may be stale after undo/redo; operations fall back to the
    /// implicit main branch instead of erroring on a ghost name.
    fn active_branch_name(&self) -> String {
        let active = &self.viewport.active_branch;
        if active == MAIN_BRANCH || self.document.branches.iter().any(|b| b.name == *active) {
            active.clone()
        } else {
            MAIN_BRANCH.to_owned()
        }
    }

    /// Serialize the live project edit state for storage in a branch record.
    fn snapshot_state(&self) -> Result<Value, DispatchError> {
        serde_json::to_value(&self.document.project)
            .map_err(|error| DispatchError::Operation(format!("Branch snapshot failed: {error}")))
    }

    /// Restore a branch snapshot into the live document state. Snapshots are
    /// always written from a live `Project` in this process, so a failure
    /// here means corrupted data, never expected input.
    fn restore_snapshot(&mut self, state: Value) -> Result<(), DispatchError> {
        let project: crate::project::Project = serde_json::from_value(state).map_err(|error| {
            DispatchError::Operation(format!("Branch snapshot is unreadable: {error}"))
        })?;
        self.document.project = project;
        Ok(())
    }

    fn now_branch(
        &self,
        name: String,
        note: String,
        parent: Option<String>,
        state: Value,
    ) -> TimelineBranch {
        TimelineBranch {
            // Branch ids are bookkeeping only (media/tracks use project
            // next_id); the highest existing record plus one.
            id: self
                .document
                .branches
                .iter()
                .map(|branch| branch.id)
                .max()
                .unwrap_or(0)
                + 1,
            name,
            note,
            parent,
            created_at_unix_ms: now_epoch_ms(),
            state,
        }
    }

    pub(super) fn dispatch_branch(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.branch_create" => Some((|| {
                let params: BranchCreateParams = parse_params(params)?;
                let name = TimelineBranch::validate_name(&params.name)
                    .map_err(DispatchError::InvalidParameters)?;
                if self.document.branches.iter().any(|b| b.name == name) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Branch `{name}` already exists"
                    )));
                }
                let parent = self.active_branch_name();
                let state = self.snapshot_state()?;
                self.checkpoint();
                self.document.branches.push(self.now_branch(
                    name.clone(),
                    params.note.trim().to_owned(),
                    Some(parent),
                    state,
                ));
                let created_at_unix_ms = self
                    .document
                    .branches
                    .iter()
                    .find(|b| b.name == name)
                    .map(|b| b.created_at_unix_ms)
                    .unwrap_or_default();
                Ok(json!({"created": true, "branch": name, "created_at_unix_ms": created_at_unix_ms}))
            })()),
            "project.branch_list" => Some(Ok(json!({
                "active": self.active_branch_name(),
                "branches": self.document.branches.iter().map(|branch| json!({
                    "name": branch.name,
                    "note": branch.note,
                    "parent": branch.parent,
                    "created_at_unix_ms": branch.created_at_unix_ms,
                })).collect::<Vec<_>>(),
            }))),
            "project.branch_switch" => Some((|| {
                let params: BranchSwitchParams = parse_params(params)?;
                let name = if params.name.eq_ignore_ascii_case(MAIN_BRANCH) {
                    // The live state is the main branch; switching to it is
                    // always possible even though no record may exist yet.
                    MAIN_BRANCH.to_owned()
                } else {
                    TimelineBranch::validate_name(&params.name)
                        .map_err(DispatchError::InvalidParameters)?
                };
                if name != MAIN_BRANCH {
                    self.document
                        .branches
                        .iter()
                        .find(|branch| branch.name == name)
                        .ok_or_else(|| {
                            DispatchError::InvalidParameters(format!(
                                "Branch `{name}` does not exist"
                            ))
                        })?;
                }
                if name == self.active_branch_name() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Branch `{name}` is already active"
                    )));
                }
                self.checkpoint();
                // The currently active branch keeps the live state before it
                // is swapped away, so switching back restores recent work.
                let current = self.active_branch_name();
                let current_state = self.snapshot_state()?;
                match self
                    .document
                    .branches
                    .iter_mut()
                    .find(|branch| branch.name == current)
                {
                    Some(existing) => existing.state = current_state,
                    None => {
                        // First switch away (main included): the live state
                        // gets a record — for main a *reserved* record of the
                        // same name, which project validation allows.
                        self.document.branches.push(self.now_branch(
                            current,
                            String::new(),
                            None,
                            current_state,
                        ));
                    }
                }
                let target_state = self
                    .document
                    .branches
                    .iter()
                    .find(|branch| branch.name == name)
                    .map(|branch| branch.state.clone());
                match target_state {
                    Some(state) => self.restore_snapshot(state)?,
                    // Main is the implicit live branch: with no parked record
                    // the live state simply continues as main.
                    None if name == MAIN_BRANCH => {}
                    // Unreachable for record branches (checked above), but be
                    // strict rather than panic on it.
                    None => {
                        return Err(DispatchError::Operation(format!(
                            "Branch `{name}` has no stored state"
                        )))
                    }
                }
                self.viewport.active_branch = name;
                Ok(json!({"switched": true, "active_branch": self.viewport.active_branch}))
            })()),
            "project.branch_diff" => Some((|| {
                let params: BranchDiffParams = parse_params(params)?;
                let target = self
                    .document
                    .branches
                    .iter()
                    .find(|branch| branch.name == params.branch)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Branch `{}` does not exist",
                            params.branch
                        ))
                    })?;
                let live = self.snapshot_state()?;
                let report: BranchDiffReport = diff_projects(&target.name, &target.state, &live);
                serde_json::to_value(report)
                    .map_err(|error| DispatchError::Operation(error.to_string()))
            })()),
            "project.branch_merge" => Some((|| {
                let params: BranchMergeParams = parse_params(params)?;
                let target = self
                    .document
                    .branches
                    .iter()
                    .find(|branch| branch.name == params.branch)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Branch `{}` does not exist",
                            params.branch
                        ))
                    })?
                    .clone();
                if target.name == self.active_branch_name() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Branch `{}` is the active branch; switch away before merging",
                        target.name
                    )));
                }
                self.checkpoint();
                match params.action {
                    BranchMergeAction::Accept => {
                        let state = target.state;
                        self.restore_snapshot(state)?;
                        self.document
                            .branches
                            .retain(|branch| branch.name != target.name);
                        Ok(json!({
                            "merged": "accept",
                            "branch": target.name,
                            "active_branch": self.active_branch_name(),
                            "note": "whole-branch accept (merge v1); no range merging",
                        }))
                    }
                    BranchMergeAction::Reject => {
                        self.document
                            .branches
                            .retain(|branch| branch.name != target.name);
                        Ok(json!({
                            "merged": "reject",
                            "branch": target.name,
                            "active_branch": self.active_branch_name(),
                            "note": "whole-branch reject (merge v1); branch discarded",
                        }))
                    }
                }
            })()),
            "project.fingerprint_media" => Some((|| {
                let params: FingerprintMediaParams = parse_params(params)?;
                for id in &params.media_ids {
                    if !self.document.project.media.iter().any(|m| m.id == *id) {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Media item {id} does not exist"
                        )));
                    }
                }
                let mut fingerprinted = 0u32;
                let mut failed: Vec<Value> = Vec::new();
                self.checkpoint();
                for media in &mut self.document.project.media {
                    if !params.media_ids.is_empty() && !params.media_ids.contains(&media.id) {
                        continue;
                    }
                    if media.kind == crate::project::MediaKind::Title
                        || media.kind == crate::project::MediaKind::CompoundSequence
                        || !media.path.is_file()
                    {
                        continue;
                    }
                    match crate::media_fingerprint::compute_file(&media.path) {
                        Ok(fingerprint) => {
                            media.fingerprint = Some(fingerprint);
                            fingerprinted += 1;
                        }
                        Err(error) => {
                            failed.push(json!({"media_id": media.id, "error": error}))
                        }
                    }
                }
                Ok(json!({
                    "fingerprinted": fingerprinted,
                    "failed": failed,
                }))
            })()),
            "media.relink_suggest" => Some((|| {
                let params: MediaAutoRelinkParams = parse_params(params)?;
                if params.search_dirs.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "relink_suggest needs at least one search directory".into(),
                    ));
                }
                let mut dirs: Vec<std::path::PathBuf> = Vec::new();
                for dir in &params.search_dirs {
                    let path = std::path::PathBuf::from(dir);
                    if !path.is_dir() {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Search directory {dir} does not exist"
                        )));
                    }
                    dirs.push(path);
                }
                // Read-only: suggestions never mutate the document. Executing
                // a relink is `project.relink_media`, which the user confirms
                // (plan review §4.10: the fingerprint suggests, never decides).
                let suggestions: Vec<Value> = self
                    .document
                    .project
                    .media
                    .iter()
                    .filter(|media| {
                        !matches!(
                            media.kind,
                            crate::project::MediaKind::Title
                                | crate::project::MediaKind::CompoundSequence
                        ) && !media.path.exists()
                    })
                    .map(|media| {
                        let mut candidates: Vec<Value> = Vec::new();
                        let recorded = media.fingerprint.as_ref();
                        for dir in &dirs {
                            let Some(file_name) = media.path.file_name() else { continue };
                            let candidate = dir.join(file_name);
                            if !candidate.is_file() {
                                continue;
                            }
                            let fingerprint_match = recorded
                                .map(|note| {
                                    crate::media_fingerprint::compute_file(&candidate)
                                        .map(|computed| note.matches(&computed))
                                        .unwrap_or(false)
                                })
                                .unwrap_or(false);
                            // A same-named file is weaker evidence than a
                            // fingerprint match, but still worth listing.
                            let kind = if fingerprint_match {
                                "fingerprint"
                            } else {
                                "name"
                            };
                            candidates.push(json!({
                                "path": candidate.to_string_lossy(),
                                "match": kind,
                            }));
                        }
                        // Fingerprint matches first: strictly stronger
                        // evidence than a shared file name.
                        candidates.sort_by_key(|candidate| {
                            candidate["match"] != json!("fingerprint")
                        });
                        json!({
                            "media_id": media.id,
                            "name": media.name,
                            "missing_path": media.path.to_string_lossy(),
                            "candidates": candidates,
                        })
                    })
                    .collect();
                Ok(json!({
                    "suggested": suggestions
                        .iter()
                        .filter(|suggestion| suggestion["candidates"]
                            .as_array()
                            .is_some_and(|candidates| !candidates.is_empty()))
                        .count(),
                    "suggestions": suggestions,
                    "note": "suggestions only; confirm via project.relink_media",
                }))
            })()),
            _ => None,
        }
    }
}
