use super::super::*;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderProgressEvent {
    fraction_percent: u8,
    phase: String,
}

#[tauri::command]
pub async fn start_export(
    state: State<'_, AppState>,
    output_path: String,
    use_mark_range: bool,
    settings: saturn_render::RenderSettings,
    on_progress: Channel<RenderProgressEvent>,
) -> Result<(), String> {
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
    let label = format!("Export: {file_name}");
    let kind = JobKind::ExportRendering {
        output_path: target_path.clone(),
    };

    let (_job_id, _cancel, rx) = state
        .1
        .submit(label, kind, JobPriority::Immediate, move |ctx| {
            saturn_render::render_project_range_with_control(
                &document,
                &target_path,
                range,
                settings,
                |progress| {
                    let fraction = progress.fraction_percent as f32 / 100.0;
                    ctx.update_progress(fraction, progress.phase);
                    let _ = on_progress.send(RenderProgressEvent {
                        fraction_percent: progress.fraction_percent,
                        phase: progress.phase.to_string(),
                    });
                },
                || ctx.is_cancelled(),
            )?;
            Ok(Some(target_path.to_string_lossy().into_owned()))
        });

    let res = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|error| format!("Render job channel error: {error}"))?
        .map_err(|error| format!("Render failed: {error}"))?;

    let _ = res;
    Ok(())
}

#[tauri::command]
pub fn detect_encoder_capabilities() -> saturn_render::HardwareEncoderInfo {
    saturn_render::detect_hardware_encoders()
}

#[tauri::command]
pub async fn queue_export(
    app: AppHandle,
    state: State<'_, AppState>,
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
        preset: preset_name,
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

    Ok(job_id.0)
}
