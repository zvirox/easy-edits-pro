use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::Discoverer;
use saturn_core::{
    command_catalogue, export_cmx3600_edl, export_fcpxml, export_otio, export_premiere_xml,
    is_filler_word, CommandRequest, CommandStatus, JobId, JobInfo, JobKind, JobPriority, MediaKind,
    Tick, TICKS_PER_SECOND,
};
use saturn_render::{analyze_pcm_loudness, LoudnessReport, LoudnessStandard, PresentationMetrics};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{ipc::Channel, AppHandle, Emitter, Manager, State};

mod commands;
mod gpu_detect;
#[cfg(unix)]
mod mcp_bridge;
mod media_server;
mod net;
mod path_validation;
mod preview_cache;
mod recovery;
mod state;

use commands::*;
pub use state::AppState;

pub use media_server::{
    get_media_server_port, get_media_server_token, is_allowed_media_path,
    register_allowed_media_path, set_app_cache_dir, set_app_data_dir, start_media_server,
};
pub use path_validation::{
    validate_dir_path, validate_read_path, validate_write_path, PathValidationError,
};
pub use recovery::{
    autosave_once, discard_autosave, recovery_status, restore_autosave, start_autosave,
};

fn register_project_media(app: &AppHandle, document: &saturn_core::ProjectDocument) {
    for media in &document.project.media {
        let _ = app.asset_protocol_scope().allow_file(&media.path);
        register_allowed_media_path(&media.path);
    }
}

fn main() {
    // Phase 0 proved that WebKit's DMA-BUF renderer is the difference between
    // a 59 fps editor and a 2 fps editor on Intel Iris Xe. It stays enabled by
    // default; EEP_DISABLE_DMABUF is the opt-in escape hatch.
    gpu_detect::configure_dmabuf();

    start_media_server();

    let state = AppState::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .setup({
            let state = state.clone();
            move |app| {
                setup_crash_logging(&app.handle());
                if let Ok(data_dir) = app.path().app_data_dir() {
                    set_app_data_dir(data_dir.clone());
                    if let Ok(mut guard) = state.0.lock() {
                        guard.load_settings(data_dir.join("settings.json"));
                    }
                }
                if let Ok(cache_dir) = app.path().app_cache_dir() {
                    set_app_cache_dir(cache_dir.clone());
                    // Orphaned preview proxies (media deleted or relinked)
                    // cannot be invalidated precisely, so the lifecycle is
                    // age-based: prune at startup, off the UI thread.
                    let proxy_dir = cache_dir.join(preview_cache::CACHE_SUBDIR);
                    std::thread::spawn(move || {
                        let (removed, bytes) = preview_cache::prune_stale_proxies(
                            &proxy_dir,
                            preview_cache::MAX_AGE,
                            std::time::SystemTime::now(),
                        );
                        if removed > 0 {
                            eprintln!(
                                "pruned {removed} stale preview proxies ({bytes} bytes)"
                            );
                        }
                    });
                }
                if let Ok(guard) = state.0.lock() {
                    for media in &guard.document().project.media {
                        let _ = app.asset_protocol_scope().allow_file(&media.path);
                        register_allowed_media_path(&media.path);
                    }
                }
                start_autosave(app.handle().clone(), state.clone());
                #[cfg(unix)]
                mcp_bridge::start(app.handle().clone(), state.clone());
                let app_handle = app.handle().clone();
                state.1.add_listener(move |info| {
                    let _ = app_handle.emit("job_queue_updated", info);
                });
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            execute_command,
            gpu_detect::glass_environment,
            gpu_detect::save_glass_report,
            prepare_video_preview,
            get_media_server_port,
            get_media_server_token,
            get_system_diagnostics,
            command_status,
            get_command_catalogue,
            recovery_status,
            restore_autosave,
            discard_autosave,
            check_media_availability,
            import_media,
            start_export,
            detect_encoder_capabilities,
            export_frame,
            get_active_jobs,
            get_all_jobs,
            cancel_job,
            cancel_all_jobs,
            clear_job_history,
            search_openverse,
            download_openverse_asset,
            ask_local_ai,
            import_subtitles_file,
            export_subtitles_file,
            read_text_file,
            read_markers_file,
            write_markers_file,
            load_lut_cube_file,
            get_lut_preset_cube,
            queue_export,
            pause_job_queue,
            resume_job_queue,
            is_job_queue_paused,
            sync_multicam_by_waveform,
            analyze_motion_tracking,
            synthesize_speed_ramp_frames,
            transcribe_audio_speech,
            detect_timeline_silence,
            ripple_delete_silence,
            ripple_delete_fillers,
            export_timeline_interop,
            import_timeline_interop,
            get_gpu_preview_metrics,
            present_gpu_preview_frame,
            measure_ebu_r128_loudness,
            import_network_media,
            run_background_matting
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Easy Edits Pro");
}
