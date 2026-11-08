// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Shared, decode-free validation for a rolled picture cut and baked sound cut.
use crate::{Clip, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick, TrackKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsymmetricTrimParams {
    pub track_id: u64,
    /// Incoming clip; its unique abutting predecessor is the outgoing clip.
    pub clip_index: usize,
    pub video_delta: Tick,
    pub audio_delta: Tick,
    #[serde(default = "default_fade")]
    pub fade_ticks: Tick,
}

fn default_fade() -> Tick {
    Tick(TICKS_PER_SECOND / 25) // 40 ms
}

pub struct AsymmetricTrimPlan {
    pub track_index: usize,
    pub left_index: usize,
    pub start: Tick,
    pub duration: Tick,
    pub video_cut: Tick,
    pub audio_cut: Tick,
    pub fade_start: Tick,
    pub fade_end: Tick,
    pub left_picture: Clip,
    pub right_picture: Clip,
    pub left_audio: Clip,
    pub right_audio: Clip,
}

fn add(a: Tick, b: i64) -> Result<Tick, String> {
    a.checked_add(b)
        .ok_or_else(|| "Split-edit timing overflow".into())
}
fn sub(a: Tick, b: Tick) -> Result<Tick, String> {
    a.checked_sub(b.0)
        .ok_or_else(|| "Split-edit timing overflow".into())
}

fn range(clip: &Clip, start: Tick, end: Tick) -> Result<Clip, String> {
    let mut result = clip.clone();
    result.source_in = add(clip.source_in, sub(start, clip.timeline_start)?.0)?;
    result.timeline_start = start;
    result.duration = sub(end, start)?;
    Ok(result)
}

fn check_source(doc: &ProjectDocument, clip: &Clip) -> Result<(), String> {
    let media = doc
        .project
        .media
        .iter()
        .find(|m| m.id == clip.media_id)
        .ok_or("Split-edit source media is missing; relink the source")?;
    if media.kind != MediaKind::Video || !matches!(media.audio_channels, Some(1 | 2)) {
        return Err("Select video sources with mono or stereo audio".into());
    }
    let limit = media
        .duration
        .ok_or("Probe source duration before making a split edit")?;
    if clip.source_in.0 < 0 || clip.duration.0 <= 0 || add(clip.source_in, clip.duration.0)? > limit
    {
        return Err("Insufficient source handles; reduce the cut offset or fade duration".into());
    }
    Ok(())
}

pub fn plan(doc: &ProjectDocument, p: &AsymmetricTrimParams) -> Result<AsymmetricTrimPlan, String> {
    let track_index = doc
        .project
        .sequence
        .tracks
        .iter()
        .position(|t| t.id == p.track_id)
        .ok_or("Select a video track for the split edit")?;
    let track = &doc.project.sequence.tracks[track_index];
    let group_locked = track.group_id.is_some_and(|id| {
        doc.project
            .sequence
            .track_groups
            .iter()
            .any(|g| g.id == id && g.locked)
    });
    if track.kind != TrackKind::Video || track.locked || group_locked {
        return Err("Select an unlocked video track and unlock its track group".into());
    }
    // Video-track automation/pan/plugins have different native routing from audio
    // tracks. Reject these rather than silently changing their sound on detachment.
    if track.pan != 0.0
        || track.bus != crate::BusTarget::Master
        || track.audio_processing != Default::default()
        || track.automation != Default::default()
        || !track.dsp_plugin_chain.is_empty()
        || !track.transitions.is_empty()
    {
        return Err("Use a video track without transitions, audio processing, automation, pan or bus routing".into());
    }
    let right = track
        .clips
        .get(p.clip_index)
        .ok_or("Select the incoming video clip")?;
    let cut = right.timeline_start;
    let predecessors: Vec<_> = track
        .clips
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            *i != p.clip_index && c.timeline_start.checked_add(c.duration.0) == Some(cut)
        })
        .collect();
    if predecessors.len() != 1 {
        return Err("Select an incoming clip with exactly one adjacent outgoing clip".into());
    }
    let (left_index, left) = predecessors[0];
    for clip in [left, right] {
        if !clip.enabled
            || !clip.audio_enabled
            || clip.speed != 1.0
            || clip.speed_ramp.is_some()
            || clip.compound_sequence_id.is_some()
            || clip.multicam_id.is_some()
            || clip.multicam_angle_id.is_some()
            || clip.group_id.is_some()
            || !clip.effects.is_empty()
            || !clip.keyframes.is_empty()
            || clip.tracking.is_some()
            || clip.node_graph.is_some()
            || !clip.color_nodes.is_empty()
            || clip.motion_blur.is_some()
            || !clip.volume_db.is_finite()
            || !(-60.0..=12.0).contains(&clip.volume_db)
        {
            return Err("Use enabled, unretimed, unlinked video clips with attached audio and no effects, keyframes or tracking; undo the previous split edit to revise its cut".into());
        }
        check_source(doc, clip)?;
    }
    let start = left.timeline_start;
    let end = add(right.timeline_start, right.duration.0)?;
    let duration = sub(end, start)?;
    if start.0 < 0 || duration.0 <= 0 || duration.0 > 120 * TICKS_PER_SECOND {
        return Err("The selected clip pair must span at most 120 seconds".into());
    }
    if track.clips.iter().enumerate().any(|(i, c)| {
        i != left_index
            && i != p.clip_index
            && c.timeline_start < end
            && c.timeline_start.saturating_add(c.duration.0) > start
    }) {
        return Err("Remove overlapping clips from the selected pair's range".into());
    }
    if p.fade_ticks.0 < 0 || p.fade_ticks.0 > 5 * TICKS_PER_SECOND {
        return Err("Choose a fade duration between 0 and 5 seconds".into());
    }
    let video_cut = add(cut, p.video_delta.0)?;
    let audio_cut = add(cut, p.audio_delta.0)?;
    let fade_start = add(audio_cut, -(p.fade_ticks.0 / 2))?;
    let fade_end = add(fade_start, p.fade_ticks.0)?;
    if video_cut <= start
        || video_cut >= end
        || audio_cut <= start
        || audio_cut >= end
        || fade_start < start
        || fade_end > end
    {
        return Err("Keep both cuts and the entire fade inside the selected clip pair".into());
    }
    let mut left_picture = range(left, start, video_cut)?;
    let mut right_picture = range(right, video_cut, end)?;
    let left_audio = range(left, start, fade_end)?;
    let right_audio = range(right, fade_start, end)?;
    for clip in [&left_picture, &right_picture, &left_audio, &right_audio] {
        check_source(doc, clip)?;
    }
    left_picture.audio_enabled = false;
    right_picture.audio_enabled = false;
    Ok(AsymmetricTrimPlan {
        track_index,
        left_index,
        start,
        duration,
        video_cut,
        audio_cut,
        fade_start,
        fade_end,
        left_picture,
        right_picture,
        left_audio,
        right_audio,
    })
}
