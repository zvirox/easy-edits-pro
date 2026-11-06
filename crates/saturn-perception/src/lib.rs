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

pub mod models;
pub mod whisper;

pub use models::{ensure_model, verify_model, WhisperModel};
pub use whisper::{transcribe_pcm, TranscribeOptions, Transcript, WHISPER_SAMPLE_RATE};
