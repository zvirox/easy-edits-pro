// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Data models, presets, and evidence structures for `verify.run` (AF3).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Delivery preset configurations governing target metrics and tolerance thresholds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerifyPreset {
    pub name: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: String,
    pub target_loudness_lufs: f64,
    pub loudness_tolerance_lu: f64,
    pub max_true_peak_dbtp: f64,
    pub max_silence_duration_seconds: f64,
    pub max_caption_cps: f64,
    pub title_safe_margin_pct: f64,
}

impl VerifyPreset {
    /// Web 1080p delivery preset (YouTube, Vimeo, Web, -14 LUFS, max -1 dBTP).
    pub fn web_1080p() -> Self {
        Self {
            name: "web-1080p".into(),
            label: "Web 1080p (16:9, -14 LUFS)".into(),
            width: 1920,
            height: 1080,
            aspect_ratio: "16:9".into(),
            target_loudness_lufs: -14.0,
            loudness_tolerance_lu: 1.0,
            max_true_peak_dbtp: -1.0,
            max_silence_duration_seconds: 2.0,
            max_caption_cps: 25.0,
            title_safe_margin_pct: 10.0,
        }
    }

    /// Vertical 9:16 delivery preset (Shorts, Reels, TikTok, -14 LUFS, 15% UI safe margin).
    pub fn vertical_9x16() -> Self {
        Self {
            name: "vertical-9x16".into(),
            label: "Vertical 9:16 (Shorts/Reels, -14 LUFS)".into(),
            width: 1080,
            height: 1920,
            aspect_ratio: "9:16".into(),
            target_loudness_lufs: -14.0,
            loudness_tolerance_lu: 1.0,
            max_true_peak_dbtp: -1.0,
            max_silence_duration_seconds: 1.5,
            max_caption_cps: 22.0,
            title_safe_margin_pct: 15.0,
        }
    }

    /// Broadcast EBU R128 delivery preset (Television/Linear, -23 LUFS, max -1 dBTP).
    pub fn broadcast_r128() -> Self {
        Self {
            name: "broadcast-r128".into(),
            label: "Broadcast EBU R128 (16:9, -23 LUFS)".into(),
            width: 1920,
            height: 1080,
            aspect_ratio: "16:9".into(),
            target_loudness_lufs: -23.0,
            loudness_tolerance_lu: 0.5,
            max_true_peak_dbtp: -1.0,
            max_silence_duration_seconds: 1.0,
            max_caption_cps: 20.0,
            title_safe_margin_pct: 10.0,
        }
    }

    /// Resolves a preset by name, defaulting to `web-1080p`.
    pub fn from_name_or_default(name: Option<&str>) -> Self {
        match name.unwrap_or("web-1080p").trim().to_lowercase().as_str() {
            "vertical-9x16" | "vertical" | "9x16" | "shorts" | "tiktok" => Self::vertical_9x16(),
            "broadcast-r128" | "broadcast" | "ebu-r128" | "r128" => Self::broadcast_r128(),
            _ => Self::web_1080p(),
        }
    }
}

/// Evaluation verdict for an individual check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckVerdict {
    Pass,
    Warn,
    Fail,
}

/// Measurement availability status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Measured,
    Unavailable,
    Skipped,
}

/// A timeline tick range highlighting an offending segment with detail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickRange {
    pub start_tick: u64,
    pub end_tick: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip_id: Option<u64>,
    pub detail: String,
}

/// Evidence and measured outcome for a single named check.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckEvidence {
    pub check_id: String,
    pub name: String,
    pub verdict: CheckVerdict,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Value>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offending_ranges: Vec<TickRange>,
    pub sample_coverage_pct: f64,
}

/// Overall summary count of checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifySummary {
    pub total_checks: usize,
    pub passed: usize,
    pub warned: usize,
    pub failed: usize,
    pub unavailable: usize,
}

/// Range of ticks analyzed by verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzedRange {
    pub start_tick: u64,
    pub end_tick: u64,
    pub duration_ticks: u64,
}

/// Notes accompanying verification (enforces no invented aggregate quality score).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerifyNotes {
    pub aggregate_score: Option<f64>,
    pub message: String,
}

impl Default for VerifyNotes {
    fn default() -> Self {
        Self {
            aggregate_score: None,
            message: "Measured evidence only; no invented aggregate quality score.".into(),
        }
    }
}

/// Complete verification report returned by `verify.run`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerifyReport {
    pub snapshot_id: String,
    pub sequence_id: u64,
    pub preset: String,
    pub analyzed_range: AnalyzedRange,
    pub checks: Vec<CheckEvidence>,
    pub summary: VerifySummary,
    pub notes: VerifyNotes,
}

/// Input parameters for `verify.run`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct VerifyParams {
    pub preset: Option<String>,
    pub start_tick: Option<u64>,
    pub end_tick: Option<u64>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub audio_metrics: Option<Value>,
    #[serde(default)]
    pub video_metrics: Option<Value>,
}
