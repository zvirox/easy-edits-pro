// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Offline local neural perception for Easy Edits Pro (Phase 2).
//!
//! Packet 2.1: real Whisper transcription (whisper.cpp via `whisper-rs`)
//! with word-level tick alignment, plus script-to-take sync matching. Heavy
//! inference lives here — the engine core stays inference-free and receives
//! finished data through command parameters.
//!
//! Honesty model (plan §4.4): every reported word/segment is a *direct*
//! transcription output with measured timestamps; no invented confidence
//! inflation, no speaker labels (diarization is reviewed as its own packet).

pub mod color_metrics;
pub mod match_cut;
pub mod depth_ort;
pub mod embeddings;
pub mod index_db;
pub mod matting_ort;
pub mod media_search;
pub mod motion_track;
pub mod models;
pub mod shot_detect;
pub mod siglip_model;
pub mod whisper;

pub use color_metrics::ciede2000;
pub use depth_ort::{DepthEstimator, DepthGrid};
pub use embeddings::{cosine_similarity, SiglipEmbedder, SIGLIP_EMBED_DIM};
pub use index_db::{SearchHit, SemanticIndex};
pub use matting_ort::{feather_alpha, MatteSmoother, ModNetMatting};
pub use media_search::{
    decode_cut_frames, embed_media_frames, media_sample_ticks, query_media_index, FrameEmbedder,
    MediaIndexOutcome,
};
pub use motion_track::{
    apply_homography, downscale_frame_rgb, homography_from_point_pairs, ransac_homography,
    track_patch_displacement, track_region_frames, PatchTrackPoint, RansacOutcome, TrackedRegion,
};
pub use models::{ensure_model, verify_model, WhisperModel};
pub use shot_detect::{
    detect_shot_boundaries, evaluate_cut_continuity, CutContinuityScore, CutContext,
    SampledFrame, ShotBoundary, ShotDetectOptions,
};
pub use whisper::{transcribe_pcm, TranscribeOptions, Transcript, WHISPER_SAMPLE_RATE};
