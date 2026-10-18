use super::super::*;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnostics {
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
pub fn get_system_diagnostics(app: AppHandle) -> SystemDiagnostics {
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
        .map(|p| {
            p.join("logs")
                .join("easy-edits-pro.log")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_default();

    let os_name = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return Some(
                            line.trim_start_matches("PRETTY_NAME=")
                                .trim_matches('"')
                                .to_string(),
                        );
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
        media_server_port: media_server::get_port(),
        log_file_path,
    }
}

pub fn setup_crash_logging(app: &AppHandle) {
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
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path_clone)
            {
                let _ = f.write_all(msg.as_bytes());
            }
            default_hook(info);
        }));
    }
}
