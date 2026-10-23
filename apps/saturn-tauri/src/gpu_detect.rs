//! Linux runtime probing for the WebKitGTK preview path.
//!
//! The editor used to force `WEBKIT_DISABLE_DMABUF_RENDERER=1` on every Linux
//! start. Phase 0 showed that this single variable is the difference between a
//! 59 fps editor and a 2 fps editor on Intel Iris Xe, so DMA-BUF is now enabled
//! by default. Disabling it is opt-in:
//!
//! * `EEP_DISABLE_DMABUF=1` disables it for every driver.
//! * `EEP_DISABLE_DMABUF=nvidia,amdgpu` disables it only when one of the
//!   detected DRM cards uses a matching driver or vendor.
//! * `EEP_KEEP_DMABUF=1` is the old Phase 0 escape hatch and still forces the
//!   renderer on; it is deprecated because enabled is now the default.
//!
//! An externally set `WEBKIT_DISABLE_DMABUF_RENDERER` is respected when no
//! `EEP_*` override is present. The decision is made before Tauri/GTK/WebKit
//! start, and the result is exposed to the UI through `glass_environment` so the
//! `[data-webkit-gtk-slow]` fallback can be applied without a benchmark.

use saturn_render::HardwareEncoderInfo;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Process-wide cache of the hardware-encoder probe result (F5). Startup
/// seeds this on a background job so the Deliver workspace can read it
/// without ever triggering GStreamer initialization on the UI path.
static ENCODER_INFO_CACHE: OnceLock<HardwareEncoderInfo> = OnceLock::new();

/// Returns the cached hardware-encoder probe result, running `probe` exactly
/// once per process when the cache is still empty.
pub fn cached_encoder_info(probe: impl FnOnce() -> HardwareEncoderInfo) -> HardwareEncoderInfo {
    cached_encoder_info_with(&ENCODER_INFO_CACHE, probe)
}

fn cached_encoder_info_with(
    cache: &OnceLock<HardwareEncoderInfo>,
    probe: impl FnOnce() -> HardwareEncoderInfo,
) -> HardwareEncoderInfo {
    if let Some(info) = cache.get() {
        return info.clone();
    }
    let info = probe();
    let _ = cache.set(info.clone());
    info
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuDevice {
    pub vendor: String,
    pub model: String,
    pub class: String,
    pub driver: String,
    pub pci_slot: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEnvironment {
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
    pub gpu_devices: Vec<GpuDevice>,
    pub dmabuf_renderer_disabled: bool,
    pub dmabuf_policy: String,
    pub webkitgtk_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmabufDecision {
    pub disabled: bool,
    /// A stable, machine-readable reason. See the match in `apply_dmabuf`.
    pub policy: &'static str,
    pub reason: String,
}

static DMABUF_DECISION: OnceLock<DmabufDecision> = OnceLock::new();

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

fn vendor_name(vendor_id: &str) -> String {
    match vendor_id.to_ascii_lowercase().as_str() {
        "0x8086" => "Intel".into(),
        "0x1002" | "0x1022" => "AMD".into(),
        "0x10de" => "NVIDIA".into(),
        "0x1af4" => "Virtio".into(),
        _ => vendor_id.to_string(),
    }
}

fn pci_slot(path: &std::path::Path) -> String {
    std::fs::read_link(path)
        .ok()
        .and_then(|target| {
            target
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// `lspci` model for this card's PCI slot, when available.
fn gpu_model_for_slot(vendor: &str, slot: &str) -> String {
    if let Ok(output) = Command::new("lspci").arg("-nn").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let short_slot = slot.strip_prefix("0000:").unwrap_or(slot);
            for line in text.lines() {
                let lower = line.to_ascii_lowercase();
                if !(lower.contains("vga compatible controller")
                    || lower.contains("3d controller")
                    || lower.contains("display controller"))
                {
                    continue;
                }
                let line_slot = line.split_whitespace().next().unwrap_or("");
                if slot != "unknown" && line_slot != slot && line_slot != short_slot {
                    continue;
                }
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
    format!("{vendor} GPU")
}

fn gpu_class(vendor: &str, driver: &str, model: &str) -> String {
    let model = model.to_ascii_lowercase();
    match vendor {
        "NVIDIA" => "discrete".into(),
        // Intel Arc is a discrete card even though the vendor is Intel.
        "Intel" if model.contains("arc") => "discrete".into(),
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

/// Every DRM card, not just card0. On hybrid laptops the compositor and the
/// card the user wants to disable may differ, so the per-driver escape hatch
/// matches against all of them.
fn drm_cards() -> Vec<GpuDevice> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir("/sys/class/drm")
        .map(|directory| {
            directory
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.strip_prefix("card")
                                .is_some_and(|suffix| !suffix.is_empty() && !suffix.contains('-'))
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort();

    entries
        .into_iter()
        .map(|card| {
            let device = card.join("device");
            let vendor = read_trim(&device.join("vendor").to_string_lossy())
                .map(|value| vendor_name(&value))
                .unwrap_or_else(|| "unknown".into());
            let slot = pci_slot(&device);
            let driver = std::fs::read_link(device.join("driver"))
                .ok()
                .and_then(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| "unknown".into());
            let model = gpu_model_for_slot(&vendor, &slot);
            let class = gpu_class(&vendor, &driver, &model);
            GpuDevice {
                vendor,
                model,
                class,
                driver,
                pci_slot: slot,
            }
        })
        .collect()
}

fn webkitgtk_version() -> String {
    if let Some(version) = std::env::var("WEBKITGTK_VERSION")
        .ok()
        .filter(|value| !value.is_empty())
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

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_falsey(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

/// Pure policy used by `configure_dmabuf` and unit tests.
///
/// `drivers` and `vendors` are the identities of every detected DRM card.
/// `webkit` is the value the process inherited from its parent, if any.
fn decide_dmabuf(
    disable: Option<&str>,
    keep: Option<&str>,
    webkit: Option<&str>,
    drivers: &[String],
    vendors: &[String],
) -> DmabufDecision {
    let external_disabled = webkit.is_some_and(is_truthy);
    let external_enabled = webkit.is_some_and(is_falsey);

    if keep.is_some_and(is_truthy) {
        if external_disabled {
            return DmabufDecision {
                disabled: true,
                policy: "disabled-external",
                reason: "WEBKIT_DISABLE_DMABUF_RENDERER was set by the environment; EEP_KEEP_DMABUF no longer overrides it".into(),
            };
        }
        return DmabufDecision {
            disabled: false,
            policy: "enabled-keep",
            reason: "EEP_KEEP_DMABUF set (deprecated; DMA-BUF is enabled by default)".into(),
        };
    }

    if let Some(raw) = disable {
        if is_falsey(raw) {
            return DmabufDecision {
                disabled: false,
                policy: "enabled-explicit",
                reason: "EEP_DISABLE_DMABUF is explicitly off".into(),
            };
        }
        if is_truthy(raw) || raw.trim().eq_ignore_ascii_case("all") {
            return DmabufDecision {
                disabled: true,
                policy: "disabled-env",
                reason: "EEP_DISABLE_DMABUF is set".into(),
            };
        }

        let tokens: Vec<String> = raw
            .split(',')
            .map(|token| token.trim().to_ascii_lowercase())
            .filter(|token| !token.is_empty())
            .collect();
        let matched = tokens.iter().any(|token| {
            drivers
                .iter()
                .any(|driver| driver.to_ascii_lowercase().contains(token))
                || vendors
                    .iter()
                    .any(|vendor| vendor.to_ascii_lowercase().contains(token))
        });
        if matched {
            return DmabufDecision {
                disabled: true,
                policy: "disabled-driver",
                reason: format!("EEP_DISABLE_DMABUF matched {}", drivers.join(",")),
            };
        }
        if external_disabled || external_enabled {
            return DmabufDecision {
                disabled: external_disabled,
                policy: if external_disabled {
                    "disabled-external"
                } else {
                    "enabled-external"
                },
                reason: format!(
                    "EEP_DISABLE_DMABUF did not match {}; inherited WebKit setting is used",
                    drivers.join(",")
                ),
            };
        }
        return DmabufDecision {
            disabled: false,
            policy: "enabled-unmatched",
            reason: format!(
                "EEP_DISABLE_DMABUF did not match {}; DMA-BUF stays enabled",
                drivers.join(",")
            ),
        };
    }

    if external_disabled {
        return DmabufDecision {
            disabled: true,
            policy: "disabled-external",
            reason: "WEBKIT_DISABLE_DMABUF_RENDERER was set by the environment".into(),
        };
    }
    if external_enabled {
        return DmabufDecision {
            disabled: false,
            policy: "enabled-external",
            reason: "WEBKIT_DISABLE_DMABUF_RENDERER was explicitly enabled by the environment"
                .into(),
        };
    }
    DmabufDecision {
        disabled: false,
        policy: "enabled-default",
        reason: "DMA-BUF is enabled by default".into(),
    }
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Must run before GTK/WebKit initialize. Safe to call more than once.
pub fn configure_dmabuf() -> DmabufDecision {
    if let Some(decision) = DMABUF_DECISION.get() {
        return decision.clone();
    }

    let cards = drm_cards();
    let drivers: Vec<String> = cards.iter().map(|card| card.driver.clone()).collect();
    let vendors: Vec<String> = cards.iter().map(|card| card.vendor.clone()).collect();
    let disable = env_string("EEP_DISABLE_DMABUF");
    let keep = env_string("EEP_KEEP_DMABUF");
    let webkit = env_string("WEBKIT_DISABLE_DMABUF_RENDERER");
    let decision = decide_dmabuf(
        disable.as_deref(),
        keep.as_deref(),
        webkit.as_deref(),
        &drivers,
        &vendors,
    );

    #[cfg(target_os = "linux")]
    match decision.policy {
        // An explicit EEP request wins over an inherited WebKit value.
        "enabled-keep" | "enabled-explicit" => unsafe {
            std::env::remove_var("WEBKIT_DISABLE_DMABUF_RENDERER");
        },
        "disabled-env" | "disabled-driver" => unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        },
        _ => {}
    }

    let cards_label = if cards.is_empty() {
        "no DRM card".to_string()
    } else {
        cards
            .iter()
            .map(|card| format!("{} {} [{}]", card.vendor, card.model, card.driver))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let webkit_after = std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok();
    eprintln!(
        "[dmabuf] enabled={} policy={} webkit={:?} gpu={} reason={}",
        !decision.disabled, decision.policy, webkit_after, cards_label, decision.reason
    );

    let _ = DMABUF_DECISION.set(decision.clone());
    decision
}

fn current_decision() -> DmabufDecision {
    if let Some(decision) = DMABUF_DECISION.get() {
        return decision.clone();
    }
    let cards = drm_cards();
    let drivers: Vec<String> = cards.iter().map(|card| card.driver.clone()).collect();
    let vendors: Vec<String> = cards.iter().map(|card| card.vendor.clone()).collect();
    decide_dmabuf(
        env_string("EEP_DISABLE_DMABUF").as_deref(),
        env_string("EEP_KEEP_DMABUF").as_deref(),
        env_string("WEBKIT_DISABLE_DMABUF_RENDERER").as_deref(),
        &drivers,
        &vendors,
    )
}

fn runtime_environment() -> RuntimeEnvironment {
    let cards = drm_cards();
    let primary = cards.first();
    let decision = current_decision();
    RuntimeEnvironment {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_name: os_name(),
        kernel: kernel_version(),
        session_type: session_type(),
        cpu_model: cpu_model(),
        ram_mb: ram_mb(),
        gpu_vendor: primary
            .map(|card| card.vendor.clone())
            .unwrap_or_else(|| "unknown".into()),
        gpu_model: primary
            .map(|card| format!("{} [{}]", card.model, card.pci_slot))
            .unwrap_or_else(|| "unknown".into()),
        gpu_class: primary
            .map(|card| card.class.clone())
            .unwrap_or_else(|| "unknown".into()),
        gpu_driver: primary
            .map(|card| card.driver.clone())
            .unwrap_or_else(|| "unknown".into()),
        gpu_devices: cards,
        dmabuf_renderer_disabled: decision.disabled,
        dmabuf_policy: decision.policy.to_string(),
        webkitgtk_version: webkitgtk_version(),
    }
}

#[tauri::command]
pub fn glass_environment() -> RuntimeEnvironment {
    runtime_environment()
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

/// Persist one benchmark report in the user's state directory, never in the repo.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_probe_cache_runs_the_probe_once() {
        let calls = std::cell::Cell::new(0_u32);
        let cache = OnceLock::new();
        let first = cached_encoder_info_with(&cache, || {
            calls.set(calls.get() + 1);
            HardwareEncoderInfo {
                has_hardware_encoder: false,
                preferred_encoder: "x264enc".to_string(),
                encoder_type: "software".to_string(),
                description: "software fallback".to_string(),
                available_encoders: vec!["x264enc".to_string()],
            }
        });
        assert!(!first.has_hardware_encoder, "cold cache probes");
        // A second read must come from the cache; the probe closure panics to
        // prove it never runs again.
        let second = cached_encoder_info_with(&cache, || {
            panic!("cached probe ran twice");
        });
        assert_eq!(second.preferred_encoder, "x264enc");
        assert_eq!(calls.get(), 1);
    }

    fn decide(disable: Option<&str>, keep: Option<&str>, webkit: Option<&str>) -> DmabufDecision {
        decide_dmabuf(
            disable,
            keep,
            webkit,
            &["i915".to_string()],
            &["Intel".to_string()],
        )
    }

    #[test]
    fn defaults_to_enabled() {
        let decision = decide(None, None, None);
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-default");
    }

    #[test]
    fn explicit_one_disables_every_driver() {
        let decision = decide(Some("1"), None, None);
        assert!(decision.disabled);
        assert_eq!(decision.policy, "disabled-env");
    }

    #[test]
    fn explicit_zero_forces_enabled() {
        let decision = decide(Some("0"), None, Some("1"));
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-explicit");
    }

    #[test]
    fn matching_driver_disables() {
        let decision = decide(Some("intel"), None, None);
        assert!(decision.disabled);
        assert_eq!(decision.policy, "disabled-driver");
    }

    #[test]
    fn non_matching_driver_stays_enabled() {
        let decision = decide(Some("nvidia"), None, None);
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-unmatched");
    }

    #[test]
    fn non_matching_driver_respects_external_disable() {
        let decision = decide(Some("nvidia"), None, Some("1"));
        assert!(decision.disabled);
        assert_eq!(decision.policy, "disabled-external");
    }

    #[test]
    fn keep_hatch_forces_enabled() {
        let decision = decide(Some("1"), Some("1"), None);
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-keep");
    }

    #[test]
    fn keep_hatch_does_not_override_an_external_disable() {
        let decision = decide(None, Some("1"), Some("1"));
        assert!(decision.disabled);
        assert_eq!(decision.policy, "disabled-external");
    }

    #[test]
    fn external_webkit_value_is_respected() {
        let decision = decide(None, None, Some("1"));
        assert!(decision.disabled);
        assert_eq!(decision.policy, "disabled-external");
    }

    #[test]
    fn external_webkit_zero_is_enabled() {
        let decision = decide(None, None, Some("0"));
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-external");
    }

    #[test]
    fn vendor_name_matches_too() {
        let decision = decide(Some("nvidia,amd"), None, None);
        assert!(!decision.disabled);
        assert_eq!(decision.policy, "enabled-unmatched");
    }
}
