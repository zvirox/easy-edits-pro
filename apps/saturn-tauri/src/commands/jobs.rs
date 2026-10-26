use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use super::super::*;
use tauri::AppHandle;

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

/// Parameters needed to re-run a queued export (the retry registry keeps
/// these per job id; the document is re-fetched fresh on retry).
#[derive(Clone)]
pub struct ExportJobDescriptor {
    pub output_path: String,
    pub use_mark_range: bool,
    pub preset_name: String,
    pub settings: saturn_render::RenderSettings,
}

/// Shared retry registry: job id → the export parameters that produced it.
#[derive(Default)]
pub struct ExportJobRegistry(pub Mutex<HashMap<u64, ExportJobDescriptor>>);

/// The real queue entry point, shared by queue_export and retry_export:
/// a detached job on the JobManager workers that reports progress to the
/// UI through the `export_progress` event.
pub(super) fn spawn_queued_export(
    app: &AppHandle,
    state: &State<'_, AppState>,
    registry: &State<'_, ExportJobRegistry>,
    output_path: String,
    use_mark_range: bool,
    preset_name: String,
    settings: saturn_render::RenderSettings,
) -> Result<u64, String> {
    let document = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .document()
        .clone();
    let range = if use_mark_range {
        match (
            document.project.sequence.mark_in,
            document.project.sequence.mark_out,
        ) {
            (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
            _ => return Err("Set both In and Out points before exporting the marked range".into()),
        }
    } else {
        None
    };

    let target_path = PathBuf::from(&output_path);
    let file_name = target_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| output_path.clone());
    let label = format!("{preset_name}: {file_name}");
    let kind = JobKind::BatchExport {
        output_path: target_path.clone(),
        preset: preset_name.clone(),
    };

    let app_handle = app.clone();
    let (job_id, _cancel) = state
        .1
        .submit_detached(label, kind, JobPriority::Normal, move |ctx| {
            saturn_render::render_project_range_with_control(
                &document,
                &target_path,
                range,
                settings,
                |progress| {
                    let fraction = progress.fraction_percent as f32 / 100.0;
                    ctx.update_progress(fraction, progress.phase);
                    let _ = app_handle.emit(
                        "export_progress",
                        json!({
                            "jobId": ctx.job_id.0,
                            "fractionPercent": progress.fraction_percent,
                            "phase": progress.phase
                        }),
                    );
                },
                || ctx.is_cancelled(),
            )?;
            Ok(Some(target_path.to_string_lossy().into_owned()))
        });

    if let Ok(mut map) = registry.0.lock() {
        map.insert(
            job_id.0,
            ExportJobDescriptor {
                output_path,
                use_mark_range,
                preset_name,
                settings,
            },
        );
    }
    Ok(job_id.0)
}

/// Re-run a finished or failed export with the exact parameters it had; the
/// project document is re-read, so uncommitted edits are not included.
#[tauri::command]
pub fn retry_export(
    app: AppHandle,
    state: State<'_, AppState>,
    registry: State<'_, ExportJobRegistry>,
    job_id: u64,
) -> Result<u64, String> {
    let descriptor = registry
        .0
        .lock()
        .map_err(|_| "Export registry unavailable".to_string())?
        .get(&job_id)
        .cloned()
        .ok_or_else(|| format!("No stored parameters for job {job_id}"))?;
    spawn_queued_export(
        &app,
        &state,
        &registry,
        descriptor.output_path,
        descriptor.use_mark_range,
        descriptor.preset_name,
        descriptor.settings,
    )
}

/// Reorder a queued (not yet running) export.
#[tauri::command]
pub fn move_queued_export(
    state: State<'_, AppState>,
    job_id: u64,
    target_index: usize,
) -> Result<bool, String> {
    Ok(state.1.move_pending(JobId(job_id), target_index))
}

/// Drop a queued job from the list without a history entry. Running jobs
/// refuse; cancel them first.
#[tauri::command]
pub fn remove_queued_export(
    state: State<'_, AppState>,
    registry: State<'_, ExportJobRegistry>,
    job_id: u64,
) -> Result<bool, String> {
    let removed = state.1.remove(JobId(job_id));
    if removed {
        if let Ok(mut map) = registry.0.lock() {
            map.remove(&job_id);
        }
    }
    Ok(removed)
}
