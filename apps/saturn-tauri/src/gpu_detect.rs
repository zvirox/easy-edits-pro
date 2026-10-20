//! Phase 0 glass spike: environment probing and report capture.
//!
//! This module is intentionally small and self-contained so the spike branch can
//! be thrown away without touching the editor. It answers three questions the
//! plan needs before committing to glass:
//!
//! 1. Which GPU/WebKitGTK stack is the measurement running on?
//! 2. Is WebKit's DMA-BUF renderer enabled for this window?
//! 3. Where do the measured frame-time reports go?

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassEnvironment {
    pub app_version: String,
    pub os_name: String,
    pub kernel: String,
    pub session_type: String,
    pub cpu_model: String,
    pub ram_mb: u64,
    pub gpu_vendor: String,
    pub gpu_model: String,
    pub gpu_class: String,
    pub gpu_driver: String,
    pub dmabuf_renderer_disabled: bool,
    pub webkitgtk_version: String,
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn os_name() -> String {
    read_trim("/etc/os-release")
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "Linux".into())
}

fn kernel_version() -> String {
    read_trim("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".into())
}

fn session_type() -> String {
    if let Ok(value) = std::env::var("XDG_SESSION_TYPE") {
        if !value.is_empty() {
            return value;
        }
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "wayland".into()
    } else if std::env::var_os("DISPLAY").is_some() {
        "x11".into()
    } else {
        "unknown".into()
    }
}

fn cpu_model() -> String {
    read_trim("/proc/cpuinfo")
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("model name")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(_, value)| value.trim().to_string())
            })
        })
        .unwrap_or_else(|| "unknown".into())
}

fn ram_mb() -> u64 {
    read_trim("/proc/meminfo")
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .map(|kilobytes| kilobytes / 1024)
        .unwrap_or(0)
}

/// First DRM card's PCI vendor/device ids and kernel driver.
fn drm_card() -> (String, String, String) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir("/sys/class/drm")
        .map(|directory| {
            directory
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("card") && !name.contains('-'))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort();

    let Some(card) = entries.first() else {
        return ("unknown".into(), "unknown".into(), "unknown".into());
    };

    let device = card.join("device");
    let vendor = read_trim(&device.join("vendor").to_string_lossy())
        .map(|value| vendor_name(&value))
        .unwrap_or_else(|| "unknown".into());
    let device_id =
        read_trim(&device.join("device").to_string_lossy()).unwrap_or_else(|| "unknown".into());
    let driver = std::fs::read_link(device.join("driver"))
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".into());

    (vendor, format!("{device_id} ({driver})"), driver)
}

fn vendor_name(vendor_id: &str) -> String {
    match vendor_id.to_ascii_lowercase().as_str() {
        "0x8086" => "Intel".into(),
        "0x1002" => "AMD".into(),
        "0x10de" => "NVIDIA".into(),
        "0x1af4" => "Virtio".into(),
        _ => vendor_id.to_string(),
    }
}

/// A human-readable model name. `lspci` is optional; the id is still useful.
fn gpu_model(vendor: &str) -> String {
    if let Ok(output) = Command::new("lspci").arg("-nn").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let lower = line.to_ascii_lowercase();
                if lower.contains("vga compatible controller")
                    || lower.contains("3d controller")
                    || lower.contains("display controller")
                {
                    if let Some((_, model)) = line.split_once(": ") {
                        return model
                            .split(" (rev")
                            .next()
                            .unwrap_or(model)
                            .trim()
                            .to_string();
                    }
                }
            }
        }
    }
    format!("{vendor} GPU")
}

fn gpu_class(vendor: &str, driver: &str) -> String {
    match vendor {
        "NVIDIA" => "discrete".into(),
        "Intel" => "integrated".into(),
        "AMD" => {
            let vram = read_trim("/sys/class/drm/card0/device/mem_info_vram_total")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            if vram > 0 {
                "discrete".into()
            } else {
                "integrated".into()
            }
        }
        _ if driver == "unknown" => "unknown".into(),
        _ => "unknown".into(),
    }
}

fn webkitgtk_version() -> String {
    if let Some(version) = std::env::var("WEBKITGTK_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
    {
        return version;
    }
    Command::new("pkg-config")
        .args(["--modversion", "webkit2gtk-4.1"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

#[tauri::command]
pub fn glass_environment() -> GlassEnvironment {
    let (vendor, device, driver) = drm_card();
    let model = gpu_model(&vendor);
    GlassEnvironment {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_name: os_name(),
        kernel: kernel_version(),
        session_type: session_type(),
        cpu_model: cpu_model(),
        ram_mb: ram_mb(),
        gpu_vendor: vendor.clone(),
        gpu_model: format!("{model} [{device}]"),
        gpu_class: gpu_class(&vendor, &driver),
        gpu_driver: driver,
        dmabuf_renderer_disabled: std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some(),
        webkitgtk_version: webkitgtk_version(),
    }
}

fn report_directory() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir);
    state.join("easy-edits-pro").join("glass-spike")
}

fn safe_label(label: &str) -> Result<&str, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err("report label must be 1-64 characters".into());
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err("report label may only contain letters, digits, '-', '_' and '.'".into());
    }
    Ok(trimmed)
}

/// Persist one benchmark report next to the CLI harness state, not in the repo.
#[tauri::command]
pub fn save_glass_report(label: String, report: String) -> Result<String, String> {
    let label = safe_label(&label)?;
    let directory = report_directory();
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;

    let path = directory.join(format!("{label}.json"));
    std::fs::write(&path, report)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}
