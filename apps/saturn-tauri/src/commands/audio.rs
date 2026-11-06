// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
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
) -> Result<serde_json::Value, String> {
    let opts = options.unwrap_or(TranscribeAudioOptions {
        media_id: None,
        language: Some("en".to_string()),
        replace_existing: Some(false),
        whisper_model: Some("whisper-base-en".to_string()),
        text_override: None,
    });

    // User-authored text override: the words are the user's own (not a
    // transcription claim); only the timing is synthesized at a fixed
    // reading pace. Reported as `timingSource: "user_text_synthetic"`.
    if let Some(text) = opts.text_override.clone() {
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
                    confidence: 1.0, // authored, not inferred — by definition
                    is_filler: is_filler_word(w, None),
                });
                curr_tick += word_duration_ticks;
            }
            let segments = vec![saturn_core::SpeechSegment {
                id: 1,
                start_ticks: saturn_core::Tick(0),
                end_ticks: saturn_core::Tick(curr_tick),
                text: trimmed.to_string(),
                speaker: None,
                words: speech_words,
                confidence: 1.0,
            }];
            return run_transcribe_command(state, opts, segments, "user_text_synthetic", None);
        }
    }

    // Real offline transcription (Packet 2.1). The engine lock is taken
    // twice — briefly to resolve the source, and again to file the
    // subtitles — never across decode/inference.
    let resolved = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        resolve_transcription_source(&engine.document().project, opts.media_id)?
    };

    let model = saturn_perception::WhisperModel::parse(
        opts.whisper_model.as_deref().unwrap_or("whisper-base-en"),
    )
    .ok_or_else(|| {
        format!(
            "Unknown Whisper model {:?} (expected e.g. \"tiny.en\", \"base.en\", \"small\", \
             \"medium.en\", \"large-v3-turbo\")",
            opts.whisper_model
        )
    })?;

    if resolved.path.is_missing {
        return Err(format!(
            "Media asset {} is missing from disk ({}); relink it before transcribing",
            resolved.asset_id,
            resolved.path.path.display()
        ));
    }

    let decoded = saturn_render::decode_audio_pcm_mono_16k(
        &resolved.path.path,
        saturn_core::Tick(0),
        resolved.path.duration,
    )?;

    let transcript = saturn_perception::transcribe_pcm(
        &decoded,
        model,
        &saturn_perception::TranscribeOptions {
            language: opts.language.clone(),
            detect_fillers: true,
            prompt: None,
        },
    )?;

    // Map media-relative ticks to timeline ticks through the first clip
    // that references this asset (constant-speed mapping). Speed ramps are
    // nonlinear: they are reported as approximated rather than silently
    // mislocated.
    let mut segments = transcript.segments.clone();
    let mut timing_approximated = false;
    if let Some(mapping) = resolved.clip_mapping {
        timing_approximated = mapping.ramped;
        for segment in &mut segments {
            segment.start_ticks = mapping.timeline_tick_from_media(segment.start_ticks);
            segment.end_ticks = mapping.timeline_tick_from_media(segment.end_ticks);
            for word in &mut segment.words {
                word.start_ticks = mapping.timeline_tick_from_media(word.start_ticks);
                word.end_ticks = mapping.timeline_tick_from_media(word.end_ticks);
            }
        }
    }

    run_transcribe_command(
        state,
        opts,
        segments,
        "whisper",
        Some((model.name().to_owned(), transcript, timing_approximated)),
    )
}

/// Files the segments into subtitle cues through the engine and assembles
/// the command response (shared by the Whisper path and the text override).
fn run_transcribe_command(
    state: State<'_, AppState>,
    opts: TranscribeAudioOptions,
    segments: Vec<saturn_core::SpeechSegment>,
    timing_source: &str,
    provenance: Option<(String, saturn_perception::Transcript, bool)>,
) -> Result<serde_json::Value, String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let _ = engine
        .execute(CommandRequest::new(
            "speech.transcribe_to_subtitles",
            json!({
                "segments": segments,
                "replace_existing": opts.replace_existing.unwrap_or(false)
            }),
        ))
        .map_err(|e| e.to_string())?;

    let subtitles = engine
        .document()
        .project
        .sequence
        .caption_tracks
        .first()
        .map(|track| track.cues.clone())
        .unwrap_or_default();
    let mut response = json!({
        "subtitles": subtitles,
        "segments": segments,
        "timingSource": timing_source,
    });
    if let Some((model_name, transcript, approximated)) = provenance {
        response["transcription"] = json!({
            "model": model_name,
            "language": transcript.language,
            "audioSeconds": transcript.audio_seconds,
            "timingApproximated": approximated,
        });
    }
    Ok(response)
}

struct TranscriptionSource {
    asset_id: u64,
    path: std::sync::Arc<saturn_core::project::MediaAsset>,
    /// First timeline clip referencing this asset, in timeline order —
    /// the media→timeline tick mapping. `None` when the asset is not
    /// placed (segments stay in media time).
    clip_mapping: Option<ClipTickMapping>,
}

struct ClipTickMapping {
    timeline_start: Tick,
    source_in: Tick,
    speed: f64,
    /// `true` when the clip carries a nonlinear speed ramp; the constant-
    /// speed formula below cannot be exact, so callers surface this.
    ramped: bool,
}

impl ClipTickMapping {
    fn timeline_tick_from_media(&self, media_tick: Tick) -> Tick {
        let delta = (media_tick.0 - self.source_in.0) as f64 * self.speed;
        Tick((self.timeline_start.0 as f64 + delta).max(0.0).round() as i64)
    }
}

/// Picks the transcription source: the requested asset, or — when no id is
/// given — the media of the earliest timeline clip with audio, falling back
/// to the first Audio then Video asset in the bin.
fn resolve_transcription_source(
    project: &saturn_core::project::Project,
    media_id: Option<u64>,
) -> Result<TranscriptionSource, String> {
    let assets = &project.media;
    if assets.is_empty() {
        return Err("No media assets are imported yet — import a clip with audio first".into());
    }
    let asset_for = |id: u64| assets.iter().find(|a| a.id == id).cloned();
    let audio_capable =
        |asset: &saturn_core::project::MediaAsset| matches!(asset.kind, MediaKind::Audio | MediaKind::Video);

    if let Some(id) = media_id {
        let asset = asset_for(id)
            .ok_or_else(|| format!("Media asset {id} not found"))?;
        return Ok(TranscriptionSource {
            asset_id: asset.id,
            clip_mapping: first_clip_mapping(project, asset.id),
            path: std::sync::Arc::new(asset),
        });
    }
    // Earliest audio-capable clip on the timeline drives the choice.
    let mut timeline_clips: Vec<&saturn_core::project::Clip> = project
        .sequence
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .collect();
    timeline_clips.sort_by_key(|c| c.timeline_start.0);
    for clip in timeline_clips {
        if let Some(asset) = asset_for(clip.media_id).filter(|a| audio_capable(a)) {
            return Ok(TranscriptionSource {
                asset_id: asset.id,
                clip_mapping: Some(ClipTickMapping {
                    timeline_start: clip.timeline_start,
                    source_in: clip.source_in,
                    speed: clip.speed,
                    ramped: clip.speed_ramp.is_some(),
                }),
                path: std::sync::Arc::new(asset),
            });
        }
    }
    // Nothing placed: first Audio, then Video asset in the bin.
    if let Some(asset) = assets
        .iter()
        .find(|a| a.kind == MediaKind::Audio)
        .cloned()
        .or_else(|| assets.iter().find(|a| a.kind == MediaKind::Video).cloned())
    {
        return Ok(TranscriptionSource {
            asset_id: asset.id,
            clip_mapping: None,
            path: std::sync::Arc::new(asset),
        });
    }
    Err("No audio-capable media asset to transcribe".into())
}

fn first_clip_mapping(
    project: &saturn_core::project::Project,
    media_id: u64,
) -> Option<ClipTickMapping> {
    project
        .sequence
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .filter(|c| c.media_id == media_id)
        .min_by_key(|c| c.timeline_start.0)
        .map(|clip| ClipTickMapping {
            timeline_start: clip.timeline_start,
            source_in: clip.source_in,
            speed: clip.speed,
            ramped: clip.speed_ramp.is_some(),
        })
}

/// Transcribes the requested media assets (default: every audio-capable
/// asset in the bin) and aligns the script against the resulting takes
/// through the engine's `speech.script_sync_align` — take ranking with
/// per-line locations. Inference runs **outside** the engine lock; only
/// the final analysis and result assembly touch it.
#[tauri::command]
pub async fn align_script_to_takes(
    state: State<'_, AppState>,
    script_text: String,
    media_ids: Option<Vec<u64>>,
    whisper_model: Option<String>,
) -> Result<serde_json::Value, String> {
    if script_text.trim().is_empty() {
        return Err("Script text is empty".into());
    }
    let model = saturn_perception::WhisperModel::parse(
        whisper_model.as_deref().unwrap_or("whisper-base-en"),
    )
    .ok_or_else(|| format!("Unknown Whisper model {whisper_model:?}"))?;

    let sources: Vec<(u64, PathBuf, Option<Tick>)> = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        let project = &engine.document().project;
        let ids = match media_ids {
            Some(ids) if !ids.is_empty() => ids,
            _ => project
                .media
                .iter()
                .filter(|a| matches!(a.kind, MediaKind::Audio | MediaKind::Video))
                .map(|a| a.id)
                .collect(),
        };
        if ids.is_empty() {
            return Err("No audio-capable media assets to align".into());
        }
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let asset = project
                .media
                .iter()
                .find(|a| a.id == id)
                .ok_or_else(|| format!("Media asset {id} not found"))?;
            if asset.is_missing {
                return Err(format!(
                    "Media asset {id} is missing from disk; relink it before aligning"
                ));
            }
            out.push((asset.id, asset.path.clone(), asset.duration));
        }
        out
    };

    let mut takes = Vec::with_capacity(sources.len());
    for (media_id, path, duration) in sources {
        let decoded = saturn_render::decode_audio_pcm_mono_16k(&path, Tick(0), duration)?;
        let transcript = saturn_perception::transcribe_pcm(
            &decoded,
            model,
            &saturn_perception::TranscribeOptions {
                language: Some("en".to_owned()),
                detect_fillers: false,
                prompt: None,
            },
        )?;
        takes.push(saturn_core::script_sync::ScriptTake {
            media_id,
            segments: transcript.segments,
        });
    }

    let mut engine = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?;
    let report = engine
        .execute(CommandRequest::new(
            "speech.script_sync_align",
            json!({ "script_text": script_text, "takes": takes }),
        ))
        .map_err(|e| e.to_string())?;
    Ok(report)
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
