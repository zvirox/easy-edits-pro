// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration scaffold for the Packet 3.1 colour pipeline (engine-facing
//! surface, not unit internals): every shipped IDT decodes an 18% grey card
//! display-neutral through the full `IDT -> working -> ODT` chain.
//!
//! Packet 3.2 extends this file with exposure/OOZ coverage once the
//! tone-building blocks land (noted in
//! `docs/AI_NATIVE_PLAN_CHECKLIST.md` Phase 3). Serde-name stability of the
//! selections is pinned in `saturn_core::engine::tests::ocio` (single source).

use saturn_core::color::transforms::{
    apple_log_encode, bmd_gen5_encode, log3g10_encode, logc4_encode, rec709_oetf_encode,
    slog3_encode,
};
use saturn_core::color::{CameraIdt, DisplayOdt, WorkingSpace};
use saturn_render::ocio_pipeline::{transform_reference, OcioConfig};

const ALL_IDTS: &[CameraIdt] = CameraIdt::ALL;

/// 18% scene-linear grey encoded through each camera's own transfer — the
/// same construction the core curve tests use, so no anchor digits are
/// duplicated here.
fn camera_grey_cv(idt: CameraIdt) -> [f64; 3] {
    let cv: f64 = match idt {
        CameraIdt::Rec709Video => rec709_oetf_encode(0.18),
        CameraIdt::ArriLogC4 => logc4_encode(0.18),
        CameraIdt::RedLog3G10 => log3g10_encode(0.18),
        CameraIdt::SonySLog3 => slog3_encode(0.18),
        CameraIdt::BlackmagicGen5 => bmd_gen5_encode(0.18),
        // Apple Log is defined on reflection; the IDT divides by 0.9 on
        // decode, mirroring the documented `E = R * 0.9` relationship.
        CameraIdt::AppleLog => apple_log_encode(0.18 / 0.9),
    };
    [cv, cv, cv]
}

#[test]
fn mid_grey_is_display_neutral_for_every_selection_combination() {
    for idt in ALL_IDTS {
        for working in [WorkingSpace::AcesCg, WorkingSpace::AcesCct] {
            for odt in [DisplayOdt::Rec709, DisplayOdt::DisplayP3, DisplayOdt::Rec2020Pq] {
                let config = OcioConfig {
                    idt: *idt,
                    working,
                    odt,
                };
                let out = transform_reference(camera_grey_cv(*idt), config);
                // Chroma of the neutral card must vanish; luminance rides the
                // display transfer (not asserted here — Packet 3.2 exposure).
                let chroma = (out[0] - out[1]).abs().max((out[1] - out[2]).abs());
                assert!(
                    chroma < 2e-4,
                    "{idt:?} {working:?} {odt:?}: grey card shows chroma {chroma} ({out:?})"
                );
            }
        }
    }
}

// --- Packet 3.2: GPU colour-node chain vs CPU reference ---------------------

use saturn_core::project::{
    ColorNode, ColorNodeKind, PowerWindow, WindowShape, WindowTransform,
};
use saturn_render::color_nodes_eval::tetra_lut_f64;
use saturn_render::color_nodes_gpu::{
    build_plan, ColorNodesError, EvalNode, FilmPlan, ResolvedWindow, WindowId, KIND_FILM_GRAIN,
    KIND_HDR_ZONES, KIND_HSL_QUALIFIER, KIND_LUT_3D, KIND_PRIMARY_WHEELS, KIND_RGB_CURVES,
    KIND_WARPER,
};
use saturn_render::color_nodes_shader::{reference_dispatch, ColorNodesPipeline};
use saturn_render::gpu_context::{GpuContext, GpuContextError};

/// Creates a GPU context, or reports plainly when the host has no adapter
/// (an explicit skip, not a fabricated pass).
fn gpu() -> Option<GpuContext> {
    match GpuContext::new() {
        Ok(ctx) => Some(ctx),
        Err(GpuContextError::NoVulkanAdapter(reason)) => {
            eprintln!("skipping GPU test: no Vulkan adapter ({reason})");
            None
        }
        Err(e) => panic!("GPU context creation failed unexpectedly: {e}"),
    }
}

/// Deterministic test frame with genuine HDR headroom (values above 1.0) and
/// chroma variation, 16x12.
const TW: u32 = 16;
const TH: u32 = 12;

fn test_pixels() -> Vec<f32> {
    let mut px = Vec::new();
    for y in 0..TH {
        for x in 0..TW {
            let u = x as f32 / (TW - 1) as f32;
            let v = y as f32 / (TH - 1) as f32;
            px.extend_from_slice(&[
                0.02 + u * 0.9 + (x % 4 == 0) as u8 as f32 * 4.0,
                0.01 + v * 0.8,
                0.03 + (1.0 - u) * 0.7,
                1.0,
            ]);
        }
    }
    px
}

fn window(shape: WindowId, inverted: bool) -> ResolvedWindow {
    ResolvedWindow {
        shape,
        center: [0.5, 0.5],
        half_size: [0.3, 0.25],
        softness: 0.1,
        inverted,
        opacity: 1.0,
        pivot: [0.5, 0.5],
        scale: 1.2,
        rotation_deg: 15.0,
        polygon: vec![[0.2, 0.2], [0.8, 0.25], [0.75, 0.8], [0.25, 0.7]],
    }
}

/// Asserts GPU output matches the f64 CPU reference within `tol`.
fn assert_gpu_matches_cpu(
    nodes: &[EvalNode],
    film: &FilmPlan,
    frame: u32,
    tol: f32,
    label: &str,
) {
    let Some(ctx) = gpu() else { return };
    let pipeline = ColorNodesPipeline::new(&ctx).expect("chain pipeline");
    let pixels = test_pixels();
    let got = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, nodes, film, frame, None)
        .expect("GPU dispatch");
    let want = reference_dispatch(TW, TH, &pixels, nodes, film, frame);
    for i in 0..(TW * TH) as usize {
        for ch in 0..4 {
            let g = got[i * 4 + ch];
            let w = want[i * 4 + ch];
            assert!(
                (g - w).abs() <= tol,
                "{label}: px {i} ch {ch} gpu {g} vs cpu {w} (diff {})",
                (g - w).abs()
            );
        }
    }
}

#[test]
fn gpu_matches_cpu_for_primary_wheels() {
    let nodes = [EvalNode {
        kind: KIND_PRIMARY_WHEELS,
        enabled: true,
        data: vec![
            1.5,            // exposure stops
            0.3,            // gamma
            0.2, -0.1, 0.1, 0.05, // gain
            0.01, 0.0, -0.01, 0.0, // offset
            0.0, -0.05, 0.0, 0.2, // lift
        ],
        windows: vec![],
    }];
    let film = FilmPlan::default();
    assert_gpu_matches_cpu(&nodes, &film, 7, 5e-4, "wheels");
}

#[test]
fn gpu_matches_cpu_for_hdr_zones() {
    let mut data = Vec::new();
    for (center, width) in [(-7.0, 2.0), (-5.0, 2.0), (-3.0, 1.5), (-1.5, 1.0), (-0.5, 0.75), (0.0, 0.5)] {
        data.extend_from_slice(&[center as f32, width as f32]);
        // deliberately strong band offsets to exercise the bands
        data.extend_from_slice(&[0.1, -0.05, 0.0, 0.02]);
    }
    let nodes = [EvalNode { kind: KIND_HDR_ZONES, enabled: true, data, windows: vec![] }];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 3, 5e-4, "zones");
}

#[test]
fn gpu_matches_cpu_for_rgb_curves() {
    let mut data = Vec::new();
    // master: soft S-curve
    let master: [[f64; 2]; 4] = [[0.0, 0.0], [0.33, 0.28], [0.66, 0.72], [1.0, 1.0]];
    let red: [[f64; 2]; 3] = [[0.0, 0.05], [0.5, 0.5], [1.0, 0.95]];
    let green: [[f64; 2]; 2] = [[0.0, 0.0], [1.0, 1.0]];
    let blue: [[f64; 2]; 4] = [[0.0, 0.1], [0.4, 0.3], [0.8, 0.9], [1.0, 1.0]];
    let curves: [&[[f64; 2]]; 4] = [&master, &red, &green, &blue];
    for curve in curves {
        data.push(curve.len() as f32);
        for p in curve {
            data.extend_from_slice(&[p[0] as f32, p[1] as f32]);
        }
        for _ in curve.len()..8 {
            data.extend_from_slice(&[0.0, 0.0]);
        }
    }
    let nodes = [EvalNode { kind: KIND_RGB_CURVES, enabled: true, data, windows: vec![] }];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 1, 5e-3, "curves");
}

#[test]
fn gpu_matches_cpu_for_lut_3d() {
    // A non-neutral size-4 LUT: sharpened diagonal + chroma twist.
    let mut data = vec![4.0f32];
    for b in 0..4u32 {
        for g in 0..4u32 {
            for r in 0..4u32 {
                let fr = r as f32 / 3.0;
                let fg = g as f32 / 3.0;
                let fb = b as f32 / 3.0;
                data.extend_from_slice(&[fr * 1.1, (fg * 0.9 + 0.05).min(1.0), fb]);
            }
        }
    }
    let nodes = [EvalNode { kind: KIND_LUT_3D, enabled: true, data, windows: vec![] }];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 0, 2e-3, "lut");
}

#[test]
fn gpu_matches_cpu_for_hsl_qualifier() {
    let nodes = [EvalNode {
        kind: KIND_HSL_QUALIFIER,
        enabled: true,
        data: vec![
            1.0,  // enabled
            0.58, // hue center (cyan-green)
            0.3, 0.1, // width, softness
            0.5, 0.6, 0.1, // sat center/width/softness
            0.5, 0.8, 0.1, // luma center/width/softness
            0.05, 1.8, 0.4, // hue shift, sat boost, exposure shift
            0.0, // invert
        ],
        windows: vec![],
    }];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 2, 5e-3, "hsl");
}

#[test]
fn gpu_matches_cpu_for_warper() {
    let nodes = [EvalNode {
        kind: KIND_WARPER,
        enabled: true,
        data: vec![
            2.0,
            0.55, 0.5, -0.06, 0.2, // cyan landmark: desaturate + hue drift
            0.08, 0.3, 0.04, -0.1, // red landmark
        ],
        windows: vec![],
    }];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 4, 5e-3, "warper");
}

#[test]
fn gpu_matches_cpu_with_windows_and_weave_and_grain() {
    // One wheel node gated by an ellipse window, plus grain, on a weaved
    // frame — the full dispatch path against the CPU mirror.
    let nodes = [
        EvalNode {
            kind: KIND_PRIMARY_WHEELS,
            enabled: true,
            data: vec![
                0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.0,
            ],
            windows: vec![window(WindowId::Ellipse, false)],
        },
        EvalNode {
            kind: KIND_FILM_GRAIN,
            enabled: true,
            data: vec![0.8, 0.0, 0.0, 0.0],
            windows: vec![],
        },
    ];
    let film = FilmPlan { halation: 0.0, grain: 0.8, weave_mm: 0.5 };
    assert_gpu_matches_cpu(&nodes, &film, 9, 5e-3, "windows+weave+grain");
}

#[test]
fn gpu_matches_cpu_for_polygon_and_inverted_windows() {
    let nodes = [
        EvalNode {
            kind: KIND_PRIMARY_WHEELS,
            enabled: true,
            data: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.0, 0.05, 0.0, 0.0, 0.0, 0.0, 0.0],
            windows: vec![window(WindowId::Polygon, false)],
        },
        EvalNode {
            kind: KIND_PRIMARY_WHEELS,
            enabled: true,
            data: vec![0.0, 0.0, 0.2, 0.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            windows: vec![window(WindowId::Rect, true)],
        },
    ];
    assert_gpu_matches_cpu(&nodes, &FilmPlan::default(), 5, 5e-4, "polygon+inverted");
}

#[test]
fn rect_window_pixel_is_untouched_and_centre_pixel_is_fully_graded() {
    let Some(ctx) = gpu() else { return };
    let pipeline = ColorNodesPipeline::new(&ctx).expect("chain pipeline");
    let pixels = test_pixels();
    let nodes = [EvalNode {
        kind: KIND_PRIMARY_WHEELS,
        enabled: true,
        data: vec![2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        windows: vec![ResolvedWindow {
            shape: WindowId::Rect,
            center: [0.5, 0.5],
            half_size: [0.1, 0.1],
            softness: 0.0,
            inverted: false,
            opacity: 1.0,
            pivot: [0.5, 0.5],
            scale: 1.0,
            rotation_deg: 0.0,
            polygon: vec![],
        }],
    }];
    let film = FilmPlan::default();
    let got = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &nodes, &film, 0, None)
        .expect("dispatch");
    let idx = |x: u32, y: u32| ((y * TW + x) * 4) as usize;
    // Corner (outside the window): exactly the source.
    for ch in 0..3 {
        assert_eq!(got[idx(0, 0) + ch], pixels[idx(0, 0) + ch]);
    }
    // Centre (inside): exactly 4x exposure (2^2) in scene-linear.
    let centre = idx(TW / 2, TH / 2);
    let expected = pixels[centre] * 4.0;
    assert!(
        (got[centre] - expected).abs() < 1e-4,
        "centre {} vs expected {expected}",
        got[centre]
    );
}

fn identity_grid(size: usize) -> Vec<f64> {
    let mut grid = Vec::new();
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                grid.extend_from_slice(&[
                    r as f64 / (size - 1) as f64,
                    g as f64 / (size - 1) as f64,
                    b as f64 / (size - 1) as f64,
                ]);
            }
        }
    }
    grid
}

#[test]
fn tetra_lut_tolerance_and_exactness_at_lattice_points() {
    // Identity LUT: any interpolation of the identity grid is the identity.
    let size = 4usize;
    let grid = identity_grid(size);
    for r in [0.0f64, 0.13, 0.5, 0.87, 1.0] {
        for g in [0.0f64, 0.25, 0.75, 1.0] {
            for b in [0.0f64, 0.4, 1.0] {
                let out = tetra_lut_f64(size, &grid, [r, g, b]);
                for ch in 0..3 {
                    let expected = [r, g, b][ch];
                    assert!(
                        (out[ch] - expected).abs() < 1e-9,
                        "identity LUT drift at ({r},{g},{b}) ch {ch}: {out:?}"
                    );
                }
            }
        }
    }
    // Lattice points return their own grid value exactly, also on a
    // non-identity LUT. Size 5 keeps lattice coordinates exact in binary
    // (k/4 steps), so `cs = c * (size-1)` lands exactly on integers.
    let size5 = 5usize;
    let mut twisted = identity_grid(size5);
    for v in twisted.iter_mut() {
        *v = (*v * 1.2).min(1.0);
    }
    for (i, expected) in twisted.chunks_exact(3).enumerate() {
        let b = i / (size5 * size5);
        let g = (i / size5) % size5;
        let r = i % size5;
        let out = tetra_lut_f64(size5, &twisted, [
            r as f64 / (size5 - 1) as f64,
            g as f64 / (size5 - 1) as f64,
            b as f64 / (size5 - 1) as f64,
        ]);
        for ch in 0..3 {
            assert!((out[ch] - expected[ch]).abs() < 1e-9);
        }
    }
}

/// Phase 3 gate: +3 stops then −3 stops must recover the source with zero
/// highlight clipping in ACEScg scene-linear (HDR headroom proof).
#[test]
fn exposure_roundtrip_recovers_highlights_with_zero_clipping() {
    let Some(ctx) = gpu() else { return };
    let pipeline = ColorNodesPipeline::new(&ctx).expect("chain pipeline");
    let nodes = [
        EvalNode {
            kind: KIND_PRIMARY_WHEELS,
            enabled: true,
            data: vec![3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            windows: vec![],
        },
        EvalNode {
            kind: KIND_PRIMARY_WHEELS,
            enabled: true,
            data: vec![-3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            windows: vec![],
        },
    ];
    let film = FilmPlan::default();
    let pixels = test_pixels();
    let got = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &nodes, &film, 0, None)
        .expect("dispatch");
    for i in 0..(TW * TH) as usize {
        for ch in 0..3 {
            let src = pixels[i * 4 + ch];
            let out = got[i * 4 + ch];
            // +3 stops then −3 stops is a net identity: the output cell must
            // carry the un-clipped HDR value straight through (the point of
            // scene-linear headroom).
            assert!(
                out <= src * (1.0 + 5e-4) + 1e-6,
                "highlights must not clip: src {src} → {out}"
            );
            let rel = ((out - src) / src).abs();
            assert!(
                rel < 5e-4,
                "exposure roundtrip drift {rel} at px {i} ch {ch}: {src} → {out}"
            );
        }
    }
}

#[test]
fn grain_is_deterministic_per_frame_and_frame_dependent() {
    let Some(ctx) = gpu() else { return };
    let pipeline = ColorNodesPipeline::new(&ctx).expect("chain pipeline");
    let nodes = [EvalNode {
        kind: KIND_FILM_GRAIN,
        enabled: true,
        data: vec![1.0, 0.0, 0.0, 0.0],
        windows: vec![],
    }];
    let film = FilmPlan { halation: 0.0, grain: 1.0, weave_mm: 0.0 };
    let pixels = test_pixels();
    let a = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &nodes, &film, 10, None)
        .expect("dispatch");
    let b = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &nodes, &film, 10, None)
        .expect("dispatch");
    let c = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &nodes, &film, 11, None)
        .expect("dispatch");
    assert_eq!(a, b, "same frame must be bit-identical");
    assert_ne!(a, c, "grain must vary per frame");
    // Zero strength is exactly the source.
    let off = [EvalNode {
        kind: KIND_FILM_GRAIN,
        enabled: true,
        data: vec![0.0, 0.0, 0.0, 0.0],
        windows: vec![],
    }];
    let none = pipeline
        .transform_rgba(&ctx, TW, TH, &pixels, &off, &film, 10, None)
        .expect("dispatch");
    for i in 0..(TW * TH) as usize {
        for ch in 0..3 {
            assert_eq!(none[i * 4 + ch], pixels[i * 4 + ch]);
        }
    }
}

#[test]
fn full_chain_through_build_plan_matches_reference() {
    // Engine-shaped payloads via build_plan: wheels + curves + a windowed
    // zone bump, packed through the real packer.
    let chain = vec![
        ColorNode {
            id: 1,
            kind: ColorNodeKind::PrimaryWheels,
            enabled: true,
            payload: serde_json::json!({
                "exposure_stops": 1.0,
                "gain": {"red": 0.1, "green": 0.0, "blue": -0.05, "luma": 0.0},
            }),
            windows: vec![],
        },
        ColorNode {
            id: 2,
            kind: ColorNodeKind::HdrZones,
            enabled: true,
            payload: serde_json::json!({
                "highlight": {"wheel": {"red": 0.1, "green": 0.05, "blue": -0.1, "luma": 0.0}},
                "shadow": {"wheel": {"red": -0.05, "green": 0.0, "blue": 0.05, "luma": 0.0}},
            }),
            windows: vec![],
        },
        ColorNode {
            id: 3,
            kind: ColorNodeKind::RgbCurves,
            enabled: true,
            payload: serde_json::json!({
                "master": [[0.0, 0.0], [0.5, 0.62], [1.0, 1.0]],
            }),
            windows: vec![PowerWindow {
                id: 90,
                enabled: true,
                shape: WindowShape::Ellipse {
                    center: [0.5, 0.5],
                    half_size: [0.35, 0.3],
                },
                softness: 0.15,
                inverted: false,
                opacity: 0.9,
                transform: WindowTransform {
                    pos_x: 0.5,
                    pos_y: 0.5,
                    scale: 1.15,
                    rotation_deg: -20.0,
                },
            }],
        },
    ];
    let (nodes, film) = build_plan(&chain).expect("plan");
    assert_gpu_matches_cpu(&nodes, &film, 6, 5e-3, "full chain");
}

#[test]
fn too_many_nodes_and_windows_error_typed() {
    let chain: Vec<ColorNode> = (0..17)
        .map(|i| ColorNode {
            id: i,
            kind: ColorNodeKind::PrimaryWheels,
            enabled: true,
            payload: serde_json::json!({}),
            windows: vec![],
        })
        .collect();
    let error = build_plan(&chain).expect_err("over-limit chain must refuse");
    let ColorNodesError::TooManyNodes(17) = error else {
        panic!("wrong error: {error:?}")
    };
    // Then windows: rebuild with the last node carrying 5 windows.
    let mut chain = chain[..16].to_vec();
    chain[15].windows = (0..5)
        .map(|i| PowerWindow {
            id: 100 + i,
            enabled: true,
            shape: WindowShape::Rect { center: [0.5, 0.5], half_size: [0.1, 0.1] },
            softness: 0.0,
            inverted: false,
            opacity: 1.0,
            transform: WindowTransform::default(),
        })
        .collect();
    let error = build_plan(&chain).expect_err("over-limit windows must refuse");
    let ColorNodesError::TooManyWindows(15, 5) = error else {
        panic!("wrong error: {error:?}")
    };
}

/// Halation invariant (GPU-only check): the spill texture must be
/// red-dominant near the bright point and exactly zero away from it.
#[test]
fn halation_spill_is_red_dominant_and_localised() {
    use saturn_render::film_emulation::HalationPipeline;
    let Some(ctx) = gpu() else { return };
    let hal = HalationPipeline::new(&ctx).expect("halation pipeline");
    // 16x12 frame, one bright source pixel at (8, 6).
    let mut pixels = vec![0.0f32; (TW * TH * 4) as usize];
    let bright = ((6 * TW + 8) * 4) as usize;
    pixels[bright] = 10.0;
    pixels[bright + 1] = 10.0;
    pixels[bright + 2] = 10.0;
    let spill = hal.spill_texture(&ctx, TW, TH, &pixels).expect("spill");
    let got = ctx.readback_rgba32float(&spill, TW, TH).expect("readback");
    let at = |x: u32, y: u32| ((y * TW + x) * 4) as usize;
    // At the source: red spill clearly present and red-dominant.
    let (r, g, b) = (got[at(8, 6)], got[at(8, 6) + 1], got[at(8, 6) + 2]);
    assert!(r > 0.05, "bright pixel must produce spill, got {r}");
    assert!(g < r && b < r, "spill must be red-dominant: {r} {g} {b}");
    // One pixel away: spill decays but stays red-dominant.
    let (r1, g1, b1) = (got[at(9, 6)], got[at(9, 6) + 1], got[at(9, 6) + 2]);
    assert!(r1 > 0.0 && r1 < r, "neighbour spill {r1} must decay from {r}");
    assert!(g1 < r1 && b1 < r1, "neighbour spill must stay red-dominant");
    // Far corner: exactly zero (outside the 2-sigma kernel radius).
    for (x, y) in [(0u32, 0u32), (15, 11)] {
        let o = at(x, y);
        assert_eq!(got[o], 0.0, "spill must be zero far from highlights");
        assert_eq!(got[o + 1], 0.0);
        assert_eq!(got[o + 2], 0.0);
    }
}
