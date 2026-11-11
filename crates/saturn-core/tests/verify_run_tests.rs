// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration tests and negative controls for `verify.run` (AF3).

use serde_json::json;

use saturn_core::actor::ActorId;
use saturn_core::audio_edit::audio_clip;
use saturn_core::brief::{ClipAgentNote, ConfidenceLevel, ProjectBrief};
use saturn_core::command::CommandRequest;
use saturn_core::engine::EditorEngine;
use saturn_core::{
    CaptionTrack, MediaAsset, MediaKind, ProjectDocument, SubtitleItem,
    Tick, TICKS_PER_SECOND,
};
use saturn_core::verification::{
    run_verification, CheckStatus, CheckVerdict, VerifyParams, VerifyPreset, VerifyReport,
};

fn t(seconds: f64) -> Tick {
    Tick((seconds * TICKS_PER_SECOND as f64) as i64)
}

fn create_clean_document() -> ProjectDocument {
    let mut doc = ProjectDocument::new("Clean Test Project");
    doc.project.settings.width = 1920;
    doc.project.settings.height = 1080;

    // Brief target: 30 seconds ± 1 second, -14 LUFS
    doc.project.brief = ProjectBrief {
        goal: "30-second web promo".into(),
        platform: Some("youtube".into()),
        target_duration_ticks: Some(t(30.0).0 as u64),
        duration_tolerance_ticks: Some(t(1.0).0 as u64),
        loudness_target_lufs: Some(-14.0),
        ..Default::default()
    };

    // Valid media asset
    doc.project.media.push(MediaAsset {
        id: 1,
        name: "Promo Source".into(),
        kind: MediaKind::Video,
        duration: Some(t(60.0)),
        is_missing: false,
        ..Default::default()
    });

    // Video track (0..30s)
    let mut video_clip = audio_clip(1, t(0.0), t(30.0));
    video_clip.id = 10;
    video_clip.audio_enabled = false;
    doc.project.sequence.tracks[0].clips = vec![video_clip];

    // Audio track with two clips (0..15s and 15..30s, seamless, no dead air)
    let mut audio_clip_1 = audio_clip(1, t(0.0), t(15.0));
    audio_clip_1.id = 20;
    audio_clip_1.audio_enabled = true;

    let mut audio_clip_2 = audio_clip(1, t(15.0), t(15.0));
    audio_clip_2.id = 21;
    audio_clip_2.audio_enabled = true;

    doc.project.sequence.tracks[1].clips = vec![audio_clip_1, audio_clip_2];

    // Caption track with two healthy cues
    // Cue 1: 3.0s duration, 26 chars => 8.7 CPS (< 25 CPS limit), 1 line (< 42 chars)
    let cue_1 = SubtitleItem {
        id: 100,
        start_ticks: t(1.0),
        end_ticks: t(4.0),
        text: "Welcome to Easy Edits Pro.".into(),
        speaker: None,
        style_id: None,
    };
    // Cue 2: 3.0s duration, 24 chars => 8.0 CPS, 1 line, no overlap
    let cue_2 = SubtitleItem {
        id: 101,
        start_ticks: t(5.0),
        end_ticks: t(8.0),
        text: "A powerful video editor.".into(),
        speaker: None,
        style_id: None,
    };
    doc.project.sequence.caption_tracks.push(CaptionTrack {
        id: 1,
        name: "English".into(),
        language: "en".into(),
        visible: true,
        cues: vec![cue_1, cue_2],
        styles: Vec::new(),
    });

    doc
}

#[test]
fn test_clean_fixture_passes_all_checks() {
    let doc = create_clean_document();

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        audio_metrics: Some(json!({
            "loudness": {
                "integrated_lufs": -14.1,
                "true_peak_dbtp": -1.5
            }
        })),
        video_metrics: Some(json!({
            "analysis": {
                "clipped_highlights_pct": 0.2,
                "crushed_shadows_pct": 0.4
            },
            "black_frames": [],
            "frozen_frames": []
        })),
        ..Default::default()
    };

    let report = run_verification(&doc, &params).expect("verification should run cleanly");

    assert_eq!(report.preset, "web-1080p");
    assert_eq!(report.summary.total_checks, 13);
    assert_eq!(report.summary.failed, 0, "Clean fixture must have 0 failed checks");
    assert_eq!(report.summary.warned, 0, "Clean fixture must have 0 warned checks");
    assert_eq!(report.summary.unavailable, 0, "Clean fixture must have 0 unavailable checks");
    assert_eq!(report.summary.passed, 13, "All 13 checks must pass on clean fixture");

    // Honesty check: never invent an aggregate quality score
    assert!(report.notes.aggregate_score.is_none(), "Aggregate score must be null (no invented scores)");
    assert_eq!(
        report.notes.message,
        "Measured evidence only; no invented aggregate quality score."
    );
}

#[test]
fn test_seeded_bad_captions_negative_controls() {
    let mut doc = create_clean_document();

    // 1. High reading speed: 100 chars in 1.0s = 100 CPS (> 25 CPS threshold)
    let fast_cue = SubtitleItem {
        id: 201,
        start_ticks: t(10.0),
        end_ticks: t(11.0),
        text: "This caption has way too many words to read in just one second and completely blows past the speed.".into(),
        speaker: None,
        style_id: None,
    };

    // 2. Timecode overlap: Cue 301 starts at 15.0s, Cue 302 starts at 14.0s (overlapping by 1s)
    let overlap_prev = SubtitleItem {
        id: 301,
        start_ticks: t(12.0),
        end_ticks: t(15.0),
        text: "First cue ending at 15s".into(),
        speaker: None,
        style_id: None,
    };
    let overlap_curr = SubtitleItem {
        id: 302,
        start_ticks: t(14.0),
        end_ticks: t(17.0),
        text: "Second cue starting at 14s (overlaps)".into(),
        speaker: None,
        style_id: None,
    };

    // 3. Safe area line limit violation: 3 lines (> 2 allowed)
    let safe_area_cue = SubtitleItem {
        id: 401,
        start_ticks: t(20.0),
        end_ticks: t(24.0),
        text: "Line 1\nLine 2\nLine 3 overflows safe area".into(),
        speaker: None,
        style_id: None,
    };

    doc.project.sequence.caption_tracks[0].cues =
        vec![fast_cue, overlap_prev, overlap_curr, safe_area_cue];

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec![
            "captions.reading_speed".into(),
            "captions.timecode_overlap".into(),
            "captions.safe_area".into(),
        ],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).expect("verification should run");

    // Reading speed check failed
    let speed_check = report
        .checks
        .iter()
        .find(|c| c.check_id == "captions.reading_speed")
        .expect("reading speed check present");
    assert_eq!(speed_check.verdict, CheckVerdict::Fail);
    assert!(!speed_check.offending_ranges.is_empty());
    assert_eq!(speed_check.offending_ranges[0].clip_id, Some(201));

    // Overlap check failed
    let overlap_check = report
        .checks
        .iter()
        .find(|c| c.check_id == "captions.timecode_overlap")
        .expect("overlap check present");
    assert_eq!(overlap_check.verdict, CheckVerdict::Fail);
    assert!(!overlap_check.offending_ranges.is_empty());
    assert_eq!(overlap_check.offending_ranges[0].clip_id, Some(302));

    // Safe area check warned (both cue 201 for exceeding 42 chars and cue 401 for 3 lines)
    let safe_area_check = report
        .checks
        .iter()
        .find(|c| c.check_id == "captions.safe_area")
        .expect("safe area check present");
    assert_eq!(safe_area_check.verdict, CheckVerdict::Warn);
    assert!(safe_area_check.offending_ranges.iter().any(|r| r.clip_id == Some(201)));
    assert!(safe_area_check.offending_ranges.iter().any(|r| r.clip_id == Some(401)));
}

#[test]
fn test_seeded_bad_brief_duration_negative_control() {
    let mut doc = create_clean_document();
    // Brief target is 60s ± 1s, but sequence duration is 30s (deficit of 30s)
    doc.project.brief.target_duration_ticks = Some(t(60.0).0 as u64);
    doc.project.brief.duration_tolerance_ticks = Some(t(1.0).0 as u64);

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec!["project.target_duration".into()],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();
    let check = &report.checks[0];
    assert_eq!(check.verdict, CheckVerdict::Fail);
    assert!(!check.offending_ranges.is_empty());
    assert!(check.message.contains("shorter than brief target"));
}

#[test]
fn test_seeded_bad_resolution_aspect_negative_control() {
    let mut doc = create_clean_document();
    // Project is vertical 1080x1920 (9:16)
    doc.project.settings.width = 1080;
    doc.project.settings.height = 1920;

    // Evaluated against 16:9 web preset
    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec!["project.resolution_aspect".into()],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();
    let check = &report.checks[0];
    assert_eq!(check.verdict, CheckVerdict::Fail);
    assert!(check.message.contains("mismatches delivery preset"));
}

#[test]
fn test_seeded_timeline_silence_negative_control() {
    let mut doc = create_clean_document();
    // Break the audio clips so there is an 8-second dead air gap between 10s and 18s (preset limit is 2.0s)
    doc.project.sequence.tracks[1].clips[0].duration = t(10.0); // 0..10s
    doc.project.sequence.tracks[1].clips[1].timeline_start = t(18.0); // 18..30s
    doc.project.sequence.tracks[1].clips[1].duration = t(12.0);

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec!["project.timeline_silence".into()],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();
    let check = &report.checks[0];
    assert_eq!(check.verdict, CheckVerdict::Warn);
    assert!(!check.offending_ranges.is_empty());
    assert_eq!(check.offending_ranges[0].start_tick, t(10.0).0 as u64);
    assert_eq!(check.offending_ranges[0].end_tick, t(18.0).0 as u64);
}

#[test]
fn test_seeded_unreviewed_low_confidence_negative_control() {
    let mut doc = create_clean_document();

    // Attach active low-confidence note to clip 20
    let note = ClipAgentNote::new(
        "Auto-trimmed dialogue pause without clear boundary",
        ConfidenceLevel::Low,
        ActorId::agent("test-agent"),
        "sess-1",
        1_000_000,
        42,
    )
    .unwrap();
    doc.project.sequence.tracks[1].clips[0].agent_note = Some(note);

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec!["provenance.unreviewed_low_confidence".into()],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();
    let check = &report.checks[0];
    assert_eq!(check.verdict, CheckVerdict::Warn);
    assert_eq!(check.offending_ranges.len(), 1);
    assert_eq!(check.offending_ranges[0].clip_id, Some(20));

    // Now mark as human corrected
    let mut corrected_doc = doc.clone();
    corrected_doc.project.sequence.tracks[1].clips[0]
        .agent_note
        .as_mut()
        .unwrap()
        .record_correction(ActorId::Human, 1_000_500, "Approved trim boundary");

    let report_after = run_verification(&corrected_doc, &params).unwrap();
    assert_eq!(report_after.checks[0].verdict, CheckVerdict::Pass);
    assert!(report_after.checks[0].offending_ranges.is_empty());
}

#[test]
fn test_seeded_offline_media_negative_control() {
    let mut doc = create_clean_document();

    // Make clip 10 point to a non-existent media id
    doc.project.sequence.tracks[0].clips[0].media_id = 9999;

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        checks: vec!["provenance.offline_media".into()],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();
    let check = &report.checks[0];
    assert_eq!(check.verdict, CheckVerdict::Fail);
    assert_eq!(check.offending_ranges.len(), 1);
    assert_eq!(check.offending_ranges[0].clip_id, Some(10));

    // Also test media item marked missing
    let mut missing_doc = create_clean_document();
    missing_doc.project.media[0].is_missing = true;

    let report_missing = run_verification(&missing_doc, &params).unwrap();
    let check_missing = &report_missing.checks[0];
    assert_eq!(check_missing.verdict, CheckVerdict::Fail);
    assert!(!check_missing.offending_ranges.is_empty());
}

#[test]
fn test_seeded_audio_video_telemetry_negative_controls() {
    let doc = create_clean_document();

    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        audio_metrics: Some(json!({
            "loudness": {
                // Target is -14.0 LUFS ± 1.0; -20.0 LUFS is 6.0 LU below target
                "integrated_lufs": -20.0,
                // Peak limit is -1.0 dBTP; +1.2 dBTP is in severe clipping
                "true_peak_dbtp": 1.2
            }
        })),
        video_metrics: Some(json!({
            "analysis": {
                // Clipping > 5.0% fails
                "clipped_highlights_pct": 7.5,
                // Crushed shadows > 10.0% fails
                "crushed_shadows_pct": 14.2
            },
            "black_frames": [
                { "start_tick": 1000, "end_tick": 2000, "detail": "Black frame in act 1" }
            ],
            "frozen_frames": [
                { "start_tick": 5000, "end_tick": 7000, "detail": "Frozen frame glitch" }
            ]
        })),
        checks: vec![
            "audio.loudness".into(),
            "audio.true_peak".into(),
            "video.clipped_highlights".into(),
            "video.crushed_blacks".into(),
            "video.black_and_frozen_frames".into(),
        ],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();

    let loudness_check = report.checks.iter().find(|c| c.check_id == "audio.loudness").unwrap();
    assert_eq!(loudness_check.verdict, CheckVerdict::Fail);

    let peak_check = report.checks.iter().find(|c| c.check_id == "audio.true_peak").unwrap();
    assert_eq!(peak_check.verdict, CheckVerdict::Fail);

    let clip_check = report.checks.iter().find(|c| c.check_id == "video.clipped_highlights").unwrap();
    assert_eq!(clip_check.verdict, CheckVerdict::Fail);

    let crush_check = report.checks.iter().find(|c| c.check_id == "video.crushed_blacks").unwrap();
    assert_eq!(crush_check.verdict, CheckVerdict::Fail);

    let anomaly_check = report.checks.iter().find(|c| c.check_id == "video.black_and_frozen_frames").unwrap();
    assert_eq!(anomaly_check.verdict, CheckVerdict::Warn);
    assert_eq!(anomaly_check.offending_ranges.len(), 2);
}

#[test]
fn test_unavailable_metrics_honesty_check() {
    let doc = create_clean_document();

    // No audio or video metrics provided
    let params = VerifyParams {
        preset: Some("web-1080p".into()),
        audio_metrics: None,
        video_metrics: None,
        checks: vec![
            "audio.loudness".into(),
            "audio.true_peak".into(),
            "video.clipped_highlights".into(),
            "video.crushed_blacks".into(),
            "video.black_and_frozen_frames".into(),
        ],
        ..Default::default()
    };

    let report = run_verification(&doc, &params).unwrap();

    for check in &report.checks {
        assert_eq!(
            check.status,
            CheckStatus::Unavailable,
            "Check {} must be marked Unavailable when metrics are absent",
            check.check_id
        );
        assert_ne!(
            check.verdict,
            CheckVerdict::Pass,
            "Check {} must NOT silently pass when metrics are absent",
            check.check_id
        );
    }

    assert_eq!(report.summary.unavailable, 5);
    assert_eq!(report.summary.passed, 0);
}

#[test]
fn test_delivery_presets_configuration() {
    let web = VerifyPreset::web_1080p();
    assert_eq!(web.target_loudness_lufs, -14.0);
    assert_eq!(web.max_true_peak_dbtp, -1.0);
    assert_eq!(web.max_caption_cps, 25.0);
    assert_eq!(web.aspect_ratio, "16:9");

    let vertical = VerifyPreset::vertical_9x16();
    assert_eq!(vertical.target_loudness_lufs, -14.0);
    assert_eq!(vertical.aspect_ratio, "9:16");
    assert_eq!(vertical.max_caption_cps, 22.0);

    let broadcast = VerifyPreset::broadcast_r128();
    assert_eq!(broadcast.target_loudness_lufs, -23.0);
    assert_eq!(broadcast.loudness_tolerance_lu, 0.5);
    assert_eq!(broadcast.max_caption_cps, 20.0);
}

#[test]
fn test_engine_command_execution() {
    let doc = create_clean_document();
    let mut engine = EditorEngine::default();
    engine.restore_recovery(doc, None).unwrap();

    let cmd = CommandRequest::new(
        "verify.run",
        json!({
            "preset": "web-1080p",
            "audio_metrics": {
                "loudness": {
                    "integrated_lufs": -14.0,
                    "true_peak_dbtp": -1.2
                }
            },
            "video_metrics": {
                "analysis": {
                    "clipped_highlights_pct": 0.1,
                    "crushed_shadows_pct": 0.2
                }
            }
        }),
    );

    let output = engine.execute(cmd).expect("verify.run command must succeed");
    let report: VerifyReport = serde_json::from_value(output).expect("valid VerifyReport");
    assert_eq!(report.preset, "web-1080p");
    assert_eq!(report.summary.failed, 0);
    assert!(report.notes.aggregate_score.is_none());
}
