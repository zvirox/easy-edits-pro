// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! SigLIP embedding model catalogue and fetch-on-first-run downloads.
//!
//! Same policy as the whisper catalogue (`models.rs`): weights are never
//! bundled; they download on first use from the canonical upstream host at a
//! pinned revision and are SHA-256-verified before installation.
//!
//! Provenance (recorded in `docs/model-licences.md` in this change):
//! - Upstream weights: `google/siglip-base-patch16-224` @
//!   `7fd15f0689c79d79e38b1c2e2e2370a7bf2761ed` — Apache-2.0 (Google).
//! - ONNX exports: `Xenova/siglip-base-patch16-224` (transformers.js org,
//!   non-gated mirror of the Google weights) @
//!   `4649052661e53c7000355844105f8a1792088239` — same Apache-2.0 grant via
//!   the upstream model card. We use the int8-quantized vision/text exports
//!   and the repo's tokenizer.json.
//!
//! Preprocessing is pinned to the repo's `preprocessor_config.json` at the
//! same revision: 224×224, bicubic resample, rescale 1/255, normalize with
//! mean 0.5 / std 0.5 per channel.

use crate::models::{fetch_verified, models_dir, sha256_file, verified_local_file};
use std::path::{Path, PathBuf};

/// Pinned revision of the ONNX export repo (checksums recorded 2026-11-06).
pub const SIGLIP_REVISION: &str = "4649052661e53c7000355844105f8a1792088239";
const SIGLIP_HOST: &str = "https://huggingface.co/Xenova/siglip-base-patch16-224/resolve";

#[derive(Clone, Copy)]
struct SiglipFileSpec {
    /// Path within the repo (`onnx/…` for model weights; the tokenizer sits
    /// at the repo root — verified against the pinned tree).
    remote_path: &'static str,
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

const VISION_MODEL: SiglipFileSpec = SiglipFileSpec {
    remote_path: "onnx/vision_model_quantized.onnx",
    file: "vision_model_quantized.onnx",
    size: 99_499_129,
    sha256: "ef14a954f3d57e1806666432bd9785004c1dc27100aa260eee0cb0f10a5de058",
};
const TEXT_MODEL: SiglipFileSpec = SiglipFileSpec {
    remote_path: "onnx/text_model_quantized.onnx",
    file: "text_model_quantized.onnx",
    size: 111_475_220,
    sha256: "ad0329b1f35acc66d8953ff2559ce358da8eb0a7011794cf951523d63a4dbce2",
};
const TOKENIZER: SiglipFileSpec = SiglipFileSpec {
    remote_path: "tokenizer.json",
    file: "tokenizer.json",
    size: 2_398_744,
    sha256: "4a17c975210be5ab4c36b47d8dae4eefb866dbfb1e676e394aad85dc30a3ae08",
};

impl SiglipFileSpec {
    fn url(self) -> String {
        format!("{SIGLIP_HOST}/{SIGLIP_REVISION}/{}", self.remote_path)
    }
}

/// Directory holding the SigLIP assets: `<models>/siglip/`.
pub fn siglip_dir() -> Result<PathBuf, String> {
    let dir = models_dir()?.join("siglip");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create the siglip model directory: {error}"))?;
    Ok(dir)
}

/// Which encoder weights to ensure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiglipEncoder {
    Vision,
    Text,
}

impl SiglipEncoder {
    fn spec(self) -> SiglipFileSpec {
        match self {
            Self::Vision => VISION_MODEL,
            Self::Text => TEXT_MODEL,
        }
    }
}

/// Resolve an encoder's ONNX file to a verified local path, downloading on
/// first use. Errors name the file and URL so offline machines can place it
/// manually.
pub fn ensure_siglip_encoder(encoder: SiglipEncoder) -> Result<PathBuf, String> {
    ensure_siglip_file(encoder.spec(), "SigLIP encoder model")
}

/// Resolve the tokenizer to a verified local path (same policy).
pub fn ensure_siglip_tokenizer() -> Result<PathBuf, String> {
    ensure_siglip_file(TOKENIZER, "SigLIP tokenizer")
}

fn ensure_siglip_file(spec: SiglipFileSpec, label: &'static str) -> Result<PathBuf, String> {
    let path = siglip_dir()?.join(spec.file);
    if verified_local_file(&path, label, spec.size, spec.sha256, &spec.url()).is_some() {
        fetch_verified(&spec.url(), &path, spec.sha256, label, Some(spec.size))?;
        if let Some(problem) =
            verified_local_file(&path, label, spec.size, spec.sha256, &spec.url())
        {
            return Err(problem);
        }
    }
    Ok(path)
}

/// SHA-256 of an arbitrary file (used by the e2e test to double-check the
/// tokenizer it loads; also keeps [`sha256_file`] exercised outside models).
pub fn file_sha256(path: &Path) -> Result<String, String> {
    sha256_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siglip_checksums_are_well_formed() {
        for spec in [VISION_MODEL, TEXT_MODEL, TOKENIZER] {
            assert_eq!(spec.sha256.len(), 64, "sha256 length for {}", spec.file);
            assert!(spec.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(spec.size > 1_000_000, "sane size for {}", spec.file);
        }
    }

    #[test]
    fn siglip_urls_pin_the_revision() {
        assert!(VISION_MODEL.url().contains(SIGLIP_REVISION));
        assert!(VISION_MODEL.url().starts_with(SIGLIP_HOST));
        assert!(TOKENIZER.url().ends_with("/tokenizer.json"));
    }
}
