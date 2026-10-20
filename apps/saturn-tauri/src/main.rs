use gstreamer_pbutils::Discoverer;
use gstreamer_pbutils::prelude::*;
use saturn_core::{
    CommandRequest, CommandStatus, JobId, JobInfo, JobKind, JobPriority, MediaKind,
    TICKS_PER_SECOND, Tick, command_catalogue, export_cmx3600_edl, export_fcpxml, export_otio,
    export_premiere_xml, is_filler_word,
};
use saturn_render::{LoudnessReport, LoudnessStandard, PresentationMetrics, analyze_pcm_loudness};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State, ipc::Channel};

mod commands;
#[cfg(unix)]
mod mcp_bridge;
mod media_server;
mod net;
mod path_validation;
mod recovery;
mod state;

use commands::*;
pub use state::AppState;

pub use media_server::{
    get_media_server_port, get_media_server_token, is_allowed_media_path,
    register_allowed_media_path, set_app_cache_dir, set_app_data_dir, start_media_server,
};
pub use path_validation::{
    PathValidationError, validate_dir_path, validate_read_path, validate_write_path,
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
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

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
                    set_app_data_dir(data_dir);
                }
                if let Ok(cache_dir) = app.path().app_cache_dir() {
                    set_app_cache_dir(cache_dir);
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
            load_lut_cube_file,
            get_lut_preset_cube,
            queue_export,
            pause_job_queue,
            resume_job_queue,
            is_job_queue_paused,
            sync_multicam_by_waveform,
            analyze_motion_tracking,
            synthesize_speed_ramp_frames,
            acquire_collaboration_lock,
            release_collaboration_lock,
            get_collaboration_state,
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
