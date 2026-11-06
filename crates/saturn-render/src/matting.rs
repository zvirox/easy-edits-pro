// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Background-matting compositing (Packet 2.3).
//!
//! This module owns no inference: the alpha matte is *measured* by
//! `saturn_perception::matting_ort` (MODNet over ONNX Runtime) and handed in
//! by the caller. What lives here is the compositing pass — thresholding the
//! measured alpha, feathering it (`saturn_perception::feather_alpha`), and
//! blending the RGBA buffer against the chosen background.
//!
//! History note: until Packet 2.3 this module faked a Gaussian/skin-chroma
//! "neural" matte with an RVM-shaped recurrent smoothing state. RVM itself
//! is GPL-3.0 (weights included) and was refused upstream of this change —
//! see `docs/model-licences.md` — and the fake heuristic is gone; a matte
//! that is not backed by a real model run now simply does not exist here.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MattingOptions {
    /// Alpha values below the threshold are pulled to 0; the remainder is
    /// rescaled over the surviving range (soft knee, not a hard cut).
    pub threshold: f32,
    /// Feather radius in whole pixels (0 = off; capped at 20 like the UI).
    pub feather_radius: u32,
    pub background_mode: BackgroundMode,
}

impl Default for MattingOptions {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            feather_radius: 2,
            background_mode: BackgroundMode::Transparent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
    Transparent,
    GreenScreen,
    SolidColor { r: u8, g: u8, b: u8 },
}

/// Parse a background mode from the UI's string ids. Unknown ids are a
/// typed error, not a silent fallback to transparent.
pub fn background_mode_from_id(id: &str) -> Result<BackgroundMode, String> {
    match id {
        "transparent" => Ok(BackgroundMode::Transparent),
        "green" | "greenscreen" => Ok(BackgroundMode::GreenScreen),
        "black" => Ok(BackgroundMode::SolidColor { r: 0, g: 0, b: 0 }),
        "white" => Ok(BackgroundMode::SolidColor {
            r: 255,
            g: 255,
            b: 255,
        }),
        other => Err(format!(
            "unknown background mode `{other}` (expected transparent / green / black / white)"
        )),
    }
}

/// Composite a *measured* alpha matte into an RGBA buffer according to the
/// options: threshold → feather → background blend. Returns the final
/// (thresholded + feathered) alpha so callers can inspect/report it.
///
/// `alpha` must hold one 0..=1 value per pixel (row-major). The buffer is
/// modified in place.
pub fn composite_matting(
    rgba_buf: &mut [u8],
    width: u32,
    height: u32,
    measured_alpha: &[f32],
    options: &MattingOptions,
) -> Result<Vec<f32>, String> {
    let pixels = width as usize * height as usize;
    if rgba_buf.len() != pixels * 4 {
        return Err(format!(
            "RGBA buffer is {} bytes, expected {pixels} pixels × 4 at {}×{}",
            rgba_buf.len(),
            height,
            width
        ));
    }
    if measured_alpha.len() != pixels {
        return Err(format!(
            "alpha matte holds {} values, expected {pixels} at {}×{}",
            measured_alpha.len(),
            height,
            width
        ));
    }
    let threshold = options.threshold.clamp(0.0, 1.0);

    // Threshold: values below cut to 0; the rest rescale across the
    // surviving band so a 0.9 threshold still full-strengths a strong matte.
    let span = (1.0 - threshold).max(1e-4);
    let mut alpha: Vec<f32> = measured_alpha
        .iter()
        .map(|&a| {
            let a = a.clamp(0.0, 1.0);
            if a <= threshold {
                0.0
            } else {
                ((a - threshold) / span).clamp(0.0, 1.0)
            }
        })
        .collect();

    // Feather (shared, deterministic implementation in perception).
    if options.feather_radius > 0 {
        alpha = saturn_perception::feather_alpha(&alpha, width, height, options.feather_radius);
    }

    for (i, &a) in alpha.iter().enumerate() {
        let idx = i * 4;
        match options.background_mode {
            BackgroundMode::Transparent => {
                rgba_buf[idx + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            BackgroundMode::GreenScreen => {
                let orig_r = rgba_buf[idx] as f32;
                let orig_g = rgba_buf[idx + 1] as f32;
                let orig_b = rgba_buf[idx + 2] as f32;
                rgba_buf[idx] = (orig_r * a).round() as u8;
                rgba_buf[idx + 1] = (orig_g * a + 255.0 * (1.0 - a))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                rgba_buf[idx + 2] = (orig_b * a).round() as u8;
                rgba_buf[idx + 3] = 255;
            }
            BackgroundMode::SolidColor { r, g, b } => {
                let orig_r = rgba_buf[idx] as f32;
                let orig_g = rgba_buf[idx + 1] as f32;
                let orig_b = rgba_buf[idx + 2] as f32;
                rgba_buf[idx] = (orig_r * a + r as f32 * (1.0 - a))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                rgba_buf[idx + 1] = (orig_g * a + g as f32 * (1.0 - a))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                rgba_buf[idx + 2] = (orig_b * a + b as f32 * (1.0 - a))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                rgba_buf[idx + 3] = 255;
            }
        }
    }
    Ok(alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(mode: BackgroundMode) -> MattingOptions {
        MattingOptions {
            threshold: 0.5,
            feather_radius: 0,
            background_mode: mode,
        }
    }

    #[test]
    fn composite_writes_alpha_in_transparent_mode() {
        // 2×2: left column opaque, right column transparent.
        let mut buf = vec![10_u8; 4 * 4];
        let alpha = vec![0.9, 0.9, 0.1, 0.1];
        let out = composite_matting(&mut buf, 2, 2, &alpha, &options(BackgroundMode::Transparent))
            .expect("composites");
        assert!(
            out.iter().zip([0.8_f32, 0.8, 0.0, 0.0]).all(|(a, b)| (a - b).abs() < 1e-6),
            "threshold rescales the surviving band: {out:?}"
        );
        assert_eq!(buf[3], (0.8_f32 * 255.0).round() as u8);
        assert_eq!(buf[2 * 4 + 3], 0);
        // RGB stays untouched in transparent mode.
        assert_eq!(buf[0], 10);
    }

    #[test]
    fn composite_blends_green_and_solid_backgrounds() {
        let mut buf = vec![200, 100, 50, 255, 200, 100, 50, 255];
        let alpha = vec![1.0, 0.0];
        composite_matting(&mut buf, 2, 1, &alpha, &options(BackgroundMode::GreenScreen))
            .expect("composites");
        assert_eq!(&buf[0..4], &[200, 100, 50, 255], "full-alpha foreground is untouched");
        assert_eq!(&buf[4..8], &[0, 255, 0, 255], "background becomes pure green");

        let mut buf = vec![200, 100, 50, 255];
        // alpha 0.75 → threshold 0.5 rescales to 0.5 → half blend.
        composite_matting(&mut buf, 1, 1, &[0.75], &options(BackgroundMode::SolidColor {
            r: 20,
            g: 30,
            b: 40,
        }))
        .expect("composites");
        assert_eq!(buf[0], 110); // 200·0.5 + 20·0.5
        assert_eq!(buf[1], 65); // 100·0.5 + 30·0.5
        assert_eq!(buf[2], 45); // 50·0.5 + 40·0.5
        assert_eq!(buf[3], 255);
    }

    #[test]
    fn composite_feathers_through_the_shared_implementation() {
        // 10×10 half/half matte with radius 2: the boundary blends.
        let mut alpha = vec![0.0_f32; 100];
        for y in 0..10 {
            for x in 0..5 {
                alpha[y * 10 + x] = 1.0;
            }
        }
        let mut buf = vec![0_u8; 100 * 4];
        let options = MattingOptions {
            threshold: 0.0,
            feather_radius: 2,
            background_mode: BackgroundMode::Transparent,
        };
        let out = composite_matting(&mut buf, 10, 10, &alpha, &options).expect("composites");
        assert!(out[4 * 10 + 5] > 0.2 && out[4 * 10 + 5] < 0.8, "boundary feathered");
        assert_eq!(out[4 * 10], 1.0);
    }

    #[test]
    fn composite_rejects_mismatched_buffers() {
        let mut buf = vec![0_u8; 8];
        assert!(composite_matting(&mut buf, 3, 3, &[1.0; 9], &options(BackgroundMode::Transparent)).is_err());
        let mut buf = vec![0_u8; 36];
        assert!(composite_matting(&mut buf, 3, 3, &[1.0; 8], &options(BackgroundMode::Transparent)).is_err());
    }

    #[test]
    fn background_mode_ids_parse_strictly() {
        assert!(matches!(
            background_mode_from_id("green").unwrap(),
            BackgroundMode::GreenScreen
        ));
        assert!(matches!(
            background_mode_from_id("white").unwrap(),
            BackgroundMode::SolidColor { r: 255, g: 255, b: 255 }
        ));
        assert!(background_mode_from_id("plaid").is_err());
    }
}
