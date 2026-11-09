// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

impl EditorEngine {
    pub(super) fn dispatch_lock(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.lock_clip" => Some((|| {
                let clip_id = params.get("clip_id").and_then(Value::as_u64).ok_or_else(|| {
                    DispatchError::InvalidParameters("project.lock_clip requires `clip_id`".into())
                })?;
                let note = params.get("note").and_then(Value::as_str).unwrap_or("");
                let actor = self.current_actor();
                let session_id = Some(self.current_session_id());
                let lock_id = self
                    .document
                    .locks
                    .lock_clip(actor.clone(), session_id, clip_id, note)
                    .map_err(DispatchError::Operation)?;
                Ok(json!({
                    "lock_id": lock_id,
                    "clip_id": clip_id,
                    "owner": actor,
                    "locked": true,
                }))
            })()),
            "project.lock_region" => Some((|| {
                let sequence_id = params
                    .get("sequence_id")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.document.project.sequence.id);
                let track_id = params.get("track_id").and_then(Value::as_u64).ok_or_else(|| {
                    DispatchError::InvalidParameters("project.lock_region requires `track_id`".into())
                })?;
                let start = params
                    .get("start_tick")
                    .and_then(Value::as_i64)
                    .map(Tick)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("project.lock_region requires `start_tick`".into())
                    })?;
                let end = params
                    .get("end_tick")
                    .and_then(Value::as_i64)
                    .map(Tick)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("project.lock_region requires `end_tick`".into())
                    })?;
                let note = params.get("note").and_then(Value::as_str).unwrap_or("");
                let actor = self.current_actor();
                let session_id = Some(self.current_session_id());
                let lock_id = self
                    .document
                    .locks
                    .lock_region(actor.clone(), session_id, sequence_id, track_id, start, end, note)
                    .map_err(DispatchError::Operation)?;
                Ok(json!({
                    "lock_id": lock_id,
                    "sequence_id": sequence_id,
                    "track_id": track_id,
                    "start_tick": start.0,
                    "end_tick": end.0,
                    "owner": actor,
                    "locked": true,
                }))
            })()),
            "project.lock_release" => Some((|| {
                let lock_id = params.get("lock_id").and_then(Value::as_u64).ok_or_else(|| {
                    DispatchError::InvalidParameters("project.lock_release requires `lock_id`".into())
                })?;
                let actor = self.current_actor();
                self.document
                    .locks
                    .release_lock(lock_id, &actor)
                    .map_err(DispatchError::Operation)?;
                Ok(json!({
                    "lock_id": lock_id,
                    "released": true,
                }))
            })()),
            "project.lock_reclaim" => Some((|| {
                let lock_id = params.get("lock_id").and_then(Value::as_u64).ok_or_else(|| {
                    DispatchError::InvalidParameters("project.lock_reclaim requires `lock_id`".into())
                })?;
                let actor = self.current_actor();
                let rev = self.document
                    .locks
                    .reclaim_lock(lock_id, &actor, None)
                    .map_err(DispatchError::Operation)?;
                Ok(json!({
                    "lock_id": lock_id,
                    "reclaimed": true,
                    "revision": rev,
                }))
            })()),
            "project.locks_list" => Some((|| {
                let locks = serde_json::to_value(&self.document.locks.locks).unwrap_or_else(|_| json!([]));
                Ok(json!({ "locks": locks }))
            })()),
            _ => None,
        }
    }
}
