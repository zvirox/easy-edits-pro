// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Shared headless/desktop orchestration for offline audio edits (Packet 4.2).
//! Source analysis is explicitly bounded to 120 seconds. Rust core applies the
//! prepared edit atomically, rejecting stale snapshots. Derived WAVs are kept
//! beside the saved project and consumed identically by preview and export.
use crate::audio_edit_files::PreparedAudioEdit;
use crate::audio_pcm::decode_audio_pcm_constrained;
use saturn_audio_dsp::{ducking, phase_align, room_tone};
use saturn_core::audio_edit::snapshot_digest;
use saturn_core::{Clip, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick, Track, TrackKind};
use serde_json::{Value, json};
use std::path::Path;

const RATE: u32 = 48_000;

pub fn prepare(
    document: &ProjectDocument,
    project: Option<&Path>,
    operation: &str,
    params: &Value,
) -> Result<PreparedAudioEdit, String> {
    if operation == "asymmetric_trim" {
        let mut args = params.clone();
        if let Some(object) = args.as_object_mut() {
            object.remove("operation");
        }
        return crate::asymmetric_trim::prepare(document, project, &args);
    }
    document.validate()?;
    let id =
        match operation {
            "phase_align" => "audio.auto_phase_align",
            "room_tone" => "audio.fill_gaps_with_room_tone",
            "duck" => "audio.sidechain_duck_envelope",
            "normalize_master" => "audio.loudness_normalize_master",
            _ => return Err(
                "Unknown audio operation; use phase_align, room_tone, duck, normalize_master, or asymmetric_trim"
                    .into(),
            ),
        };
    let mut edit = PreparedAudioEdit::new(project, id)?;
    let mut command = match operation {
        "phase_align" => prepare_phase(document, params, &mut edit)?,
        "room_tone" => prepare_room(document, params, &mut edit)?,
        "duck" => prepare_duck(document, params, &mut edit)?,
        "normalize_master" => prepare_master(document, params, &mut edit)?,
        _ => unreachable!(),
    };
    command["expected_digest"] = json!(snapshot_digest(document)?);
    edit.command.params = command;
    Ok(edit)
}

fn uint(p: &Value, key: &str) -> Result<u64, String> {
    p.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("'{key}' must be an unsigned integer"))
}

fn track(doc: &ProjectDocument, id: u64) -> Result<&Track, String> {
    let t = doc
        .project
        .sequence
        .tracks
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("Unknown track {id}"))?;
    if t.kind != TrackKind::Audio || t.locked || t.clips.is_empty() {
        return Err("An unlocked nonempty audio track is required".into());
    }
    if !t.transitions.is_empty()
        || t.clips.iter().any(|c| {
            c.speed != 1.0
                || c.speed_ramp.is_some()
                || !c.enabled
                || c.compound_sequence_id.is_some()
        })
    {
        return Err("Audio analysis currently requires enabled, unretimed, non-compound clips without transitions".into());
    }
    Ok(t)
}

fn frame_count(duration: Tick) -> Result<usize, String> {
    if duration.0 <= 0 || duration.0 > 120 * TICKS_PER_SECOND {
        return Err(
            "Audio processing duration must be greater than zero and at most 120 seconds".into(),
        );
    }
    Ok((duration.0 as i128 * RATE as i128 / TICKS_PER_SECOND as i128) as usize)
}

fn ticks(frames: usize) -> Tick {
    Tick((frames as i128 * TICKS_PER_SECOND as i128 / RATE as i128) as i64)
}

fn decode_clip(
    doc: &ProjectDocument,
    clip: &Clip,
    offset: Tick,
    duration: Tick,
    channels: u32,
) -> Result<Vec<f32>, String> {
    let expected = frame_count(duration)? * channels as usize;
    let source = doc
        .project
        .media
        .iter()
        .find(|m| m.id == clip.media_id)
        .ok_or("Clip media is missing")?;
    if source.kind != MediaKind::Audio {
        return Err("Audio analysis requires audio media".into());
    }
    let start = clip
        .source_in
        .checked_add(offset.0)
        .ok_or("Audio source start overflow")?;
    let end = start
        .checked_add(duration.0)
        .ok_or("Audio source end overflow")?;
    let mut decoded =
        decode_audio_pcm_constrained(&source.path, start, Some(end), Some(RATE), Some(channels))?
            .samples;
    if decoded.len() < expected {
        return Err("Source ends before the requested audio range; relink or trim the clip".into());
    }
    decoded.truncate(expected);
    Ok(decoded)
}

fn prepare_phase(
    doc: &ProjectDocument,
    p: &Value,
    edit: &mut PreparedAudioEdit,
) -> Result<Value, String> {
    let ref_id = uint(p, "ref_track_id")?;
    let target_id = uint(p, "target_track_id")?;
    let reference = track(doc, ref_id)?;
    let target = track(doc, target_id)?;
    if ref_id == target_id || reference.clips.len() != 1 || target.clips.len() != 1 {
        return Err("Phase alignment requires two distinct single-clip audio tracks".into());
    }
    let a = &reference.clips[0];
    let b = &target.clips[0];
    let start = a.timeline_start.max(b.timeline_start);
    let end = a
        .timeline_start
        .checked_add(a.duration.0)
        .ok_or("Reference end overflow")?
        .min(
            b.timeline_start
                .checked_add(b.duration.0)
                .ok_or("Target end overflow")?,
        );
    let available = end.checked_sub(start.0).ok_or("Audio overlap overflow")?;
    if available.0 <= 0 {
        return Err("Reference and target clips must overlap".into());
    }
    let count = frame_count(available)?.min(phase_align::MAX_ANALYSIS_FRAMES);
    let duration = ticks(count);
    let reference = decode_clip(
        doc,
        a,
        start
            .checked_sub(a.timeline_start.0)
            .ok_or("Offset overflow")?,
        duration,
        1,
    )?;
    let target = decode_clip(
        doc,
        b,
        start
            .checked_sub(b.timeline_start.0)
            .ok_or("Offset overflow")?,
        duration,
        1,
    )?;
    let alignment = phase_align::estimate_delay(&reference, &target, RATE)?;
    let entire_target = decode_clip(doc, b, Tick(0), b.duration, 2)?;
    let corrected = phase_align::shift_interleaved(
        &entire_target,
        2,
        alignment.lag_samples,
        alignment.inverted_polarity,
    )?;
    let audio = edit.write(&corrected, 2, b.duration)?;
    edit.telemetry = json!({"alignment": alignment, "measurementDomain": "raw source audio over the timeline overlap",
        "edgePolicy": "Unavailable samples at the corrected clip edge are zero-filled; no wraparound",
        "scope": "one constant delay for the selected single-clip target track"});
    Ok(json!({"ref_track_id": ref_id, "target_track_id": target_id, "audio": audio}))
}

fn prepare_duck(
    doc: &ProjectDocument,
    p: &Value,
    edit: &mut PreparedAudioEdit,
) -> Result<Value, String> {
    let dialogue_id = uint(p, "dialogue_track_id")?;
    let music_id = uint(p, "music_track_id")?;
    let dialogue = track(doc, dialogue_id)?;
    let music = track(doc, music_id)?;
    if dialogue_id == music_id
        || !music.automation.volume_points.is_empty()
        || !music.automation.pan_points.is_empty()
    {
        return Err("Ducking requires distinct tracks and empty music automation lanes".into());
    }
    let duration = doc.project.sequence.duration();
    let count = frame_count(duration)?;
    let mut pcm = vec![0.0; count];
    for clip in &dialogue.clips {
        let decoded = decode_clip(doc, clip, Tick(0), clip.duration, 1)?;
        let start =
            (clip.timeline_start.0 as i128 * RATE as i128 / TICKS_PER_SECOND as i128) as usize;
        for (out, value) in pcm[start..].iter_mut().zip(decoded) {
            *out += value;
        }
    }
    let options: ducking::DuckingOptions =
        serde_json::from_value(p.clone()).map_err(|e| e.to_string())?;
    let mut points = ducking::envelope(&pcm, RATE, options)?;
    for point in &mut points {
        point.value += music.gain_db;
        if !(-60.0..=12.0).contains(&point.value) {
            return Err("Ducking plus track gain exceeds the automation range -60..12 dB".into());
        }
    }
    edit.telemetry = json!({"detector": "10 ms raw-source RMS energy on the designated dialogue track, not speech recognition",
        "thresholdDbfs": options.threshold_dbfs, "points": points.len()});
    Ok(json!({"dialogue_track_id": dialogue_id, "music_track_id": music_id, "points": points}))
}

fn prepare_room(
    doc: &ProjectDocument,
    p: &Value,
    edit: &mut PreparedAudioEdit,
) -> Result<Value, String> {
    let id = uint(p, "track_id")?;
    let track = track(doc, id)?;
    let profile_id = uint(p, "room_tone_media_id")?;
    let start = Tick(i64::try_from(uint(p, "profile_start_tick")?).map_err(|e| e.to_string())?);
    let end = Tick(i64::try_from(uint(p, "profile_end_tick")?).map_err(|e| e.to_string())?);
    let length = end.checked_sub(start.0).ok_or("Profile range overflow")?;
    if length.0 <= 0 || length.0 > 10 * TICKS_PER_SECOND {
        return Err("Room-tone profile must span 0..10 seconds of known speech-free audio".into());
    }
    let profile_clip = saturn_core::audio_edit::audio_clip(profile_id, Tick(0), end);
    let profile = decode_clip(doc, &profile_clip, start, length, 1)?;
    let min_gap_ms = uint(p, "min_gap_ms")?;
    let crossfade_ms = uint(p, "crossfade_ms")?;
    if min_gap_ms == 0 || min_gap_ms > 120000 || crossfade_ms > 5000 {
        return Err("Invalid room-tone gap/fade duration".into());
    }
    let mut clips: Vec<_> = track.clips.iter().collect();
    clips.sort_by_key(|c| c.timeline_start);
    let mut inserts = Vec::new();
    let mut total_frames = 0;
    for pair in clips.windows(2) {
        let start = pair[0]
            .timeline_start
            .checked_add(pair[0].duration.0)
            .ok_or("Clip end overflow")?;
        let length = pair[1]
            .timeline_start
            .checked_sub(start.0)
            .ok_or("Gap overflow")?;
        if length.0 <= 0 || length.0 as i128 * 1000 < min_gap_ms as i128 * TICKS_PER_SECOND as i128
        {
            continue;
        }
        if inserts.len() >= 256 {
            return Err("At most 256 room-tone gaps can be filled per operation".into());
        }
        let count = frame_count(length)?;
        total_frames += count;
        if total_frames > RATE as usize * 120 {
            return Err("Room-tone gaps exceed the total 120-second processing budget".into());
        }
        let fade = (crossfade_ms as usize * RATE as usize / 1000).min(count / 2);
        let tone = room_tone::synthesize(&profile, count, fade, start.0 as u64 + 1)?;
        inserts.push(saturn_core::audio_edit::RoomToneInsert {
            start,
            audio: edit.write(&tone, 1, length)?,
        });
    }
    if inserts.is_empty() {
        return Err("No internal empty gaps meet min_gap_ms".into());
    }
    edit.telemetry = json!({"gaps": inserts.len(), "profile": "caller-selected speech-free source",
        "method": "random-phase synthesis from the measured noise spectrum",
        "fadePolicy": "equal-power fades inside each gap; existing dialogue clips are not crossfaded or rewritten"});
    Ok(json!({"track_id": id, "inserts": inserts}))
}

fn prepare_master(
    doc: &ProjectDocument,
    p: &Value,
    edit: &mut PreparedAudioEdit,
) -> Result<Value, String> {
    let duration = doc.project.sequence.duration();
    frame_count(duration)?;
    let standard =
        serde_json::from_value(p.get("standard").cloned().ok_or("standard is required")?)
            .map_err(|e| e.to_string())?;
    let ceiling = p
        .get("true_peak_dbtp")
        .and_then(Value::as_f64)
        .ok_or("true_peak_dbtp is required")?;
    if doc.project.sequence.tracks.iter().any(|t| t.locked) {
        return Err("Unlock all tracks before creating a master bounce".into());
    }
    let (raw_path, file) = edit.allocate()?;
    drop(file);
    crate::audio_master::render_mix(doc, &raw_path)?;
    let mut pcm =
        decode_audio_pcm_constrained(&raw_path, Tick(0), Some(duration), Some(RATE), Some(2))?;
    let expected = frame_count(duration)? * 2;
    if pcm.samples.len() < expected {
        return Err("Master render ended before the sequence duration".into());
    }
    pcm.samples.truncate(expected);
    let _ = std::fs::remove_file(raw_path);
    let (normalized, telemetry) =
        crate::audio_master::normalize(&pcm.samples, RATE, 2, standard, ceiling)?;
    let audio = edit.write(&normalized, 2, duration)?;
    edit.telemetry = json!({"measurement": telemetry, "mode": "undoable_master_bounce",
        "notes": "Original tracks are retained and muted; the new float-WAV master is the audible mix. Further source edits require a new bounce. Lossy delivery codecs may change true peak; remeasure exported audio."});
    Ok(json!({"audio": audio}))
}
