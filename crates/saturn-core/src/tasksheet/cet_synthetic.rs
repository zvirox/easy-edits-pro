// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Synthetic Capture Event Track (`cet-gen`) scenario generator ahead of hardware recorder capture.

use serde::{Deserialize, Serialize};

use super::cet::{parse_cet_jsonl, CetGenerator};

/// Supported synthetic CET scenarios for testing ahead of recorder hardware.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CetScenario {
    #[default]
    TutorialBasic,
    MultiWindow,
    RedactedPrivacy,
    NamedShortcuts,
}

impl CetScenario {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "tutorial_basic" | "tutorial" | "basic" => Ok(Self::TutorialBasic),
            "multi_window" | "multiwindow" => Ok(Self::MultiWindow),
            "redacted" | "redacted_privacy" | "privacy" => Ok(Self::RedactedPrivacy),
            "named_shortcuts" | "shortcuts" => Ok(Self::NamedShortcuts),
            other => Err(format!(
                "Unknown CET scenario `{other}`; expected tutorial_basic, multi_window, redacted_privacy, or named_shortcuts"
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TutorialBasic => "tutorial_basic",
            Self::MultiWindow => "multi_window",
            Self::RedactedPrivacy => "redacted_privacy",
            Self::NamedShortcuts => "named_shortcuts",
        }
    }
}

/// Configuration for `generate_synthetic_cet` (`cet-gen`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyntheticCetOptions {
    #[serde(default)]
    pub scenario: CetScenario,
    #[serde(default = "default_video_file")]
    pub video_file: String,
    #[serde(default = "default_fps")]
    pub fps: f64,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_duration_ms")]
    pub duration_ms: u64,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default)]
    pub named_shortcuts: bool,
}

fn default_video_file() -> String {
    "desktop_capture.mkv".to_string()
}

fn default_fps() -> f64 {
    30.0
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

fn default_duration_ms() -> u64 {
    24_000
}

fn default_seed() -> u64 {
    1
}

impl Default for SyntheticCetOptions {
    fn default() -> Self {
        Self {
            scenario: CetScenario::TutorialBasic,
            video_file: default_video_file(),
            fps: default_fps(),
            width: default_width(),
            height: default_height(),
            duration_ms: default_duration_ms(),
            seed: default_seed(),
            named_shortcuts: false,
        }
    }
}

fn splitmix64(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / ((1u64 << 53) as f64)
}

fn jitter_coord(base: f64, span: f64, rng: &mut u64) -> f64 {
    let delta = (splitmix64(rng) - 0.5) * span;
    (base + delta).clamp(0.05, 0.95)
}

/// Generates a deterministic CET v0.1 JSONL track (`cet-gen`).
pub fn generate_synthetic_cet(opts: &SyntheticCetOptions) -> Result<String, String> {
    if !opts.fps.is_finite() || opts.fps <= 0.0 {
        return Err("CET fps must be a positive finite number".into());
    }
    if opts.width == 0 || opts.height == 0 {
        return Err("CET video width and height must be positive".into());
    }
    if opts.duration_ms < 2_000 {
        return Err("CET duration_ms must be at least 2000 ms".into());
    }

    let mut rng = opts.seed ^ 0x5A17_CE71_u64;
    let mut cet = CetGenerator::new(&opts.video_file, opts.fps, opts.width, opts.height);
    let enable_named =
        opts.named_shortcuts || matches!(opts.scenario, CetScenario::NamedShortcuts);
    if matches!(opts.scenario, CetScenario::RedactedPrivacy) {
        cet.set_named_shortcuts_opt_in(false);
    } else {
        cet.set_named_shortcuts_opt_in(enable_named);
    }

    let dur = opts.duration_ms;
    let mut elapsed = 0u64;
    let step = |cet: &mut CetGenerator, elapsed: &mut u64, delta_ms: u64| {
        if delta_ms > 0 {
            cet.advance_ms(delta_ms);
            *elapsed = elapsed.saturating_add(delta_ms);
        }
    };

    match opts.scenario {
        CetScenario::TutorialBasic => {
            cet.emit_window("Easy Edits Demo", "Project Setup", 0.05, 0.05, 0.9, 0.9);
            cet.emit_mark("chapter", "Intro and Workspace Tour");
            step(&mut cet, &mut elapsed, 600);
            let x1 = jitter_coord(0.28, 0.04, &mut rng);
            let y1 = jitter_coord(0.32, 0.04, &mut rng);
            cet.emit_cursor(x1 - 0.08, y1 - 0.05, "arrow");
            step(&mut cet, &mut elapsed, 300);
            cet.emit_click(x1, y1, "left");
            cet.emit_zoom_hint(
                (x1 - 0.14).clamp(0.0, 0.7),
                (y1 - 0.14).clamp(0.0, 0.7),
                0.35,
                0.35,
                "focus_panel",
            );
            step(&mut cet, &mut elapsed, 1_400);
            cet.emit_key("character", &[], None);
            // Intentional dead-air gap (>= 1.5s) in middle
            step(&mut cet, &mut elapsed, 1_800);
            cet.emit_window("Timeline Inspector", "Trim and Colour", 0.05, 0.05, 0.9, 0.9);
            cet.emit_mark("chapter", "Inspector Adjustment");
            let x2 = jitter_coord(0.74, 0.04, &mut rng);
            let y2 = jitter_coord(0.58, 0.04, &mut rng);
            step(&mut cet, &mut elapsed, 450);
            cet.emit_click(x2, y2, "left");
            cet.emit_key("shortcut", &["ctrl"], Some("Ctrl+S"));
        }
        CetScenario::MultiWindow => {
            let apps = [
                ("Browser Docs", "API Reference", 0.24, 0.26, "API Overview"),
                ("Code Workspace", "handler.rs", 0.66, 0.42, "Edit Implementation"),
                ("Terminal", "cargo test", 0.38, 0.72, "Run Verification"),
            ];
            let chunk = (dur / 3).max(2_000);
            for (idx, (app, title, bx, by, note)) in apps.iter().enumerate() {
                if elapsed >= dur.saturating_sub(500) {
                    break;
                }
                cet.emit_window(app, title, 0.04, 0.04, 0.92, 0.92);
                cet.emit_mark("chapter", note);
                step(&mut cet, &mut elapsed, 400);
                let cx = jitter_coord(*bx, 0.05, &mut rng);
                let cy = jitter_coord(*by, 0.05, &mut rng);
                cet.emit_cursor(cx - 0.04, cy - 0.03, "arrow");
                step(&mut cet, &mut elapsed, 250);
                cet.emit_click(cx, cy, "left");
                if idx == 1 {
                    cet.emit_zoom_hint(
                        (cx - 0.16).clamp(0.0, 0.68),
                        (cy - 0.16).clamp(0.0, 0.68),
                        0.32,
                        0.32,
                        "code_focus",
                    );
                }
                let wait = chunk.saturating_sub(650).min(dur.saturating_sub(elapsed));
                step(&mut cet, &mut elapsed, wait);
            }
        }
        CetScenario::RedactedPrivacy => {
            cet.emit_window("Vault Client", "Credentials [redacted]", 0.1, 0.1, 0.8, 0.8);
            cet.emit_mark("chapter", "Account Setup");
            step(&mut cet, &mut elapsed, 500);
            cet.emit_click(0.45, 0.35, "left");
            step(&mut cet, &mut elapsed, 300);
            for _ in 0..4 {
                cet.emit_key("character", &[], None);
                step(&mut cet, &mut elapsed, 150);
            }
            cet.emit_key("shortcut", &["ctrl"], Some("Ctrl+S"));
        }
        CetScenario::NamedShortcuts => {
            cet.emit_window("Editor Pro", "Shortcut Workflow", 0.0, 0.0, 1.0, 1.0);
            cet.emit_mark("chapter", "Keyboard Workflow");
            step(&mut cet, &mut elapsed, 400);
            cet.emit_click(0.52, 0.48, "left");
            step(&mut cet, &mut elapsed, 300);
            cet.emit_key("shortcut", &["ctrl"], Some("Ctrl+K"));
            step(&mut cet, &mut elapsed, 600);
            cet.emit_key("shortcut", &["ctrl", "shift"], Some("Ctrl+Shift+B"));
            step(&mut cet, &mut elapsed, 800);
            cet.emit_click(0.82, 0.22, "left");
        }
    }

    if elapsed < dur {
        let remain = dur - elapsed;
        cet.advance_ms(remain.saturating_sub(150));
        cet.emit_cursor(0.5, 0.5, "arrow");
    }

    let jsonl = cet.to_jsonl();
    parse_cet_jsonl(&jsonl).map_err(|e| format!("Generated invalid CET track: {e}"))?;
    Ok(jsonl)
}
