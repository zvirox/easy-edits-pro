// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode-free contract for bounded incoming-source match-cut slips.
use crate::{Clip, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick, TrackKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchCutMode {
    EyeTrace,
    Motion,
    Transient,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchCutParams {
    pub track_id: u64,
    pub left_clip_index: usize,
    pub right_clip_index: usize,
    pub mode: MatchCutMode,
    #[serde(default = "default_search")]
    pub search_frames: u32,
}
fn default_search() -> u32 {
    12
}

pub struct MatchCutPlan<'a> {
    pub track_index: usize,
    pub left: &'a Clip,
    pub right: &'a Clip,
    pub left_media: &'a MediaAsset,
    pub right_media: &'a MediaAsset,
    pub frame_ticks: i64,
    pub min_source_in: Tick,
    pub max_source_in: Tick,
    pub left_end: Tick,
}

fn source<'a>(
    doc: &'a ProjectDocument,
    clip: &Clip,
    mode: MatchCutMode,
) -> Result<&'a MediaAsset, String> {
    if !clip.enabled
        || !clip.audio_enabled
        || clip.speed != 1.0
        || clip.speed_ramp.is_some()
        || clip.group_id.is_some()
        || clip.compound_sequence_id.is_some()
        || clip.multicam_id.is_some()
        || clip.multicam_angle_id.is_some()
        || !clip.effects.is_empty()
        || !clip.keyframes.is_empty()
        || clip.tracking.is_some()
        || clip.node_graph.is_some()
        || !clip.color_nodes.is_empty()
        || clip.motion_blur.is_some()
    {
        return Err("Use enabled, unretimed, unlinked clips without effects or tracking; align before detaching sound".into());
    }
    let media = doc
        .project
        .media
        .iter()
        .find(|m| m.id == clip.media_id)
        .ok_or("Match-cut source media is missing")?;
    let valid_kind = media.kind == MediaKind::Video
        || (mode == MatchCutMode::Transient && media.kind == MediaKind::Audio);
    if !valid_kind
        || (mode == MatchCutMode::Transient && !matches!(media.audio_channels, Some(1 | 2)))
    {
        return Err(
            "Visual matching needs video; transient matching needs mono/stereo audio".into(),
        );
    }
    let end = clip
        .source_in
        .checked_add(clip.duration.0)
        .ok_or("Match-cut timing overflow")?;
    if clip.source_in.0 < 0 || clip.duration.0 <= 0 || media.duration.is_none_or(|d| end > d) {
        return Err("Probe source durations and keep clips inside their source handles".into());
    }
    Ok(media)
}

pub fn plan<'a>(doc: &'a ProjectDocument, p: &MatchCutParams) -> Result<MatchCutPlan<'a>, String> {
    doc.validate()?;
    let track_index = doc
        .project
        .sequence
        .tracks
        .iter()
        .position(|t| t.id == p.track_id)
        .ok_or("Select a track containing the cut")?;
    let track = &doc.project.sequence.tracks[track_index];
    if track.locked
        || track.group_id.is_some_and(|id| {
            doc.project
                .sequence
                .track_groups
                .iter()
                .any(|g| g.id == id && g.locked)
        })
    {
        return Err("Unlock the track and its group before aligning a match cut".into());
    }
    if !track.transitions.is_empty()
        || (p.mode != MatchCutMode::Transient && track.kind != TrackKind::Video)
    {
        return Err(
            "Visual matching needs a video track; remove track transitions before alignment".into(),
        );
    }
    if p.left_clip_index == p.right_clip_index {
        return Err("Select two distinct adjacent clips".into());
    }
    let left = track
        .clips
        .get(p.left_clip_index)
        .ok_or("Outgoing clip does not exist")?;
    let right = track
        .clips
        .get(p.right_clip_index)
        .ok_or("Incoming clip does not exist")?;
    if left.timeline_start.checked_add(left.duration.0) != Some(right.timeline_start) {
        return Err("The outgoing and incoming clips must meet at a hard cut".into());
    }
    let end = right
        .timeline_start
        .checked_add(right.duration.0)
        .ok_or("Match-cut timing overflow")?;
    if track.clips.iter().enumerate().any(|(i, c)| {
        i != p.left_clip_index
            && i != p.right_clip_index
            && c.timeline_start < end
            && c.timeline_start.saturating_add(c.duration.0) > left.timeline_start
    }) {
        return Err("Remove overlapping clips from the selected pair's range".into());
    }
    let left_media = source(doc, left, p.mode)?;
    let right_media = source(doc, right, p.mode)?;
    let frame_ticks = doc
        .project
        .settings
        .frame_rate
        .frame_ticks()
        .filter(|v| *v > 0)
        .ok_or("Match cuts require an exact project frame rate")?;
    let radius = frame_ticks
        .checked_mul(p.search_frames as i64)
        .ok_or("Match-cut search overflow")?;
    if !(1..=30).contains(&p.search_frames) || radius > 2 * TICKS_PER_SECOND {
        return Err(
            "Choose 1 to 30 search frames, spanning at most two seconds on each side".into(),
        );
    }
    let minimum = match p.mode {
        MatchCutMode::EyeTrace => frame_ticks,
        MatchCutMode::Motion => frame_ticks
            .checked_mul(2)
            .ok_or("Match-cut timing overflow")?,
        MatchCutMode::Transient => TICKS_PER_SECOND / 10,
    };
    if left.duration.0 < minimum || right.duration.0 < minimum {
        return Err("Clips are too short for this match mode (one/two visual frames or 100 ms of audio required)".into());
    }
    let min_source_in = Tick(right.source_in.0.saturating_sub(radius).max(0));
    let max_source_in = Tick(
        right
            .source_in
            .0
            .saturating_add(radius)
            .min(right_media.duration.unwrap().0 - right.duration.0),
    );
    Ok(MatchCutPlan {
        track_index,
        left,
        right,
        left_media,
        right_media,
        frame_ticks,
        min_source_in,
        max_source_in,
        left_end: left
            .source_in
            .checked_add(left.duration.0)
            .ok_or("Match-cut timing overflow")?,
    })
}

pub fn validate_choice(
    plan: &MatchCutPlan<'_>,
    mode: MatchCutMode,
    source_in: Tick,
) -> Result<(), String> {
    if source_in < plan.min_source_in || source_in > plan.max_source_in {
        return Err(
            "Matched source position is outside the search range or available handles".into(),
        );
    }
    if mode != MatchCutMode::Transient
        && (source_in.0 - plan.right.source_in.0) % plan.frame_ticks != 0
    {
        return Err("Visual match slips must use whole project frames".into());
    }
    Ok(())
}
