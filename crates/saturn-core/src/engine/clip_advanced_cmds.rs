use super::*;

impl EditorEngine {
    pub(super) fn dispatch_clip_advanced(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "timeline.trim_clip" => Some((|| {
                let params: TrimClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clip = find_clip(&self.document, track_index, params.clip_index)?.clone();
                let media_duration = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|media| media.id == clip.media_id)
                    .and_then(|media| media.duration);

                let delta = params.delta.0;
                let old_start = clip.timeline_start.0;
                let old_duration = clip.duration.0;
                let old_end = old_start.saturating_add(old_duration);
                let (new_start, new_duration, new_source_in) = match params.edge {
                    TrimEdge::Start => (
                        old_start.saturating_add(delta),
                        old_duration.saturating_sub(delta),
                        clip.source_in.0.saturating_add(delta),
                    ),
                    TrimEdge::End => (
                        old_start,
                        old_duration.saturating_add(delta),
                        clip.source_in.0,
                    ),
                };
                if new_duration <= 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Trim would remove the whole clip".into(),
                    ));
                }
                if new_source_in < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Trim cannot extend before the media start".into(),
                    ));
                }
                if new_start < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Trim cannot start before time zero".into(),
                    ));
                }
                if media_duration
                    .is_some_and(|length| new_source_in.saturating_add(new_duration) > length.0)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Trim cannot extend beyond the media duration".into(),
                    ));
                }

                // A ripple head trim keeps the clip anchored and closes the gap
                // by moving everything after the old out point.
                let final_start = if params.ripple && params.edge == TrimEdge::Start {
                    old_start
                } else {
                    new_start
                };
                let shift = new_duration.saturating_sub(old_duration);

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                {
                    let clip = &mut track.clips[params.clip_index];
                    clip.timeline_start = Tick(final_start);
                    clip.duration = Tick(new_duration);
                    clip.source_in = Tick(new_source_in);
                    match params.edge {
                        TrimEdge::Start => {
                            for key in &mut clip.keyframes {
                                key.time.0 = key.time.0.saturating_sub(delta);
                            }
                        }
                        TrimEdge::End => {}
                    }
                    clip.keyframes
                        .retain(|key| key.time.0 >= 0 && key.time.0 <= new_duration);
                }

                if params.ripple && shift != 0 {
                    for later in track
                        .clips
                        .iter_mut()
                        .filter(|item| item.timeline_start.0 >= old_end)
                    {
                        later.timeline_start.0 = later.timeline_start.0.saturating_add(shift);
                    }
                    for transition in track
                        .transitions
                        .iter_mut()
                        .filter(|item| item.cut_at.0 >= old_end)
                    {
                        transition.cut_at.0 = transition.cut_at.0.saturating_add(shift);
                    }
                    track.clips.sort_by_key(|item| item.timeline_start);
                }

                Ok(json!({
                    "trimmed": true,
                    "edge": params.edge,
                    "timeline_start": final_start,
                    "duration": new_duration,
                    "source_in": new_source_in,
                    "delta": delta,
                    "ripple": params.ripple,
                }))
            })()),
            "timeline.slip_clip" => Some((|| {
                let params: SlipClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let next_source_in = clip.source_in.0.saturating_add(params.delta.0);
                if next_source_in < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Slip cannot move the source before its beginning".into(),
                    ));
                }
                let media_id = clip.media_id;
                let duration = clip.duration.0;
                let media_duration = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|media| media.id == media_id)
                    .and_then(|media| media.duration);
                if media_duration
                    .is_some_and(|length| next_source_in.saturating_add(duration) > length.0)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Slip cannot move the source beyond the media duration".into(),
                    ));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.source_in = Tick(next_source_in);
                Ok(json!({"slipped": true, "source_in": clip.source_in}))
            })()),
            "timeline.roll_edit" => Some((|| {
                let params: RollEditParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                if params.left_clip_index + 1 >= track.clips.len() {
                    return Err(DispatchError::InvalidParameters(
                        "Rolling edit requires two adjacent clips".into(),
                    ));
                }
                let left = &track.clips[params.left_clip_index];
                let right = &track.clips[params.left_clip_index + 1];
                let delta = params.delta.0;

                let new_left_dur = left.duration.0.saturating_add(delta);
                let new_right_start = right.timeline_start.0.saturating_add(delta);
                let new_right_source_in = right.source_in.0.saturating_add(delta);
                let new_right_dur = right.duration.0.saturating_sub(delta);

                if new_left_dur <= 0 || new_right_dur <= 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Rolling edit exceeds clip duration limit".into(),
                    ));
                }
                if new_right_source_in < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Right clip cannot roll before media beginning".into(),
                    ));
                }

                let left_media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|m| m.id == left.media_id);
                if let Some(left_med) = left_media {
                    if let Some(med_dur) = left_med.duration {
                        if left.source_in.0.saturating_add(new_left_dur) > med_dur.0 {
                            return Err(DispatchError::InvalidParameters(
                                "Left clip cannot roll beyond media duration".into(),
                            ));
                        }
                    }
                }
                let right_media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|m| m.id == right.media_id);
                if let Some(right_med) = right_media {
                    if let Some(med_dur) = right_med.duration {
                        if new_right_source_in.saturating_add(new_right_dur) > med_dur.0 {
                            return Err(DispatchError::InvalidParameters(
                                "Right clip cannot roll beyond media duration".into(),
                            ));
                        }
                    }
                }

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips[params.left_clip_index].duration = Tick(new_left_dur);
                track.clips[params.left_clip_index + 1].timeline_start = Tick(new_right_start);
                track.clips[params.left_clip_index + 1].source_in = Tick(new_right_source_in);
                track.clips[params.left_clip_index + 1].duration = Tick(new_right_dur);

                for key in &mut track.clips[params.left_clip_index + 1].keyframes {
                    key.time.0 = key.time.0.saturating_sub(delta);
                }
                track.clips[params.left_clip_index + 1]
                    .keyframes
                    .retain(|k| k.time.0 >= 0 && k.time.0 <= new_right_dur);
                track.clips[params.left_clip_index]
                    .keyframes
                    .retain(|k| k.time.0 <= new_left_dur);

                Ok(json!({
                    "rolled": true,
                    "track_id": params.track_id,
                    "left_index": params.left_clip_index,
                    "delta": delta,
                    "edit_point": new_right_start,
                }))
            })()),
            "timeline.slide_clip" => Some((|| {
                let params: SlideClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                if params.clip_index == 0 || params.clip_index + 1 >= track.clips.len() {
                    return Err(DispatchError::InvalidParameters(
                        "Slide edit requires a clip situated between preceding and succeeding clips".into()
                    ));
                }
                let delta = params.delta.0;
                let clip_a = &track.clips[params.clip_index - 1];
                let clip_b = &track.clips[params.clip_index];
                let clip_c = &track.clips[params.clip_index + 1];

                let new_a_dur = clip_a.duration.0.saturating_add(delta);
                let new_b_start = clip_b.timeline_start.0.saturating_add(delta);
                let new_c_start = clip_c.timeline_start.0.saturating_add(delta);
                let new_c_source_in = clip_c.source_in.0.saturating_add(delta);
                let new_c_dur = clip_c.duration.0.saturating_sub(delta);

                if new_a_dur <= 0 || new_c_dur <= 0 || new_b_start < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Slide edit exceeds clip duration limit".into(),
                    ));
                }
                if new_c_source_in < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Succeeding clip source cannot start before media beginning".into(),
                    ));
                }

                let media_a = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|m| m.id == clip_a.media_id);
                if let Some(med_a) = media_a {
                    if let Some(dur) = med_a.duration {
                        if clip_a.source_in.0.saturating_add(new_a_dur) > dur.0 {
                            return Err(DispatchError::InvalidParameters(
                                "Preceding clip exceeds media duration".into(),
                            ));
                        }
                    }
                }
                let media_c = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|m| m.id == clip_c.media_id);
                if let Some(med_c) = media_c {
                    if let Some(dur) = med_c.duration {
                        if new_c_source_in.saturating_add(new_c_dur) > dur.0 {
                            return Err(DispatchError::InvalidParameters(
                                "Succeeding clip exceeds media duration".into(),
                            ));
                        }
                    }
                }

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips[params.clip_index - 1].duration = Tick(new_a_dur);
                track.clips[params.clip_index].timeline_start = Tick(new_b_start);
                track.clips[params.clip_index + 1].timeline_start = Tick(new_c_start);
                track.clips[params.clip_index + 1].source_in = Tick(new_c_source_in);
                track.clips[params.clip_index + 1].duration = Tick(new_c_dur);

                for key in &mut track.clips[params.clip_index + 1].keyframes {
                    key.time.0 = key.time.0.saturating_sub(delta);
                }
                track.clips[params.clip_index + 1]
                    .keyframes
                    .retain(|k| k.time.0 >= 0 && k.time.0 <= new_c_dur);
                track.clips[params.clip_index - 1]
                    .keyframes
                    .retain(|k| k.time.0 <= new_a_dur);

                Ok(json!({
                    "slid": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "delta": delta,
                }))
            })()),
            "timeline.rate_stretch" => Some((|| {
                let params: RateStretchParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let old_dur = clip.duration.0 as f64;
                let old_speed = if clip.speed > 0.0 { clip.speed } else { 1.0 };

                let (new_duration, new_speed) = if let Some(speed) = params.speed {
                    if speed < 0.05 || speed > 20.0 {
                        return Err(DispatchError::InvalidParameters(
                            "Speed must be between 0.05x and 20.0x".into(),
                        ));
                    }
                    let new_dur = ((old_dur * old_speed) / speed).round() as i64;
                    (Tick(new_dur.max(1)), (speed * 1000.0).round() / 1000.0)
                } else if let Some(new_dur) = params.new_duration {
                    if new_dur.0 <= 0 {
                        return Err(DispatchError::InvalidParameters(
                            "Duration must be positive".into(),
                        ));
                    }
                    let calc_speed = (old_dur * old_speed) / (new_dur.0 as f64);
                    let clamped = calc_speed.clamp(0.05, 20.0);
                    (new_dur, (clamped * 1000.0).round() / 1000.0)
                } else {
                    return Err(DispatchError::InvalidParameters(
                        "Either duration or speed must be specified".into(),
                    ));
                };

                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                if old_dur > 0.0 {
                    let ratio = new_duration.0 as f64 / old_dur;
                    for key in &mut clip.keyframes {
                        key.time.0 = ((key.time.0 as f64) * ratio).round() as i64;
                    }
                }
                clip.duration = new_duration;
                clip.speed = new_speed;

                Ok(json!({
                    "stretched": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "duration": new_duration,
                    "speed": new_speed,
                }))
            })()),
            "timeline.link_clips" => Some((|| {
                let params: LinkClipsParams = parse_params(params)?;
                if params.clips.len() < 2 {
                    return Err(DispatchError::InvalidParameters(
                        "Select at least two clips to link".into(),
                    ));
                }
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if r.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters(
                            "Timeline clip does not exist".into(),
                        ));
                    }
                }
                let max_id = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .flat_map(|t| t.clips.iter())
                    .filter_map(|c| c.group_id)
                    .max()
                    .unwrap_or(0);
                let new_group_id = max_id + 1;

                self.checkpoint();
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    self.document.project.sequence.tracks[t_idx].clips[r.clip_index].group_id =
                        Some(new_group_id);
                }

                Ok(json!({
                    "linked": true,
                    "group_id": new_group_id,
                    "count": params.clips.len(),
                }))
            })()),
            "timeline.unlink_clips" => Some((|| {
                let params: UnlinkClipsParams = parse_params(params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Select clips to unlink".into(),
                    ));
                }
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if r.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters(
                            "Timeline clip does not exist".into(),
                        ));
                    }
                }

                let mut groups_to_clear = std::collections::HashSet::new();
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if let Some(gid) =
                        self.document.project.sequence.tracks[t_idx].clips[r.clip_index].group_id
                    {
                        groups_to_clear.insert(gid);
                    }
                }

                self.checkpoint();
                let mut count = 0;
                for track in &mut self.document.project.sequence.tracks {
                    for clip in &mut track.clips {
                        if let Some(gid) = clip.group_id {
                            if groups_to_clear.contains(&gid) {
                                clip.group_id = None;
                                count += 1;
                            }
                        }
                    }
                }

                Ok(json!({
                    "unlinked": true,
                    "count": count,
                }))
            })()),
            _ => None,
        }
    }
}
