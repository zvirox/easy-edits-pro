// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Real offline Whisper transcription over whisper.cpp (Packet 2.1).
//!
//! Word-level timestamps come from whisper.cpp's token-level timestamps
//! (`token_timestamps` + DTW), grouped into words on BPE word-start
//! boundaries, then mapped to project ticks. Speaker-turn diarization is
//! deliberately **not** implemented here (plan-review §91: Whisper has no
//! diarization; it needs a separate model — deferred to its own packet), so
//! every segment reports `speaker: None` rather than an invented label.
//!
//! Transcription itself needs a downloaded model (see `models.rs`); the
//! token→word→segment mapping below is pure and unit-tested without one.

use saturn_core::speech::{is_filler_word, SpeechSegment, SpeechWord};
use saturn_core::time::TICKS_PER_SECOND;
use saturn_core::Tick;

use crate::models::{ensure_model, WhisperModel};

/// Whisper's native analysis rate.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;
/// Audio length ceiling per transcription call (≈ 2 h; typed error beyond).
pub const MAX_TRANSCRIBE_SECONDS: f64 = 7_200.0;

/// Options for one transcription run.
#[derive(Clone, Debug)]
pub struct TranscribeOptions {
    /// ISO language code (`"en"`, `"de"`, …) or `None` for auto-detect.
    /// English-only models ignore this (they are en by construction).
    pub language: Option<String>,
    /// Mark hesitation fillers on words (`is_filler`) using the engine's
    /// shared filler list so ripple-delete-fillers acts on real detections.
    pub detect_fillers: bool,
    /// Initial prompt to bias domain vocabulary (whisper `initial_prompt`).
    pub prompt: Option<String>,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            language: Some("en".to_owned()),
            detect_fillers: true,
            prompt: None,
        }
    }
}

/// Result of one transcription run — engine-ready segments plus provenance.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub segments: Vec<SpeechSegment>,
    /// Model actually used (`base.en` …) — stated, not guessed.
    pub model: String,
    /// Detected/requested language Whisper reports for the audio.
    pub language: String,
    /// Seconds of audio actually transcribed.
    pub audio_seconds: f64,
}

/// One decoded token with its DTW timing (centisecond units, whisper.cpp
/// convention: ×10 = milliseconds). Pure struct so mapping is testable.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenTiming {
    pub text: String,
    pub start_cs: i64,
    pub end_cs: i64,
    /// Token probability (0..1).
    pub probability: f32,
    /// Special/format tokens (transcription markers) — skipped for words.
    pub special: bool,
}

/// Group a token stream into words: a new word begins at a token whose text
/// starts with a space (whisper.cpp BPE carries the leading space) or at the
/// ▁ sentencepiece marker; everything else (letters, punctuation) attaches to
/// the current word. Special tokens are dropped.
pub(crate) fn words_from_tokens(
    tokens: &[TokenTiming],
    options: &TranscribeOptions,
) -> Vec<SpeechWord> {
    let mut words: Vec<(String, i64, i64, Vec<f32>)> = Vec::new();
    for token in tokens {
        if token.special {
            continue;
        }
        let text = token.text.trim_start_matches('▁');
        let starts_word = token.text.starts_with(' ')
            || token.text.starts_with("▁")
            || words.is_empty();
        if text.trim().is_empty() {
            continue;
        }
        if starts_word {
            words.push((text.trim_start().to_owned(), token.start_cs, token.end_cs, vec![token.probability]));
        } else if let Some(current) = words.last_mut() {
            current.0.push_str(text);
            current.1 = current.1.min(token.start_cs);
            current.2 = current.2.max(token.end_cs);
            current.3.push(token.probability);
        }
    }
    words
        .into_iter()
        .map(|(word, start_cs, end_cs, probs)| {
            let mean_probability =
                probs.iter().sum::<f32>() / probs.len().max(1) as f32;
            let filler = options.detect_fillers && is_filler_word(&word, None);
            SpeechWord {
                word,
                start_ticks: tick_from_centiseconds(start_cs),
                end_ticks: tick_from_centiseconds(end_cs),
                confidence: (mean_probability.clamp(0.0, 1.0)) as f64,
                is_filler: filler,
            }
        })
        .collect()
}

/// whisper.cpp timestamps are centiseconds; project ticks are exact.
pub(crate) fn tick_from_centiseconds(cs: i64) -> Tick {
    // TICKS_PER_SECOND is divisible by 100, so this stays integer-exact.
    Tick(cs.saturating_mul(TICKS_PER_SECOND / 100))
}

/// Transcribe mono 16 kHz PCM (f32, −1..1) with a specific downloaded model.
pub fn transcribe_pcm(
    samples: &[f32],
    model: WhisperModel,
    options: &TranscribeOptions,
) -> Result<Transcript, String> {
    if samples.is_empty() {
        return Err("Whisper needs non-empty PCM".into());
    }
    let audio_seconds = samples.len() as f64 / WHISPER_SAMPLE_RATE as f64;
    if audio_seconds > MAX_TRANSCRIBE_SECONDS {
        return Err(format!(
            "Audio range is {:.0} s; transcription caps at {:.0} s per call",
            audio_seconds, MAX_TRANSCRIBE_SECONDS
        ));
    }
    let model_path = ensure_model(model)?;

    // Serializing model use: one global lock keeps context loading and
    // inference off concurrent threads (whisper.cpp contexts are heavy).
    static WHISPER_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _gate = WHISPER_GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let context = whisper_rs::WhisperContext::new_with_params(
        &model_path.to_string_lossy(),
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|error| format!("Could not load the Whisper model {}: {error}", model.name()))?;
    let mut state = context
        .create_state()
        .map_err(|error| format!("Could not create the Whisper state: {error}"))?;

    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy {
        best_of: 1, // deterministic: same audio → same transcript
    });
    // English-only models have no language switch; multilingual ones take
    // None = auto-detect.
    let language = if is_english_only(model) {
        "en".to_owned()
    } else {
        options.language.clone().unwrap_or_else(|| "auto".to_owned())
    };
    params.set_language(if language == "auto" {
        None
    } else {
        Some(language.as_str())
    });
    params.set_token_timestamps(true);
    params.set_split_on_word(true);
    if let Some(prompt) = options.prompt.as_deref() {
        params.set_initial_prompt(prompt);
    }

    state
        .full(params, samples)
        .map_err(|error| format!("Whisper transcription failed: {error}"))?;

    // Detected language (whisper reports the id for the best lang guess).
    let detected_language = state
        .full_lang_id_from_state()
        .ok()
        .and_then(whisper_rs::get_lang_str)
        .map(str::to_owned)
        .unwrap_or_else(|| language.clone());

    let segment_count = state.full_n_segments().map_err(err)?;
    let mut segments = Vec::with_capacity(segment_count as usize);
    for segment_index in 0..segment_count {
        let token_count = state.full_n_tokens(segment_index).map_err(err)?;
        let mut tokens = Vec::with_capacity(token_count as usize);
        for token_index in 0..token_count {
            let text = state
                .full_get_token_text(segment_index, token_index)
                .map_err(err)?;
            // Special tokens (timestamps, control) have empty text; the DTW
            // timestamps on them are meaningless.
            let special = text.trim().is_empty();
            if special {
                continue;
            }
            let data = state
                .full_get_token_data(segment_index, token_index)
                .map_err(err)?;
            // Some control tokens still carry text; skip non-word markers.
            if text.starts_with('[') && text.ends_with(']') {
                continue;
            }
            tokens.push(TokenTiming {
                text,
                start_cs: data.t0,
                end_cs: data.t1,
                probability: data.p,
                special,
            });
        }
        let words = words_from_tokens(&tokens, options);
        if words.is_empty() {
            continue;
        }
        let text = words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" ");
        let id = segments.len() as u64 + 1;
        segments.push(SpeechSegment {
            id,
            start_ticks: words[0].start_ticks,
            end_ticks: words[words.len() - 1].end_ticks,
            text,
            // Diarization is a separate model — stays None (review §91).
            speaker: None,
            confidence: words.iter().map(|w| w.confidence).sum::<f64>()
                / words.len() as f64,
            words,
        });
    }

    Ok(Transcript {
        segments,
        model: model.name().to_owned(),
        language: detected_language,
        audio_seconds,
    })
}

fn err(error: whisper_rs::WhisperError) -> String {
    format!("Whisper state error: {error}")
}

fn is_english_only(model: WhisperModel) -> bool {
    matches!(
        model,
        WhisperModel::TinyEn
            | WhisperModel::BaseEn
            | WhisperModel::SmallEn
            | WhisperModel::MediumEn
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start_cs: i64, end_cs: i64, probability: f32) -> TokenTiming {
        TokenTiming {
            text: text.to_owned(),
            start_cs,
            end_cs,
            probability,
            special: false,
        }
    }

    #[test]
    fn tokens_group_into_words_on_space_boundaries() {
        // " And so my|fellow" — whisper.cpp token texts carry leading spaces.
        let tokens = vec![
            token(" And", 0, 20, 0.95),
            token(" so", 20, 35, 0.9),
            token(" my", 35, 50, 0.85),
            token("fellow", 50, 90, 0.8), // continuation token: no space
            token(" Americans", 90, 150, 0.99),
            token(",", 150, 152, 0.99),
            token(".", 152, 154, 0.99),
        ];
        let words = words_from_tokens(&tokens, &TranscribeOptions::default());
        assert_eq!(
            words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>(),
            vec!["And", "so", "myfellow", "Americans,."]
        );
        assert_eq!(words[0].start_ticks.0, 0);
        assert_eq!(words[1].end_ticks.0, 35 * (TICKS_PER_SECOND / 100));
        // Confidence averages across merged tokens (f32 mean — moderate tol).
        assert!((words[2].confidence - (0.85 + 0.8) as f64 / 2.0).abs() < 1e-6);
    }

    #[test]
    fn sentencepiece_markers_and_specials_are_handled() {
        let mut marker = token("▁And", 0, 20, 0.9);
        let tokens = vec![
            marker.clone(),
            token("▁so", 20, 35, 0.9),
            TokenTiming {
                text: "[BLANK_AUDIO]".into(),
                start_cs: 35,
                end_cs: 60,
                probability: 0.0,
                special: true,
            },
        ];
        marker.special = false;
        let words = words_from_tokens(&tokens, &TranscribeOptions::default());
        assert_eq!(
            words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>(),
            vec!["And", "so"]
        );
    }

    #[test]
    fn fillers_are_marked_with_the_shared_engine_list() {
        let tokens = vec![
            token(" Um", 0, 20, 0.9),
            token(",", 20, 22, 0.9),
            token(" today", 22, 60, 0.9),
            token(" we", 60, 80, 0.9),
        ];
        let words = words_from_tokens(&tokens, &TranscribeOptions::default());
        assert!(words[0].is_filler, "\"Um\" is on the shared filler list");
        assert!(!words[1].is_filler);
    }

    #[test]
    fn empty_and_degenerate_streams_arent_words() {
        assert!(words_from_tokens(&[], &TranscribeOptions::default()).is_empty());
        let silent = vec![token(" ", 0, 10, 0.5)];
        assert!(words_from_tokens(&silent, &TranscribeOptions::default()).is_empty());
    }
}
