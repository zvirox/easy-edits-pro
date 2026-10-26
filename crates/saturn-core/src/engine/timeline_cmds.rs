use super::*;

impl EditorEngine {
    pub(super) fn dispatch_timeline(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "timeline.ripple_delete" => Some((|| {
                let params: RemoveClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clips = &self.document.project.sequence.tracks[track_index].clips;
                let clip = clips.get(params.clip_index).ok_or_else(|| {
                    DispatchError::InvalidParameters("Timeline clip does not exist".into())
                })?;
                let start = clip.timeline_start.0;
                let end = start.saturating_add(clip.duration.0);
                let duration = clip.duration.0;
                self.checkpoint();
                let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                clips.remove(params.clip_index);
                for later in clips.iter_mut().filter(|item| item.timeline_start.0 >= end) {
                    later.timeline_start.0 = later.timeline_start.0.saturating_sub(duration);
                }
                clips.sort_by_key(|item| item.timeline_start);
                Ok(json!({"removed": true, "rippled_ticks": duration, "start": start}))
            })()),
            "timeline.ripple_delete_batch" => Some((|| {
                let params: RemoveClipsParams = parse_params(params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Select one or more timeline clips".into(),
                    ));
                }
                let mut removals = std::collections::BTreeMap::<usize, Vec<usize>>::new();
                for reference in &params.clips {
                    let track_index = find_track_index(&self.document, reference.track_id)?;
                    if self.document.project.sequence.tracks[track_index].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if reference.clip_index
                        >= self.document.project.sequence.tracks[track_index]
                            .clips
                            .len()
                    {
                        return Err(DispatchError::InvalidParameters(
                            "Timeline clip does not exist".into(),
                        ));
                    }
                    let indices = removals.entry(track_index).or_default();
                    if !indices.contains(&reference.clip_index) {
                        indices.push(reference.clip_index);
                    }
                }
                self.checkpoint();
                let mut removed = 0;
                for (track_index, indices) in removals {
                    let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                    let mut indices = indices;
                    indices.sort_unstable_by(|left, right| right.cmp(left));
                    for index in indices {
                        let clip = clips.remove(index);
                        let end = clip.timeline_start.0.saturating_add(clip.duration.0);
                        if params.ripple {
                            for later in
                                clips.iter_mut().filter(|item| item.timeline_start.0 >= end)
                            {
                                later.timeline_start.0 =
                                    later.timeline_start.0.saturating_sub(clip.duration.0);
                            }
                        }
                        removed += 1;
                    }
                    clips.sort_by_key(|item| item.timeline_start);
                }
                Ok(json!({"removed": removed, "ripple": params.ripple}))
            })()),
            "timeline.set_mark_in" => Some((|| {
                let params: SetMarkParams = parse_params(params)?;
                self.checkpoint();
                self.document.project.sequence.mark_in = Some(params.at);
                Ok(json!({"mark_in": params.at}))
            })()),
            "timeline.set_mark_out" => Some((|| {
                let params: SetMarkParams = parse_params(params)?;
                self.checkpoint();
                self.document.project.sequence.mark_out = Some(params.at);
                Ok(json!({"mark_out": params.at}))
            })()),
            "timeline.clear_marks" => Some((|| {
                self.checkpoint();
                self.document.project.sequence.mark_in = None;
                self.document.project.sequence.mark_out = None;
                Ok(json!({"cleared": true}))
            })()),
            "timeline.clear_mark_in" => Some((|| {
                self.checkpoint();
                self.document.project.sequence.mark_in = None;
                Ok(json!({"mark_in": serde_json::Value::Null}))
            })()),
            "timeline.clear_mark_out" => Some((|| {
                self.checkpoint();
                self.document.project.sequence.mark_out = None;
                Ok(json!({"mark_out": serde_json::Value::Null}))
            })()),
            "timeline.mark_clip" => Some((|| {
                let params: MarkClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let mark_in = clip.timeline_start;
                let mark_out = Tick(clip.timeline_start.0.saturating_add(clip.duration.0));
                self.checkpoint();
                self.document.project.sequence.mark_in = Some(mark_in);
                self.document.project.sequence.mark_out = Some(mark_out);
                Ok(json!({"mark_in": mark_in.0, "mark_out": mark_out.0}))
            })()),
            "timeline.mark_selection" => Some((|| {
                let params: MarkSelectionParams = parse_params(params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Marking a selection needs at least one clip".into(),
                    ));
                }
                let mut mark_in = Option::<Tick>::None;
                let mut mark_out = Option::<Tick>::None;
                for reference in &params.clips {
                    let track_index = find_track_index(&self.document, reference.track_id)?;
                    let clip = find_clip(&self.document, track_index, reference.clip_index)?;
                    let start = clip.timeline_start;
                    let end = Tick(clip.timeline_start.0.saturating_add(clip.duration.0));
                    mark_in = Some(match mark_in {
                        Some(current) if current.0 <= start.0 => current,
                        _ => start,
                    });
                    mark_out = Some(match mark_out {
                        Some(current) if current.0 >= end.0 => current,
                        _ => end,
                    });
                }
                self.checkpoint();
                self.document.project.sequence.mark_in = mark_in;
                self.document.project.sequence.mark_out = mark_out;
                Ok(json!({"mark_in": mark_in.unwrap().0, "mark_out": mark_out.unwrap().0}))
            })()),
            "timeline.ripple_delete_ranges" => Some((|| {
                let params: RippleDeleteRangesParams = parse_params(params)?;
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(
                    &mut self.document.project.sequence,
                    &params.ranges,
                )
                .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            })()),
            "timeline.add_track" => Some((|| {
                let params: AddTrackParams = parse_params(params)?;
                self.checkpoint();
                let track_id = self.document.project.next_id;
                self.document.project.next_id = track_id.saturating_add(1);
                let count = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .filter(|t| t.kind == params.kind)
                    .count();
                let default_prefix = match params.kind {
                    TrackKind::Video => "V",
                    TrackKind::Audio => "A",
                };
                let track_name = params
                    .name
                    .unwrap_or_else(|| format!("{}{}", default_prefix, count + 1));
                let new_track = Track {
                    id: track_id,
                    group_id: None,
                    kind: params.kind,
                    name: track_name.clone(),
                    gain_db: 0.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                    locked: false,
                    audio_processing: crate::project::TrackAudioProcessing::default(),
                    bus: crate::project::BusTarget::Master,
                    automation: crate::project::TrackAutomation::default(),
                    clips: Vec::new(),
                    transitions: Vec::new(),
                };
                self.document.project.sequence.tracks.push(new_track);
                Ok(json!({
                    "added": true,
                    "track_id": track_id,
                    "name": track_name,
                    "kind": params.kind,
                }))
            })()),
            "timeline.remove_track" => Some((|| {
                let params: RemoveTrackParams = parse_params(params)?;
                let track_idx = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|t| t.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Track {} not found",
                            params.track_id
                        ))
                    })?;
                let track = &self.document.project.sequence.tracks[track_idx];
                if track.locked {
                    return Err(DispatchError::Operation(
                        "Cannot remove locked track".into(),
                    ));
                }
                let kind = track.kind;
                let kind_count = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .filter(|t| t.kind == kind)
                    .count();
                if kind_count <= 1 {
                    return Err(DispatchError::Operation(
                        "Sequence must have at least one track of each kind".into(),
                    ));
                }
                self.checkpoint();
                self.document.project.sequence.tracks.remove(track_idx);
                Ok(json!({
                    "removed": true,
                    "track_id": params.track_id,
                }))
            })()),
            "timeline.ripple_delete_gap" => Some((|| {
                let params: RippleDeleteGapParams = parse_params(params)?;
                let at = params.at_tick.0;

                // Determine gap boundary [gap_start, gap_end]
                let (gap_start, gap_end) = if let Some(t_id) = params.track_id {
                    let t_idx = find_track_index(&self.document, t_id)?;
                    let track = &self.document.project.sequence.tracks[t_idx];
                    if track.locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    let mut clips = track.clips.clone();
                    clips.sort_by_key(|c| c.timeline_start.0);

                    if clips.is_empty() {
                        return Err(DispatchError::Operation("Track contains no clips".into()));
                    }

                    // Check before first clip
                    if at >= 0 && at < clips[0].timeline_start.0 {
                        (0, clips[0].timeline_start.0)
                    } else {
                        let mut found = None;
                        for i in 0..clips.len() - 1 {
                            let end_prev = clips[i].timeline_start.0 + clips[i].duration.0;
                            let start_next = clips[i + 1].timeline_start.0;
                            if at >= end_prev && at < start_next {
                                found = Some((end_prev, start_next));
                                break;
                            }
                        }
                        found.ok_or_else(|| {
                            DispatchError::Operation(
                                "No gap found at specified playhead time".into(),
                            )
                        })?
                    }
                } else {
                    // Global gap across all tracks
                    let mut max_end_before: i64 = 0;
                    let mut min_start_after: i64 = i64::MAX;
                    let mut has_clips_before = false;
                    let mut has_clips_after = false;

                    for track in &self.document.project.sequence.tracks {
                        for clip in &track.clips {
                            let start = clip.timeline_start.0;
                            let end = start + clip.duration.0;
                            if at >= start && at < end {
                                return Err(DispatchError::Operation(
                                    "Playhead is inside a clip, not an empty gap".into(),
                                ));
                            }
                            if end <= at {
                                max_end_before = max_end_before.max(end);
                                has_clips_before = true;
                            }
                            if start > at {
                                min_start_after = min_start_after.min(start);
                                has_clips_after = true;
                            }
                        }
                    }

                    let start = if has_clips_before { max_end_before } else { 0 };
                    let end = if has_clips_after {
                        min_start_after
                    } else {
                        return Err(DispatchError::Operation(
                            "No clips found after playhead to ripple".into(),
                        ));
                    };

                    if end <= start {
                        return Err(DispatchError::Operation("Invalid gap duration".into()));
                    }
                    (start, end)
                };

                let gap_duration = gap_end - gap_start;
                if gap_duration <= 0 {
                    return Err(DispatchError::Operation("Gap duration is zero".into()));
                }

                self.checkpoint();

                // Shift clips to close the gap
                if let Some(t_id) = params.track_id {
                    let t_idx = find_track_index(&self.document, t_id)?;
                    for clip in &mut self.document.project.sequence.tracks[t_idx].clips {
                        if clip.timeline_start.0 >= gap_end {
                            clip.timeline_start = Tick(clip.timeline_start.0 - gap_duration);
                        }
                    }
                    for trans in &mut self.document.project.sequence.tracks[t_idx].transitions {
                        if trans.cut_at.0 >= gap_end {
                            trans.cut_at = Tick(trans.cut_at.0 - gap_duration);
                        }
                    }
                } else {
                    for track in &mut self.document.project.sequence.tracks {
                        if !track.locked {
                            for clip in &mut track.clips {
                                if clip.timeline_start.0 >= gap_end {
                                    clip.timeline_start =
                                        Tick(clip.timeline_start.0 - gap_duration);
                                }
                            }
                            for trans in &mut track.transitions {
                                if trans.cut_at.0 >= gap_end {
                                    trans.cut_at = Tick(trans.cut_at.0 - gap_duration);
                                }
                            }
                        }
                    }
                }

                Ok(json!({
                    "gap_deleted": true,
                    "gap_start": gap_start,
                    "gap_end": gap_end,
                    "duration_ticks": gap_duration,
                }))
            })()),
            "timeline.create_track_group" => Some((|| {
                let params: CreateTrackGroupParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Group name cannot be empty".into(),
                    ));
                }
                self.checkpoint();
                let group_id = self.document.project.next_id;
                self.document.project.next_id = group_id.saturating_add(1);
                let group = TrackGroup {
                    id: group_id,
                    name: params.name.trim().to_string(),
                    color: params.color.unwrap_or_else(|| "#4a5568".to_string()),
                    collapsed: false,
                    muted: false,
                    solo: false,
                    locked: false,
                };
                self.document
                    .project
                    .sequence
                    .track_groups
                    .push(group.clone());
                Ok(json!({
                    "created": true,
                    "group": group,
                }))
            })()),
            "timeline.assign_track_group" => Some((|| {
                let params: AssignTrackGroupParams = parse_params(params)?;
                if let Some(gid) = params.group_id {
                    if !self
                        .document
                        .project
                        .sequence
                        .track_groups
                        .iter()
                        .any(|g| g.id == gid)
                    {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Track group {} not found",
                            gid
                        )));
                    }
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].group_id = params.group_id;
                Ok(json!({
                    "updated": true,
                    "track_id": params.track_id,
                    "group_id": params.group_id,
                }))
            })()),
            "timeline.toggle_track_group_collapsed" => Some((|| {
                let params: ToggleTrackGroupCollapsedParams = parse_params(params)?;
                let group_idx = self
                    .document
                    .project
                    .sequence
                    .track_groups
                    .iter()
                    .position(|g| g.id == params.group_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Track group {} not found",
                            params.group_id
                        ))
                    })?;
                self.checkpoint();
                let group = &mut self.document.project.sequence.track_groups[group_idx];
                group.collapsed = !group.collapsed;
                let collapsed = group.collapsed;
                Ok(json!({
                    "updated": true,
                    "group_id": params.group_id,
                    "collapsed": collapsed,
                }))
            })()),
            "timeline.delete_track_group" => Some((|| {
                let params: DeleteTrackGroupParams = parse_params(params)?;
                let pos = self
                    .document
                    .project
                    .sequence
                    .track_groups
                    .iter()
                    .position(|g| g.id == params.group_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Track group {} not found",
                            params.group_id
                        ))
                    })?;
                self.checkpoint();
                self.document.project.sequence.track_groups.remove(pos);
                for track in &mut self.document.project.sequence.tracks {
                    if track.group_id == Some(params.group_id) {
                        track.group_id = None;
                    }
                }
                Ok(json!({
                    "deleted": true,
                    "group_id": params.group_id,
                }))
            })()),
            "timeline.set_track_group_properties" => Some((|| {
                let params: SetTrackGroupPropertiesParams = parse_params(params)?;
                let group_idx = self
                    .document
                    .project
                    .sequence
                    .track_groups
                    .iter()
                    .position(|g| g.id == params.group_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Track group {} not found",
                            params.group_id
                        ))
                    })?;
                self.checkpoint();
                let group = &mut self.document.project.sequence.track_groups[group_idx];
                if let Some(name) = params.name {
                    group.name = name;
                }
                if let Some(color) = params.color {
                    group.color = color;
                }
                if let Some(muted) = params.muted {
                    group.muted = muted;
                }
                if let Some(solo) = params.solo {
                    group.solo = solo;
                }
                if let Some(locked) = params.locked {
                    group.locked = locked;
                }
                let updated_group = group.clone();
                if let Some(muted) = params.muted {
                    for t in &mut self.document.project.sequence.tracks {
                        if t.group_id == Some(params.group_id) {
                            t.muted = muted;
                        }
                    }
                }
                if let Some(solo) = params.solo {
                    for t in &mut self.document.project.sequence.tracks {
                        if t.group_id == Some(params.group_id) {
                            t.solo = solo;
                        }
                    }
                }
                if let Some(locked) = params.locked {
                    for t in &mut self.document.project.sequence.tracks {
                        if t.group_id == Some(params.group_id) {
                            t.locked = locked;
                        }
                    }
                }
                Ok(json!({
                    "updated": true,
                    "group": updated_group,
                }))
            })()),
            _ => None,
        }
    }
}
