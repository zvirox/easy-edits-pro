use super::*;

impl EditorEngine {
    pub(super) fn dispatch_collab(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "collab.acquire_lock" => Some((|| {
                let params: AcquireLockParams = parse_params(params)?;
                self.checkpoint();
                let collab = &mut self.document.project.collaboration;
                if let Some(existing) = collab.locks.iter().find(|l| {
                    l.target_kind == params.target_kind && l.target_id == params.target_id
                }) {
                    if existing.user_id != params.user_id {
                        return Ok(json!({
                            "acquired": false,
                            "locked_by": existing.user_name,
                            "user_id": existing.user_id
                        }));
                    }
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let lock = UserLock {
                    user_id: params.user_id.clone(),
                    user_name: params.user_name.clone(),
                    user_color: params.user_color.unwrap_or_else(|| "#38bdf8".into()),
                    target_kind: params.target_kind,
                    target_id: params.target_id,
                    locked_at: now,
                };
                collab.locks.retain(|l| {
                    !(l.target_kind == params.target_kind && l.target_id == params.target_id)
                });
                collab.locks.push(lock);
                if !collab.active_users.contains(&params.user_name) {
                    collab.active_users.push(params.user_name.clone());
                }
                let change_id = self.document.project.next_id;
                self.document.project.next_id = change_id.saturating_add(1);
                collab.change_log.push(TimelineChangeEntry {
                    id: change_id,
                    user_id: params.user_id,
                    user_name: params.user_name,
                    timestamp: now,
                    action: "lock_acquired".into(),
                    summary: format!(
                        "Acquired lock on {:?} #{}",
                        params.target_kind, params.target_id
                    ),
                });
                Ok(json!({"acquired": true}))
            })()),
            "collab.release_lock" => Some((|| {
                let params: ReleaseLockParams = parse_params(params)?;
                self.checkpoint();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let collab = &mut self.document.project.collaboration;
                let initial_len = collab.locks.len();
                collab.locks.retain(|l| {
                    !(l.target_kind == params.target_kind
                        && l.target_id == params.target_id
                        && l.user_id == params.user_id)
                });
                let released = collab.locks.len() < initial_len;
                if released {
                    let change_id = self.document.project.next_id;
                    self.document.project.next_id = change_id.saturating_add(1);
                    collab.change_log.push(TimelineChangeEntry {
                        id: change_id,
                        user_id: params.user_id,
                        user_name: "Collaborator".into(),
                        timestamp: now,
                        action: "lock_released".into(),
                        summary: format!(
                            "Released lock on {:?} #{}",
                            params.target_kind, params.target_id
                        ),
                    });
                }
                Ok(json!({"released": released}))
            })()),
            "collab.record_change" => Some((|| {
                let params: RecordChangeParams = parse_params(params)?;
                self.checkpoint();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let entry_id = self.document.project.next_id;
                self.document.project.next_id = entry_id.saturating_add(1);
                self.document
                    .project
                    .collaboration
                    .change_log
                    .push(TimelineChangeEntry {
                        id: entry_id,
                        user_id: params.user_id,
                        user_name: params.user_name,
                        timestamp: now,
                        action: params.action,
                        summary: params.summary,
                    });
                Ok(json!({"recorded": true, "entry_id": entry_id}))
            })()),
            _ => None,
        }
    }
}
