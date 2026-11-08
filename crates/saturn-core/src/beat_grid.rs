// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode-free contract for rolling a continuous video strip onto a music grid.
use crate::{Clip, MediaAsset, MediaKind, ProjectDocument, TICKS_PER_SECOND, Tick, TrackKind};
use serde::{Deserialize, Serialize};

pub const MAX_ANALYSIS_SECONDS: i64 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subdivision {
    Bar,
    Half,
    Quarter,
}
impl Subdivision {
    fn beats(self) -> usize {
        match self {
            Self::Bar => 4,
            Self::Half => 2,
            Self::Quarter => 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatGridParams {
    pub music_track_id: u64,
    pub video_track_id: u64,
    pub subdivision: Subdivision,
    pub strength: f64,
    /// Known bar-start timeline position. Required for bar/half; meter is 4/4.
    #[serde(default)]
    pub downbeat_tick: Option<Tick>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredBeatGrid {
    pub origin: Tick,
    pub period: Tick,
    /// Only the observed supported span; no extrapolation beyond these beats.
    pub beats: u32,
}

pub struct BeatGridPlan<'a> {
    pub video_track_index: usize,
    pub clip_indices: Vec<usize>,
    pub music_clip: &'a Clip,
    pub music_media: &'a MediaAsset,
    pub analysis_start: Tick,
    pub analysis_end: Tick,
    pub frame_ticks: i64,
}

fn end(clip: &Clip) -> Result<Tick, String> {
    clip.timeline_start
        .checked_add(clip.duration.0)
        .ok_or("Beat-grid timing overflow".into())
}
fn source<'a>(
    doc: &'a ProjectDocument,
    clip: &Clip,
    video: bool,
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
        return Err("Use enabled, unretimed, unlinked clips without effects or tracking; quantize before detaching sound".into());
    }
    let media = doc
        .project
        .media
        .iter()
        .find(|m| m.id == clip.media_id)
        .ok_or("Relink missing beat-grid source media")?;
    if (video && media.kind != MediaKind::Video)
        || (!video
            && (!matches!(media.kind, MediaKind::Audio | MediaKind::Video)
                || !matches!(media.audio_channels, Some(1 | 2))))
    {
        return Err(
            "Use video sources for cuts and mono/stereo audio for the music reference".into(),
        );
    }
    let source_end = clip
        .source_in
        .checked_add(clip.duration.0)
        .ok_or("Beat-grid source overflow")?;
    if clip.source_in.0 < 0 || clip.duration.0 <= 0 || media.duration.is_none_or(|d| source_end > d)
    {
        return Err("Probe source durations and keep clips inside their source handles".into());
    }
    Ok(media)
}

pub fn plan<'a>(doc: &'a ProjectDocument, p: &BeatGridParams) -> Result<BeatGridPlan<'a>, String> {
    doc.validate()?;
    if !p.strength.is_finite() || !(0.0..=1.0).contains(&p.strength) {
        return Err("Choose quantize strength from 0 to 1".into());
    }
    if p.subdivision != Subdivision::Quarter && p.downbeat_tick.is_none() {
        return Err(
            "Supply a known downbeat_tick for bar or half-note quantization (4/4 meter)".into(),
        );
    }
    if p.downbeat_tick.is_some_and(|t| t.0 < 0) {
        return Err("Downbeat time must be nonnegative".into());
    }
    let video_track_index = doc
        .project
        .sequence
        .tracks
        .iter()
        .position(|t| t.id == p.video_track_id)
        .ok_or("Select the video track to quantize")?;
    let video = &doc.project.sequence.tracks[video_track_index];
    if video.kind != TrackKind::Video
        || video.locked
        || video.group_id.is_some_and(|id| {
            doc.project
                .sequence
                .track_groups
                .iter()
                .any(|g| g.id == id && g.locked)
        })
    {
        return Err("Select an unlocked video track and unlock its track group".into());
    }
    if !(2..=128).contains(&video.clips.len()) || !video.transitions.is_empty() {
        return Err("Use a continuous video strip of 2 to 128 clips without transitions".into());
    }
    let frame_ticks = doc
        .project
        .settings
        .frame_rate
        .frame_ticks()
        .filter(|v| *v > 0)
        .ok_or("Beat quantization requires an exact project frame rate")?;
    let mut clip_indices: Vec<_> = (0..video.clips.len()).collect();
    clip_indices.sort_by_key(|&i| video.clips[i].timeline_start);
    for &i in &clip_indices {
        let clip = &video.clips[i];
        source(doc, clip, true)?;
        if clip.duration.0 < frame_ticks
            || clip.timeline_start.0 % frame_ticks != 0
            || clip.duration.0 % frame_ticks != 0
        {
            return Err(
                "Align video clip boundaries to project frames, with at least one frame per clip"
                    .into(),
            );
        }
    }
    for pair in clip_indices.windows(2) {
        if end(&video.clips[pair[0]])? != video.clips[pair[1]].timeline_start {
            return Err(
                "Remove gaps and overlaps from the video track before beat quantization".into(),
            );
        }
    }
    let start = video.clips[clip_indices[0]].timeline_start;
    let finish = end(&video.clips[*clip_indices.last().unwrap()])?;
    if finish.0 - start.0 > MAX_ANALYSIS_SECONDS * TICKS_PER_SECOND {
        return Err("Select a video strip spanning at most 120 seconds".into());
    }
    let music = doc
        .project
        .sequence
        .tracks
        .iter()
        .find(|t| t.id == p.music_track_id)
        .ok_or("Select the music reference track")?;
    if music.kind != TrackKind::Audio
        || music.clips.len() != 1
        || !music.transitions.is_empty()
        || music.audio_processing != Default::default()
        || music.automation != Default::default()
        || !music.dsp_plugin_chain.is_empty()
    {
        return Err("Use one music clip on an audio track without transitions, processing, automation or plugins".into());
    }
    let music_clip = &music.clips[0];
    let music_media = source(doc, music_clip, false)?;
    let music_end = end(music_clip)?;
    if music_clip.timeline_start > start || music_end < finish {
        return Err("Extend the music clip to cover the entire video strip".into());
    }
    let mut analysis_start = start
        .saturating_sub(2 * TICKS_PER_SECOND)
        .max(music_clip.timeline_start);
    let mut analysis_end = finish.saturating_add(2 * TICKS_PER_SECOND).min(music_end);
    if analysis_end.0 - analysis_start.0 > MAX_ANALYSIS_SECONDS * TICKS_PER_SECOND {
        analysis_start = start;
        analysis_end = finish;
    }
    if analysis_end.0 - analysis_start.0 < 4 * TICKS_PER_SECOND {
        return Err("Provide at least four seconds of music around the video strip".into());
    }
    Ok(BeatGridPlan {
        video_track_index,
        clip_indices,
        music_clip,
        music_media,
        analysis_start,
        analysis_end,
        frame_ticks,
    })
}

/// Plan all rolls against the original snapshot, then validate every resulting
/// clip. Collisions and handle failures reject the batch instead of clamping.
pub fn quantized_clips(
    doc: &ProjectDocument,
    p: &BeatGridParams,
    grid: &MeasuredBeatGrid,
) -> Result<(usize, Vec<Clip>, Vec<(Tick, Tick)>), String> {
    let plan = plan(doc, p)?;
    if !(TICKS_PER_SECOND * 3 / 10..=TICKS_PER_SECOND).contains(&grid.period.0)
        || !(6..=401).contains(&grid.beats)
    {
        return Err("A measured 60-200 BPM grid with at least six beats is required".into());
    }
    let last = grid
        .period
        .0
        .checked_mul((grid.beats - 1) as i64)
        .and_then(|d| grid.origin.checked_add(d))
        .ok_or("Beat-grid timing overflow")?;
    if grid.origin < plan.analysis_start || last > plan.analysis_end {
        return Err("Keep the measured beat grid inside the analyzed music range".into());
    }
    let all: Vec<Tick> = (0..grid.beats)
        .map(|i| Tick(grid.origin.0 + grid.period.0 * i as i64))
        .collect();
    let nearest = |tick: Tick| {
        all.iter()
            .enumerate()
            .min_by_key(|(_, t)| ((t.0 as i128 - tick.0 as i128).unsigned_abs(), t.0))
            .unwrap()
    };
    let phase = if let Some(tick) = p.downbeat_tick {
        let (index, beat) = nearest(tick);
        if (beat.0 as i128 - tick.0 as i128).abs()
            > i128::from((plan.frame_ticks / 2).max(TICKS_PER_SECOND / 20))
        {
            return Err(
                "Place the downbeat within 50 ms (or half a project frame) of a detected beat"
                    .into(),
            );
        }
        index % p.subdivision.beats()
    } else {
        0
    };
    let targets: Vec<Tick> = all
        .iter()
        .enumerate()
        .filter(|(i, _)| i % p.subdivision.beats() == phase)
        .map(|(_, t)| *t)
        .collect();
    let track = &doc.project.sequence.tracks[plan.video_track_index];
    let mut boundaries = vec![track.clips[plan.clip_indices[0]].timeline_start];
    let mut cuts = Vec::new();
    for &i in plan.clip_indices.iter().skip(1) {
        let old = track.clips[i].timeline_start;
        if old < grid.origin || old > last {
            return Err("Keep video cuts inside the supported beat span; trim silence or choose another music section".into());
        }
        let target = targets
            .iter()
            .min_by_key(|t| ((t.0 as i128 - old.0 as i128).unsigned_abs(), t.0))
            .ok_or("No subdivision targets in the measured grid")?;
        let frames =
            ((target.0 - old.0) as f64 * p.strength / plan.frame_ticks as f64).round() as i64;
        let new = frames
            .checked_mul(plan.frame_ticks)
            .and_then(|d| old.checked_add(d))
            .ok_or("Quantized cut overflow")?;
        boundaries.push(new);
        cuts.push((old, new));
    }
    boundaries.push(end(&track.clips[*plan.clip_indices.last().unwrap()])?);
    let mut clips = track.clips.clone();
    for (n, &i) in plan.clip_indices.iter().enumerate() {
        let start = boundaries[n];
        let finish = boundaries[n + 1];
        if finish.0 - start.0 < plan.frame_ticks {
            return Err("Quantized cuts collide or cross the strip edges; reduce strength or use a finer subdivision".into());
        }
        let original = &track.clips[i];
        clips[i].source_in = original
            .source_in
            .checked_add(start.0 - original.timeline_start.0)
            .ok_or("Quantized source overflow")?;
        clips[i].timeline_start = start;
        clips[i].duration = Tick(finish.0 - start.0);
        source(doc,&clips[i],true).map_err(|_|"Insufficient source handles for a quantized cut; reduce strength or extend source handles")?;
    }
    Ok((plan.video_track_index, clips, cuts))
}
