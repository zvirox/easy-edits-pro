// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Bounded motion-compensated dissolve (Packet 5.1, fluid morph cut).
//!
//! Honest scope: the picture transition is a real GES crossfade over an
//! overlapped cut, with a bounded source slip derived from measured displacement
//! (from `saturn-perception::motion_track`). It is NOT bidirectional optical
//! flow frame interpolation; the audio room-tone crossfade is carried as
//! metadata for the audio renderer and is not synthesised by this command.

use crate::{Tick, project::{ProjectDocument, TransitionKind}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeasuredMorph {
    /// Measured horizontal/vertical displacement in source pixels.
    pub dx_px: f32,
    pub dy_px: f32,
    /// ZNCC confidence of the measurement (0..1).
    pub confidence: f32,
    /// Tick conversion for the source resolution/fps, supplied by the caller
    /// that decoded the frames.
    pub ticks_per_px: f64,
    /// Requested room-tone crossfade length; clamped to the morph duration.
    pub audio_crossfade_ticks: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MorphReport {
    pub transition_id: u64,
    pub cut_at: Tick,
    pub duration: Tick,
    pub overlap_each_side: Tick,
    pub motion_slip_ticks: i64,
    pub audio_crossfade_ticks: i64,
}

pub fn apply_morph(
    document: &mut ProjectDocument,
    track_id: u64,
    cut_at: Tick,
    duration: Tick,
    measured: &MeasuredMorph,
    expected_digest: &str,
) -> Result<MorphReport, String> {
    if crate::audio_edit::snapshot_digest(document)? != expected_digest {
        return Err("project changed during morph analysis; rerun the measurement".into());
    }
    if duration.0 <= 0 || duration.0 > 2 * crate::TICKS_PER_SECOND {
        return Err("morph duration must be between 1 tick and 2 seconds".into());
    }
    if !(0.0..=1.0).contains(&measured.confidence) {
        return Err("morph confidence must be between 0 and 1".into());
    }
    if !measured.dx_px.is_finite() || !measured.dy_px.is_finite()
        || measured.dx_px.abs() > 64.0 || measured.dy_px.abs() > 64.0
    {
        return Err("measured displacement must be finite and within +/-64 px".into());
    }
    if !measured.ticks_per_px.is_finite() || measured.ticks_per_px <= 0.0
        || measured.ticks_per_px > 1000.0
    {
        return Err("ticks_per_px must be finite and within (0, 1000]".into());
    }

    let track_index = document.project.sequence.tracks.iter()
        .position(|track| track.id == track_id)
        .ok_or("morph track not found")?;
    let track = &document.project.sequence.tracks[track_index];
    let right = track.clips.iter().position(|clip| clip.timeline_start == cut_at)
        .ok_or("no clip starts at the requested cut")?;
    if right == 0 {
        return Err("the morph cut needs an adjacent incoming and outgoing clip".into());
    }
    let left = right - 1;
    let outgoing = &track.clips[left];
    let incoming = &track.clips[right];
    if outgoing.timeline_start.0.saturating_add(outgoing.duration.0) != cut_at.0 {
        return Err("the adjacent clips must meet exactly at the cut".into());
    }
    if outgoing.speed <= 0.0 || incoming.speed <= 0.0 {
        return Err("morph cannot be applied to retimed clips".into());
    }

    let overlap = Tick(duration.0 / 2);
    let media_for = |id: u64| document.project.media.iter().find(|media| media.id == id).and_then(|m| m.duration);
    let outgoing_media_end = media_for(outgoing.media_id)
        .ok_or("outgoing media duration is unknown")?;
    let incoming_media_start = incoming.source_in.0;
    if outgoing.source_in.0.saturating_add(outgoing.duration.0).saturating_add(overlap.0)
        > outgoing_media_end.0
    {
        return Err("outgoing clip has no picture handle for the morph overlap".into());
    }

    let magnitude = measured.dx_px.hypot(measured.dy_px).min(64.0) as f64;
    let slip = (magnitude * measured.ticks_per_px).round().clamp(-240.0, 240.0) as i64;
    let incoming_new_source = incoming_media_start.saturating_add(slip).max(0);
    let incoming_new_start = cut_at.0.saturating_sub(overlap.0);
    if incoming_new_start < 0 {
        return Err("morph overlap extends before time zero".into());
    }

    let track = &mut document.project.sequence.tracks[track_index];
    track.clips[left].duration = Tick(track.clips[left].duration.0 + overlap.0);
    track.clips[right].timeline_start = Tick(incoming_new_start);
    track.clips[right].source_in = Tick(incoming_new_source);
    let transition_id = document.project.next_id;
    document.project.next_id += 1;
    track.transitions.push(crate::project::Transition {
        id: transition_id,
        kind: TransitionKind::Crossfade,
        cut_at: Tick(cut_at.0),
        duration,
    });

    let audio_crossfade = measured.audio_crossfade_ticks.clamp(0, duration.0);
    Ok(MorphReport {
        transition_id,
        cut_at,
        duration,
        overlap_each_side: overlap,
        motion_slip_ticks: incoming_new_source - incoming_media_start,
        audio_crossfade_ticks: audio_crossfade,
    })
}
