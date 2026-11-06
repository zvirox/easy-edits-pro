// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Tests for `motion_track.rs` (loaded via `#[path]` from the module).

use super::*;

fn frame(width: u32, height: u32, tick: i64, shade: impl Fn(u32, u32) -> u8) -> SampledFrame {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            let v = shade(x, y);
            rgb.extend_from_slice(&[v, v, v]);
        }
    }
    SampledFrame {
        tick: Tick(tick),
        width,
        height,
        rgb,
    }
}

#[test]
fn translation_tracks_exactly_with_high_confidence() {
    // Textured pattern with a sharp SSD minimum at the true shift.
    let shade = |x: u32, y: u32| {
        let x = x as f64 - 60.0;
        let y = y as f64 - 60.0;
        (128.0 + 50.0 * (x / 9.0).sin() + 30.0 * (y / 7.0).cos()) as u8
    };
    // Second frame: same pattern shifted +7 x / −3 y — content at B(x, y)
    // equals A(x−7, y+3), so the patch that sat at (60,60) in A now sits
    // at (67,57) in B.
    let first = frame(120, 120, 0, shade);
    let shifted = frame(120, 120, 333, |x, y| shade(x.saturating_sub(7), y + 3));
    let tracks = track_patch_displacement(&[first, shifted], (60.0, 60.0), 16, 12).unwrap();
    assert_eq!(tracks.len(), 2);
    let measured = tracks[1];
    assert!((measured.x - 67.0).abs() < 0.2, "x: {}", measured.x);
    assert!((measured.y - 57.0).abs() < 0.2, "y: {}", measured.y);
    assert!(
        measured.confidence > 0.95,
        "confidence: {}",
        measured.confidence
    );
}

#[test]
fn subpixel_shift_is_measured_within_a_quarter_pixel() {
    // Continuous sinusoid sampled at x + 0.5: the patch that sat at 48 in
    // the reference sits at 47.5 in the target; parabolic refinement on the
    // SSD surface must recover it within ±0.25 px.
    let pattern = |x: f64, _y: f64| 128.0 + 60.0 * (2.0 * std::f64::consts::PI * x / 24.0).sin();
    let width = 96_u32;
    let first = frame(width, width, 0, |x, y| pattern(x as f64, y as f64) as u8);
    let half_shift = frame(width, width, 40, |x, y| {
        pattern(x as f64 + 0.5, y as f64) as u8
    });
    let tracks = track_patch_displacement(&[first, half_shift], (48.0, 48.0), 16, 8).unwrap();
    let measured = tracks[1];
    assert!(
        (measured.x - 47.5).abs() <= 0.25,
        "sub-pixel x: {}",
        measured.x
    );
}

#[test]
fn flat_texture_reports_low_confidence_instead_of_a_fake_score() {
    let first = frame(64, 64, 0, |_, _| 100);
    let second = frame(64, 64, 33, |_, _| 100);
    let tracks = track_patch_displacement(&[first, second], (32.0, 32.0), 16, 6).unwrap();
    // Zero-contrast patch: the ZNCC denominator collapses, so the reported
    // confidence is honestly ~0 even though pixels still "match".
    assert!(
        tracks[1].confidence < 0.5,
        "confidence: {}",
        tracks[1].confidence
    );
}

#[test]
fn off_frame_start_is_a_typed_error() {
    let first = frame(32, 32, 0, |_, _| 10);
    let error = track_patch_displacement(&[first], (3.0, 3.0), 16, 4).unwrap_err();
    assert!(error.contains("outside"), "unexpected: {error}");
    let empty: Vec<SampledFrame> = Vec::new();
    assert!(track_patch_displacement(&empty, (5.0, 5.0), 16, 4).is_err());
}

#[test]
fn homography_recovers_a_known_projective_map() {
    let h_true = [1.1_f64, 0.05, 12.0, -0.03, 0.9, -8.0, 0.0001, 0.0002, 1.0];
    let points: Vec<[f64; 2]> = vec![
        [40.0, 17.0],
        [63.0, 141.0],
        [86.0, 48.0],
        [109.0, 172.0],
        [132.0, 79.0],
        [155.0, 25.0],
    ];
    let pairs: Vec<_> = points
        .iter()
        .map(|&p| {
            let q = apply_homography(&h_true, p);
            (p, q)
        })
        .collect();
    let recovered = homography_from_point_pairs(&pairs).expect("recovered homography");
    for p in &points {
        let want = apply_homography(&h_true, *p);
        let got = apply_homography(&recovered, *p);
        assert!(
            (want[0] - got[0]).abs() < 1e-5 && (want[1] - got[1]).abs() < 1e-5,
            "point {p:?}: want {want:?}, got {got:?}"
        );
    }
    // Degenerate: exactly collinear points must fail honestly.
    let collinear: Vec<([f64; 2], [f64; 2])> = (0..6)
        .map(|i| {
            let p = [i as f64 * 10.0, i as f64 * 20.0];
            (p, [p[0] + 1.0, p[1] + 1.0])
        })
        .collect();
    assert!(
        homography_from_point_pairs(&collinear).is_none(),
        "collinear correspondences must not yield a homography"
    );
}

#[test]
fn fewer_than_four_pairs_fail_honestly() {
    let pairs = [([0.0, 0.0], [1.0, 1.0]); 3];
    assert!(homography_from_point_pairs(&pairs).is_none());
    assert!(ransac_homography(&pairs, 10, 1.0, 7).is_none());
}

#[test]
fn ransac_rejects_outlier_correspondences() {
    // 8 true correspondences under a translation, in general position
    // (two staggered rows — no 4-point subset is collinear), + 2 outliers.
    let mut pairs: Vec<([f64; 2], [f64; 2])> = (0..8)
        .map(|i| {
            let p = [
                20.0 + (i % 4) as f64 * 30.0 + (i / 4) as f64 * 7.0,
                22.0 + (i / 4) as f64 * 35.0,
            ];
            ([p[0] + 9.0, p[1] - 4.0], p) // (src, dst): ref → frame
        })
        .collect();
    pairs.push(([150.0, 60.0], [3.0, 200.0])); // outlier
    pairs.push(([90.0, 190.0], [400.0, 5.0])); // outlier
    // Pairs are (src, dst) = (translated, original), so the fitted map
    // src → dst is the INVERSE translation: −9 x / +4 y.
    let outcome = ransac_homography(&pairs, 200, 2.0, 42).expect("consensus exists");
    assert!(
        outcome.inliers.len() >= 8,
        "inliers: {}",
        outcome.inliers.len()
    );
    assert!((outcome.homography[2] + 9.0).abs() < 1e-6);
    assert!((outcome.homography[5] - 4.0).abs() < 1e-6);
    for outlier in &[8, 9] {
        assert!(
            !outcome.inliers.contains(outlier),
            "outlier {outlier} must not be an inlier"
        );
    }
}

#[test]
fn region_tracking_recovers_a_translation_homography() {
    let shade = |x: u32, y: u32| {
        (140.0
            + 45.0 * (x as f64 / 11.0).sin()
            + 35.0 * (y as f64 / 9.0).cos()
            + 20.0 * ((x + y) as f64 / 17.0).sin()) as u8
    };
    let first = frame(200, 160, 0, shade);
    // Second frame: content at B(x, y) equals A(x−5, y+2) → the region is
    // displaced (+5, −2) in B.
    let second = frame(200, 160, 666, |x, y| shade(x.saturating_sub(5), y + 2));
    let tracked = track_region_frames(&[first, second], (40.0, 30.0, 72, 54), 12).unwrap();
    assert_eq!(tracked.len(), 2);
    // Reference frame reports identity with full confidence.
    let reference = tracked[0].as_ref().expect("reference is always tracked");
    assert!(reference.confidence == 1.0 && reference.inlier_ratio == 1.0);
    // Second frame: RANSAC consensus over the 9 grid patches must recover
    // the translation to well under a pixel.
    let moved = tracked[1]
        .as_ref()
        .expect("textured translation tracks reliably");
    assert!((moved.homography[0] - 1.0).abs() < 1e-3);
    assert!((moved.homography[4] - 1.0).abs() < 1e-3);
    assert!(
        (moved.homography[2] - 5.0).abs() < 0.5,
        "tx: {}",
        moved.homography[2]
    );
    assert!(
        (moved.homography[5] + 2.0).abs() < 0.5,
        "ty: {}",
        moved.homography[5]
    );
    assert!(moved.confidence > 0.8, "confidence: {}", moved.confidence);
    assert!(moved.reprojection_error_px < 1.0);
}
