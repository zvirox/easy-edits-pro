// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Tauri GUI command wrappers exposing agent supervision & selective-undo
//! controls from [`crate::mcp_bridge`] directly to the UI (AF6.1).

use super::super::*;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn agent_session_stop(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Value, String> {
    #[cfg(unix)]
    let result = crate::mcp_bridge::session_stop(
        state.inner(),
        &json!({ "session_id": session_id }),
    )?;
    #[cfg(not(unix))]
    let result = {
        let trimmed = session_id.trim();
        if trimmed.is_empty() {
            return Err("session.stop requires non-empty string `session_id`".to_owned());
        }
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let generation = guard.stop_session(trimmed);
        json!({ "session_id": trimmed, "stopped": true, "generation": generation })
    };
    let _ = app.emit("mcp_project_changed", ());
    Ok(result)
}

#[tauri::command]
pub fn agent_session_resume(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Value, String> {
    // GUI invocation represents local human authority; bridge policy remains human-only.
    #[cfg(unix)]
    let result = crate::mcp_bridge::session_resume(
        state.inner(),
        &json!({
            "session_id": session_id,
            "caller": "human",
        }),
    )?;
    #[cfg(not(unix))]
    let result = {
        let trimmed = session_id.trim();
        if trimmed.is_empty() {
            return Err("session.resume requires non-empty string `session_id`".to_owned());
        }
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let generation = guard
            .resume_session(trimmed, &saturn_core::ActorId::Human)
            .map_err(|e| e.to_string())?;
        json!({ "session_id": trimmed, "resumed": true, "generation": generation })
    };
    let _ = app.emit("mcp_project_changed", ());
    Ok(result)
}

#[tauri::command]
pub fn agent_session_status(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<Value, String> {
    #[cfg(unix)]
    {
        crate::mcp_bridge::session_status(
            state.inner(),
            &json!({ "session_id": session_id }),
        )
    }
    #[cfg(not(unix))]
    {
        let requested = session_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        let resolved = requested.map(str::to_string).or_else(|| {
            guard
                .sessions
                .values()
                .find(|s| !s.actor.is_human())
                .map(|s| s.session_id.clone())
        });
        let Some(sid) = resolved else {
            return Ok(json!({
                "has_active_session": false,
                "session_id": Value::Null,
                "is_stopped": false,
                "generation": 0,
                "actor": Value::Null,
                "active_branch": saturn_core::branch::MAIN_BRANCH,
                "playhead": 0,
                "locks": [],
                "sessions": [],
            }));
        };
        let is_stopped = guard.is_session_stopped(&sid);
        let session = guard.sessions.get(&sid);
        Ok(json!({
            "has_active_session": session.is_some() || is_stopped,
            "session_id": sid,
            "is_stopped": is_stopped,
            "generation": session.map(|s| s.generation).unwrap_or(1),
            "actor": session.map(|s| s.actor.clone()),
            "active_branch": session.map(|s| s.active_branch.clone()).unwrap_or_else(|| saturn_core::branch::MAIN_BRANCH.to_string()),
            "playhead": session.map(|s| s.playhead.0).unwrap_or(0),
            "locks": [],
            "sessions": [],
        }))
    }
}

#[tauri::command]
pub fn agent_undo_change(
    app: AppHandle,
    state: State<'_, AppState>,
    change_id: u64,
) -> Result<Value, String> {
    #[cfg(unix)]
    {
        crate::mcp_bridge::history_undo_change(
            &app,
            state.inner(),
            &json!({
                "change_id": change_id,
                "context": saturn_core::ActorContext::human(),
            }),
        )
    }
    #[cfg(not(unix))]
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let res = guard
            .undo_change(change_id, &saturn_core::ActorContext::human())
            .map_err(|e| e.to_string())?;
        let _ = app.emit("mcp_project_changed", ());
        Ok(res)
    }
}
