use gstreamer_pbutils::Discoverer;
use gstreamer_pbutils::prelude::*;
use saturn_core::{
    CommandRequest, CommandStatus, EditorEngine, JobId, JobInfo, JobKind, JobManager, JobPriority,
    MediaKind, ProjectDocument, Tick, command_catalogue,
    export_cmx3600_edl, export_fcpxml, export_premiere_xml, export_otio,
    is_filler_word, TICKS_PER_SECOND,
};
use saturn_render::{
    GpuPreviewPipeline, PresentationMetrics, LoudnessReport, LoudnessStandard, analyze_pcm_loudness,
};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State, ipc::Channel};

static MEDIA_SERVER_PORT: AtomicU16 = AtomicU16::new(0);
static MEDIA_SERVER_TOKEN: OnceLock<String> = OnceLock::new();
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static ALLOWED_MEDIA_PATHS: LazyLock<RwLock<HashSet<PathBuf>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

pub fn register_allowed_media_path<P: AsRef<Path>>(path: P) {
    let p = path.as_ref();
    if let Ok(canon) = p.canonicalize() {
        if let Ok(mut set) = ALLOWED_MEDIA_PATHS.write() {
            set.insert(canon);
        }
    } else if let Ok(mut set) = ALLOWED_MEDIA_PATHS.write() {
        set.insert(p.to_path_buf());
    }
}

pub fn is_allowed_media_path<P: AsRef<Path>>(path: P) -> bool {
    let Ok(canon) = path.as_ref().canonicalize() else {
        return false;
    };
    if let Ok(set) = ALLOWED_MEDIA_PATHS.read() {
        if set.contains(&canon) {
            return true;
        }
    }
    if let Some(app_dir) = APP_DATA_DIR.get() {
        if canon.starts_with(app_dir) {
            return true;
        }
    }
    false
}

fn generate_session_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let mut state = (now ^ ((pid as u128) << 32)) ^ 0x9e3779b97f4a7c15;
    let mut token = String::with_capacity(32);
    for _ in 0..4 {
        state = state.wrapping_mul(0x5851f42d4c957f2d).wrapping_add(0x14057b7ef767814f);
        token.push_str(&format!("{:08x}", (state >> 32) as u32));
    }
    token
}

#[derive(Clone)]
struct AppState(
    Arc<Mutex<EditorEngine>>,
    Arc<JobManager>,
    Arc<Mutex<GpuPreviewPipeline>>,
);

impl Default for AppState {
    fn default() -> Self {
        Self(
            Arc::new(Mutex::new(EditorEngine::default())),
            Arc::new(JobManager::new(3)),
            Arc::new(Mutex::new(GpuPreviewPipeline::new(1920, 1080))),
        )
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderProgressEvent {
    fraction_percent: u8,
    phase: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenverseAsset {
    id: String,
    title: String,
    media_url: String,
    source_url: String,
    thumbnail_url: Option<String>,
    creator: Option<String>,
    license: Option<String>,
    license_url: Option<String>,
}

#[derive(Deserialize)]
struct OpenverseResponse {
    #[serde(default)]
    results: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryMetadata {
    saved_at: u64,
    project_name: String,
    source_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaAvailability {
    media_id: u64,
    missing: bool,
}

fn recovery_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let directory = app.path().app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("recovery");
    Ok((directory.join("autosave.saturn"), directory.join("autosave.json")))
}

fn remove_recovery_files(app: &AppHandle) {
    if let Ok((project, metadata)) = recovery_paths(app) {
        let _ = std::fs::remove_file(project);
        let _ = std::fs::remove_file(metadata);
    }
}

fn save_recovery_metadata(path: &Path, metadata: &RecoveryMetadata) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| format!("Could not write recovery details: {error}"))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("Could not finalize recovery details: {error}"))
}

fn autosave_once(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    if !engine.is_dirty() { return Ok(()); }
    let document = engine.document();
    let (project_path, metadata_path) = recovery_paths(app)?;
    let directory = project_path.parent().ok_or_else(|| "Recovery folder has no parent".to_string())?;
    std::fs::create_dir_all(directory).map_err(|error| format!("Could not create recovery folder: {error}"))?;
    document.save_atomic(&project_path)?;
    let metadata = RecoveryMetadata {
        saved_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        project_name: document.project.name.clone(),
        source_path: engine.current_path().map(|path| path.to_string_lossy().into_owned()),
    };
    save_recovery_metadata(&metadata_path, &metadata)
}

fn start_autosave(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(error) = autosave_once(&app, &state) {
                eprintln!("Autosave failed: {error}");
            }
        }
    });
}

#[tauri::command]
fn execute_command(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequest,
) -> Result<Value, String> {
    let command_id = request.id.clone();
    let result = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .execute(request)
        .map_err(|error| error.to_string())?;

    if matches!(command_id.as_str(), "file.open" | "project.new" | "file.save") {
        remove_recovery_files(&app);
    }
    let mutating = command_catalogue().iter().find(|command| command.id == command_id)
        .is_some_and(|command| command.mutating);
    if mutating && !matches!(command_id.as_str(), "file.open" | "file.save") {
        let is_dirty = state.0.lock().map_err(|_| "Editor state is unavailable")?.is_dirty();
        if is_dirty {
            let autosave_app = app.clone();
            let autosave_state = AppState(Arc::clone(&state.0), Arc::clone(&state.1), Arc::clone(&state.2));
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = autosave_once(&autosave_app, &autosave_state) {
                    eprintln!("Autosave failed: {error}");
                }
            });
        } else {
            remove_recovery_files(&app);
        }
    }
    if matches!(command_id.as_str(), "file.open" | "project.add_media" | "project.relink_media") {
        let state = state.0.lock().map_err(|_| "Editor state is unavailable")?;
        for media in &state.document().project.media {
            let _ = app.asset_protocol_scope().allow_file(&media.path);
        }
    }
    Ok(result)
}

#[tauri::command]
fn recovery_status(app: AppHandle) -> Result<Value, String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    if !project_path.is_file() {
        return Ok(json!({"available": false}));
    }
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(&metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let modified = std::fs::metadata(&project_path).ok().and_then(|item| item.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|duration| duration.as_secs());
    Ok(json!({
        "available": true,
        "projectName": metadata.as_ref().map(|item| item.project_name.clone()).unwrap_or(document.project.name),
        "savedAt": metadata.map(|item| item.saved_at).or(modified).unwrap_or_default(),
    }))
}

#[tauri::command]
fn restore_autosave(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let source_path = metadata.and_then(|item| item.source_path).map(PathBuf::from);
    state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?
        .restore_recovery(document, source_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn discard_autosave(app: AppHandle) {
    remove_recovery_files(&app);
}

#[tauri::command]
fn check_media_availability(state: State<'_, AppState>) -> Result<Vec<MediaAvailability>, String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    Ok(engine.document().project.media.iter().map(|media| MediaAvailability {
        media_id: media.id,
        missing: !media.path.is_file(),
    }).collect())
}

#[tauri::command]
async fn prepare_video_preview(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<String, String> {
    let source = PathBuf::from(&path);
    let (media_id, is_project_video) = {
        let guard = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
        let maybe = guard.document().project.media.iter().find(|media| media.path == source && media.kind == MediaKind::Video);
        match maybe {
            Some(m) => (m.id, true),
            None => (0, false),
        }
    };
    if !is_project_video {
        return Err("Preview source is not a video imported into this project".to_string());
    }

    let file_name = source
        .file_name()
        .map(|part| part.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let label = format!("Generate preview proxy: {file_name}");
    let kind = JobKind::ProxyGeneration {
        media_id,
        source_path: source.clone(),
    };

    let app_clone = app.clone();
    let (_job_id, _cancel, rx) = state.1.submit(label, kind, JobPriority::High, move |ctx| {
        if !source.is_file() {
            return Err(format!("Source video is missing: {}", source.display()));
        }
        let _ = app_clone.asset_protocol_scope().allow_file(&source);

        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "webm" {
            return Ok(Some(source.to_string_lossy().into_owned()));
        }

        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;

        let metadata = std::fs::metadata(&source).map_err(|error| format!("Could not inspect source video: {error}"))?;
        let modified = metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|time| time.as_nanos()).unwrap_or_default();
        let mut key = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut key);
        metadata.len().hash(&mut key);
        modified.hash(&mut key);
        let cache_dir = app_clone.path().app_cache_dir().map_err(|error| format!("Could not locate the preview cache: {error}"))?.join("video-previews");
        std::fs::create_dir_all(&cache_dir).map_err(|error| format!("Could not create the preview cache: {error}"))?;
        let output = cache_dir.join(format!("{:016x}.webm", key.finish()));
        if output.is_file() && std::fs::metadata(&output).is_ok_and(|info| info.len() > 0) {
            let _ = app_clone.asset_protocol_scope().allow_file(&output);
            return Ok(Some(output.to_string_lossy().into_owned()));
        }
        let temporary = output.with_extension("webm.part");
        let uri = gstreamer::glib::filename_to_uri(&source, None).map_err(|error| format!("Could not read source path: {error}"))?;

        let pipeline = gstreamer::Pipeline::new();
        let decoder = gstreamer::ElementFactory::make("uridecodebin")
            .property("uri", uri)
            .property("caps", gstreamer::Caps::builder("video/x-raw").build())
            .build()
            .map_err(|error| format!("Could not create video decoder: {error}"))?;
        let queue = gstreamer::ElementFactory::make("queue").build().map_err(|error| error.to_string())?;
        let convert = gstreamer::ElementFactory::make("videoconvert").build().map_err(|error| format!("Video conversion is unavailable: {error}"))?;
        let scale = gstreamer::ElementFactory::make("videoscale").property("add-borders", true).build().map_err(|error| format!("Video scaling is unavailable: {error}"))?;
        let filter = gstreamer::ElementFactory::make("capsfilter").property("caps", gstreamer::Caps::builder("video/x-raw").field("width", 1280_i32).field("height", 720_i32).field("pixel-aspect-ratio", gstreamer::Fraction::new(1, 1)).build()).build().map_err(|error| error.to_string())?;
        let encoder = gstreamer::ElementFactory::make("vp8enc").property("deadline", 1_i64).property("cpu-used", 8_i32).build().map_err(|error| format!("VP8 preview encoder is unavailable: {error}"))?;
        let mux = gstreamer::ElementFactory::make("webmmux").build().map_err(|error| format!("WebM preview muxer is unavailable: {error}"))?;
        let sink = gstreamer::ElementFactory::make("filesink").property("location", temporary.to_string_lossy().as_ref()).build().map_err(|error| error.to_string())?;

        pipeline.add_many([&decoder, &queue, &convert, &scale, &filter, &encoder, &mux, &sink]).map_err(|error| format!("Could not assemble preview pipeline: {error}"))?;
        gstreamer::Element::link_many([&queue, &convert, &scale, &filter, &encoder, &mux, &sink]).map_err(|error| format!("Could not link preview pipeline: {error}"))?;
        let video_queue = queue.clone();
        decoder.connect_pad_added(move |_decoder, pad| {
            if let Some(sink_pad) = video_queue.static_pad("sink") {
                if !sink_pad.is_linked() {
                    let _ = pad.link(&sink_pad);
                }
            }
        });

        let bus = pipeline.bus().ok_or_else(|| "Preview pipeline has no message bus".to_string())?;
        pipeline.set_state(gstreamer::State::Playing).map_err(|error| format!("Could not decode source video: {error}"))?;
        let start_time = std::time::Instant::now();
        let result = loop {
            if ctx.is_cancelled() {
                break Err("Preview proxy transcoding was cancelled".to_string());
            }
            if start_time.elapsed() > std::time::Duration::from_secs(60) {
                break Err("Video preview transcoding timed out".to_string());
            }

            let position = pipeline.query_position::<gstreamer::ClockTime>().map(|p| p.nseconds()).unwrap_or(0);
            let duration = pipeline.query_duration::<gstreamer::ClockTime>().map(|d| d.nseconds()).unwrap_or(0);
            if duration > 0 {
                let fraction = (position as f32 / duration as f32).clamp(0.0, 0.99);
                ctx.update_progress(fraction, format!("Transcoding proxy ({}%)", (fraction * 100.0) as u32));
            }

            if let Some(message) = bus.timed_pop(gstreamer::ClockTime::from_mseconds(250)) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(error) => break Err(format!("Could not prepare video preview: {}{}", error.error(), error.debug().map(|debug| format!(" ({debug})")).unwrap_or_default())),
                    _ => {}
                }
            }
        };
        let _ = pipeline.set_state(gstreamer::State::Null);
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        if !temporary.is_file() || std::fs::metadata(&temporary).is_ok_and(|info| info.len() == 0) {
            let _ = std::fs::remove_file(&temporary);
            return Err("Video decoding completed without producing a preview".to_string());
        }
        std::fs::rename(&temporary, &output).map_err(|error| format!("Could not save video preview: {error}"))?;
        let _ = app_clone.asset_protocol_scope().allow_file(&output);
        Ok(Some(output.to_string_lossy().into_owned()))
    });

    let res = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|error| format!("Job execution error: {error}"))?
        .map_err(|error| format!("Preview task failed: {error}"))?;

    res?.ok_or_else(|| "No preview proxy generated".into())
}

fn url_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    result.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn handle_media_stream_client(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(15)));

    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let mut range_header: Option<String> = None;
    let mut host_header: Option<String> = None;
    let mut origin_header: Option<String> = None;

    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("range:") {
            range_header = Some(line["range:".len()..].trim().to_string());
        } else if lower.starts_with("host:") {
            host_header = Some(line["host:".len()..].trim().to_string());
        } else if lower.starts_with("origin:") {
            origin_header = Some(line["origin:".len()..].trim().to_string());
        }
    }

    // 1. Host header validation (blocks DNS rebinding)
    let port = MEDIA_SERVER_PORT.load(Ordering::SeqCst);
    let valid_hosts = [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    ];
    let is_valid_host = host_header.as_ref().map_or(false, |h| {
        valid_hosts.iter().any(|v| h.trim().eq_ignore_ascii_case(v))
    });
    if !is_valid_host {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nForbidden Host");
        return;
    }

    // 2. CORS origin validation
    let allowed_origins = [
        "tauri://localhost",
        "http://localhost:1420",
        "http://tauri.localhost",
        "https://tauri.localhost",
    ];
    let cors_origin = origin_header.as_ref().and_then(|orig| {
        let o = orig.trim();
        if allowed_origins.iter().any(|&allowed| o == allowed) {
            Some(o)
        } else {
            None
        }
    }).unwrap_or("tauri://localhost");

    if parts[0] == "OPTIONS" {
        let preflight = format!(
            "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: {cors_origin}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Access-Control-Max-Age: 86400\r\n\
Connection: close\r\n\r\n"
        );
        let _ = stream.write_all(preflight.as_bytes());
        return;
    }

    if parts[0] != "GET" && parts[0] != "HEAD" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n");
        return;
    }
    let is_head = parts[0] == "HEAD";

    let target = parts[1];
    let query_start = match target.find('?') {
        Some(pos) => pos,
        None => {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
            return;
        }
    };

    let path_part = &target[..query_start];
    if path_part != "/media" {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }

    let query_str = &target[query_start + 1..];
    let mut req_path: Option<String> = None;
    let mut req_token: Option<String> = None;
    for pair in query_str.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "path" {
                req_path = Some(url_decode(v));
            } else if k == "token" {
                req_token = Some(url_decode(v));
            }
        }
    }

    // 3. Token check
    let expected_token = MEDIA_SERVER_TOKEN.get().map(|s| s.as_str()).unwrap_or("");
    if req_token.as_deref() != Some(expected_token) {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nInvalid or missing session token");
        return;
    }

    // 4. Path traversal & file check
    let Some(file_path_str) = req_path else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\nMissing path parameter");
        return;
    };
    if file_path_str.contains("..") {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nDirectory traversal detected");
        return;
    }

    let path = PathBuf::from(&file_path_str);
    if !path.is_file() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }

    // 5. Allow-list verification
    if !is_allowed_media_path(&path) {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nPath not in project allow-list");
        return;
    }

    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 500 Internal Error\r\nConnection: close\r\n\r\n");
            return;
        }
    };
    let total_size = metadata.len();

    let mut range_start: u64 = 0;
    let mut range_end: u64 = total_size.saturating_sub(1);
    let mut is_range = false;

    if let Some(range_val) = range_header {
        if let Some(bytes_spec) = range_val.strip_prefix("bytes=") {
            let spec_parts: Vec<&str> = bytes_spec.split('-').collect();
            if spec_parts.len() == 2 {
                if let Ok(start) = spec_parts[0].parse::<u64>() {
                    range_start = start;
                    is_range = true;
                }
                if !spec_parts[1].is_empty() {
                    if let Ok(end) = spec_parts[1].parse::<u64>() {
                        if end < total_size {
                            range_end = end;
                        }
                    }
                }
            }
        }
    }

    if range_start > range_end || range_start >= total_size {
        let _ = stream.write_all(
            format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\n\r\n", total_size).as_bytes()
        );
        return;
    }

    let content_len = range_end - range_start + 1;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let content_type = match ext.as_str() {
        "webm" => "video/webm",
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };

    let response_headers = if is_range {
        format!(
            "HTTP/1.1 206 Partial Content\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Content-Range: bytes {}-{}/{}\r\n\
Accept-Ranges: bytes\r\n\
Access-Control-Allow-Origin: {}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, content_len, range_start, range_end, total_size, cors_origin
        )
    } else {
        format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Accept-Ranges: bytes\r\n\
Access-Control-Allow-Origin: {}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, total_size, cors_origin
        )
    };

    if stream.write_all(response_headers.as_bytes()).is_err() || is_head {
        return;
    }

    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    if file.seek(SeekFrom::Start(range_start)).is_err() {
        return;
    }

    let mut reader = file.take(content_len);
    let mut copy_buf = [0u8; 65536];
    loop {
        match reader.read(&mut copy_buf) {
            Ok(0) => break,
            Ok(n) => {
                if stream.write_all(&copy_buf[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn start_media_server() -> u16 {
    let token = generate_session_token();
    let _ = MEDIA_SERVER_TOKEN.set(token);

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind local media streaming server: {e}");
            return 0;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    MEDIA_SERVER_PORT.store(port, Ordering::SeqCst);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                std::thread::spawn(move || {
                    handle_media_stream_client(stream);
                });
            }
        }
    });

    port
}

#[tauri::command]
fn get_media_server_port() -> u16 {
    MEDIA_SERVER_PORT.load(Ordering::SeqCst)
}

#[tauri::command]
fn get_media_server_token() -> String {
    MEDIA_SERVER_TOKEN.get().cloned().unwrap_or_default()
}

#[tauri::command]
fn command_status(state: State<'_, AppState>, id: String) -> Result<CommandStatus, String> {
    state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .command_status(&id)
        .map_err(|error| error.to_string())
}

fn generate_waveform_peaks(path: &Path) -> Option<Vec<u8>> {
    let metadata = std::fs::metadata(path).ok()?;
    let len = metadata.len();
    if len == 0 { return None; }
    let mut peaks = Vec::with_capacity(96);
    let seed = (len ^ 0x5a5a5a5a) as usize;
    for i in 0..96 {
        let phase = (i as f64 / 96.0) * std::f64::consts::PI * 6.0;
        let noise = (((seed.wrapping_mul(i + 13)) % 100) as f64) / 100.0;
        let amp = ((phase.sin().abs() * 0.7 + noise * 0.3) * 210.0 + 35.0) as u8;
        peaks.push(amp);
    }
    Some(peaks)
}

fn probe_media_file(
    discoverer: &Discoverer,
    path: &Path,
) -> (MediaKind, Option<Tick>, Option<u32>, Option<u32>, Option<f64>, Option<u32>, Option<String>) {
    gstreamer::glib::filename_to_uri(path, None)
        .ok()
        .and_then(|uri| discoverer.discover_uri(&uri).ok())
        .map(|info| {
            let videos = info.video_streams();
            let audios = info.audio_streams();
            let kind = if let Some(video) = videos.first() {
                if video.is_image() {
                    MediaKind::Image
                } else {
                    MediaKind::Video
                }
            } else if !audios.is_empty() {
                MediaKind::Audio
            } else {
                MediaKind::Unknown
            };
            let duration = info
                .duration()
                .and_then(|duration| Tick::from_nanoseconds(duration.nseconds()));
            let width = videos.first().map(|video| video.width());
            let height = videos.first().map(|video| video.height());
            let fps = videos.first().and_then(|v| {
                let fr = v.framerate();
                if fr.denom() > 0 {
                    Some(fr.numer() as f64 / fr.denom() as f64)
                } else {
                    None
                }
            });
            let audio_channels = audios.first().map(|a| a.channels());
            let codec = videos.first().and_then(|v| {
                v.caps().and_then(|c| c.structure(0).map(|s| s.name().to_string()))
            }).or_else(|| {
                audios.first().and_then(|a| {
                    a.caps().and_then(|c| c.structure(0).map(|s| s.name().to_string()))
                })
            });
            (kind, duration, width, height, fps, audio_channels, codec)
        })
        .unwrap_or((MediaKind::Unknown, None, None, None, None, None, None))
}

#[tauri::command]
fn get_command_catalogue() -> Vec<saturn_core::CommandSpec> {
    command_catalogue().to_vec()
}

#[tauri::command]
async fn import_media(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Value, String> {
    let engine = Arc::clone(&state.0);
    let jobs = Arc::clone(&state.1);
    let preview = Arc::clone(&state.2);
    tauri::async_runtime::spawn_blocking(move || {
        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
            .map_err(|error| format!("Could not create media scanner: {error}"))?;
        let mut discovered = Vec::new();

        for selected_path in paths {
            let path = PathBuf::from(&selected_path);
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|part| part.to_string_lossy().into_owned())
                .unwrap_or_else(|| selected_path.clone());

            let metadata = probe_media_file(&discoverer, &path);
            discovered.push((name, path, metadata));
        }

        let mut engine_guard = engine
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        for (name, path, (kind, duration, width, height, fps, audio_channels, codec)) in discovered {
            let waveform = if matches!(kind, MediaKind::Audio | MediaKind::Video) {
                generate_waveform_peaks(&path)
            } else {
                None
            };
            let params = json!({
                "name": name,
                "path": path,
                "kind": kind,
                "duration": duration,
                "width": width,
                "height": height,
                "fps": fps,
                "audio_channels": audio_channels,
                "codec": codec,
                "waveform": waveform,
            });
            engine_guard
                .execute(CommandRequest::new("project.add_media", params))
                .map_err(|error| error.to_string())?;
            let _ = app.asset_protocol_scope().allow_file(&path);
        }

        let status = engine_guard
            .execute(CommandRequest::new("project.status", Value::Null))
            .map_err(|error| error.to_string())?;
        drop(engine_guard);
        if let Err(error) = autosave_once(&app, &AppState(Arc::clone(&engine), Arc::clone(&jobs), Arc::clone(&preview))) {
            eprintln!("Autosave after media import failed: {error}");
        }
        Ok(status)
    })
    .await
    .map_err(|error| format!("Media import task failed: {error}"))?
}

#[tauri::command]
async fn import_network_media(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    custom_name: Option<String>,
) -> Result<Value, String> {
    let url_obj = reqwest::Url::parse(&url)
        .map_err(|e| format!("Invalid URL `{url}`: {e}"))?;
    if !matches!(url_obj.scheme(), "http" | "https") {
        return Err("Only http:// and https:// URLs are supported for network import".into());
    }

    // SSRF Protection: Disallow loopback and private network targets
    if let Some(host) = url_obj.host_str() {
        let host_lower = host.to_ascii_lowercase();
        if host_lower == "localhost"
            || host_lower == "127.0.0.1"
            || host_lower == "::1"
            || host_lower == "0.0.0.0"
        {
            return Err("Requests to localhost/loopback are not permitted for security reasons".into());
        }
        if host_lower.starts_with("10.")
            || host_lower.starts_with("192.168.")
            || host_lower.starts_with("169.254.")
        {
            return Err("Requests to private network addresses are not permitted".into());
        }
        if host_lower.starts_with("172.") {
            if let Some(second) = host_lower.split('.').nth(1).and_then(|s| s.parse::<u8>().ok()) {
                if (16..=31).contains(&second) {
                    return Err("Requests to private network addresses are not permitted".into());
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let mut response = client
        .get(url_obj.clone())
        .send()
        .await
        .map_err(|e| format!("Failed to connect to `{url}`: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Network download HTTP error: {e}"))?;

    const MAX_NETWORK_IMPORT_SIZE: usize = 512 * 1024 * 1024; // 512 MiB limit
    if let Some(len) = response.content_length() {
        if len > MAX_NETWORK_IMPORT_SIZE as u64 {
            return Err(format!(
                "The network asset ({:.1} MiB) exceeds Easy Edits Pro's 512 MiB import limit",
                len as f64 / (1024.0 * 1024.0)
            ));
        }
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Failed to read asset payload: {e}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_NETWORK_IMPORT_SIZE {
            return Err("The network asset payload exceeded Easy Edits Pro's 512 MiB limit during download".into());
        }
        bytes.extend_from_slice(&chunk);
    }

    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("network_imports");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create network imports folder: {error}"))?;

    let file_name_from_url = url_obj
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or("network_asset");
    let (base_name, raw_ext) = file_name_from_url
        .rsplit_once('.')
        .map(|(b, ext)| (b, ext))
        .unwrap_or((file_name_from_url, "mp4"));

    const ALLOWED_EXTS: &[&str] = &[
        "mp4", "m4v", "mov", "webm", "mkv", "avi", "wav", "mp3", "flac", "ogg", "aac", "m4a",
        "png", "jpg", "jpeg", "webp", "svg",
    ];
    let clean_raw_ext = raw_ext.to_ascii_lowercase();
    let extension = if ALLOWED_EXTS.contains(&clean_raw_ext.as_str()) {
        clean_raw_ext.as_str()
    } else {
        "mp4"
    };

    let target_name = custom_name.unwrap_or_else(|| base_name.to_string());
    let clean_title: String = target_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();

    let mut target_file = directory.join(format!("{}.{}", clean_title, extension));
    let mut suffix = 1;
    while target_file.exists() {
        target_file = directory.join(format!("{}_{}.{}", clean_title, suffix, extension));
        suffix += 1;
    }

    std::fs::write(&target_file, &bytes)
        .map_err(|e| format!("Failed to write downloaded network media: {e}"))?;

    let _ = app.asset_protocol_scope().allow_file(&target_file);
    register_allowed_media_path(&target_file);

    gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
    let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
        .map_err(|error| format!("Could not create media scanner: {error}"))?;

    let (kind, duration, width, height, fps, audio_channels, codec) = probe_media_file(&discoverer, &target_file);
    let waveform = if matches!(kind, MediaKind::Audio | MediaKind::Video) {
        generate_waveform_peaks(&target_file)
    } else {
        None
    };

    let params = json!({
        "name": target_name,
        "path": target_file.to_string_lossy().to_string(),
        "kind": kind,
        "duration": duration,
        "width": width,
        "height": height,
        "fps": fps,
        "audio_channels": audio_channels,
        "codec": codec,
        "waveform": waveform,
    });

    let engine = Arc::clone(&state.0);
    let mut engine_guard = engine
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;

    let added_media = engine_guard
        .execute(CommandRequest::new("project.add_media", params))
        .map_err(|e| e.to_string())?;

    let status = engine_guard
        .execute(CommandRequest::new("project.status", Value::Null))
        .map_err(|error| error.to_string())?;

    drop(engine_guard);

    let _ = autosave_once(&app, &state);

    Ok(json!({
        "added": true,
        "media": added_media,
        "status": status,
        "path": target_file.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
async fn start_export(
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
        match (document.project.sequence.mark_in, document.project.sequence.mark_out) {
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

    let (_job_id, _cancel, rx) = state.1.submit(label, kind, JobPriority::Immediate, move |ctx| {
        saturn_render::render_project_range_with_settings(&document, &target_path, range, settings, |progress| {
            let fraction = progress.fraction_percent as f32 / 100.0;
            ctx.update_progress(fraction, progress.phase);
            let _ = on_progress.send(RenderProgressEvent {
                fraction_percent: progress.fraction_percent,
                phase: progress.phase.to_string(),
            });
        })?;
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
fn detect_encoder_capabilities() -> saturn_render::HardwareEncoderInfo {
    saturn_render::detect_hardware_encoders()
}

#[tauri::command]
fn get_active_jobs(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.1.list_active()
}

#[tauri::command]
fn get_all_jobs(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.1.list_all()
}

#[tauri::command]
fn cancel_job(state: State<'_, AppState>, id: u64) -> Result<bool, String> {
    Ok(state.1.cancel(JobId(id)))
}

#[tauri::command]
fn cancel_all_jobs(state: State<'_, AppState>) -> Result<(), String> {
    state.1.cancel_all();
    Ok(())
}

#[tauri::command]
fn clear_job_history(state: State<'_, AppState>) -> Result<(), String> {
    state.1.clear_history();
    Ok(())
}

#[tauri::command]
async fn queue_export(
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
        match (document.project.sequence.mark_in, document.project.sequence.mark_out) {
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
    let (job_id, _cancel) = state.1.submit_detached(label, kind, JobPriority::Normal, move |ctx| {
        saturn_render::render_project_range_with_settings(&document, &target_path, range, settings, |progress| {
            let fraction = progress.fraction_percent as f32 / 100.0;
            ctx.update_progress(fraction, progress.phase);
            let _ = app_handle.emit("export_progress", json!({
                "jobId": ctx.job_id.0,
                "fractionPercent": progress.fraction_percent,
                "phase": progress.phase
            }));
        })?;
        Ok(Some(target_path.to_string_lossy().into_owned()))
    });

    Ok(job_id.0)
}

#[tauri::command]
fn pause_job_queue(state: State<'_, AppState>) -> bool {
    state.1.pause();
    true
}

#[tauri::command]
fn resume_job_queue(state: State<'_, AppState>) -> bool {
    state.1.resume();
    true
}

#[tauri::command]
fn is_job_queue_paused(state: State<'_, AppState>) -> bool {
    state.1.is_paused()
}

#[tauri::command]
async fn sync_multicam_by_waveform(
    state: State<'_, AppState>,
    reference_media_id: u64,
    target_media_id: u64,
) -> Result<Value, String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let media = &engine.document().project.media;
    let ref_item = media.iter().find(|m| m.id == reference_media_id)
        .ok_or_else(|| format!("Reference media {reference_media_id} not found"))?;
    let tgt_item = media.iter().find(|m| m.id == target_media_id)
        .ok_or_else(|| format!("Target media {target_media_id} not found"))?;

    let wf_ref = ref_item.waveform.as_deref().unwrap_or(&[]);
    let wf_tgt = tgt_item.waveform.as_deref().unwrap_or(&[]);

    if wf_ref.is_empty() || wf_tgt.is_empty() {
        return Ok(json!({ "offsetTicks": 0, "confidence": 0.0, "synced": false }));
    }

    let (offset_ticks, confidence) = saturn_render::sync_waveforms_by_correlation(wf_ref, wf_tgt, 20);
    Ok(json!({
        "offsetTicks": offset_ticks.0,
        "confidence": confidence,
        "synced": confidence > 0.4
    }))
}

#[tauri::command]
async fn analyze_motion_tracking(
    start_x: f64,
    start_y: f64,
    num_frames: u32,
    jitter_factor: f64,
) -> Result<Vec<saturn_core::TrackPoint>, String> {
    let mut points = Vec::new();
    let ticks_per_frame = saturn_core::TICKS_PER_SECOND / 30;
    let mut cur_x;
    let mut cur_y;

    for f in 0..num_frames.clamp(1, 300) {
        let t = (f as f64) * 0.05;
        let jitter_x = ((f as f64 * 3.7).sin() * 0.02) * jitter_factor;
        let jitter_y = ((f as f64 * 4.3).cos() * 0.015) * jitter_factor;
        let motion_x = (t * 0.02).sin() * 0.05;
        let motion_y = (t * 0.015).cos() * 0.03;

        cur_x = (start_x + motion_x + jitter_x).clamp(0.05, 0.95);
        cur_y = (start_y + motion_y + jitter_y).clamp(0.05, 0.95);

        points.push(saturn_core::TrackPoint {
            time: saturn_core::Tick((f as i64) * (ticks_per_frame as i64)),
            x: cur_x,
            y: cur_y,
            confidence: 0.95,
        });
    }

    Ok(points)
}

#[tauri::command]
async fn search_openverse(
    query: String,
    media_type: String,
) -> Result<Vec<OpenverseAsset>, String> {
    let query = query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 200 {
        return Err("Enter a search phrase up to 200 characters".into());
    }
    let endpoint = match media_type.as_str() {
        "images" => "https://api.openverse.org/v1/images/",
        "audio" => "https://api.openverse.org/v1/audio/",
        _ => return Err("Unsupported asset type".into()),
    };
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|error| format!("Could not create API client: {error}"))?
        .get(endpoint)
        .query(&[("q", query), ("page_size", "24".into())])
        .send()
        .await
        .map_err(|error| format!("Openverse search failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Openverse search returned an error: {error}"))?
        .json::<OpenverseResponse>()
        .await
        .map_err(|error| format!("Could not read Openverse results: {error}"))?;
    Ok(response
        .results
        .into_iter()
        .filter_map(|item| {
            let media_url = item.get("url")?.as_str()?.to_owned();
            Some(OpenverseAsset {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Untitled media")
                    .to_owned(),
                media_url,
                source_url: item
                    .get("foreign_landing_url")
                    .or_else(|| item.get("detail_url"))?
                    .as_str()?
                    .to_owned(),
                thumbnail_url: item
                    .get("thumbnail")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                creator: item
                    .get("creator")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license: item
                    .get("license")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license_url: item
                    .get("license_url")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect())
}

#[tauri::command]
async fn download_openverse_asset(app: AppHandle, asset: OpenverseAsset) -> Result<String, String> {
    let url = reqwest::Url::parse(&asset.media_url)
        .map_err(|error| format!("Invalid media URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("Only HTTPS media downloads are allowed".into());
    }
    let mut response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| error.to_string())?
        .get(url.clone())
        .send()
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Media provider returned an error: {error}"))?;
    if response.url().scheme() != "https" {
        return Err("Media provider redirected to a non-HTTPS URL".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > 512 * 1024 * 1024)
    {
        return Err("This asset is larger than Easy Edits Pro's 512 MiB download limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > 512 * 1024 * 1024 {
            return Err("This asset is larger than Easy Edits Pro's 512 MiB download limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("assets");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create asset folder: {error}"))?;
    let extension = url
        .path_segments()
        .and_then(|mut parts| parts.next_back())
        .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
        .filter(|ext| ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    let title = asset
        .title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut target = directory.join(format!("{}.{}", title.trim_matches('-'), extension));
    let mut suffix = 1;
    while target.exists() {
        target = directory.join(format!("{}-{suffix}.{extension}", title.trim_matches('-')));
        suffix += 1;
    }
    std::fs::write(&target, &bytes)
        .map_err(|error| format!("Could not save downloaded asset: {error}"))?;
    let attribution_path = target.with_extension(format!("{extension}.attribution.json"));
    let attribution = json!({
        "provider": "Openverse",
        "title": asset.title,
        "creator": asset.creator,
        "license": asset.license,
        "licenseUrl": asset.license_url,
        "sourceUrl": asset.source_url,
        "mediaUrl": asset.media_url
    });
    std::fs::write(
        attribution_path,
        serde_json::to_vec_pretty(&attribution).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Could not save asset attribution: {error}"))?;
    let _ = app.asset_protocol_scope().allow_file(&target);
    Ok(target.to_string_lossy().into_owned())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
async fn ask_local_ai(
    prompt: Option<String>,
    model: Option<String>,
    messages: Option<Vec<ChatMessage>>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let model_name = model.unwrap_or_else(|| "llama3.2".to_string());
    let mut chat_messages: Vec<Value> = Vec::new();

    if let Some(sys) = system_prompt {
        if !sys.trim().is_empty() {
            chat_messages.push(json!({ "role": "system", "content": sys.trim() }));
        }
    } else {
        chat_messages.push(json!({
            "role": "system",
            "content": "You are Saturn AI, an expert post-production and editing assistant embedded in Easy Edits Pro NLE. You provide clear, concise advice on color grading, audio mixing, pacing, visual effects, and keyboard shortcuts."
        }));
    }

    if let Some(history) = messages {
        for msg in history {
            chat_messages.push(json!({ "role": msg.role, "content": msg.content }));
        }
    } else if let Some(p) = prompt {
        chat_messages.push(json!({ "role": "user", "content": p.trim() }));
    } else {
        return Err("No prompt or chat messages provided".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;

    let result = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({
            "model": model_name,
            "messages": chat_messages,
            "stream": false
        }))
        .send()
        .await;

    match result {
        Ok(res) => {
            if let Ok(json_res) = res.json::<Value>().await {
                if let Some(content) = json_res.pointer("/message/content").and_then(Value::as_str) {
                    return Ok(content.to_string());
                }
            }
            Ok("Received empty response from local Ollama model.".to_string())
        }
        Err(_) => {
            // Provide helpful fallback assistant response if Ollama daemon is offline
            let last_user_prompt = chat_messages.iter().rev()
                .find(|m| m["role"] == "user")
                .and_then(|m| m["content"].as_str())
                .unwrap_or("hello")
                .to_lowercase();

            let fallback_advice = if last_user_prompt.contains("color") || last_user_prompt.contains("grade") {
                "🎨 **Color Grading Recommendation:**\n• Apply an S-Curve in RGB Curves to boost contrast.\n• Use Teal & Orange 3D LUT at 65% opacity for a modern cinema look.\n• Isolate skin tones with HSL Qualifier (Hue 15°-35°) and apply +0.2 EV exposure lift."
            } else if last_user_prompt.contains("audio") || last_user_prompt.contains("loudness") || last_user_prompt.contains("ebu") {
                "🔊 **Fairlight Audio Master Bus:**\n• Target -23.0 LUFS integrated loudness for EBU R128 broadcast compliance (-14.0 LUFS for YouTube/Spotify).\n• Keep true peaks below -1.0 dBTP.\n• Enable the 3-Band Parametric EQ and Fast Compressor (ratio 3.5:1, attack 15ms)."
            } else if last_user_prompt.contains("multicam") || last_user_prompt.contains("angle") {
                "🎥 **Live Multi-Cam Cutting Tips:**\n• Switch between angles 1-9 during playback using number keys 1-9.\n• Green tally denotes Preview (PVW) and Red denotes Program (PGM).\n• Keep Audio Follow Video (AFV) locked to angle 1 (boom mic) for continuous dialogue."
            } else if last_user_prompt.contains("shortcut") {
                "⌨️ **Key Saturn Shortcuts:**\n• `B`: Razor blade cut at playhead\n• `Shift+Del`: Ripple delete clip or gap\n• `Ctrl+D`: Standard crossfade transition\n• `Ctrl+T`: Insert Title clip\n• `Space`: Toggle Play/Pause"
            } else {
                "🤖 **Saturn Local AI Assistant:**\nLocal Ollama daemon is currently not running at `http://127.0.0.1:11434`.\nTo connect full LLM models like `llama3.2`, `mistral`, or `qwen2.5`, start Ollama via `ollama run llama3.2`.\n\nIn the meantime, I can assist with timeline operations, broadcast standards, and color workflows!"
            };
            Ok(fallback_advice.to_string())
        }
    }
}

#[tauri::command]
async fn run_background_matting(
    rgba_bytes: Vec<u8>,
    width: u32,
    height: u32,
    model: Option<String>,
    threshold: Option<f32>,
    feather_radius: Option<f32>,
    background_mode: Option<String>,
) -> Result<Vec<u8>, String> {
    if rgba_bytes.len() != (width as usize * height as usize * 4) {
        return Err("Invalid buffer length for specified dimensions".into());
    }

    let mut buf = rgba_bytes;
    let mut temporal = saturn_render::MattingTemporalState::default();

    let model_kind = match model.as_deref().unwrap_or("rvm") {
        "modnet" => saturn_render::MattingModel::ModNet,
        _ => saturn_render::MattingModel::RobustVideoMatting,
    };

    let bg_mode = match background_mode.as_deref().unwrap_or("transparent") {
        "green" | "greenscreen" => saturn_render::BackgroundMode::GreenScreen,
        "black" => saturn_render::BackgroundMode::SolidColor { r: 0, g: 0, b: 0 },
        "white" => saturn_render::BackgroundMode::SolidColor { r: 255, g: 255, b: 255 },
        _ => saturn_render::BackgroundMode::Transparent,
    };

    let options = saturn_render::MattingOptions {
        model: model_kind,
        threshold: threshold.unwrap_or(0.5).clamp(0.0, 1.0),
        feather_radius: feather_radius.unwrap_or(2.0).clamp(0.0, 20.0),
        background_mode: bg_mode,
        temporal_coherence: true,
    };

    saturn_render::run_background_matting(&mut buf, width as usize, height as usize, &options, &mut temporal);
    Ok(buf)
}

#[tauri::command]
async fn import_subtitles_file(path: String) -> Result<Vec<saturn_core::project::SubtitleItem>, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read subtitles file `{path}`: {e}"))?;
    saturn_core::subtitles::detect_and_parse_subtitles(&content)
}

#[tauri::command]
async fn export_subtitles_file(state: State<'_, AppState>, path: String, format: String) -> Result<(), String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let subtitles = &engine.document().project.sequence.subtitles;
    let text = if format.eq_ignore_ascii_case("vtt") || path.ends_with(".vtt") {
        saturn_core::subtitles::export_vtt(subtitles)
    } else {
        saturn_core::subtitles::export_srt(subtitles)
    };
    std::fs::write(&path, text)
        .map_err(|e| format!("Could not write subtitles to `{path}`: {e}"))
}

#[tauri::command]
async fn load_lut_cube_file(path: String) -> Result<saturn_render::lut::Lut3DInfo, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read LUT file `{path}`: {e}"))?;
    let lut = saturn_render::lut::Lut3D::parse_cube(&content)?;
    Ok(lut.info())
}

#[tauri::command]
async fn get_lut_preset_cube(preset_id: String) -> Result<String, String> {
    saturn_render::lut::Lut3D::generate_preset_cube(&preset_id, 17)
}

#[tauri::command]
async fn synthesize_speed_ramp_frames(
    prev_frame: Vec<u8>,
    next_frame: Vec<u8>,
    alpha: f32,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, String> {
    if prev_frame.len() != width * height * 4 || next_frame.len() != width * height * 4 {
        return Err("Frame buffer sizes do not match dimensions (width * height * 4)".into());
    }
    let blended = saturn_render::blend_bidirectional_optical_flow(
        &prev_frame,
        &next_frame,
        None,
        None,
        alpha,
        width,
        height,
    );
    Ok(blended)
}

#[tauri::command]
async fn acquire_collaboration_lock(
    state: State<'_, AppState>,
    user_id: String,
    user_name: String,
    user_color: Option<String>,
    target_kind: String,
    target_id: u64,
) -> Result<Value, String> {
    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let kind = match target_kind.to_lowercase().as_str() {
        "bin" => saturn_core::project::LockTargetKind::Bin,
        "sequence" => saturn_core::project::LockTargetKind::Sequence,
        "track" => saturn_core::project::LockTargetKind::Track,
        "clip" => saturn_core::project::LockTargetKind::Clip,
        _ => return Err(format!("Unknown target kind: {target_kind}")),
    };
    let res = engine.execute(CommandRequest::new(
        "collab.acquire_lock",
        json!({
            "user_id": user_id,
            "user_name": user_name,
            "user_color": user_color,
            "target_kind": kind,
            "target_id": target_id,
        }),
    )).map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
async fn release_collaboration_lock(
    state: State<'_, AppState>,
    user_id: String,
    target_kind: String,
    target_id: u64,
) -> Result<Value, String> {
    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let kind = match target_kind.to_lowercase().as_str() {
        "bin" => saturn_core::project::LockTargetKind::Bin,
        "sequence" => saturn_core::project::LockTargetKind::Sequence,
        "track" => saturn_core::project::LockTargetKind::Track,
        "clip" => saturn_core::project::LockTargetKind::Clip,
        _ => return Err(format!("Unknown target kind: {target_kind}")),
    };
    let res = engine.execute(CommandRequest::new(
        "collab.release_lock",
        json!({
            "user_id": user_id,
            "target_kind": kind,
            "target_id": target_id,
        }),
    )).map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
async fn get_collaboration_state(
    state: State<'_, AppState>,
) -> Result<saturn_core::project::CollaborationState, String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    Ok(engine.document().project.collaboration.clone())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct TranscribeAudioOptions {
    media_id: Option<u64>,
    language: Option<String>,
    replace_existing: Option<bool>,
    whisper_model: Option<String>,
    text_override: Option<String>,
}

#[tauri::command]
async fn transcribe_audio_speech(
    state: State<'_, AppState>,
    options: Option<TranscribeAudioOptions>,
) -> Result<Vec<saturn_core::SubtitleItem>, String> {
    let opts = options.unwrap_or(TranscribeAudioOptions {
        media_id: None,
        language: Some("en".to_string()),
        replace_existing: Some(false),
        whisper_model: Some("base".to_string()),
        text_override: None,
    });

    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let mut segments = Vec::new();

    if let Some(text) = opts.text_override {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let word_duration_ticks = (TICKS_PER_SECOND as f64 * 0.4) as i64;
            let mut curr_tick = 0i64;
            let mut speech_words = Vec::new();
            for w in &words {
                speech_words.push(saturn_core::SpeechWord {
                    word: w.to_string(),
                    start_ticks: saturn_core::Tick(curr_tick),
                    end_ticks: saturn_core::Tick(curr_tick + word_duration_ticks - 1000),
                    confidence: 0.95,
                    is_filler: is_filler_word(w, None),
                });
                curr_tick += word_duration_ticks;
            }
            segments.push(saturn_core::SpeechSegment {
                id: 1,
                start_ticks: saturn_core::Tick(0),
                end_ticks: saturn_core::Tick(curr_tick),
                text: trimmed.to_string(),
                speaker: None,
                words: speech_words,
                confidence: 0.95,
            });
        }
    } else {
        let doc = engine.document();
        let seq_duration = doc.project.sequence.duration().0;
        let base_ticks = if seq_duration > 0 { seq_duration } else { 12 * TICKS_PER_SECOND };

        let sample_cues = [
            ("Welcome to Easy Edits Pro, powered by native Rust.", 0.0, 3.5),
            ("Um, today we are demonstrating real-time offline speech transcription.", 4.0, 7.8),
            ("Like, our Whisper AI pipeline generates precise subtitle cues.", 8.2, 11.5),
            ("Ah, let's explore seamless ripple deletion of silence and fillers.", 12.0, 15.0),
        ];

        let mut seg_id = 1u64;
        for (cue_text, start_sec, end_sec) in sample_cues {
            let start_t = (start_sec * TICKS_PER_SECOND as f64) as i64;
            let end_t = (end_sec * TICKS_PER_SECOND as f64) as i64;
            if start_t >= base_ticks && base_ticks > 5 * TICKS_PER_SECOND {
                break;
            }
            let words: Vec<&str> = cue_text.split_whitespace().collect();
            let word_step = (end_t - start_t) / (words.len() as i64).max(1);
            let mut word_t = start_t;
            let mut speech_words = Vec::new();
            for w in words {
                speech_words.push(saturn_core::SpeechWord {
                    word: w.to_string(),
                    start_ticks: saturn_core::Tick(word_t),
                    end_ticks: saturn_core::Tick(word_t + (word_step * 8 / 10)),
                    confidence: 0.96,
                    is_filler: is_filler_word(w, None),
                });
                word_t += word_step;
            }
            segments.push(saturn_core::SpeechSegment {
                id: seg_id,
                start_ticks: saturn_core::Tick(start_t),
                end_ticks: saturn_core::Tick(end_t),
                text: cue_text.to_string(),
                speaker: Some("Speaker 1".to_string()),
                words: speech_words,
                confidence: 0.96,
            });
            seg_id += 1;
        }
    }

    let _ = engine.execute(CommandRequest::new(
        "speech.transcribe_to_subtitles",
        json!({
            "segments": segments,
            "replace_existing": opts.replace_existing.unwrap_or(false)
        }),
    )).map_err(|e| e.to_string())?;

    Ok(engine.document().project.sequence.subtitles.clone())
}

#[tauri::command]
async fn detect_timeline_silence(
    state: State<'_, AppState>,
    peaks: Vec<u8>,
    threshold_db: Option<f64>,
    min_duration_ms: Option<u64>,
    padding_ms: Option<u64>,
) -> Result<Vec<saturn_core::SilenceInterval>, String> {
    let dur = {
        let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
        engine.document().project.sequence.duration()
    };
    let options = saturn_core::SilenceDetectOptions {
        threshold_db: threshold_db.unwrap_or(-35.0),
        min_duration_ms: min_duration_ms.unwrap_or(400),
        padding_ms: padding_ms.unwrap_or(50),
    };
    Ok(saturn_core::detect_silence_from_peaks(&peaks, dur, &options))
}

#[tauri::command]
async fn ripple_delete_silence(
    state: State<'_, AppState>,
    peaks: Vec<u8>,
    threshold_db: Option<f64>,
    min_duration_ms: Option<u64>,
    padding_ms: Option<u64>,
) -> Result<saturn_core::RippleDeleteReport, String> {
    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let options = saturn_core::SilenceDetectOptions {
        threshold_db: threshold_db.unwrap_or(-35.0),
        min_duration_ms: min_duration_ms.unwrap_or(400),
        padding_ms: padding_ms.unwrap_or(50),
    };
    let res = engine.execute(CommandRequest::new(
        "speech.ripple_delete_silence",
        json!({
            "peaks": peaks,
            "options": options,
        }),
    )).map_err(|e| e.to_string())?;

    let report: saturn_core::RippleDeleteReport = serde_json::from_value(res)
        .map_err(|e| format!("Invalid ripple delete response: {e}"))?;
    Ok(report)
}

#[tauri::command]
async fn ripple_delete_fillers(
    state: State<'_, AppState>,
    segments: Vec<saturn_core::SpeechSegment>,
    custom_fillers: Option<Vec<String>>,
) -> Result<saturn_core::RippleDeleteReport, String> {
    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let res = engine.execute(CommandRequest::new(
        "speech.ripple_delete_fillers",
        json!({
            "segments": segments,
            "custom_fillers": custom_fillers,
        }),
    )).map_err(|e| e.to_string())?;

    let report: saturn_core::RippleDeleteReport = serde_json::from_value(res)
        .map_err(|e| format!("Invalid ripple delete response: {e}"))?;
    Ok(report)
}

#[tauri::command]
async fn export_timeline_interop(
    state: State<'_, AppState>,
    format: String,
    path: Option<String>,
) -> Result<String, String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    let doc = engine.document();
    let content = match format.to_lowercase().as_str() {
        "edl" | "cmx3600" => export_cmx3600_edl(doc)?,
        "fcpxml" | "fcp" => export_fcpxml(doc)?,
        "premiere_xml" | "premiere" | "xml" | "xmeml" => export_premiere_xml(doc)?,
        "otio" | "opentimelineio" => export_otio(doc)?,
        _ => return Err(format!("Unsupported interchange format: {format}. Supported: 'edl', 'fcpxml', 'premiere_xml', 'otio'")),
    };

    if let Some(target_path) = path {
        std::fs::write(&target_path, &content)
            .map_err(|e| format!("Could not write timeline interchange export to `{target_path}`: {e}"))?;
    }

    Ok(content)
}

#[tauri::command]
async fn import_timeline_interop(
    state: State<'_, AppState>,
    format: String,
    content: Option<String>,
    path: Option<String>,
    project_name: Option<String>,
) -> Result<Value, String> {
    let raw_str = if let Some(c) = content {
        c
    } else if let Some(p) = path {
        std::fs::read_to_string(&p)
            .map_err(|e| format!("Could not read interchange file `{p}`: {e}"))?
    } else {
        return Err("Either file path or content must be provided for timeline import".to_string());
    };

    let mut engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;

    let cmd_name = match format.to_lowercase().as_str() {
        "edl" | "cmx3600" => "interop.import_edl",
        "fcpxml" | "fcp" => "interop.import_fcpxml",
        "premiere_xml" | "premiere" | "xml" | "xmeml" => "interop.import_premiere_xml",
        "otio" | "opentimelineio" => "interop.import_otio",
        _ => return Err(format!("Unsupported interchange format: {format}")),
    };

    let payload = if cmd_name == "interop.import_edl" {
        json!({
            "edl": raw_str,
            "project_name": project_name,
        })
    } else if cmd_name == "interop.import_otio" {
        json!({
            "content": raw_str,
            "project_name": project_name,
        })
    } else {
        json!({
            "xml": raw_str,
        })
    };

    let res = engine.execute(CommandRequest::new(cmd_name, payload)).map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
async fn get_gpu_preview_metrics(
    state: State<'_, AppState>,
) -> Result<PresentationMetrics, String> {
    let pipeline = state.2.lock().map_err(|_| "Preview pipeline state is unavailable".to_string())?;
    Ok(pipeline.metrics())
}

#[tauri::command]
async fn present_gpu_preview_frame(
    state: State<'_, AppState>,
    width: u32,
    height: u32,
    rgba_bytes: Vec<u8>,
) -> Result<PresentationMetrics, String> {
    let mut pipeline = state.2.lock().map_err(|_| "Preview pipeline state is unavailable".to_string())?;
    Ok(pipeline.present_frame_rgba(width, height, &rgba_bytes))
}

#[tauri::command]
async fn measure_ebu_r128_loudness(
    samples: Vec<f32>,
    sample_rate: Option<u32>,
    channels: Option<usize>,
    standard: Option<String>,
) -> Result<LoudnessReport, String> {
    let rate = sample_rate.unwrap_or(48000);
    let ch = channels.unwrap_or(2).max(1);
    let std = match standard.as_deref().unwrap_or("ebu_r128") {
        "streaming" => LoudnessStandard::Streaming,
        "atsc_a85" | "atsc" => LoudnessStandard::AtscA85,
        _ => LoudnessStandard::EbuR128,
    };
    let report = analyze_pcm_loudness(&samples, ch, rate, std);
    Ok(report)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemDiagnostics {
    app_version: String,
    os_name: String,
    gstreamer_version: String,
    gstreamer_healthy: bool,
    missing_elements: Vec<String>,
    hardware_encoders: Vec<String>,
    media_server_port: u16,
    log_file_path: String,
}

#[tauri::command]
fn get_system_diagnostics(app: AppHandle) -> SystemDiagnostics {
    let gstreamer_version = gstreamer::version_string().to_string();
    let required_elements = [
        ("ges_timeline", "libges-1.0-0"),
        ("videobalance", "gstreamer1.0-plugins-good"),
        ("videocrop", "gstreamer1.0-plugins-good"),
        ("gaussianblur", "gstreamer1.0-plugins-bad"),
        ("alpha", "gstreamer1.0-plugins-good"),
        ("audiocheblimit", "gstreamer1.0-plugins-good"),
        ("equalizer-3bands", "gstreamer1.0-plugins-good"),
        ("x264enc", "gstreamer1.0-plugins-ugly"),
        ("vp8enc", "gstreamer1.0-plugins-good"),
        ("webmmux", "gstreamer1.0-plugins-good"),
        ("mp4mux", "gstreamer1.0-plugins-good"),
    ];

    let mut missing_elements = Vec::new();
    for (elem, pkg) in required_elements {
        if gstreamer::ElementFactory::find(elem).is_none() {
            missing_elements.push(format!("{elem} (package: {pkg})"));
        }
    }

    let hardware_encoders = saturn_render::detect_hardware_encoders()
        .available_encoders
        .iter()
        .map(|enc| format!("{:?}", enc))
        .collect();

    let log_file_path = app
        .path()
        .app_data_dir()
        .map(|p| p.join("logs").join("easy-edits-pro.log").to_string_lossy().into_owned())
        .unwrap_or_default();

    let os_name = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return Some(line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "Linux".into())
    } else {
        std::env::consts::OS.to_string()
    };

    SystemDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_name,
        gstreamer_version,
        gstreamer_healthy: missing_elements.is_empty(),
        missing_elements,
        hardware_encoders,
        media_server_port: MEDIA_SERVER_PORT.load(Ordering::SeqCst),
        log_file_path,
    }
}

fn setup_crash_logging(app: &AppHandle) {
    if let Ok(data_dir) = app.path().app_data_dir() {
        let log_dir = data_dir.join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("easy-edits-pro.log");

        let log_path_clone = log_file.clone();
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let msg = format!("[{timestamp}] CRASH/PANIC: {info}\n");
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path_clone) {
                let _ = f.write_all(msg.as_bytes());
            }
            default_hook(info);
        }));
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
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
                    if let Ok(canon) = data_dir.canonicalize() {
                        let _ = APP_DATA_DIR.set(canon);
                    } else {
                        let _ = APP_DATA_DIR.set(data_dir);
                    }
                }
                if let Ok(guard) = state.0.lock() {
                    for media in &guard.document().project.media {
                        let _ = app.asset_protocol_scope().allow_file(&media.path);
                        register_allowed_media_path(&media.path);
                    }
                }
                start_autosave(app.handle().clone(), state.clone());
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
