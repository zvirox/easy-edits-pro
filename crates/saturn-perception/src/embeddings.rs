// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! SigLIP image/text embeddings for semantic media search (Packet 2.2).
//!
//! Runs the pinned ONNX exports (see `siglip_model.rs`) through ONNX Runtime
//! (`ort`, CPU EP — deterministic, no GPU providers configured). Public
//! surface: [`SiglipEmbedder::embed_image_rgb`] / [`SiglipEmbedder::embed_text`]
//! produce L2-normalized vectors in a shared projection space, and
//! [`cosine_similarity`] compares them.
//!
//! Honesty notes: SigLIP is a zero-shot image *classifier* — the similarity
//! it reports is cosine similarity in its learned space, not human relevance.
//! Both pinned exports emit an on-graph pooled head (`pooler_output`,
//! [batch, 768]): the vision encoder pools over patch tokens, the text
//! encoder over the EOS token, exactly as HF's SigLIP implementation does.
//! We take that head directly and L2-normalize (normalization does not
//! change cosine ranking); no other post-processing is applied.

use crate::shot_detect::SampledFrame;
use crate::siglip_model::{ensure_siglip_encoder, ensure_siglip_tokenizer, SiglipEncoder};
use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;

/// Embedding dimension of `siglip-base-patch16-224` (`projection_dim`/hidden).
pub const SIGLIP_EMBED_DIM: usize = 768;

const IMAGE_SIZE: u32 = 224;

/// A loaded SigLIP embedder. Sessions take `&mut self` for inference (ort
/// requirement), so one embedder is bound to one indexing/query thread.
pub struct SiglipEmbedder {
    _env: Environment,
    vision: Option<Session>,
    text: Option<Session>,
    tokenizer: Option<tokenizers::Tokenizer>,
}

impl SiglipEmbedder {
    /// Verify (downloading on first use) and load the two encoder sessions
    /// and the tokenizer. Only the encoders actually used need loading.
    pub fn load() -> Result<Self, String> {
        let env = ort::init()
            .with_name("easy-edits-pro-perception")
            .build()
            .map_err(|error| format!("Could not initialize ONNX Runtime: {error}"))?;
        Ok(Self {
            _env: env,
            vision: None,
            text: None,
            tokenizer: None,
        })
    }

    fn vision_session(&mut self) -> Result<&mut Session, String> {
        if self.vision.is_none() {
            let path = ensure_siglip_encoder(SiglipEncoder::Vision)?;
            let session = Session::builder(&self._env)
                .and_then(|mut b| b.commit_from_file(&path))
                .map_err(|error| {
                    format!("Could not load the SigLIP vision model at {}: {error}", path.display())
                })?;
            self.vision = Some(session);
        }
        self.vision
            .as_mut()
            .ok_or_else(|| "SigLIP vision session missing after load".to_owned())
    }

    fn text_session(&mut self) -> Result<&mut Session, String> {
        if self.text.is_none() {
            let path = ensure_siglip_encoder(SiglipEncoder::Text)?;
            let session = Session::builder(&self._env)
                .and_then(|mut b| b.commit_from_file(&path))
                .map_err(|error| {
                    format!("Could not load the SigLIP text model at {}: {error}", path.display())
                })?;
            self.text = Some(session);
        }
        self.text
            .as_mut()
            .ok_or_else(|| "SigLIP text session missing after load".to_owned())
    }

    fn tokenizer(&mut self) -> Result<&mut tokenizers::Tokenizer, String> {
        if self.tokenizer.is_none() {
            let path = ensure_siglip_tokenizer()?;
            let tokenizer = tokenizers::Tokenizer::from_file(&path)
                .map_err(|error| format!("Could not load the SigLIP tokenizer: {error}"))?;
            self.tokenizer = Some(tokenizer);
        }
        self.tokenizer
            .as_mut()
            .ok_or_else(|| "SigLIP tokenizer missing after load".to_owned())
    }

    /// Embed one RGB8 frame sampled at `frame.tick` (the tick only rides
    /// along for callers; it is not part of the vector).
    pub fn embed_frame(
        &mut self,
        frame: &SampledFrame,
    ) -> Result<Vec<f32>, String> {
        let expected = frame.width as usize * frame.height as usize * 3;
        if frame.rgb.len() != expected {
            return Err(format!(
                "Frame RGB payload is {} bytes, expected {expected} for {}×{}",
                frame.rgb.len(),
                frame.height,
                frame.width
            ));
        }
        let pixel_values = preprocess_image(&frame.rgb, frame.width, frame.height);
        let tensor = Tensor::from_array((
            vec![1_i64, 3_i64, IMAGE_SIZE as i64, IMAGE_SIZE as i64],
            pixel_values,
        ))
        .map_err(|error| format!("Could not build the vision input tensor: {error}"))?;
        let session = self.vision_session()?;
        let outputs = session
            .run(ort::inputs![tensor])
            .map_err(|error| format!("SigLIP vision inference failed: {error}"))?;
        // The export emits `pooler_output` [1, 768] (mean pooling over patch
        // tokens, done on-graph) alongside `last_hidden_state` [1, 196, 768].
        let pooled_2d =
            extract_pooled_output(&outputs, "vision")?;
        let mut pooled = pooled_2d.to_vec();
        l2_normalize(&mut pooled);
        Ok(pooled)
    }

    /// Embed a natural-language query for cross-modal cosine search.
    pub fn embed_query(&mut self, text: &str) -> Result<Vec<f32>, String> {
        let query = text.trim();
        if query.is_empty() {
            return Err("Text query is empty".into());
        }
        let tokenizer = self.tokenizer()?;
        let encoding = tokenizer
            .encode(query, true)
            .map_err(|error| format!("Could not tokenize the query: {error}"))?;
        let ids: Vec<i64> = encoding
            .get_ids()
            .iter()
            .map(|&id| id as i64)
            .collect();
        if ids.is_empty() {
            return Err("Text query produced no tokens".into());
        }
        self.run_text_encoder(ids)
    }

    fn run_text_encoder(&mut self, ids: Vec<i64>) -> Result<Vec<f32>, String> {
        let length = ids.len() as i64;
        let id_tensor = Tensor::from_array((vec![1_i64, length], ids.clone()))
            .map_err(|error| format!("Could not build the id tensor: {error}"))?;
        let session = self.text_session()?;
        // The pinned text export declares a single input, `input_ids` (its
        // EOS pooling runs on-graph); match names defensively anyway.
        let declared: Vec<String> = session.inputs().iter().map(|o| o.name().to_owned()).collect();
        if declared.is_empty() {
            return Err("SigLIP text model declares no inputs".into());
        }
        let ids_name = declared
            .iter()
            .find(|n| n.contains("input_ids"))
            .cloned()
            .unwrap_or_else(|| declared[0].clone());
        let outputs = session
            .run(ort::inputs![ids_name => id_tensor])
            .map_err(|error| format!("SigLIP text inference failed: {error}"))?;
        // The export emits `pooler_output` [1, 768] (EOS-token pooling, done
        // on-graph) alongside `last_hidden_state` [1, L, 768].
        let pooled_2d =
            extract_pooled_output(&outputs, "text")?;
        let mut pooled = pooled_2d.to_vec();
        l2_normalize(&mut pooled);
        Ok(pooled)
    }
}

/// Extract a [batch, SIGLIP_EMBED_DIM] pooled vector from the session
/// outputs, preferring `pooler_output` and falling back to the first output
/// shaped [batch, dim]. Returns the 768-value row of batch item 0.
fn extract_pooled_output<'a>(
    outputs: &'a ort::session::SessionOutputs<'_>,
    label: &str,
) -> Result<&'a [f32], String> {
    for name in ["pooler_output"] {
        if let Ok((shape, data)) = outputs[name].try_extract_tensor::<f32>() {
            let dims: Vec<i64> = shape.as_ref().to_vec();
            if dims.len() == 2 && dims[1] as usize == SIGLIP_EMBED_DIM && dims[0] >= 1 {
                if data.len() < SIGLIP_EMBED_DIM {
                    return Err(format!(
                        "SigLIP {label} pooled output holds {} values, expected at least {SIGLIP_EMBED_DIM}",
                        data.len()
                    ));
                }
                return Ok(&data[..SIGLIP_EMBED_DIM]);
            }
        }
    }
    // Not found by name: fall back to the first output when it is a
    // [batch, dim] matrix (handles differently named exports).
    let (shape, data) = outputs[0usize]
        .try_extract_tensor::<f32>()
        .map_err(|error| {
            format!("SigLIP {label} output unreadable and no `pooler_output` found: {error}")
        })?;
    let dims: Vec<i64> = shape.as_ref().to_vec();
    if dims.len() == 2 && dims[1] as usize == SIGLIP_EMBED_DIM && dims[0] >= 1 {
        if data.len() < SIGLIP_EMBED_DIM {
            return Err(format!(
                "SigLIP {label} pooled output holds {} values, expected at least {SIGLIP_EMBED_DIM}",
                data.len()
            ));
        }
        return Ok(&data[..SIGLIP_EMBED_DIM]);
    }
    Err(format!(
        "SigLIP {label} has no [batch, {SIGLIP_EMBED_DIM}] pooled output (outputs: {dims:?})"
    ))
}

/// Cosine similarity in the SigLIP space, −1..1. Zero-norm inputs yield 0.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (dot, norm_a, norm_b) = a
        .iter()
        .zip(b.iter())
        .fold((0.0_f64, 0.0_f64, 0.0_f64), |(d, na, nb), (x, y)| {
            (d + *x as f64 * *y as f64, na + *x as f64 * *x as f64, nb + *y as f64 * *y as f64)
        });
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b).sqrt()) as f32
}

fn l2_normalize(vector: &mut [f32]) {
    let norm: f64 = vector.iter().map(|v| *v as f64 * *v as f64).sum::<f64>().sqrt();
    if norm > f64::EPSILON {
        for slot in vector.iter_mut() {
            *slot = (*slot as f64 / norm) as f32;
        }
    }
}

/// Preprocess RGB8 into the pinned SigLIP input: Catmull-Rom bicubic resize
/// to 224×224 (matching `resample: 3`), rescale 1/255, normalize with mean
/// 0.5 / std 0.5 per channel, channels-first.
pub(crate) fn preprocess_image(rgb: &[u8], width: u32, height: u32) -> Vec<f32> {
    let (w, h) = (width as usize, height as usize);
    let (out_w, out_h) = (IMAGE_SIZE as usize, IMAGE_SIZE as usize);
    let mut resized = vec![0_u8; out_w * out_h * 3];
    if (w, h) == (out_w, out_h) {
        resized.copy_from_slice(rgb);
    } else {
        resize_bicubic(rgb, w, h, &mut resized, out_w, out_h);
    }
    let mut out = Vec::with_capacity(3 * out_w * out_h);
    // CHW: plane R, then G, then B. mean=0.5/std=0.5 ⇒ (v/255−0.5)·2.
    for channel in 0..3 {
        for pixel in resized.chunks_exact(3) {
            out.push((pixel[channel] as f32 / 255.0 - 0.5) * 2.0);
        }
    }
    out
}

/// Catmull-Rom bicubic resample (PIL `BICUBIC`, a = −0.5), clamped edges.
fn resize_bicubic(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    dst: &mut [u8],
    dst_w: usize,
    dst_h: usize,
) {
    let pixel = |px: usize, py: usize, c: usize| -> f64 {
        src[((py.min(src_h - 1) * src_w + px.min(src_w - 1)) * 3) + c] as f64
    };
    // Map destination pixel center to source coordinates.
    let bc = |t: f64, ratio: f64| -> f64 { (t + 0.5) * ratio - 0.5 };
    let x_ratio = src_w as f64 / dst_w as f64;
    let y_ratio = src_h as f64 / dst_h as f64;
    for dy in 0..dst_h {
        let src_y = bc(dy as f64, y_ratio).max(0.0);
        let fy = src_y.floor() as isize;
        let ty = src_y - fy as f64;
        for dx in 0..dst_w {
            let src_x = bc(dx as f64, x_ratio).max(0.0);
            let fx = src_x.floor() as isize;
            let tx = src_x - fx as f64;
            let di = (dy * dst_w + dx) * 3;
            for c in 0..3 {
                let (mut acc, mut wsum) = (0.0_f64, 0.0_f64);
                for my in -1_isize..=2 {
                    let wy = bicubic_weight(my as f64 - 1.0 - ty);
                    for mx in -1_isize..=2 {
                        let wx = bicubic_weight(mx as f64 - 1.0 - tx);
                        let weight = wx * wy;
                        let px = (fx + mx).clamp(0, src_w as isize - 1) as usize;
                        let py = (fy + my).clamp(0, src_h as isize - 1) as usize;
                        acc += pixel(px, py, c) * weight;
                        wsum += weight;
                    }
                }
                let value = if wsum.abs() < 1e-12 {
                    0.0
                } else {
                    (acc / wsum).round().clamp(0.0, 255.0)
                };
                dst[di + c] = value as u8;
            }
        }
    }
}

/// Bicubic kernel weight (a = −0.5), evaluated at the tap offset.
fn bicubic_weight(t: f64) -> f64 {
    let t = t.abs();
    if t <= 1.0 {
        (1.5 * t - 2.5) * t * t + 1.0
    } else if t < 2.0 {
        -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, (r, g, b): (u8, u8, u8)) -> SampledFrame {
        SampledFrame {
            tick: saturn_core::Tick(0),
            width: w,
            height: h,
            rgb: vec![r, g, b].repeat((w * h) as usize),
        }
    }

    #[test]
    fn preprocess_solid_image_is_centered_normalized() {
        let frame = solid(448, 448, (128, 128, 128));
        let out = preprocess_image(&frame.rgb, 448, 448);
        assert_eq!(out.len(), 3 * 224 * 224);
        let neuron = out[100_000];
        // (128/255 − 0.5)·2 ≈ 0.0039
        assert!((neuron - (128.0 / 255.0 - 0.5) * 2.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_basics() {
        let mut a = vec![0.0; SIGLIP_EMBED_DIM];
        a[0] = 1.0;
        let mut b = vec![0.0; SIGLIP_EMBED_DIM];
        b[0] = 2.0;
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        b[1] = 1.0; // not parallel anymore
        let sim = cosine_similarity(&a, &b);
        assert!(sim < 1.0 && sim > 0.0);
        assert_eq!(cosine_similarity(&a, &a[..6]), 0.0, "length mismatch");
    }

    #[test]
    fn bicubic_downscale_preserves_mean_roughly() {
        // 32×32 uniform-224 → 16×16: every output pixel must stay ≈224.
        let src = vec![224_u8; 32 * 32 * 3];
        let mut dst = vec![0_u8; 16 * 16 * 3];
        resize_bicubic(&src, 32, 32, &mut dst, 16, 16);
        assert!(dst.iter().all(|v| (i32::from(*v) - 224).abs() <= 1));
    }
}
