// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MODNet portrait matting over ONNX Runtime (Packet 2.3).
//!
//! Licence-driven model choice: the plan suggested "RVM v2 / MODNet".
//! RobustVideoMatting is GPL-3.0 — repository *and* its exported models
//! (incl. the official ONNX exports; upstream maintainers confirmed the
//! exports inherit the licence, see PaddlePaddle/FastDeploy#1059) — and is
//! therefore **refused** without maintainer approval (provenance rule:
//! permissive licences only; a GPL weight would poison the EEP commercial
//! licence). MODNet (ZHKKKe/MODNet) is Apache-2.0 for code, models and
//! demos since 2022-01 (issue #70 documents the switch from CC-BY-NC-SA).
//!
//! Provenance (recorded in `docs/model-licences.md` in this change):
//! - Upstream weights: `ZHKKKe/MODNet` (Apache-2.0) photographic portrait
//!   checkpoint.
//! - ONNX export: `Xenova/modnet` @
//!   `fa2fa546052fba4c08921230a26cc69a333fca12`, file `onnx/model.onnx`
//!   (fp32, 25,888,640 bytes, SHA-256
//!   `07c308cf0fc7e6e8b2065a12ed7fc07e1de8febb7dc7839d7b7f15dd66584df9`).
//!
//! Preprocessing is pinned to the export repo's `preprocessor_config.json`
//! at the same revision: resize so the shortest edge is 512 (each dimension
//! rounded to a multiple of 32), rescale 1/255, normalize mean 0.5 / std
//! 0.5 per channel (i.e. to [−1, 1]), bilinear resample.
//!
//! Honesty notes: MODNet is a *portrait* matting model (humans). Frames
//! without a person report low alpha almost everywhere — that is the model
//! being honest, not a failure. It is also a per-frame model: temporal
//! stability comes from callers, not from network memory (we expose an
//! explicit exponential smoothing helper, labelled as such; it is *not*
//! RVM-style recurrent state).

use crate::models::{fetch_verified, models_dir, verified_local_file};
use crate::shot_detect::SampledFrame;
use ort::session::Session;
use ort::value::Tensor;
use std::path::PathBuf;

/// Pinned revision of the ONNX export repo (checksum recorded 2026-11-06).
pub const MODNET_REVISION: &str = "fa2fa546052fba4c08921230a26cc69a333fca12";
const MODNET_HOST: &str = "https://huggingface.co/Xenova/modnet/resolve";

const MODEL: FileSpec = FileSpec {
    remote_path: "onnx/model.onnx",
    file: "model.onnx",
    size: 25_888_640,
    sha256: "07c308cf0fc7e6e8b2065a12ed7fc07e1de8febb7dc7839d7b7f15dd66584df9",
};

struct FileSpec {
    remote_path: &'static str,
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

impl FileSpec {
    fn url(self) -> String {
        format!("{MODNET_HOST}/{MODNET_REVISION}/{}", self.remote_path)
    }
}

/// Directory holding the MODNet assets: `<models>/modnet/`.
pub fn modnet_dir() -> Result<PathBuf, String> {
    let dir = models_dir()?.join("modnet");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create the MODNet model directory: {error}"))?;
    Ok(dir)
}

/// Resolve the MODNet ONNX file to a verified local path, downloading on
/// first use. Errors name the file and URL so offline machines can place it
/// manually.
pub fn ensure_modnet_model() -> Result<PathBuf, String> {
    let path = modnet_dir()?.join(MODEL.file);
    if verified_local_file(&path, "MODNet matting model", MODEL.size, MODEL.sha256, &MODEL.url())
        .is_some()
    {
        fetch_verified(&MODEL.url(), &path, MODEL.sha256, "MODNet matting model", Some(MODEL.size))?;
        if let Some(problem) =
            verified_local_file(&path, "MODNet matting model", MODEL.size, MODEL.sha256, &MODEL.url())
        {
            return Err(problem);
        }
    }
    Ok(path)
}

/// Inference size: the shortest frame edge is scaled to this (the export's
/// `preprocessor_config.json` pins `shortest_edge: 512`, `size_divisibility:
/// 32`); both edges are rounded down to a multiple of 32 afterwards.
const SHORTEST_EDGE: u32 = 512;
const SIZE_DIVISIBILITY: u32 = 32;

/// Compute the model-space (width, height) for a frame, deterministically:
/// scale so the shorter edge is `SHORTEST_EDGE`, round both edges down to
/// multiples of 32 (at least 32, and at most the scaled size so pixels are
/// never invented — the round-down only crops edge pixels).
pub fn model_input_size(width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (width.max(1), height.max(1));
    let scale = SHORTEST_EDGE as f32 / w.min(h) as f32;
    let mut sw = (w as f32 * scale).floor() as u32;
    let mut sh = (h as f32 * scale).floor() as u32;
    sw = (sw / SIZE_DIVISIBILITY * SIZE_DIVISIBILITY).max(SIZE_DIVISIBILITY);
    sh = (sh / SIZE_DIVISIBILITY * SIZE_DIVISIBILITY).max(SIZE_DIVISIBILITY);
    (sw, sh)
}

/// A loaded MODNet session. Takes `&mut self` for inference (ort
/// requirement); bind one session per thread.
pub struct ModNetMatting {
    _env: ort::environment::Environment,
    session: Option<Session>,
}

impl ModNetMatting {
    pub fn load() -> Result<Self, String> {
        let env = ort::init()
            .with_name("easy-edits-pro-perception")
            .build()
            .map_err(|error| format!("Could not initialize ONNX Runtime: {error}"))?;
        Ok(Self {
            _env: env,
            session: None,
        })
    }

    fn session(&mut self) -> Result<&mut Session, String> {
        if self.session.is_none() {
            let path = ensure_modnet_model()?;
            let session = Session::builder(&self._env)
                .and_then(|mut b| b.commit_from_file(&path))
                .map_err(|error| {
                    format!("Could not load the MODNet model at {}: {error}", path.display())
                })?;
            self.session = Some(session);
        }
        self.session
            .as_mut()
            .ok_or_else(|| "MODNet session missing after load".to_owned())
    }

    /// Produce the alpha matte for one RGB8 frame, returned at the frame's
    /// own resolution (row-major, one f32 in 0.0..1.0 per pixel).
    pub fn matte_frame(&mut self, frame: &SampledFrame) -> Result<Vec<f32>, String> {
        let expected = frame.width as usize * frame.height as usize * 3;
        if frame.rgb.len() != expected {
            return Err(format!(
                "Frame RGB payload is {} bytes, expected {expected} for {}×{}",
                frame.rgb.len(),
                frame.height,
                frame.width
            ));
        }
        let (mw, mh) = model_input_size(frame.width, frame.height);
        let resized = bilinear_resize_rgb(&frame.rgb, frame.width, frame.height, mw, mh);
        let input = normalize_to_unit_scale(&resized);
        let tensor = Tensor::from_array((
            vec![1_i64, 3_i64, mh as i64, mw as i64],
            input,
        ))
        .map_err(|error| format!("Could not build the MODNet input tensor: {error}"))?;
        let session = self.session()?;
        let outputs = session
            .run(ort::inputs!["input" => tensor])
            .map_err(|error| format!("MODNet inference failed: {error}"))?;
        // The export emits a single `output` [1, 1, H, W] alpha plane.
        let (shape, alpha) = outputs["output"]
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("MODNet output unreadable: {error}"))?;
        let dims: Vec<i64> = shape.as_ref().to_vec();
        if dims != [1, 1, mh as i64, mw as i64] {
            return Err(format!(
                "MODNet output shape {dims:?} is not [1, 1, {mh}, {mw}]"
            ));
        }
        let mut clamped: Vec<f32> = alpha
            .iter()
            .map(|&a| a.clamp(0.0, 1.0))
            .collect();
        if (mw, mh) != (frame.width, frame.height) {
            // Upsample the matte back to frame resolution (bilinear). The
            // model grid covers the *scaled* frame; scale factors map frame
            // pixels into grid space exactly like the forward resize.
            let upsampled = bilinear_upsample(
                &clamped,
                mw,
                mh,
                frame.width,
                frame.height,
            );
            clamped = upsampled;
        }
        Ok(clamped)
    }
}

/// CHW layout with the pinned rescale + normalization (1/255 then
/// (x − 0.5)/0.5 → [−1, 1]).
fn normalize_to_unit_scale(resized: &[u8]) -> Vec<f32> {
    let pixels = resized.len() / 3;
    let mut out = Vec::with_capacity(pixels * 3);
    for channel in 0..3 {
        for pixel in 0..pixels {
            let value = resized[pixel * 3 + channel] as f32 / 255.0;
            out.push((value - 0.5) / 0.5);
        }
    }
    out
}

/// Bilinear RGB resize (deterministic, no SIMD; scale factor matches the
/// matte upsample's inverse so resample round-trips stay consistent).
pub(crate) fn bilinear_resize_rgb(
    rgb: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<u8> {
    let (src_w, src_h, dst_w, dst_h) = (src_w.max(1), src_h.max(1), dst_w.max(1), dst_h.max(1));
    let mut out = vec![0_u8; dst_w as usize * dst_h as usize * 3];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;
    for y in 0..dst_h {
        let sy = ((y as f32 + 0.5) * y_ratio - 0.5).max(0.0);
        let y0 = (sy.floor() as u32).min(src_h - 1);
        let y1 = (y0 + 1).min(src_h - 1);
        let fy = sy - y0 as f32;
        for x in 0..dst_w {
            let sx = ((x as f32 + 0.5) * x_ratio - 0.5).max(0.0);
            let x0 = (sx.floor() as u32).min(src_w - 1);
            let x1 = (x0 + 1).min(src_w - 1);
            let fx = sx - x0 as f32;
            let dst_idx = (y * dst_w + x) as usize * 3;
            for c in 0..3 {
                let p00 = rgb[(y0 * src_w + x0) as usize * 3 + c] as f32;
                let p01 = rgb[(y0 * src_w + x1) as usize * 3 + c] as f32;
                let p10 = rgb[(y1 * src_w + x0) as usize * 3 + c] as f32;
                let p11 = rgb[(y1 * src_w + x1) as usize * 3 + c] as f32;
                let top = p00 + (p01 - p00) * fx;
                let bottom = p10 + (p11 - p10) * fx;
                out[dst_idx + c] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// Bilinear single-channel upsample (matte resize), same center-origin
/// convention as [`bilinear_resize_rgb`].
pub(crate) fn bilinear_upsample(
    plane: &[f32],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<f32> {
    let (src_w, src_h, dst_w, dst_h) = (src_w.max(1), src_h.max(1), dst_w.max(1), dst_h.max(1));
    let mut out = vec![0.0_f32; dst_w as usize * dst_h as usize];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;
    for y in 0..dst_h {
        let sy = ((y as f32 + 0.5) * y_ratio - 0.5).max(0.0);
        let y0 = (sy.floor() as u32).min(src_h - 1);
        let y1 = (y0 + 1).min(src_h - 1);
        let fy = sy - y0 as f32;
        for x in 0..dst_w {
            let sx = ((x as f32 + 0.5) * x_ratio - 0.5).max(0.0);
            let x0 = (sx.floor() as u32).min(src_w - 1);
            let x1 = (x0 + 1).min(src_w - 1);
            let fx = sx - x0 as f32;
            let p00 = plane[(y0 * src_w + x0) as usize];
            let p01 = plane[(y0 * src_w + x1) as usize];
            let p10 = plane[(y1 * src_w + x0) as usize];
            let p11 = plane[(y1 * src_w + x1) as usize];
            let top = p00 + (p01 - p00) * fx;
            let bottom = p10 + (p11 - p10) * fx;
            out[(y * dst_w + x) as usize] = top + (bottom - top) * fy;
        }
    }
    out
}

/// Explicit exponential smoothing across frames (per-frame model: this is
/// caller-side temporal smoothing, NOT recurrent network memory). `factor`
/// is the weight of the previous frame's matte (0 = none, 0.9 = heavy).
/// State holds the previous matte; reset whenever the source changes.
#[derive(Clone, Debug, Default)]
pub struct MatteSmoother {
    previous: Vec<f32>,
}

impl MatteSmoother {
    pub fn reset(&mut self) {
        self.previous.clear();
    }

    pub fn smooth(&mut self, matte: Vec<f32>, factor: f32) -> Vec<f32> {
        let factor = factor.clamp(0.0, 0.95);
        if self.previous.len() != matte.len() || factor == 0.0 {
            self.previous = matte.clone();
            return matte;
        }
        let smoothed: Vec<f32> = matte
            .iter()
            .zip(&self.previous)
            .map(|(&current, &prev)| current * (1.0 - factor) + prev * factor)
            .collect();
        self.previous = smoothed.clone();
        smoothed
    }
}

/// Feather an alpha matte with a rounded box blur (radius in matte pixels).
/// Lives here (not in the compositor) so both perception tests and callers
/// share one definition; radius 0 returns the input unchanged.
pub fn feather_alpha(matte: &[f32], width: u32, height: u32, radius: u32) -> Vec<f32> {
    if radius == 0 || matte.len() != width as usize * height as usize {
        return matte.to_vec();
    }
    let (w, h) = (width as usize, height as usize);
    let r = radius as usize;
    // Prefix sums make the box blur O(1) per pixel (deterministic order).
    let mut prefix = vec![0.0_f64; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row_acc = 0.0_f64;
        for x in 0..w {
            row_acc += matte[y * w + x] as f64;
            prefix[(y + 1) * (w + 1) + (x + 1)] =
                prefix[y * (w + 1) + (x + 1)] + row_acc;
        }
    }
    let mut out = vec![0.0_f32; w * h];
    for y in 0..h {
        let y_min = y.saturating_sub(r);
        let y_max = (y + r + 1).min(h);
        for x in 0..w {
            let x_min = x.saturating_sub(r);
            let x_max = (x + r + 1).min(w);
            let sum = prefix[y_max * (w + 1) + x_max]
                + prefix[y_min * (w + 1) + x_min]
                - prefix[y_max * (w + 1) + x_min]
                - prefix[y_min * (w + 1) + x_max];
            // Every cell in the rect counts 1 — no prefix table needed.
            let count = ((y_max - y_min) * (x_max - x_min)) as f64;
            // Alpha is semantically bounded — clamp away f64/f32 mean drift.
            out[y * w + x] = ((sum / count.max(1.0)) as f32).clamp(0.0, 1.0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modnet_checksum_is_well_formed() {
        assert_eq!(MODEL.sha256.len(), 64);
        assert!(MODEL.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(MODEL.file, "model.onnx");
 assert!(MODEL.size > 20_000_000);
    }

    #[test]
    fn modnet_url_pins_the_revision() {
        assert!(MODEL.url().contains(MODNET_REVISION));
        assert!(MODEL.url().starts_with(MODNET_HOST));
        assert!(MODEL.url().ends_with("/onnx/model.onnx"));
    }

    #[test]
    fn model_input_size_follows_the_pinned_shortest_edge() {
        // Landscape 1920×1080: shortest edge 1080 → 512; long edge scales to
        // ~910 → rounds down to 896 (multiple of 32).
        assert_eq!(model_input_size(1920, 1080), (896, 512));
        // Portrait flips.
        assert_eq!(model_input_size(1080, 1920), (512, 896));
        // Already smaller than 512: shortest edge still scales UP to 512
        // (HF shortest-edge semantics), long edge follows the aspect ratio.
        assert_eq!(model_input_size(200, 100), (1024, 512));
        // Degenerate input is simply scaled up like any other frame.
        assert_eq!(model_input_size(1, 1), (512, 512));
    }

    #[test]
    fn resize_and_upsample_round_trip_preserves_center() {
        // A 4×4 plane with one hot pixel scales to 8×8 and back; the center
        // of the hot region stays hot, the border stays ~0.
        let mut plane = vec![0.0_f32; 16];
        plane[5] = 1.0; // row 1, col 1
        plane[6] = 1.0;
        plane[9] = 1.0;
        plane[10] = 1.0;
        let up = bilinear_upsample(&plane, 4, 4, 8, 8);
        assert_eq!(up.len(), 64);
        assert!(up[18 + 2] > 0.5, "hot center survives: {}", up[20]);
        assert!(up[0] < 0.3, "corner stays cool: {}", up[0]);
        let down = bilinear_upsample(&up, 8, 8, 4, 4);
        assert!(down[5] > 0.5);
    }

    #[test]
    fn feather_spreads_hard_edges_by_the_radius() {
        // 10×10: hard 1.0 block in the left half, 0.0 right — radius 2 blurs
        // the boundary but keeps both sides' plateaus near their value.
        let mut matte = vec![0.0_f32; 100];
        for y in 0..10 {
            for x in 0..5 {
                matte[y * 10 + x] = 1.0;
            }
        }
        let feathered = feather_alpha(&matte, 10, 10, 2);
        assert!(feathered[4 * 10 + 0] > 0.9);
        assert!(feathered[4 * 10 + 9] < 0.1);
        // The boundary column moved toward the middle.
        assert!(feathered[4 * 10 + 5] > 0.2 && feathered[4 * 10 + 5] < 0.8);
        // Radius 0 is identity.
        assert_eq!(feather_alpha(&matte, 10, 10, 0), matte);
    }

    #[test]
    fn smoothing_is_a_real_ema() {
        let mut smoother = MatteSmoother::default();
        let first = smoother.smooth(vec![1.0; 4], 0.5);
        assert_eq!(first, vec![1.0; 4]);
        let second = smoother.smooth(vec![0.0; 4], 0.5);
        assert!(second.iter().all(|&v| (v - 0.5).abs() < 1e-6));
        // Shaped change resets cleanly.
        let third = smoother.smooth(vec![0.25; 8], 0.5);
        assert_eq!(third, vec![0.25; 8]);
    }
}
