// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::audio_edit::{RenderedAudio, RoomToneInsert, audio_clip, snapshot_digest};
use crate::project::AutomationPoint;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PhaseParams {
    expected_digest: String,
    ref_track_id: u64,
    target_track_id: u64,
    audio: RenderedAudio,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToneParams {
    expected_digest: String,
    track_id: u64,
    inserts: Vec<RoomToneInsert>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DuckParams {
    expected_digest: String,
    dialogue_track_id: u64,
    music_track_id: u64,
    points: Vec<AutomationPoint>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MasterParams {
    expected_digest: String,
    audio: RenderedAudio,
}

fn invalid(text: impl Into<String>) -> DispatchError {
    DispatchError::InvalidParameters(text.into())
}

fn audio_track(doc: &ProjectDocument, id: u64) -> Result<usize, DispatchError> {
    let i = find_track_index(doc, id)?;
    let t = &doc.project.sequence.tracks[i];
    if t.kind != TrackKind::Audio || t.locked {
        return Err(invalid("An unlocked audio track is required"));
    }
    Ok(i)
}

fn add_audio(
    doc: &mut ProjectDocument,
    audio: RenderedAudio,
    name: &str,
) -> Result<u64, DispatchError> {
    if audio.duration.0 <= 0
        || audio.duration.0 > TICKS_PER_SECOND * 120
        || !(1..=2).contains(&audio.channels)
        || !audio.path.is_absolute()
        || !audio.path.is_file()
        || audio
            .path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        || audio.path.extension().and_then(|s| s.to_str()) != Some("wav")
    {
        return Err(invalid(
            "Rendered audio must be an existing absolute WAV, 1..2 channels, duration 0..120 seconds",
        ));
    }
    let id = doc.project.next_id;
    doc.project.next_id = id
        .checked_add(1)
        .ok_or_else(|| invalid("Project id space exhausted"))?;
    doc.project.media.push(MediaAsset {
        id,
        name: name.into(),
        path: audio.path,
        duration: Some(audio.duration),
        kind: MediaKind::Audio,
        audio_channels: Some(audio.channels),
        codec: Some("pcm_f32le".into()),
        ..Default::default()
    });
    Ok(id)
}

impl EditorEngine {
    pub(super) fn dispatch_audio_edit(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if !matches!(
            id,
            "audio.auto_phase_align"
                | "audio.fill_gaps_with_room_tone"
                | "audio.sidechain_duck_envelope"
                | "audio.loudness_normalize_master"
        ) {
            return None;
        }
        Some((|| {
            let expected = params.get("expected_digest").and_then(Value::as_str)
                .ok_or_else(|| invalid("Use the audio processing tool to prepare this edit; expected_digest is required"))?;
            if snapshot_digest(&self.document).map_err(DispatchError::Operation)? != expected {
                return Err(invalid(
                    "Project changed during audio analysis; rerun the audio tool",
                ));
            }
            // Construct and fully validate the candidate before touching history.
            let mut next = self.document.clone();
            let result = match id {
                "audio.auto_phase_align" => {
                    let p: PhaseParams = parse_params(params)?;
                    let _ = p.expected_digest;
                    let ref_index = audio_track(&next, p.ref_track_id)?;
                    let i = audio_track(&next, p.target_track_id)?;
                    if i == ref_index
                        || next.project.sequence.tracks[i].clips.len() != 1
                        || next.project.sequence.tracks[ref_index].clips.len() != 1
                    {
                        return Err(invalid(
                            "Phase alignment currently requires two distinct single-clip audio tracks",
                        ));
                    }
                    let target = &next.project.sequence.tracks[i].clips[0];
                    if target.duration != p.audio.duration
                        || target.speed != 1.0
                        || target.speed_ramp.is_some()
                    {
                        return Err(invalid(
                            "Phase replacement must match the unretimed target clip duration",
                        ));
                    }
                    let media = add_audio(&mut next, p.audio, "Phase-aligned audio")?;
                    let clip = &mut next.project.sequence.tracks[i].clips[0];
                    clip.media_id = media;
                    clip.source_in = Tick(0);
                    json!({"media_id": media, "target_track_id": p.target_track_id})
                }
                "audio.fill_gaps_with_room_tone" => {
                    let p: ToneParams = parse_params(params)?;
                    let _ = p.expected_digest;
                    let i = audio_track(&next, p.track_id)?;
                    if p.inserts.is_empty() || p.inserts.len() > 256 {
                        return Err(invalid("Room-tone inserts must contain 1..256 gaps"));
                    }
                    for insert in p.inserts {
                        let end = insert
                            .start
                            .checked_add(insert.audio.duration.0)
                            .ok_or_else(|| invalid("Room-tone range overflow"))?;
                        let track = &next.project.sequence.tracks[i];
                        let left = track.clips.iter().any(|c| {
                            c.timeline_start.checked_add(c.duration.0) == Some(insert.start)
                        });
                        let right = track.clips.iter().any(|c| c.timeline_start == end);
                        let overlap = track.clips.iter().any(|c| {
                            c.timeline_start < end
                                && c.timeline_start.saturating_add(c.duration.0) > insert.start
                        });
                        if insert.start.0 < 0 || !left || !right || overlap {
                            return Err(invalid(
                                "Room tone must exactly fill an internal empty clip gap",
                            ));
                        }
                        let duration = insert.audio.duration;
                        let media = add_audio(&mut next, insert.audio, "Room tone")?;
                        next.project.sequence.tracks[i].clips.push(audio_clip(
                            media,
                            insert.start,
                            duration,
                        ));
                    }
                    next.project.sequence.tracks[i]
                        .clips
                        .sort_by_key(|c| c.timeline_start);
                    json!({"track_id": p.track_id, "filled": true})
                }
                "audio.sidechain_duck_envelope" => {
                    let p: DuckParams = parse_params(params)?;
                    let _ = p.expected_digest;
                    let dialogue = audio_track(&next, p.dialogue_track_id)?;
                    let i = audio_track(&next, p.music_track_id)?;
                    let track = &mut next.project.sequence.tracks[i];
                    if dialogue == i
                        || !track.automation.volume_points.is_empty()
                        || !track.automation.pan_points.is_empty()
                    {
                        return Err(invalid(
                            "Use distinct dialogue/music tracks with empty music automation lanes",
                        ));
                    }
                    if p.points.len() < 2
                        || p.points.len() > 50_000
                        || p.points.iter().any(|p| {
                            p.time.0 < 0
                                || !p.value.is_finite()
                                || !(-60.0..=12.0).contains(&p.value)
                        })
                        || p.points.windows(2).any(|p| p[0].time >= p[1].time)
                    {
                        return Err(invalid(
                            "Ducking points must be finite, strictly time-sorted, and within -60..12 dB",
                        ));
                    }
                    track.automation.volume_points = p.points;
                    track.automation.mode = AutomationMode::Read;
                    track.automation.interpolation = AutomationInterpolation::Linear;
                    json!({"music_track_id": p.music_track_id, "points": track.automation.volume_points.len()})
                }
                "audio.loudness_normalize_master" => {
                    let p: MasterParams = parse_params(params)?;
                    let _ = p.expected_digest;
                    if p.audio.duration != next.project.sequence.duration()
                        || next.project.sequence.tracks.iter().any(|t| t.locked)
                    {
                        return Err(invalid(
                            "Master bounce must cover the entire sequence and all tracks must be unlocked",
                        ));
                    }
                    let duration = p.audio.duration;
                    let media = add_audio(&mut next, p.audio, "Normalized master")?;
                    let track_id = next.project.next_id;
                    next.project.next_id = track_id
                        .checked_add(1)
                        .ok_or_else(|| invalid("Project id space exhausted"))?;
                    for track in &mut next.project.sequence.tracks {
                        track.muted = true;
                        track.solo = false;
                    }
                    next.project.sequence.tracks.push(Track {
                        id: track_id,
                        name: "Normalized master".into(),
                        kind: TrackKind::Audio,
                        group_id: None,
                        gain_db: 0.0,
                        pan: 0.0,
                        muted: false,
                        solo: false,
                        locked: false,
                        audio_processing: Default::default(),
                        bus: BusTarget::Master,
                        automation: Default::default(),
                        dsp_plugin_chain: vec![],
                        clips: vec![audio_clip(media, Tick(0), duration)],
                        transitions: vec![],
                    });
                    json!({"track_id": track_id, "media_id": media, "mode": "undoable_master_bounce"})
                }
                _ => unreachable!(),
            };
            next.validate().map_err(DispatchError::Operation)?;
            self.checkpoint();
            self.document = next;
            Ok(result)
        })())
    }
}
