use super::super::*;

#[tauri::command]
pub async fn acquire_collaboration_lock(
    state: State<'_, AppState>,
    user_id: String,
    user_name: String,
    user_color: Option<String>,
    target_kind: String,
    target_id: u64,
) -> Result<Value, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let kind = match target_kind.to_lowercase().as_str() {
        "bin" => saturn_core::project::LockTargetKind::Bin,
        "sequence" => saturn_core::project::LockTargetKind::Sequence,
        "track" => saturn_core::project::LockTargetKind::Track,
        "clip" => saturn_core::project::LockTargetKind::Clip,
        _ => return Err(format!("Unknown target kind: {target_kind}")),
    };
    let res = engine
        .execute(CommandRequest::new(
            "collab.acquire_lock",
            json!({
                "user_id": user_id,
                "user_name": user_name,
                "user_color": user_color,
                "target_kind": kind,
                "target_id": target_id,
            }),
        ))
        .map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
pub async fn release_collaboration_lock(
    state: State<'_, AppState>,
    user_id: String,
    target_kind: String,
    target_id: u64,
) -> Result<Value, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let kind = match target_kind.to_lowercase().as_str() {
        "bin" => saturn_core::project::LockTargetKind::Bin,
        "sequence" => saturn_core::project::LockTargetKind::Sequence,
        "track" => saturn_core::project::LockTargetKind::Track,
        "clip" => saturn_core::project::LockTargetKind::Clip,
        _ => return Err(format!("Unknown target kind: {target_kind}")),
    };
    let res = engine
        .execute(CommandRequest::new(
            "collab.release_lock",
            json!({
                "user_id": user_id,
                "target_kind": kind,
                "target_id": target_id,
            }),
        ))
        .map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
pub async fn get_collaboration_state(
    state: State<'_, AppState>,
) -> Result<saturn_core::project::CollaborationState, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    Ok(engine.document().project.collaboration.clone())
}
