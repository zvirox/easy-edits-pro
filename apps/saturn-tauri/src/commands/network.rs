use super::super::*;
use super::media::{generate_waveform_peaks, probe_media_file};

#[tauri::command]
pub async fn import_network_media(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    custom_name: Option<String>,
) -> Result<Value, String> {
    let url_obj = reqwest::Url::parse(&url).map_err(|e| format!("Invalid URL `{url}`: {e}"))?;
    if !matches!(url_obj.scheme(), "http" | "https") {
        return Err("Only http:// and https:// URLs are supported for network import".into());
    }

    let mut response = net::fetch_public_http(url_obj.clone())
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
            return Err(
                "The network asset payload exceeded Easy Edits Pro's 512 MiB limit during download"
                    .into(),
            );
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
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
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

    let (kind, duration, width, height, fps, audio_channels, codec, _probe_ok) =
        probe_media_file(&discoverer, &target_file);
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
