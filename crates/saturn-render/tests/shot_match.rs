// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 3.3 shot matcher — the Phase 3 verification gate:
//! `color.match_shot_to_reference` brings mismatched camera angles within
//! CIEDE2000 ΔE < 1.5. Pure CPU (matcher fit + the CPU reference node
//! dispatch); no GPU device is required.

use saturn_core::color::spaces::{CameraIdt, WorkingSpace, rec709_from_xyz_d65};
use saturn_core::color::transforms::{M_AP0_TO_XYZ_D65, mat3_apply, rec709_oetf_encode};
use saturn_core::project::{ColorNode, ColorNodeKind};
use saturn_perception::color_metrics::{ciede2000, rgb_continuous_to_lab, rgb_to_lab};
use saturn_render::color_nodes_gpu::build_plan;
use saturn_render::color_nodes_shader::reference_dispatch;
use saturn_render::shot_match::{fit_match, measurement};

const W: u32 = 96;
const H: u32 = 96;

/// Continuous (unquantised) Rec.709 signal in 0..1 for an ACEScg pixel —
/// the encode side of the matcher's decode path with no 8-bit rounding,
/// used where the metric must average smoothly (the patch gate).
fn encode_signal_continuous(acescg: [f64; 3]) -> [f64; 3] {
    let ap0 = WorkingSpace::AcesCg.to_ap0(acescg);
    let xyz = mat3_apply(&M_AP0_TO_XYZ_D65, ap0);
    mat3_apply(&rec709_from_xyz_d65(), xyz).map(|v| rec709_oetf_encode(v.clamp(0.0, 1.0)))
}

/// ACEScg-linear → 8-bit Rec.709 signal pixel (the inverse of the matcher's
/// decode path, via the project's real display transform).
fn encode_pixel(acescg: [f64; 3]) -> [u8; 3] {
    encode_signal_continuous(acescg).map(|v| (v * 255.0).round() as u8)
}

/// One synthetic shot: a textured luma ramp of greys plus a skin-band patch,
/// both in ACEScg-linear. The per-pixel scene values are deterministic
/// (`.scene()`), so a second shot rendered through a global colour map keeps
/// the exact same content — the only difference between the two frames is
/// the map itself plus 8-bit code quantisation, which is precisely what the
/// matcher must absorb.
struct Shot {
    /// Scene values per pixel in ACEScg-linear (textured, unclipped).
    scene: Vec<[f64; 3]>,
}

impl Shot {
    fn render_rgb8(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.scene.len() * 3);
        for pixel in &self.scene {
            buf.extend_from_slice(&encode_pixel(*pixel));
        }
        buf
    }

    /// The full frame decoded back to ACEScg f32 RGBA (grading input).
    fn decoded_acescg(&self) -> Vec<f32> {
        let rgb = self.render_rgb8();
        let mut out = Vec::with_capacity(self.scene.len() * 4);
        for p in 0..self.scene.len() {
            let cv = [
                f64::from(rgb[p * 3]) / 255.0,
                f64::from(rgb[p * 3 + 1]) / 255.0,
                f64::from(rgb[p * 3 + 2]) / 255.0,
            ];
            let ap0 = CameraIdt::Rec709Video.decode_to_ap0(cv);
            let acescg = WorkingSpace::AcesCg.from_ap0(ap0);
            out.extend_from_slice(&[acescg[0] as f32, acescg[1] as f32, acescg[2] as f32, 1.0]);
        }
        out
    }
}

/// The reference content: 96×96, ~12% skin-band pixels in a striped layout,
/// the rest a luma ramp of 64 greys (ACEScct CV 0.15..0.52 — linear
/// ~0.017..0.56, never clipping in the 8-bit signal; adjacent rungs sit
/// several codes apart) with a deterministic per-pixel ±1.9% multiplier
/// (film-grain-like texture) so the ladder resolves into distinct 8-bit
/// codes instead of piling onto shared code runs — with a near-pure ladder
/// the quantile pairs lock onto run centres and the statistics degenerate.
fn reference_scene(skin: [f64; 3]) -> Vec<[f64; 3]> {
    use saturn_core::color::transforms::acescct_decode;
    let pixels = (W * H) as usize;
    let skin_count = (pixels as f64 * 0.12) as usize;
    let rungs = 128usize;
    let greys: Vec<f64> = (0..rungs)
        .map(|i| {
            let cv = 0.15 + (i as f64 + 0.5) * (0.52 - 0.15) / rungs as f64;
            acescct_decode(cv).clamp(0.001, 10.0)
        })
        .collect();
    (0..pixels)
        .map(|p| {
            if p % 8 == 0 && p / 8 < skin_count {
                // The skin patch is textured like the greys, not constant:
                // with a constant value all pixels round the same 8-bit
                // codes in the same direction, baking a systematic ~half-code
                // transverse chroma shift into the frame's band-box mean —
                // a phantom hue-offset the skin lock would then "correct".
                // Real skin is textured and rounds without such a bias.
                let jitter = ((p % 89) as f64 / 89.0 - 0.5) * 0.06;
                skin.map(|c| (c * (1.0 + jitter)).max(1e-4))
            } else {
                let grow = greys[p % greys.len()];
                let jitter = ((p % 97) as f64 / 97.0 - 0.5) * (1.2 / rungs as f64);
                [grow * (1.0 + jitter); 3].map(|c| c.max(1e-4))
            }
        })
        .collect()
}

#[test]
fn matched_angle_lands_within_ciede2000_1_5() {
    // Camera A (reference hero shot): the textured luma ramp + skin patch.
    let reference = Shot {
        scene: reference_scene([0.30, 0.16, 0.085]),
    };
    // Camera B (target): the same content through a different pipeline —
    // −0.45 stops, cool white balance, gamma crush — applied per pixel to
    // the reference's scene values, so the frames are related by one exact
    // global monotone map. The residual is 8-bit code quantisation alone:
    // exactly what the matcher is supposed to absorb.
    let target_shift = |c: [f64; 3]| {
        let k = 0.22_f64.exp2();
        [
            (c[0] * 0.73 * 1.06).powf(k),
            (c[1] * 0.73).powf(k),
            (c[2] * 0.73 * 0.92).powf(k),
        ]
    };
    let target = Shot {
        scene: reference.scene.iter().map(|c| target_shift(*c)).collect(),
    };

    let target_m = measurement(W, H, &target.render_rgb8()).expect("target measurement");
    let reference_m = measurement(W, H, &reference.render_rgb8()).expect("ref measurement");
    let fit = fit_match(&target_m, &reference_m, true).expect("fit");

    // Grade the target through the real persisted form: wheels payload →
    // schema node → build_plan → CPU reference dispatch. The exact artefact
    // the engine would store on the clip.
    let payload = fit.wheels_payload();
    let json = serde_json::to_value(&payload).expect("payload serialises");
    let node = ColorNode {
        id: 1,
        kind: ColorNodeKind::PrimaryWheels,
        enabled: true,
        payload: json,
        windows: vec![],
        origin: Some("shot-match".into()),
    };
    let (nodes, film) = build_plan(&[node]).expect("plan");
    let graded = reference_dispatch(W, H, &target.decoded_acescg(), &nodes, &film, 0);

    // Per-pixel CIEDE2000 between the graded target signal and the
    // reference's own signal (the mean-gate population).
    let reference_signal = reference.render_rgb8();
    let mut sum = 0.0f64;
    for p in 0..(W * H) as usize {
        let r = graded[p * 4].clamp(0.0, 1.0);
        let g = graded[p * 4 + 1].clamp(0.0, 1.0);
        let b = graded[p * 4 + 2].clamp(0.0, 1.0);
        // Graded ACEScg → 8-bit signal through the same encode path.
        let graded_signal = encode_pixel([r as f64, g as f64, b as f64]);
        let lab_a = rgb_to_lab(graded_signal[0], graded_signal[1], graded_signal[2]);
        let lab_b = rgb_to_lab(
            reference_signal[p * 3],
            reference_signal[p * 3 + 1],
            reference_signal[p * 3 + 2],
        );
        let d = ciede2000(lab_a, lab_b);
        sum += d;
    }
    let mean = sum / ((W * H) as f64);

    // Patch-level gate. At the single-pixel scale the 8-bit format itself,
    // not the fit, sets the ΔE floor: near L*≈15 one code step is worth
    // ~ΔE 1.8 on its own (graded [39,38,40] vs [39,39,39] measures a*
    // +0.94, b* −1.16), so the independent ±half-code rounding of the two
    // frames puts individual sub-code pixels beyond ΔE 1.5 for any
    // algorithm. Averaging each 3×3 block's continuous signal values
    // before the Lab conversion cancels that sub-code noise, leaving the
    // quantity the matcher is actually responsible for: the fit residual.
    let patch = 3usize;
    let patch_cols = W as usize / patch;
    let mut worst_patch = (0.0f64, 0usize);
    for by in 0..(H as usize / patch) {
        for bx in 0..patch_cols {
            let mut graded_sum = [0.0f64; 3];
            let mut reference_sum = [0.0f64; 3];
            for dy in 0..patch {
                for dx in 0..patch {
                    let p = (by * patch + dy) * W as usize + bx * patch + dx;
                    let g = [
                        f64::from(graded[p * 4]).clamp(0.0, 1.0),
                        f64::from(graded[p * 4 + 1]).clamp(0.0, 1.0),
                        f64::from(graded[p * 4 + 2]).clamp(0.0, 1.0),
                    ];
                    let graded_signal = encode_signal_continuous(g);
                    for ch in 0..3 {
                        graded_sum[ch] += graded_signal[ch];
                        reference_sum[ch] += f64::from(reference_signal[p * 3 + ch]) / 255.0;
                    }
                }
            }
            let n = (patch * patch) as f64;
            let lab_graded =
                rgb_continuous_to_lab(graded_sum[0] / n, graded_sum[1] / n, graded_sum[2] / n);
            let lab_reference = rgb_continuous_to_lab(
                reference_sum[0] / n,
                reference_sum[1] / n,
                reference_sum[2] / n,
            );
            let d = ciede2000(lab_graded, lab_reference);
            if d > worst_patch.0 {
                worst_patch = (d, by * patch_cols + bx);
            }
        }
    }

    assert!(
        mean < 1.5,
        "shot match gate: mean per-pixel CIEDE2000 ΔE {mean} — did not converge below 1.5"
    );
    assert!(
        worst_patch.0 < 1.5,
        "shot match gate: worst 3×3-patch CIEDE2000 ΔE {} at patch {} — did not converge below 1.5 (mean per-pixel {mean})",
        worst_patch.0,
        worst_patch.1
    );
}
