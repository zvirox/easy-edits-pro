// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Depth-Anything-V2 monocular depth estimation over ONNX Runtime
//! (Packet 2.3).
//!
//! Licence-driven model choice: only **Depth-Anything-V2-Small** is used. Its
//! weights are Apache-2.0 (upstream README + model card); the Base/Large/
//! Giant variants are CC-BY-NC-4.0 and are therefore **excluded** — the
//! catalogue has no entries for them and the code naming `Small` explicitly
//! (provenance rule: no non-commercial model weights).
//!
//! Provenance (recorded in `docs/model-licences.md` in this change):
//! - Upstream weights: `depth-anything/Depth-Anything-V2-Small` @
//!   `03876f8…` (HF tree), `depth_anything_v2_vits.pth`, 99.2 MB, SHA-256
//!   `715fade13be8f229f8a70cc02066f656f2423a59effd0579197bbf57860e1378` —
//!   Apache-2.0. (Recorded for traceability; we consume the ONNX export.)
//! - ONNX export: `onnx-community/depth-anything-v2-small` @
//!   `4472b7362082ad9968fee890ca0f1e5aca36b93d`, file
//!   `onnx/model_quantized.onnx` (int8 weights / f32 activations export,
//!   27,258,801 bytes, SHA-256
//!   `fcf51f1b230362b28690bb9d1809bf0431f29cad20534e3f589bd7285547f20d`).
//!
//! Preprocessing is pinned to the export repo's `preprocessor_config.json`
//! at the same revision (a `DPTImageProcessor` config): rescale 1/255,
//! normalize with ImageNet mean [0.485, 0.456, 0.406] / std [0.229, 0.224,
//! 0.225], bicubic resample, target 518×518, dimensions multiple of 14.
//!
//! Honesty notes:
//! - The Small model is **relative** depth: outputs are an unbounded,
//!   inverse-scale ordering (near = low, far = high in the raw export), not
//!   meters. We only min–max normalize *within one frame* for display; no
//!   metric claims are made.
//! - The upstream HF default `keep_aspect_ratio` resize differs from our
//!   fixed square 518×518 (a documented deviation, chosen so the matte grid
//!   matches the frame 1:1); depth ordering within a frame is preserved,
//!   per-pixel values on strongly non-square frames differ slightly.
//! - The export is int8-quantized; small numeric drift vs the fp32 weights
//!   is expected and accepted.

use crate::matting_ort::{bilinear_resize_rgb, bilinear_upsample};
use crate::models::{fetch_verified, models_dir, verified_local_file};
use crate::shot_detect::SampledFrame;
use ort::session::Session;
use ort::value::Tensor;
use std::path::PathBuf;

/// Pinned revision of the ONNX export repo (checksum recorded 2026-11-06).
pub const DEPTH_MODEL_REVISION: &str = "4472b7362082ad9968fee890ca0f1e5aca36b93d";
const DEPTH_MODEL_HOST: &str =
    "https://huggingface.co/onnx-community/depth-anything-v2-small/resolve";

const MODEL: FileSpec = FileSpec {
    remote_path: "onnx/model_quantized.onnx",
    file: "model_quantized.onnx",
    size: 27_258_801,
    sha256: "fcf51f1b230362b28690bb9d1809bf0431f29cad20534e3f589bd7285547f20d",
};

struct FileSpec {
    remote_path: &'static str,
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

impl FileSpec {
    fn url(self) -> String {
        format!("{DEPTH_MODEL_HOST}/{DEPTH_MODEL_REVISION}/{}", self.remote_path)
    }
}

/// Directory holding the depth assets: `<models>/depth-anything-v2/`.
pub fn depth_model_dir() -> Result<PathBuf, String> {
    let dir = models_dir()?.join("depth-anything-v2");
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!("Could not create the depth model directory: {error}")
    })?;
    Ok(dir)
}

/// Resolve the depth model to a verified local path, downloading on first
/// use. Errors name the file and URL for manual offline placement.
pub fn ensure_depth_model() -> Result<PathBuf, String> {
    let path = depth_model_dir()?.join(MODEL.file);
    if verified_local_file(
        &path,
        "Depth-Anything-V2-Small model",
        MODEL.size,
        MODEL.sha256,
        &MODEL.url(),
    )
    .is_some()
    {
        fetch_verified(
            &MODEL.url(),
            &path,
            MODEL.sha256,
            "Depth-Anything-V2-Small model",
            Some(MODEL.size),
        )?;
        if let Some(problem) = verified_local_file(
            &path,
            "Depth-Anything-V2-Small model",
            MODEL.size,
            MODEL.sha256,
            &MODEL.url(),
        ) {
            return Err(problem);
        }
    }
    Ok(path)
}

/// Model grid: both edges pinned to 518 (a multiple of the 14-pixel patch
/// grid the ViT family requires; the export's dynamic shape accepts any
/// 14-multiple, 518 matches the upstream processor's `size` block).
pub const DEPTH_INPUT_SIZE: u32 = 518;

/// A raw depth grid as produced by the model (`values` are the model's own
/// relative-depth ordering: larger = farther in the raw export).
#[derive(Clone, Debug, PartialEq)]
pub struct DepthGrid {
    pub width: u32,
    pub height: u32,
    /// Row-major, `width * height` values in the model's own scale.
    pub values: Vec<f32>,
}

impl DepthGrid {
    /// Min–max normalized copy in 0.0..1.0 (0 = nearest, 1 = farthest).
    /// A flat grid (min == max) maps honestly to all-zeros with `flat: true`.
    pub fn normalized_relative(&self) -> (Vec<f32>, bool) {
        let min = self
            .values
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max = self
            .values
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let span = max - min;
        if span <= f32::EPSILON {
            return (vec![0.0; self.values.len()], true);
        }
        (
            self.values
                .iter()
                .map(|&v| ((v - min) / span).clamp(0.0, 1.0))
                .collect(),
            false,
        )
    }

    /// Upsample the grid to `dst_w × dst_h` (e.g. frame resolution).
    pub fn upsampled(&self, dst_w: u32, dst_h: u32) -> Vec<f32> {
        bilinear_upsample(&self.values, self.width, self.height, dst_w, dst_h)
    }
}

/// A loaded depth session (one per thread; `&mut self` inference).
pub struct DepthEstimator {
    _env: ort::environment::Environment,
    session: Option<Session>,
}

impl DepthEstimator {
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
            let path = ensure_depth_model()?;
            let session = Session::builder(&self._env)
                .and_then(|mut b| b.commit_from_file(&path))
                .map_err(|error| {
                    format!(
                        "Could not load the depth model at {}: {error}",
                        path.display()
                    )
                })?;
            self.session = Some(session);
        }
        self.session
            .as_mut()
            .ok_or_else(|| "Depth session missing after load".to_owned())
    }

    /// Estimate relative depth for one RGB8 frame. The returned grid is at
    /// the model's 518×518 input size (use `upsampled`/`normalized_relative`
    /// to adapt it); no metric units are implied.
    pub fn depth_frame(&mut self, frame: &SampledFrame) -> Result<DepthGrid, String> {
        let expected = frame.width as usize * frame.height as usize * 3;
        if frame.rgb.len() != expected {
            return Err(format!(
                "Frame RGB payload is {} bytes, expected {expected} for {}×{}",
                frame.rgb.len(),
                frame.height,
                frame.width
            ));
        }
        let size = DEPTH_INPUT_SIZE;
        let resized =
            bilinear_resize_rgb(&frame.rgb, frame.width, frame.height, size, size);
        let input = normalize_imagenet(&resized);
        let tensor = Tensor::from_array((
            vec![1_i64, 3_i64, size as i64, size as i64],
            input,
        ))
        .map_err(|error| format!("Could not build the depth input tensor: {error}"))?;
        let session = self.session()?;
        // The export declares `predicted_depth` alone; check declared names
        // BEFORE running so a renamed export fails loudly here, not by
        // panicking on a missing output key.
        let declared: Vec<String> = session
            .outputs()
            .iter()
            .map(|o| o.name().to_owned())
            .collect();
        let expected_name = "predicted_depth";
        if !declared.iter().any(|n| n == expected_name) {
            return Err(format!(
                "Depth model declares outputs {declared:?}, expected a `{expected_name}` plane"
            ));
        }
        let outputs = session
            .run(ort::inputs!["pixel_values" => tensor])
            .map_err(|error| format!("Depth inference failed: {error}"))?;
        let (shape, values) = outputs[expected_name]
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("Depth output unreadable: {error}"))?;
        let dims: Vec<i64> = shape.as_ref().to_vec();
        let expected_dims = [1_i64, size as i64, size as i64];
        if dims != expected_dims && dims != [1, size as i64 - 1, size as i64 - 1] {
            return Err(format!(
                "Depth output shape {dims:?} is not the {expected_dims:?} grid"
            ));
        }
        if values.len() < (size as usize) * (size as usize) {
            return Err(format!(
                "Depth output holds {} values, expected at least {}",
                values.len(),
                size as usize * size as usize
            ));
        }
        // All-finite check: a quantized model gone wrong surfaces as NaN.
        if values.iter().any(|v| !v.is_finite()) {
            return Err("Depth model produced non-finite values".into());
        }
        Ok(DepthGrid {
            width: size,
            height: size,
            values: values[..(size as usize) * (size as usize)].to_vec(),
        })
    }
}

/// CHW layout with the pinned DPTImageProcessor scalings.
fn normalize_imagenet(resized: &[u8]) -> Vec<f32> {
    const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
    const STD: [f32; 3] = [0.229, 0.224, 0.225];
    let pixels = resized.len() / 3;
    let mut out = Vec::with_capacity(pixels * 3);
    for channel in 0..3 {
        for pixel in 0..pixels {
            let value = resized[pixel * 3 + channel] as f32 / 255.0;
            out.push((value - MEAN[channel]) / STD[channel]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_checksum_is_well_formed() {
        assert_eq!(MODEL.sha256.len(), 64);
        assert!(MODEL.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(MODEL.file.contains("quantized"));
        assert!(MODEL.size > 20_000_000);
    }

    #[test]
    fn depth_url_pins_the_revision() {
        assert!(MODEL.url().contains(DEPTH_MODEL_REVISION));
        assert!(MODEL.url().contains("depth-anything-v2-small"));
        assert!(MODEL.url().ends_with("/onnx/model_quantized.onnx"));
    }

    #[test]
    fn depth_grid_normalization_is_relative_and_flags_flat() {
        let grid = DepthGrid {
            width: 2,
            height: 2,
            values: vec![2.0, 4.0, 6.0, 8.0],
        };
        let (normalized, flat) = grid.normalized_relative();
        assert!(!flat);
        assert_eq!(normalized, vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]);
        // Raw export ordering: larger = farther stays preserved (monotone).
        let flat_grid = DepthGrid {
            width: 1,
            height: 2,
            values: vec![3.0, 3.0],
        };
        let (normalized, flat) = flat_grid.normalized_relative();
        assert!(flat, "constant depth is reported as flat, not faked");
        assert_eq!(normalized, vec![0.0, 0.0]);
    }

    #[test]
    fn depth_grid_upsample_matches_frame() {
        let grid = DepthGrid {
            width: 2,
            height: 2,
            values: vec![0.0, 1.0, 1.0, 2.0],
        };
        let up = grid.upsampled(4, 4);
        assert_eq!(up.len(), 16);
        assert!(up[0] < up[5], "near corner stays nearer than center");
        assert!(up[15] > up[5], "far corner stays farther than center");
    }
}
