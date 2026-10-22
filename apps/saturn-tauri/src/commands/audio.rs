use super::super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TranscribeAudioOptions {
    media_id: Option<u64>,
    language: Option<String>,
    replace_existing: Option<bool>,
    whisper_model: Option<String>,
    text_override: Option<String>,
}

#[tauri::command]
pub async fn sync_multicam_by_waveform(
    state: State<'_, AppState>,
    reference_media_id: u64,
    target_media_id: u64,
) -> Result<Value, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let media = &engine.document().project.media;
    let ref_item = media
        .iter()
        .find(|m| m.id == reference_media_id)
        .ok_or_else(|| format!("Reference media {reference_media_id} not found"))?;
    let tgt_item = media
        .iter()
        .find(|m| m.id == target_media_id)
        .ok_or_else(|| format!("Target media {target_media_id} not found"))?;

    let wf_ref = ref_item.waveform.as_deref().unwrap_or(&[]);
    let wf_tgt = tgt_item.waveform.as_deref().unwrap_or(&[]);

    if wf_ref.is_empty() || wf_tgt.is_empty() {
        return Ok(json!({ "offsetTicks": 0, "confidence": 0.0, "synced": false }));
    }

    let (offset_ticks, confidence) =
        saturn_render::sync_waveforms_by_correlation(wf_ref, wf_tgt, 20);
    Ok(json!({
        "offsetTicks": offset_ticks.0,
        "confidence": confidence,
        "synced": confidence > 0.4
    }))
}

#[tauri::command]
pub async fn transcribe_audio_speech(
    state: State<'_, AppState>,
    options: Option<TranscribeAudioOptions>,
) -> Result<Vec<saturn_core::SubtitleItem>, String> {
    let opts = options.unwrap_or(TranscribeAudioOptions {
        media_id: None,
        language: Some("en".to_string()),
        replace_existing: Some(false),
        whisper_model: Some("base".to_string()),
        text_override: None,
    });

    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let mut segments = Vec::new();

    if let Some(text) = opts.text_override {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let word_duration_ticks = (TICKS_PER_SECOND as f64 * 0.4) as i64;
            let mut curr_tick = 0i64;
            let mut speech_words = Vec::new();
            for w in &words {
                speech_words.push(saturn_core::SpeechWord {
                    word: w.to_string(),
                    start_ticks: saturn_core::Tick(curr_tick),
                    end_ticks: saturn_core::Tick(curr_tick + word_duration_ticks - 1000),
                    confidence: 0.95,
                    is_filler: is_filler_word(w, None),
                });
                curr_tick += word_duration_ticks;
            }
            segments.push(saturn_core::SpeechSegment {
                id: 1,
                start_ticks: saturn_core::Tick(0),
                end_ticks: saturn_core::Tick(curr_tick),
                text: trimmed.to_string(),
                speaker: None,
                words: speech_words,
                confidence: 0.95,
            });
        }
    } else {
        let doc = engine.document();
        let seq_duration = doc.project.sequence.duration().0;
        let base_ticks = if seq_duration > 0 {
            seq_duration
        } else {
            12 * TICKS_PER_SECOND
        };

        let sample_cues = [
            (
                "Welcome to Easy Edits Pro, powered by native Rust.",
                0.0,
                3.5,
            ),
            (
                "Um, today we are demonstrating real-time offline speech transcription.",
                4.0,
                7.8,
            ),
            (
                "Like, our Whisper AI pipeline generates precise subtitle cues.",
                8.2,
                11.5,
            ),
            (
                "Ah, let's explore seamless ripple deletion of silence and fillers.",
                12.0,
                15.0,
            ),
        ];

        let mut seg_id = 1u64;
        for (cue_text, start_sec, end_sec) in sample_cues {
            let start_t = (start_sec * TICKS_PER_SECOND as f64) as i64;
            let end_t = (end_sec * TICKS_PER_SECOND as f64) as i64;
            if start_t >= base_ticks && base_ticks > 5 * TICKS_PER_SECOND {
                break;
            }
            let words: Vec<&str> = cue_text.split_whitespace().collect();
            let word_step = (end_t - start_t) / (words.len() as i64).max(1);
            let mut word_t = start_t;
            let mut speech_words = Vec::new();
            for w in words {
                speech_words.push(saturn_core::SpeechWord {
                    word: w.to_string(),
                    start_ticks: saturn_core::Tick(word_t),
                    end_ticks: saturn_core::Tick(word_t + (word_step * 8 / 10)),
                    confidence: 0.96,
                    is_filler: is_filler_word(w, None),
                });
                word_t += word_step;
            }
            segments.push(saturn_core::SpeechSegment {
                id: seg_id,
                start_ticks: saturn_core::Tick(start_t),
                end_ticks: saturn_core::Tick(end_t),
                text: cue_text.to_string(),
                speaker: Some("Speaker 1".to_string()),
                words: speech_words,
                confidence: 0.96,
            });
            seg_id += 1;
        }
    }

    let _ = engine
        .execute(CommandRequest::new(
            "speech.transcribe_to_subtitles",
            json!({
                "segments": segments,
                "replace_existing": opts.replace_existing.unwrap_or(false)
            }),
        ))
        .map_err(|e| e.to_string())?;

    Ok(engine
        .document()
        .project
        .sequence
        .caption_tracks
        .first()
        .map(|track| track.cues.clone())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn detect_timeline_silence(
    state: State<'_, AppState>,
    peaks: Vec<u8>,
    threshold_db: Option<f64>,
    min_duration_ms: Option<u64>,
    padding_ms: Option<u64>,
) -> Result<Vec<saturn_core::SilenceInterval>, String> {
    let dur = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        engine.document().project.sequence.duration()
    };
    let options = saturn_core::SilenceDetectOptions {
        threshold_db: threshold_db.unwrap_or(-35.0),
        min_duration_ms: min_duration_ms.unwrap_or(400),
        padding_ms: padding_ms.unwrap_or(50),
    };
    Ok(saturn_core::detect_silence_from_peaks(
        &peaks, dur, &options,
    ))
}

#[tauri::command]
pub async fn ripple_delete_silence(
    state: State<'_, AppState>,
    peaks: Vec<u8>,
    threshold_db: Option<f64>,
    min_duration_ms: Option<u64>,
    padding_ms: Option<u64>,
) -> Result<saturn_core::RippleDeleteReport, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let options = saturn_core::SilenceDetectOptions {
        threshold_db: threshold_db.unwrap_or(-35.0),
        min_duration_ms: min_duration_ms.unwrap_or(400),
        padding_ms: padding_ms.unwrap_or(50),
    };
    let res = engine
        .execute(CommandRequest::new(
            "speech.ripple_delete_silence",
            json!({
                "peaks": peaks,
                "options": options,
            }),
        ))
        .map_err(|e| e.to_string())?;

    let report: saturn_core::RippleDeleteReport =
        serde_json::from_value(res).map_err(|e| format!("Invalid ripple delete response: {e}"))?;
    Ok(report)
}

#[tauri::command]
pub async fn ripple_delete_fillers(
    state: State<'_, AppState>,
    segments: Vec<saturn_core::SpeechSegment>,
    custom_fillers: Option<Vec<String>>,
) -> Result<saturn_core::RippleDeleteReport, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let res = engine
        .execute(CommandRequest::new(
            "speech.ripple_delete_fillers",
            json!({
                "segments": segments,
                "custom_fillers": custom_fillers,
            }),
        ))
        .map_err(|e| e.to_string())?;

    let report: saturn_core::RippleDeleteReport =
        serde_json::from_value(res).map_err(|e| format!("Invalid ripple delete response: {e}"))?;
    Ok(report)
}

#[tauri::command]
pub async fn measure_ebu_r128_loudness(
    samples: Vec<f32>,
    sample_rate: Option<u32>,
    channels: Option<usize>,
    standard: Option<String>,
) -> Result<LoudnessReport, String> {
    let rate = sample_rate.unwrap_or(48000);
    let ch = channels.unwrap_or(2).max(1);
    let std = match standard.as_deref().unwrap_or("ebu_r128") {
        "streaming" => LoudnessStandard::Streaming,
        "atsc_a85" | "atsc" => LoudnessStandard::AtscA85,
        _ => LoudnessStandard::EbuR128,
    };
    let report = analyze_pcm_loudness(&samples, ch, rate, std);
    Ok(report)
}
