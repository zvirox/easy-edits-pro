// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Audio and video perception checks: loudness, true peak, scopes clipping, crushed blacks, and frame anomalies (AF3).

use serde_json::{json, Value};

use crate::project::ProjectDocument;
use crate::time::Tick;
use super::types::{CheckEvidence, CheckStatus, CheckVerdict, TickRange, VerifyPreset};

/// Helper to extract an f64 field from json value either directly or nested under an object.
fn extract_f64(metrics: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(val) = metrics.get(key).and_then(Value::as_f64) {
            return Some(val);
        }
    }
    // Try nested "loudness" or "analysis"
    if let Some(loudness) = metrics.get("loudness").or_else(|| metrics.get("analysis")) {
        for key in keys {
            if let Some(val) = loudness.get(key).and_then(Value::as_f64) {
                return Some(val);
            }
        }
    }
    None
}

/// Helper to extract a list of `TickRange` anomalies from a metrics value.
fn extract_tick_ranges(metrics: &Value, key: &str, default_detail: &str) -> Vec<TickRange> {
    let mut ranges = Vec::new();
    if let Some(items) = metrics.get(key).and_then(Value::as_array) {
        for item in items {
            let start_tick = item.get("start_tick").and_then(Value::as_u64).unwrap_or(0);
            let end_tick = item.get("end_tick").and_then(Value::as_u64).unwrap_or(start_tick);
            let clip_id = item.get("clip_id").and_then(Value::as_u64);
            let detail = item
                .get("detail")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default_detail.to_owned());
            ranges.push(TickRange {
                start_tick,
                end_tick,
                clip_id,
                detail,
            });
        }
    }
    ranges
}

/// Verifies integrated audio loudness (LUFS) against brief/preset targets.
pub fn verify_loudness(
    document: &ProjectDocument,
    preset: &VerifyPreset,
    audio_metrics: Option<&Value>,
) -> CheckEvidence {
    let target_lufs = document
        .project
        .brief
        .loudness_target_lufs
        .unwrap_or(preset.target_loudness_lufs);
    let tolerance_lu = preset.loudness_tolerance_lu;

    let Some(metrics) = audio_metrics else {
        return CheckEvidence {
            check_id: "audio.loudness".into(),
            name: "Integrated Loudness".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({
                "target_lufs": target_lufs,
                "tolerance_lu": tolerance_lu
            })),
            message: "Audio telemetry unavailable; render audio mix to measure integrated loudness.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let measured_lufs = extract_f64(metrics, &["integrated_lufs", "integrated"]);
    let Some(lufs) = measured_lufs else {
        return CheckEvidence {
            check_id: "audio.loudness".into(),
            name: "Integrated Loudness".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({
                "target_lufs": target_lufs,
                "tolerance_lu": tolerance_lu
            })),
            message: "Integrated loudness missing in supplied audio telemetry.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let diff = (lufs - target_lufs).abs();
    let within_tolerance = diff <= tolerance_lu;

    let (verdict, message, offending_ranges) = if within_tolerance {
        (
            CheckVerdict::Pass,
            format!(
                "Integrated loudness ({:.1} LUFS) conforms to target ({:.1} LUFS ±{:.1} LU).",
                lufs, target_lufs, tolerance_lu
            ),
            Vec::new(),
        )
    } else {
        let msg = format!(
            "Integrated loudness ({:.1} LUFS) deviates from target ({:.1} LUFS ±{:.1} LU) by {:.1} LU.",
            lufs, target_lufs, tolerance_lu, diff
        );
        let offending = vec![TickRange {
            start_tick: 0,
            end_tick: document.project.sequence.duration().0.max(0) as u64,
            clip_id: None,
            detail: msg.clone(),
        }];
        (CheckVerdict::Fail, msg, offending)
    };

    CheckEvidence {
        check_id: "audio.loudness".into(),
        name: "Integrated Loudness".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({
            "integrated_lufs": lufs,
            "deviation_lu": diff
        })),
        threshold: Some(json!({
            "target_lufs": target_lufs,
            "tolerance_lu": tolerance_lu,
            "min_allowed_lufs": target_lufs - tolerance_lu,
            "max_allowed_lufs": target_lufs + tolerance_lu
        })),
        message,
        offending_ranges,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies true peak audio levels (dBTP) against preset peak ceiling.
pub fn verify_true_peak(
    document: &ProjectDocument,
    preset: &VerifyPreset,
    audio_metrics: Option<&Value>,
) -> CheckEvidence {
    let max_peak_dbtp = preset.max_true_peak_dbtp;

    let Some(metrics) = audio_metrics else {
        return CheckEvidence {
            check_id: "audio.true_peak".into(),
            name: "True Peak Ceiling".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({"max_true_peak_dbtp": max_peak_dbtp})),
            message: "True peak telemetry unavailable; render audio mix to measure true peak.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let measured_peak = extract_f64(metrics, &["true_peak_dbtp", "true_peak"]);
    let Some(peak) = measured_peak else {
        return CheckEvidence {
            check_id: "audio.true_peak".into(),
            name: "True Peak Ceiling".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({"max_true_peak_dbtp": max_peak_dbtp})),
            message: "True peak metric missing in supplied audio telemetry.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let within_limit = peak <= max_peak_dbtp;
    let (verdict, message, offending_ranges) = if within_limit {
        (
            CheckVerdict::Pass,
            format!("Maximum true peak ({:.1} dBTP) is within ceiling ({:.1} dBTP).", peak, max_peak_dbtp),
            Vec::new(),
        )
    } else {
        let excess = peak - max_peak_dbtp;
        let msg = format!(
            "Maximum true peak ({:.1} dBTP) exceeds ceiling ({:.1} dBTP) by {:.1} dB.",
            peak, max_peak_dbtp, excess
        );
        let offending = vec![TickRange {
            start_tick: 0,
            end_tick: document.project.sequence.duration().0.max(0) as u64,
            clip_id: None,
            detail: msg.clone(),
        }];
        (CheckVerdict::Fail, msg, offending)
    };

    CheckEvidence {
        check_id: "audio.true_peak".into(),
        name: "True Peak Ceiling".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({"true_peak_dbtp": peak})),
        threshold: Some(json!({"max_true_peak_dbtp": max_peak_dbtp})),
        message,
        offending_ranges,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies highlight clipping percentage from video scopes.
pub fn verify_clipped_highlights(
    document: &ProjectDocument,
    _preset: &VerifyPreset,
    video_metrics: Option<&Value>,
) -> CheckEvidence {
    let warn_threshold_pct = 1.0f64;
    let fail_threshold_pct = 5.0f64;

    let Some(metrics) = video_metrics else {
        return CheckEvidence {
            check_id: "video.clipped_highlights".into(),
            name: "Highlight Clipping".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({
                "warn_threshold_pct": warn_threshold_pct,
                "fail_threshold_pct": fail_threshold_pct
            })),
            message: "Color scopes telemetry unavailable; sample video frames to evaluate clipping.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let clip_pct = extract_f64(metrics, &["clipped_highlights_pct", "clipped_pct"]).unwrap_or(0.0);
    let offending_ranges = extract_tick_ranges(metrics, "clipped_ranges", "Highlight clipping detected in frame");

    let (verdict, message) = if clip_pct <= warn_threshold_pct && offending_ranges.is_empty() {
        (
            CheckVerdict::Pass,
            format!("Highlight clipping is within acceptable limits ({:.2}% of pixels).", clip_pct),
        )
    } else if clip_pct <= fail_threshold_pct {
        (
            CheckVerdict::Warn,
            format!("Moderate highlight clipping observed: {:.2}% of analyzed pixels.", clip_pct),
        )
    } else {
        (
            CheckVerdict::Fail,
            format!("Severe highlight clipping observed: {:.2}% of analyzed pixels exceed 98 IRE.", clip_pct),
        )
    };

    let offending = if offending_ranges.is_empty() && verdict != CheckVerdict::Pass {
        vec![TickRange {
            start_tick: 0,
            end_tick: document.project.sequence.duration().0.max(0) as u64,
            clip_id: None,
            detail: format!("Clipped highlights: {:.2}% of frame pixels", clip_pct),
        }]
    } else {
        offending_ranges
    };

    CheckEvidence {
        check_id: "video.clipped_highlights".into(),
        name: "Highlight Clipping".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({"clipped_highlights_pct": clip_pct})),
        threshold: Some(json!({
            "warn_threshold_pct": warn_threshold_pct,
            "fail_threshold_pct": fail_threshold_pct
        })),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies crushed blacks / shadows percentage from video scopes.
pub fn verify_crushed_blacks(
    document: &ProjectDocument,
    _preset: &VerifyPreset,
    video_metrics: Option<&Value>,
) -> CheckEvidence {
    let warn_threshold_pct = 2.0f64;
    let fail_threshold_pct = 10.0f64;

    let Some(metrics) = video_metrics else {
        return CheckEvidence {
            check_id: "video.crushed_blacks".into(),
            name: "Crushed Blacks".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({
                "warn_threshold_pct": warn_threshold_pct,
                "fail_threshold_pct": fail_threshold_pct
            })),
            message: "Color scopes telemetry unavailable; sample video frames to evaluate crushed blacks.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let crush_pct = extract_f64(metrics, &["crushed_shadows_pct", "crushed_pct"]).unwrap_or(0.0);
    let offending_ranges = extract_tick_ranges(metrics, "crushed_ranges", "Crushed shadows detected in frame");

    let (verdict, message) = if crush_pct <= warn_threshold_pct && offending_ranges.is_empty() {
        (
            CheckVerdict::Pass,
            format!("Shadow detail preserved; crushed blacks within limits ({:.2}%).", crush_pct),
        )
    } else if crush_pct <= fail_threshold_pct {
        (
            CheckVerdict::Warn,
            format!("Moderate shadow crushing observed: {:.2}% of analyzed pixels.", crush_pct),
        )
    } else {
        (
            CheckVerdict::Fail,
            format!("Severe crushed blacks observed: {:.2}% of analyzed pixels fall below 1 IRE.", crush_pct),
        )
    };

    let offending = if offending_ranges.is_empty() && verdict != CheckVerdict::Pass {
        vec![TickRange {
            start_tick: 0,
            end_tick: document.project.sequence.duration().0.max(0) as u64,
            clip_id: None,
            detail: format!("Crushed shadows: {:.2}% of frame pixels", crush_pct),
        }]
    } else {
        offending_ranges
    };

    CheckEvidence {
        check_id: "video.crushed_blacks".into(),
        name: "Crushed Blacks".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({"crushed_shadows_pct": crush_pct})),
        threshold: Some(json!({
            "warn_threshold_pct": warn_threshold_pct,
            "fail_threshold_pct": fail_threshold_pct
        })),
        message,
        offending_ranges: offending,
        sample_coverage_pct: 100.0,
    }
}

/// Verifies presence of unintended black or frozen frames.
pub fn verify_black_and_frozen_frames(
    _document: &ProjectDocument,
    _preset: &VerifyPreset,
    video_metrics: Option<&Value>,
    _start_tick: Tick,
    _end_tick: Tick,
) -> CheckEvidence {
    let Some(metrics) = video_metrics else {
        return CheckEvidence {
            check_id: "video.black_and_frozen_frames".into(),
            name: "Black and Frozen Frames".into(),
            verdict: CheckVerdict::Warn,
            status: CheckStatus::Unavailable,
            measured: None,
            threshold: Some(json!({"max_anomalous_frames": 0})),
            message: "Temporal video frame analysis unavailable; multi-frame sampling required.".into(),
            offending_ranges: Vec::new(),
            sample_coverage_pct: 0.0,
        };
    };

    let black_ranges = extract_tick_ranges(metrics, "black_frames", "Unintended black frame detected");
    let frozen_ranges = extract_tick_ranges(metrics, "frozen_frames", "Unintended frozen frame detected");

    let mut combined_offending = black_ranges;
    combined_offending.extend(frozen_ranges);

    let verdict = if combined_offending.is_empty() {
        CheckVerdict::Pass
    } else {
        CheckVerdict::Warn
    };

    let message = if combined_offending.is_empty() {
        "No unintended black or frozen frames detected across sampled intervals.".into()
    } else {
        format!(
            "Detected {} anomalous black or frozen frame segment(s).",
            combined_offending.len()
        )
    };

    CheckEvidence {
        check_id: "video.black_and_frozen_frames".into(),
        name: "Black and Frozen Frames".into(),
        verdict,
        status: CheckStatus::Measured,
        measured: Some(json!({"anomalies_count": combined_offending.len()})),
        threshold: Some(json!({"max_anomalous_frames": 0})),
        message,
        offending_ranges: combined_offending,
        sample_coverage_pct: 100.0,
    }
}
