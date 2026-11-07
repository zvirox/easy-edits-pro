// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Unit tests for the Packet 3.3 shot matcher (private fit internals;
//! the public-path verification gate lives in `tests/shot_match.rs`).

use super::measurement::CDF_BINS;
use super::*;

/// Signed hue difference wrapped to −180..180 degrees (hue-gap assertions).
fn signed_hue_deg(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

/// Encodes an ACEScg-linear colour to an 8-bit Rec.709 signal pixel
/// (the inverse of the measurement decode path).
fn encode_pixel(acescg: [f64; 3]) -> [u8; 3] {
    let ap0 = WorkingSpace::AcesCg.to_ap0(acescg);
    let xyz = mat3_apply(&M_AP0_TO_XYZ_D65, ap0);
    let linear = mat3_apply(&rec709_from_xyz_d65(), xyz);
    linear.map(|v| (rec709_oetf_encode(v.clamp(0.0, 1.0)) * 255.0).round() as u8)
}

/// Renders a synthetic frame from a scene generator: a spread of greys
/// plus a skin-band patch, so both the CDF and the skin lock have signal.
struct Scene {
    /// ACEScg-linear greys (uniformly spread in ACEScct CV).
    greys: Vec<[f64; 3]>,
    /// Skin-band patch colour (ACEScg-linear).
    skin: [f64; 3],
    /// Ratio of skin pixels to total.
    skin_share: f64,
}

impl Scene {
    fn render(&self, width: u32, height: u32) -> Vec<u8> {
        let pixels = (width as usize) * (height as usize);
        let skin_count = (pixels as f64 * self.skin_share) as usize;
        let mut buf = Vec::with_capacity(pixels * 3);
        for p in 0..pixels {
            // Striped layout keeps skin patches spread over the frame so
            // the stride sampling picks them up.
            let scene = if p % 8 == 0 && p / 8 < skin_count {
                self.skin
            } else {
                self.greys[p % self.greys.len()]
            };
            buf.extend_from_slice(&encode_pixel(scene));
        }
        buf
    }
}

fn grey_scene(n: usize) -> Vec<[f64; 3]> {
    use saturn_core::color::transforms::acescct_decode;
    // ACEScct CVs 0.15..0.52 decode to linear ~0.017..0.56 — inside the
    // 8-bit Rec.709 signal range (linear 1.0 = CV 0.5548) so the test
    // scene never clips and the CDF stays informative end to end.
    (0..n)
        .map(|i| {
            let cv = 0.15 + (i as f64 + 0.5) * (0.52 - 0.15) / n as f64;
            let v = acescct_decode(cv).clamp(0.001, 10.0);
            [v, v, v]
        })
        .collect()
}

fn measurement_of(scene: &Scene) -> FrameMeasurement {
    let rgb = scene.render(64, 64);
    measurement(64, 64, &rgb).expect("measurement")
}

#[test]
fn measurement_shape_and_normalisation() {
    let scene = Scene {
        greys: grey_scene(24),
        skin: [0.28, 0.15, 0.08],
        skin_share: 0.1,
    };
    let m = measurement_of(&scene);
    assert_eq!(m.luma_cdf.len(), CDF_BINS);
    assert_eq!(m.sampled_pixels, (64 * 64) as usize);
    for (i, row) in m.luma_cdf.iter().enumerate() {
        assert!((0.0..=1.0).contains(row), "luma_cdf[{i}] out of range");
        if i > 0 {
            assert!(*row >= m.luma_cdf[i - 1], "luma cdf not ascending");
        }
    }
    // Stride 1 (test-sized frame) samples every pixel exactly once and
    // sorts: the quantile source must already be ascending.
    for (ch, v) in m.channel_samples.iter().enumerate() {
        assert_eq!(v.len(), m.sampled_pixels, "channel {ch} sample count");
        for w in v.windows(2) {
            assert!(w[0] <= w[1], "channel {ch} samples not sorted");
        }
    }
    let skin = m.skin.as_ref().expect("synthetic skin patch must detect");
    assert!(skin.pixels >= SKIN_LOCK_MIN_PIXELS);
}

#[test]
fn fit_recovers_known_parametric_correction() {
    // True correction: +0.5 stops, warm gain, slight gamma lift. The
    // reference frame is the target through this exact model, so the
    // fit should invert it back to ~neutral.
    let true_stops = 0.5f64;
    let true_gain = [1.1f64, 1.0, 0.9];
    let true_gamma = -0.2f64; // wheels gamma parameter

    let base = Scene {
        greys: grey_scene(32),
        skin: [0.28, 0.15, 0.08],
        skin_share: 0.1,
    };
    // Apply the forward model to the base frame to build the reference.
    let forward = |c: [f64; 3]| {
        let k = (-true_gamma).exp2();
        let e2 = true_stops.exp2();
        [
            (c[0] * e2 * true_gain[0]).powf(k),
            (c[1] * e2 * true_gain[1]).powf(k),
            (c[2] * e2 * true_gain[2]).powf(k),
        ]
    };
    let reference = Scene {
        greys: base.greys.iter().map(|c| forward(*c)).collect(),
        skin: forward(base.skin),
        skin_share: 0.1,
    };

    let target_m = measurement_of(&base);
    let reference_m = measurement_of(&reference);
    let fit = fit_match(&target_m, &reference_m, false).expect("fit");

    assert!(
        (fit.exposure_stops - true_stops).abs() < 0.15,
        "{:?}",
        fit.exposure_stops
    );
    for ch in 0..3 {
        assert!(
            (fit.gain[ch] - true_gain[ch]).abs() < 0.08,
            "gain[{ch}] {} vs {}",
            fit.gain[ch],
            true_gain[ch]
        );
    }
    assert!((fit.gamma - true_gamma).abs() < 0.15, "{:?}", fit.gamma);
    // A pure multiplicative truth needs no additive term.
    for ch in 0..3 {
        assert!(fit.offset[ch].abs() < 0.02, "{:?}", fit.offset);
    }
    assert!(fit.skin.is_none());
}

#[test]
fn identical_frames_fit_neutral() {
    let scene = Scene {
        greys: grey_scene(32),
        skin: [0.28, 0.15, 0.08],
        skin_share: 0.1,
    };
    let m = measurement_of(&scene);
    let fit = fit_match(&m, &m, false).expect("fit");
    assert!(fit.exposure_stops.abs() < 0.01, "{:?}", fit);
    for ch in 0..3 {
        assert!((fit.gain[ch] - 1.0).abs() < 0.01);
        assert!(fit.offset[ch].abs() < 0.01);
    }
    assert!(fit.gamma.abs() < 0.05);
}

#[test]
fn skin_lock_pulls_hue_to_reference_without_one() {
    // Target and reference have different exposure AND different skin
    // hue; the lock must report both detected and land the graded hue
    // near the reference hue while the CDF part handles exposure.
    let base = Scene {
        greys: grey_scene(32),
        skin: [0.28, 0.15, 0.08],
        skin_share: 0.12,
    };
    // Reference: greener skin and darker overall.
    let reference = Scene {
        greys: base
            .greys
            .iter()
            .map(|c| [c[0] * 0.7, c[1] * 0.7, c[2] * 0.7])
            .collect(),
        skin: [0.22, 0.16, 0.075],
        skin_share: 0.12,
    };
    let target_m = measurement_of(&base);
    let reference_m = measurement_of(&reference);
    let fit = fit_match(&target_m, &reference_m, true).expect("fit");
    let skin = fit.skin.as_ref().expect("lock requested");
    assert!(skin.applied, "synthetic hue gap must move the lock");
    let src = skin.source_hue_deg.expect("target skin detected");
    let dst = skin.reference_hue_deg.expect("reference skin detected");
    let graded = skin.graded_hue_deg.expect("lock applied");
    let before = signed_hue_deg(src, dst).abs();
    let after = signed_hue_deg(graded, dst).abs();
    assert!(
        after < before,
        "lock must reduce the hue gap: {before} -> {after}"
    );
    // Exposure part still handled by the CDF fit (reference is darker).
    assert!(fit.exposure_stops < -0.2, "{:?}", fit.exposure_stops);
}

#[test]
fn lock_without_skin_reports_not_detected() {
    let greys_only = Scene {
        greys: grey_scene(32),
        skin: [0.0, 0.0, 0.0], // black patch: outside the skin band
        skin_share: 0.5,
    };
    let m = measurement_of(&greys_only);
    assert!(m.skin.is_none(), "pure grey frame must not detect skin");
    let fit = fit_match(&m, &m, true).expect("fit");
    let skin = fit.skin.as_ref().expect("lock requested");
    assert!(!skin.applied);
    assert!(skin.source_hue_deg.is_none());
    assert!(skin.reference_hue_deg.is_none());
}

#[test]
fn wheels_payload_round_trips_through_validation() {
    let scene = Scene {
        greys: grey_scene(32),
        skin: [0.28, 0.15, 0.08],
        skin_share: 0.1,
    };
    let m = measurement_of(&scene);
    let fit = fit_match(&m, &m, true).expect("fit");
    let payload = fit.wheels_payload();
    let json = serde_json::to_value(&payload).expect("serialise");
    // The engine's schema validator must accept the fitted payload (its
    // ranges are the hard limits this module types against).
    let validated = saturn_core::project::validate_payload(
        saturn_core::project::ColorNodeKind::PrimaryWheels,
        &json,
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
    .expect("wheels payload must validate");
    let _ = validated;
}

#[test]
fn frame_size_mismatch_is_typed() {
    let err = measurement(4, 4, &[0u8; 10]).unwrap_err();
    assert_eq!(
        err,
        ShotMatchError::FrameSize {
            width: 4,
            height: 4,
            got: 10
        }
    );
    assert!(err.to_string().contains("frame buffer mismatch"));
}
