//! Three-point editing and edit-point inspection.
//!
//! `timeline.insert_clip` and `timeline.overwrite_clip` are the engine half of
//! the Source-monitor workflow: the UI supplies a source range and a timeline
//! insertion point, and the engine performs the ripple/overwrite maths on one
//! target track. Both are transactional, so one edit is one undo entry.

use super::*;

impl EditorEngine {
    pub(super) fn dispatch_edit(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "timeline.insert_clip" => Some(self.three_point(params, true)),
            "timeline.overwrite_clip" => Some(self.three_point(params, false)),
            "timeline.inspect" => Some((|| {
                let params: InspectParams = parse_params(params)?;
                self.inspect_edit(params)
            })()),
            "trim.inspect" => Some((|| {
                let params: TrimInspectParams = parse_params(params)?;
                self.inspect_trim(params)
            })()),
            _ => None,
        }
    }

    fn three_point(&mut self, params: Value, insert: bool) -> Result<Value, DispatchError> {
        let params: ThreePointParams = parse_params(params)?;
        let media = self
            .document
            .project
            .media
            .iter()
            .find(|item| item.id == params.media_id)
            .ok_or_else(|| {
                DispatchError::InvalidParameters(format!(
                    "Media item {} does not exist",
                    params.media_id
                ))
            })?;
        let default_kind = match media.kind {
            MediaKind::Audio => TrackKind::Audio,
            _ => TrackKind::Video,
        };
        let media_duration = media.duration;
        let track_index = self
            .document
            .project
            .sequence
            .tracks
            .iter()
            .position(|track| {
                params
                    .track_id
                    .map(|track_id| track.id == track_id)
                    .unwrap_or_else(|| {
                        std::mem::discriminant(&track.kind) == std::mem::discriminant(&default_kind)
                    })
            })
            .ok_or_else(|| {
                DispatchError::InvalidParameters("No matching timeline track exists".into())
            })?;
        let track = &self.document.project.sequence.tracks[track_index];
        if track.locked {
            return Err(DispatchError::Operation("Track is locked".into()));
        }
        let duration = params
            .duration
            .or(media_duration)
            .unwrap_or(Tick(TICKS_PER_SECOND * 5));
        if duration.0 <= 0 || params.source_in.0 < 0 {
            return Err(DispatchError::InvalidParameters(
                "Source range and duration must be positive".into(),
            ));
        }
        if media_duration
            .is_some_and(|length| params.source_in.0.saturating_add(duration.0) > length.0)
        {
            return Err(DispatchError::InvalidParameters(
                "Source range exceeds the media duration".into(),
            ));
        }
        let at = params.at.map(|tick| tick.0).unwrap_or_else(|| {
            track
                .clips
                .iter()
                .filter_map(|clip| clip.timeline_start.0.checked_add(clip.duration.0))
                .max()
                .unwrap_or(0)
        });
        if at < 0 {
            return Err(DispatchError::InvalidParameters(
                "The timeline insertion point cannot be negative".into(),
            ));
        }

        self.checkpoint();
        let track = &mut self.document.project.sequence.tracks[track_index];
        if insert {
            split_at(track, at);
            let end = at.saturating_add(duration.0);
            for clip in track.clips.iter_mut() {
                if clip.timeline_start.0 >= at {
                    clip.timeline_start.0 = clip.timeline_start.0.saturating_add(duration.0);
                }
            }
            for transition in track.transitions.iter_mut() {
                if transition.cut_at.0 >= at {
                    transition.cut_at.0 = transition.cut_at.0.saturating_add(duration.0);
                }
            }
            track.clips.push(new_clip(
                params.media_id,
                at,
                params.source_in.0,
                duration.0,
            ));
            track.clips.sort_by_key(|clip| clip.timeline_start.0);
            return Ok(json!({
                "track_id": track.id,
                "clip_index": track.clips.iter().position(|clip| {
                    clip.media_id == params.media_id
                        && clip.timeline_start.0 == at
                        && clip.duration.0 == duration.0
                }),
                "start": at,
                "end": end,
                "mode": "insert",
            }));
        }

        overwrite_range(track, at, at.saturating_add(duration.0));
        track.clips.push(new_clip(
            params.media_id,
            at,
            params.source_in.0,
            duration.0,
        ));
        track.clips.sort_by_key(|clip| clip.timeline_start.0);
        Ok(json!({
            "track_id": track.id,
            "clip_index": track.clips.iter().position(|clip| {
                clip.media_id == params.media_id
                    && clip.timeline_start.0 == at
                    && clip.duration.0 == duration.0
            }),
            "start": at,
            "end": at.saturating_add(duration.0),
            "mode": "overwrite",
        }))
    }

    fn inspect_edit(&self, params: InspectParams) -> Result<Value, DispatchError> {
        let at = params.at.map(|tick| tick.0);
        if let Some(clip_index) = params.clip_index {
            let track_id = params.track_id.ok_or_else(|| {
                DispatchError::InvalidParameters("track_id is required with clip_index".into())
            })?;
            let track_index = find_track_index(&self.document, track_id)?;
            let track = &self.document.project.sequence.tracks[track_index];
            let clip = track.clips.get(clip_index).ok_or_else(|| {
                DispatchError::InvalidParameters("Timeline clip does not exist".into())
            })?;
            let media = self
                .document
                .project
                .media
                .iter()
                .find(|item| item.id == clip.media_id);
            let start = clip.timeline_start.0;
            let end = start.saturating_add(clip.duration.0);
            return Ok(json!({
                "track_id": track_id,
                "track_name": track.name,
                "clip_index": clip_index,
                "media_id": clip.media_id,
                "media_name": media.map(|item| item.name.clone()),
                "start": start,
                "end": end,
                "duration": clip.duration.0,
                "source_in": clip.source_in.0,
                "source_out": clip.source_in.0.saturating_add(clip.duration.0),
                "media_duration": media.and_then(|item| item.duration.map(|tick| tick.0)),
                "previous": track.clips.get(clip_index.wrapping_sub(1)).map(|item| {
                    json!({"start": item.timeline_start.0, "end": item.timeline_start.0.saturating_add(item.duration.0)})
                }),
                "next": track.clips.get(clip_index + 1).map(|item| {
                    json!({"start": item.timeline_start.0, "end": item.timeline_start.0.saturating_add(item.duration.0)})
                }),
            }));
        }

        let tick = at.unwrap_or(0);
        let mut active = Vec::new();
        for track in &self.document.project.sequence.tracks {
            if let Some((index, clip)) = track.clips.iter().enumerate().find(|(_, clip)| {
                tick >= clip.timeline_start.0
                    && tick < clip.timeline_start.0.saturating_add(clip.duration.0)
            }) {
                active.push(json!({
                    "track_id": track.id,
                    "track_name": track.name,
                    "clip_index": index,
                    "media_id": clip.media_id,
                    "start": clip.timeline_start.0,
                    "end": clip.timeline_start.0.saturating_add(clip.duration.0),
                }));
            }
        }
        Ok(json!({
            "at": tick,
            "active_clips": active,
            "sequence_duration": self.document.project.sequence.duration().0,
            "mark_in": self.document.project.sequence.mark_in.map(|tick| tick.0),
            "mark_out": self.document.project.sequence.mark_out.map(|tick| tick.0),
        }))
    }

    fn inspect_trim(&self, params: TrimInspectParams) -> Result<Value, DispatchError> {
        let track_index = find_track_index(&self.document, params.track_id)?;
        let track = &self.document.project.sequence.tracks[track_index];
        let clip = track.clips.get(params.clip_index).ok_or_else(|| {
            DispatchError::InvalidParameters("Timeline clip does not exist".into())
        })?;
        let media = self
            .document
            .project
            .media
            .iter()
            .find(|item| item.id == clip.media_id);
        let start = clip.timeline_start.0;
        let end = start.saturating_add(clip.duration.0);
        let previous_end = params
            .clip_index
            .checked_sub(1)
            .and_then(|index| track.clips.get(index))
            .map(|item| item.timeline_start.0.saturating_add(item.duration.0));
        let next_start = track
            .clips
            .get(params.clip_index + 1)
            .map(|item| item.timeline_start.0);
        let media_end = media
            .and_then(|item| item.duration)
            .map(|tick| tick.0)
            .unwrap_or(i64::MAX);
        let max_tail = media_end.saturating_sub(clip.source_in.0 + clip.duration.0);
        let min_duration = (TICKS_PER_SECOND / 30).min(clip.duration.0);
        let (cut, head_room, tail_room) = match params.edge {
            TrimEdge::Start => {
                let neighbour = previous_end.map(|edge| start.saturating_sub(edge));
                (
                    start,
                    neighbour.unwrap_or(clip.source_in.0).min(clip.source_in.0),
                    clip.duration.0.saturating_sub(min_duration),
                )
            }
            TrimEdge::End => (
                end,
                clip.duration.0.saturating_sub(min_duration),
                next_start
                    .map(|edge| edge.saturating_sub(end))
                    .unwrap_or(max_tail)
                    .min(max_tail),
            ),
        };
        Ok(json!({
            "track_id": params.track_id,
            "clip_index": params.clip_index,
            "edge": params.edge,
            "cut": cut,
            "start": start,
            "end": end,
            "duration": clip.duration.0,
            "source_in": clip.source_in.0,
            "source_out": clip.source_in.0.saturating_add(clip.duration.0),
            "media_duration": media.and_then(|item| item.duration.map(|tick| tick.0)),
            "head_room": head_room,
            "tail_room": tail_room,
            "min_duration": min_duration,
            "previous_end": previous_end,
            "next_start": next_start,
        }))
    }
}

fn new_clip(media_id: u64, start: i64, source_in: i64, duration: i64) -> Clip {
    Clip {
        media_id,
        timeline_start: Tick(start),
        source_in: Tick(source_in),
        duration: Tick(duration),
        speed: 1.0,
        group_id: None,
        color: ColorAdjustments::default(),
        effects: Vec::new(),
        keyframes: Vec::new(),
        multicam_id: None,
        multicam_angle_id: None,
        tracking: None,
        node_graph: None,
        speed_ramp: None,
        motion_blur: None,
        compound_sequence_id: None,
    }
}

/// Split the clip that straddles `at` so insert/overwrite only has to reason
/// about clip boundaries. A cut exactly on a boundary is left alone.
fn split_at(track: &mut Track, at: i64) {
    if let Some(index) = track.clips.iter().position(|clip| {
        clip.timeline_start.0 < at && at < clip.timeline_start.0.saturating_add(clip.duration.0)
    }) {
        let mut right = track.clips[index].clone();
        let offset = at - right.timeline_start.0;
        right.timeline_start = Tick(at);
        right.source_in = Tick(right.source_in.0.saturating_add(offset));
        right.duration = Tick(right.duration.0.saturating_sub(offset));
        track.clips[index].duration = Tick(offset);
        track.clips.push(right);
    }
}

/// Remove or shorten everything in `[at, end)` on the target track.
fn overwrite_range(track: &mut Track, at: i64, end: i64) {
    let mut result: Vec<Clip> = Vec::with_capacity(track.clips.len() + 1);
    for clip in track.clips.drain(..) {
        let start = clip.timeline_start.0;
        let clip_end = start.saturating_add(clip.duration.0);
        if clip_end <= at || start >= end {
            result.push(clip);
            continue;
        }
        if start < at {
            let mut left = clip.clone();
            left.duration = Tick(at - start);
            result.push(left);
        }
        if clip_end > end {
            let mut right = clip;
            let trim = end - start;
            right.timeline_start = Tick(end);
            right.source_in = Tick(right.source_in.0.saturating_add(trim));
            right.duration = Tick(clip_end - end);
            result.push(right);
        }
    }
    result.sort_by_key(|clip| clip.timeline_start.0);
    track.clips = result;
    track
        .transitions
        .retain(|transition| transition.cut_at.0 < at || transition.cut_at.0 >= end);
}
