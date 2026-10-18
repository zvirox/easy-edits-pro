use super::super::*;

#[tauri::command]
pub fn get_active_jobs(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.1.list_active()
}

#[tauri::command]
pub fn get_all_jobs(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.1.list_all()
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: u64) -> Result<bool, String> {
    Ok(state.1.cancel(JobId(id)))
}

#[tauri::command]
pub fn cancel_all_jobs(state: State<'_, AppState>) -> Result<(), String> {
    state.1.cancel_all();
    Ok(())
}

#[tauri::command]
pub fn clear_job_history(state: State<'_, AppState>) -> Result<(), String> {
    state.1.clear_history();
    Ok(())
}

#[tauri::command]
pub fn pause_job_queue(state: State<'_, AppState>) -> bool {
    state.1.pause();
    true
}

#[tauri::command]
pub fn resume_job_queue(state: State<'_, AppState>) -> bool {
    state.1.resume();
    true
}

#[tauri::command]
pub fn is_job_queue_paused(state: State<'_, AppState>) -> bool {
    state.1.is_paused()
}
