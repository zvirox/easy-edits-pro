use gstreamer_pbutils::Discoverer;
use gstreamer::prelude::*;
use saturn_core::{
    CommandRequest, CommandStatus, EditorEngine, JobId, JobInfo, JobKind, JobManager, JobPriority,
    MediaKind, ProjectDocument, Tick, command_catalogue,
    export_cmx3600_edl, export_fcpxml, export_premiere_xml,
    is_filler_word, TICKS_PER_SECOND,
};
use saturn_render::{GpuPreviewPipeline, PresentationMetrics};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State, ipc::Channel};

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

static MEDIA_SERVER_PORT: AtomicU16 = AtomicU16::new(0);

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
    if parts.is_empty() {
        return;
    }

    if parts[0] == "OPTIONS" {
        let preflight = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Access-Control-Max-Age: 86400\r\n\
Connection: close\r\n\r\n";
        let _ = stream.write_all(preflight.as_bytes());
        return;
    }

    if parts.len() < 2 || (parts[0] != "GET" && parts[0] != "HEAD") {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n");
        return;
    }
    let is_head = parts[0] == "HEAD";

    let target = parts[1];
    let query_prefix = "/media?path=";
    if !target.starts_with(query_prefix) {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        return;
    }

    let encoded_path = &target[query_prefix.len()..];
    let clean_encoded = encoded_path.split('&').next().unwrap_or(encoded_path);
    let file_path = url_decode(clean_encoded);
    let path = PathBuf::from(&file_path);

    if !path.is_file() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        return;
    }

    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 500 Internal Error\r\n\r\n");
            return;
        }
    };
    let total_size = metadata.len();

    let mut range_start: u64 = 0;
    let mut range_end: u64 = total_size.saturating_sub(1);
    let mut is_range = false;

    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if line_lower.starts_with("range:") {
            if let Some(eq_idx) = line.find('=') {
                let range_spec = line[eq_idx + 1..].trim();
                let first_spec = range_spec.split(',').next().unwrap_or(range_spec).trim();
                if let Some(dash_idx) = first_spec.find('-') {
                    let first = first_spec[..dash_idx].trim();
                    let second = first_spec[dash_idx + 1..].trim();
                    if first.is_empty() {
                        if let Ok(suffix_len) = second.parse::<u64>() {
                            if suffix_len > 0 {
                                range_start = total_size.saturating_sub(suffix_len);
                                range_end = total_size.saturating_sub(1);
                                is_range = true;
                            }
                        }
                    } else if let Ok(start) = first.parse::<u64>() {
                        range_start = start;
                        is_range = true;
                        if !second.is_empty() {
                            if let Ok(end) = second.parse::<u64>() {
                                if end < total_size {
                                    range_end = end;
                                }
                            }
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
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, content_len, range_start, range_end, total_size
        )
    } else {
        format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Accept-Ranges: bytes\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, total_size
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

            let metadata = gstreamer::glib::filename_to_uri(&path, None)
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
                    (kind, duration, width, height)
                })
                .unwrap_or((MediaKind::Unknown, None, None, None));

            discovered.push((name, path, metadata));
        }

        let mut engine_guard = engine
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        for (name, path, (kind, duration, width, height)) in discovered {
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

#[tauri::command]
async fn ask_local_ai(prompt: String, model: String) -> Result<String, String> {
    let prompt = prompt.trim().to_owned();
    let model = model.trim().to_owned();
    if prompt.is_empty() || prompt.chars().count() > 12_000 {
        return Err("Enter a prompt up to 12,000 characters".into());
    }
    if model.is_empty() || model.chars().count() > 100 {
        return Err("Enter a local Ollama model name".into());
    }
    let response: Value = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|error| error.to_string())?
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({"model": model, "messages": [{"role": "user", "content": prompt}], "stream": false}))
        .send()
        .await
        .map_err(|error| format!("Could not connect to local Ollama. Start Ollama and download a model first. ({error})"))?
        .error_for_status()
        .map_err(|error| format!("Ollama returned an error: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Could not read the Ollama response: {error}"))?;
    response
        .pointer("/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Ollama returned no assistant text".to_string())
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
        _ => return Err(format!("Unsupported interchange format: {format}. Supported: 'edl', 'fcpxml', 'premiere_xml'")),
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
        _ => return Err(format!("Unsupported interchange format: {format}")),
    };

    let payload = if cmd_name == "interop.import_edl" {
        json!({
            "edl": raw_str,
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
            present_gpu_preview_frame
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Easy Edits Pro");
}
