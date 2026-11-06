// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! End-to-end MODNet matting + Depth-Anything-V2-Small depth check
//! (Packet 2.3).
//!
//! Gated like `siglip_e2e.rs`: the pinned ONNX exports (~53 MB total,
//! SHA-256-verified) download on first run, so constrained CI skips this
//! unless `SATURN_PERCEPTION_E2E=1` or the files are already cached in
//! `SATURN_MODELS_DIR`.
//!
//! What is verified against real inference (honest assertions only — these
//! are synthetic frames, so portrait/depth *content* claims are limited to
//! measured model behavior):
//! 1. Matting: deterministic alpha in 0..=1 at exactly frame resolution,
//!    and MODNet's documented portrait-only bias — a person-less synthetic
//!    scene reports a low mean alpha (the model honestly saying "no human
//!    foreground here"). Feathering and smoothing compose on top.
//! 2. Depth: deterministic finite grids at the pinned 518×518 size with
//!    measurable spread (a flat grid would contradict the textured input),
//!    and the relative normalization helper maps into 0..=1.

use saturn_core::Tick;
use saturn_perception::shot_detect::SampledFrame;
use saturn_perception::{DepthEstimator, MatteSmoother, ModNetMatting};

fn e2e_enabled() -> bool {
    std::env::var("SATURN_PERCEPTION_E2E").is_ok_and(|v| v != "0")
        || std::env::var("SATURN_MODELS_DIR").is_ok()
}

/// A deterministic "studio" frame: mid-gray backdrop with a brighter soft
/// block (no human subject — exactly the honest case MODNet also reports).
fn studio_frame(tick_millis: i64) -> SampledFrame {
    let (w, h) = (640_u32, 360_u32);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let in_block = (w / 4..3 * w / 4).contains(&x) && (h / 4..3 * h / 4).contains(&y);
            let ripple = ((x as f32 * 0.13 + y as f32 * 0.07).sin() * 9.0) as i32;
            if in_block {
                rgb.push((198 + ripple).clamp(0, 255) as u8);
                rgb.push((172 + ripple).clamp(0, 255) as u8);
                rgb.push((150 + ripple).clamp(0, 255) as u8);
            } else {
                rgb.push((118 + ripple).clamp(0, 255) as u8);
                rgb.push((118 + ripple).clamp(0, 255) as u8);
                rgb.push((122 + ripple).clamp(0, 255) as u8);
            }
        }
    }
    SampledFrame {
        tick: Tick::from_millis(tick_millis),
        width: w,
        height: h,
        rgb,
    }
}

#[test]
fn modnet_real_inference_matte_properties() {
    if !e2e_enabled() {
        eprintln!("skipping: set SATURN_PERCEPTION_E2E=1 to run the MODNet end-to-end test");
        return;
    }

    let frame = studio_frame(0);
    let mut matting = ModNetMatting::load().expect("MODNet loads (downloaded on first run)");

    let first = matting.matte_frame(&frame).expect("frame mattes");
    let second = matting.matte_frame(&frame).expect("frame re-mattes");

    // 1. Deterministic, frame-sized, in-range.
    assert_eq!(first.len(), frame.width as usize * frame.height as usize);
    assert_eq!(first, second, "matting must be deterministic per frame");
    let min = first.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = first.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!((0.0..=1.0).contains(&min) && (0.0..=1.0).contains(&max));

    // 2. Portrait-only honesty: a human-less synthetic scene must NOT be
    // reported as mostly foreground. (Measured behavior of the pinned
    // export on such frames; a person would raise this a lot.)
    let mean = first.iter().sum::<f32>() / first.len() as f32;
    assert!(
        mean < 0.5,
        "person-less frame reported mean alpha {mean} — expected low foreground coverage"
    );

    // 3. Compositing helpers compose: feather stays frame-sized and the
    // smoother's EMA lands between its inputs.
    let feathered = saturn_perception::feather_alpha(&first, frame.width, frame.height, 2);
    assert_eq!(feathered.len(), first.len());
    let fmin = feathered.iter().cloned().fold(f32::INFINITY, f32::min);
    let fmax = feathered.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(fmin >= 0.0 && fmax <= 1.0, "feather must stay in range");
    let mut smoother = MatteSmoother::default();
    let blended = smoother.smooth(first.clone(), 0.5);
    // First call passes through; a second call halves toward the previous.
    let blended_again = smoother.smooth(first.clone(), 0.5);
    for (a, b) in blended.iter().zip(&blended_again) {
        assert!((*a - *b).abs() < 1e-6, "EMA must be stable on identical input");
    }
}

#[test]
fn depth_real_inference_grid_properties() {
    if !e2e_enabled() {
        eprintln!("skipping: set SATURN_PERCEPTION_E2E=1 to run the depth end-to-end test");
        return;
    }

    let frame = studio_frame(0);
    let mut estimator = DepthEstimator::load().expect("depth model loads (downloaded on first run)");

    let grid = estimator.depth_frame(&frame).expect("frame gets depth");
    let again = estimator.depth_frame(&frame).expect("frame re-gets depth");

    // 1. Pinned output geometry and determinism.
    assert_eq!(
        (grid.width, grid.height),
        (
            saturn_perception::depth_ort::DEPTH_INPUT_SIZE,
            saturn_perception::depth_ort::DEPTH_INPUT_SIZE
        )
    );
    assert_eq!(grid, again, "depth must be deterministic per frame");

    // 2. Finite, positive, and with measurable spread (the export is
    // int8-quantized relative depth; a textured scene must not collapse to
    // a constant).
    assert!(grid.values.iter().all(|v| v.is_finite() && *v > 0.0));
    let min = grid.values.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = grid.values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max - min > 0.01,
        "depth spread {min}..{max} is flat on a textured frame"
    );

    // 3. Relative normalization maps into 0..=1 and the upsample covers the
    // requested frame exactly.
    let (normalized, flat) = grid.normalized_relative();
    assert!(!flat, "textured frame must not be flagged flat");
    assert!(normalized.iter().all(|v| (0.0..=1.0).contains(v)));
    let upsampled = grid.upsampled(frame.width, frame.height);
    assert_eq!(upsampled.len(), frame.width as usize * frame.height as usize);
}
