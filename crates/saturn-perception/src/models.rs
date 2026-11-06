// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Whisper model catalogue and fetch-on-first-run download manager.
//!
//! Weights are **never bundled** (plan §10, `docs/model-licences.md`): they
//! download on first use from the canonical upstream host —
//! `huggingface.co/ggerganov/whisper.cpp` at a pinned revision — and are
//! verified against a SHA-256 recorded from the upstream LFS object before a
//! partially-written file is ever renamed into place. Offline machines get a
//! typed error naming the exact file to place manually.
//!
//! Licences: the models are MIT (OpenAI Whisper is MIT; the ggml conversions
//! in ggerganov/whisper.cpp are the same upstream licence). Recorded in
//! `docs/model-licences.md` in the same change.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Upstream repo revision the checksums were recorded against.
/// Re-verify on every model-table change (model-licences rule 4).
pub const UPSTREAM_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";
const UPSTREAM_HOST: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve";

/// A Whisper model size, matching the plan's supported list
/// (base/small/medium/large-v3-turbo) plus the tiny size used by tests and
/// low-resource machines. English-only variants are faster on en material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    TinyEn,
    Tiny,
    BaseEn,
    Base,
    SmallEn,
    Small,
    MediumEn,
    Medium,
    LargeV3Turbo,
}

/// Catalog entry: upstream file name, exact size, and the SHA-256 LFS object
/// id fetched from the pinned revision (HF API `tree?recursive`, 2026-11-06).
struct ModelSpec {
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

impl WhisperModel {
    /// ggml file name on the upstream host.
    pub fn file_name(self) -> &'static str {
        self.spec().file
    }

    pub fn exact_size(self) -> u64 {
        self.spec().size
    }

    pub fn sha256(self) -> &'static str {
        self.spec().sha256
    }

    /// Expected semantic name (`tiny`, `base.en`, `large-v3-turbo`) — the
    /// string the desktop options and config use.
    pub fn name(self) -> &'static str {
        match self {
            Self::TinyEn => "tiny.en",
            Self::Tiny => "tiny",
            Self::BaseEn => "base.en",
            Self::Base => "base",
            Self::SmallEn => "small.en",
            Self::Small => "small",
            Self::MediumEn => "medium.en",
            Self::Medium => "medium",
            Self::LargeV3Turbo => "large-v3-turbo",
        }
    }

    /// Parse the user-facing model name. Exact names map one-to-one;
    /// bare size names select the multilingual variant (`.en` twins are
    /// explicit); legacy `whisper-*` strings from the engine's old options
    /// default map to their English twins where the old default said so.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "tiny.en" | "whisper-tiny-en" => Some(Self::TinyEn),
            "tiny" | "whisper-tiny" => Some(Self::Tiny),
            "base.en" | "whisper-base-en" => Some(Self::BaseEn),
            "base" | "whisper-base" => Some(Self::Base),
            "small.en" | "whisper-small-en" => Some(Self::SmallEn),
            "small" | "whisper-small" => Some(Self::Small),
            "medium.en" | "whisper-medium-en" => Some(Self::MediumEn),
            "medium" | "whisper-medium" => Some(Self::Medium),
            "large-v3-turbo" | "large-v3" | "large" | "whisper-large-v3-turbo" => {
                Some(Self::LargeV3Turbo)
            }
            _ => None,
        }
    }

    fn spec(self) -> ModelSpec {
        // SHA-256 LFS oids from the pinned revision — verify on update.
        const TINY_EN: ModelSpec = ModelSpec {
            file: "ggml-tiny.en.bin",
            size: 77_704_715,
            sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
        };
        match self {
            Self::TinyEn => TINY_EN,
            Self::Tiny => ModelSpec {
                file: "ggml-tiny.bin",
                size: 77_691_713,
                sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
            },
            Self::BaseEn => ModelSpec {
                file: "ggml-base.en.bin",
                size: 147_964_211,
                sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
            },
            Self::Base => ModelSpec {
                file: "ggml-base.bin",
                size: 147_951_465,
                sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
            },
            Self::SmallEn => ModelSpec {
                file: "ggml-small.en.bin",
                size: 487_614_201,
                sha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
            },
            Self::Small => ModelSpec {
                file: "ggml-small.bin",
                size: 487_601_967,
                sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
            },
            Self::MediumEn => ModelSpec {
                file: "ggml-medium.en.bin",
                size: 1_533_774_781,
                sha256: "cc37e93478338ec7700281a7ac30a10128929eb8f427dda2e865faa8f6da4356",
            },
            Self::Medium => ModelSpec {
                file: "ggml-medium.bin",
                size: 1_533_763_059,
                sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
            },
            Self::LargeV3Turbo => ModelSpec {
                file: "ggml-large-v3-turbo.bin",
                size: 1_624_555_275,
                sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
            },
        }
    }

    fn url(self) -> String {
        format!(
            "{UPSTREAM_HOST}/{UPSTREAM_REVISION}/{}",
            self.spec().file
        )
    }
}

/// Root directory for cached models (`SATURN_MODELS_DIR` override, else
/// XDG data home), `<root>/whisper/<file>`.
pub fn models_dir() -> Result<PathBuf, String> {
    let root = if let Some(overridden) = std::env::var_os("SATURN_MODELS_DIR") {
        PathBuf::from(overridden)
    } else if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local").join("share")
    } else {
        return Err(
            "No SATURN_MODELS_DIR, XDG_DATA_HOME or HOME: cannot resolve the models directory"
                .to_owned(),
        );
    };
    let dir = root.join("easy-edits-pro").join("models").join("whisper");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create the models directory: {error}"))?;
    Ok(dir)
}

/// Resolve a model to a local path, downloading on first use (fetch-on-first-
/// run, plan-review §92). Verifies the SHA-256 on both fresh downloads and
/// pre-existing files; corrupt or truncated files are re-downloaded.
pub fn ensure_model(model: WhisperModel) -> Result<PathBuf, String> {
    let dir = models_dir()?;
    let path = dir.join(model.file_name());
    if verify_model(&path, model).is_some() {
        // Missing, truncated or corrupt: (re)download from upstream.
        download_model(&path, model)?;
        if let Some(problem) = verify_model(&path, model) {
            return Err(problem);
        }
    }
    Ok(path)
}

/// `Some(problem)` when the file at `path` is not a valid copy of `model`
/// (missing, wrong size — e.g. a partial download — or a SHA-256 mismatch).
pub fn verify_model(path: &Path, model: WhisperModel) -> Option<String> {
    match std::fs::metadata(path) {
        Err(_) => Some(format!(
            "Whisper model {} is not downloaded (expected at {}; it fetches on first use \
             from {} — offline: place the file there manually)",
            model.name(),
            path.display(),
            model.url()
        )),
        Ok(metadata) => {
            if metadata.len() != model.exact_size() {
                return Some(format!(
                    "Whisper model {} at {} is {} bytes, expected {} (delete it and retry to \
                     re-download from {})",
                    model.name(),
                    path.display(),
                    metadata.len(),
                    model.exact_size(),
                    model.url()
                ));
            }
            let mismatch = |actual: &str| {
                Some(format!(
                    "Whisper model {} at {} failed SHA-256 verification (got {actual}, \
                     expected {}); deleted and re-download needed from {}",
                    model.name(),
                    path.display(),
                    model.sha256(),
                    model.url()
                ))
            };
            let actual = match sha256_file(path) {
                Ok(digest) => digest,
                Err(error) => return mismatch(&format!("unreadable: {error}")),
            };
            if actual != model.sha256() {
                return mismatch(&actual);
            }
            None
        }
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("Could not open: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not read: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn download_model(path: &Path, model: WhisperModel) -> Result<(), String> {
    let partial = path.with_extension("part");
    let tls_connector = native_tls::TlsConnector::new()
        .map_err(|error| format!("Could not set up TLS for the model download: {error}"))?;
    let agent = ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(tls_connector))
        .build();
    let response = agent
        .get(&model.url())
        .timeout(std::time::Duration::from_secs(600))
        .call()
        .map_err(|error| {
            format!(
                "Could not download model {} from {}: {error}",
                model.name(),
                model.url()
            )
        })?;
    let mut reader = response.into_reader();
    let mut hasher = Sha256::new();
    let mut file_writer = std::fs::File::create(&partial).map_err(|error| {
        format!("Could not create {}: {error}", partial.display())
    })?;
    let mut hashing_reader = HashingReader {
        inner: &mut reader,
        hasher: &mut hasher,
    };
    std::io::copy(&mut hashing_reader, &mut file_writer)
        .and_then(|_| std::io::Write::flush(&mut file_writer))
    .map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        format!("Download of model {} failed: {error}", model.name())
    })?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != model.sha256() {
        let _ = std::fs::remove_file(&partial);
        return Err(format!(
            "Downloaded model {} failed SHA-256 verification (got {actual}, expected {}); \
             refusing to install — the upstream revision or mirror may have changed",
            model.name(),
            model.sha256()
        ));
    }
    std::fs::rename(&partial, path).map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        format!("Could not install the model download: {error}")
    })?;
    Ok(())
}

/// Locality wrapper so the download hash matches the bytes on disk.
struct HashingReader<'a> {
    inner: &'a mut dyn Read,
    hasher: &'a mut Sha256,
}

impl Read for HashingReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        if read > 0 {
            self.hasher.update(&buf[..read]);
        }
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_names_parse_and_map_one_to_one() {
        for model in [
            WhisperModel::TinyEn,
            WhisperModel::Tiny,
            WhisperModel::BaseEn,
            WhisperModel::Base,
            WhisperModel::SmallEn,
            WhisperModel::Small,
            WhisperModel::MediumEn,
            WhisperModel::Medium,
            WhisperModel::LargeV3Turbo,
        ] {
            assert_eq!(
                WhisperModel::parse(model.name()),
                Some(model),
                "round-trip {}",
                model.name()
            );
        }
    }

    #[test]
    fn catalog_checksums_are_well_formed() {
        for model in [
            WhisperModel::TinyEn,
            WhisperModel::Tiny,
            WhisperModel::BaseEn,
            WhisperModel::Base,
            WhisperModel::SmallEn,
            WhisperModel::Small,
            WhisperModel::MediumEn,
            WhisperModel::Medium,
            WhisperModel::LargeV3Turbo,
        ] {
            let digest = model.sha256();
            assert_eq!(digest.len(), 64, "sha256 length for {}", model.name());
            assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(model.exact_size() > 1_000_000, "sane size for {}", model.name());
            assert!(model.file_name().starts_with("ggml-"));
        }
    }

    #[test]
    fn urls_pin_the_revision() {
        assert!(WhisperModel::BaseEn
            .url()
            .contains(UPSTREAM_REVISION));
        assert!(WhisperModel::BaseEn.url().starts_with(UPSTREAM_HOST));
    }
}
