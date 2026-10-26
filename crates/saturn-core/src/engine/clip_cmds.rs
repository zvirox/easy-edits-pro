use super::*;

impl EditorEngine {
    pub(super) fn dispatch_clip(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "timeline.add_clip" => Some((|| {
                let params: AddClipParams = parse_params(params)?;
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
                let default_kind = match &media.kind {
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
                                std::mem::discriminant(&track.kind)
                                    == std::mem::discriminant(&default_kind)
                            })
                    })
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("No matching timeline track exists".into())
                    })?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let duration = params
                    .duration
                    .or(media_duration)
                    .unwrap_or(Tick(TICKS_PER_SECOND * 5));
                if duration.0 <= 0 || params.source_in.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Clip source and duration must be positive".into(),
                    ));
                }
                if params.timeline_start.is_some_and(|start| start.0 < 0) {
                    return Err(DispatchError::InvalidParameters(
                        "Clip timeline position cannot be negative".into(),
                    ));
                }
                if media_duration
                    .is_some_and(|length| params.source_in.0.saturating_add(duration.0) > length.0)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Clip range exceeds the media duration".into(),
                    ));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let timeline_start =
                    params
                        .timeline_start
                        .map(|start| start.0)
                        .unwrap_or_else(|| {
                            track
                                .clips
                                .iter()
                                .filter_map(|clip| {
                                    clip.timeline_start.0.checked_add(clip.duration.0)
                                })
                                .max()
                                .unwrap_or(0)
                        });
                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips.push(Clip {
                    media_id: params.media_id,
                    timeline_start: Tick(timeline_start),
                    source_in: params.source_in,
                    duration,
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
                label: String::new(),
                });
                Ok(json!({"track_id": track.id, "clip_index": track.clips.len() - 1}))
            })()),
            "timeline.remove_clip" => Some((|| {
                let params: RemoveClipParams = parse_params(params)?;
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                self.checkpoint();
                self.document.project.sequence.tracks[track_index]
                    .clips
                    .remove(params.clip_index);
                Ok(json!({"removed": true}))
            })()),
            "timeline.split_clip" => Some((|| {
                let params: SplitClipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let clip = track.clips.get(params.clip_index).ok_or_else(|| {
                    DispatchError::InvalidParameters("Timeline clip does not exist".into())
                })?;
                let split_offset = params.at.0.saturating_sub(clip.timeline_start.0);
                if split_offset <= 0 || split_offset >= clip.duration.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Playhead must be inside the selected clip, away from its edges".into(),
                    ));
                }
                let mut left = clip.clone();
                let mut right = clip.clone();
                left.duration = Tick(split_offset);
                right.timeline_start = params.at;
                right.source_in.0 = right.source_in.0.saturating_add(split_offset);
                right.duration.0 = clip.duration.0 - split_offset;
                let mut properties = Vec::new();
                for key in &clip.keyframes {
                    if !properties.contains(&key.property) {
                        properties.push(key.property.clone());
                    }
                }
                for property in &properties {
                    if let Some(value) = keyframe_value_at(&clip.keyframes, property, split_offset)
                    {
                        if !left
                            .keyframes
                            .iter()
                            .any(|key| &key.property == property && key.time.0 == split_offset)
                        {
                            left.keyframes.push(Keyframe::new(
                                property.clone(),
                                Tick(split_offset),
                                value,
                            ));
                        }
                    }
                }
                left.keyframes.retain(|key| key.time.0 <= split_offset);
                right.keyframes.retain(|key| key.time.0 > split_offset);
                for key in &mut right.keyframes {
                    key.time.0 -= split_offset;
                }
                for property in properties {
                    if let Some(value) = keyframe_value_at(&clip.keyframes, &property, split_offset)
                    {
                        right
                            .keyframes
                            .push(Keyframe::new(property, Tick(0), value));
                    }
                }
                self.checkpoint();
                let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                clips[params.clip_index] = left;
                clips.insert(params.clip_index + 1, right);
                Ok(
                    json!({"split": true, "track_id": params.track_id, "left_index": params.clip_index, "right_index": params.clip_index + 1}),
                )
            })()),

            "timeline.move_clips_batch" => Some((|| {
                let params: MoveClipsBatchParams = parse_params(params)?;
                if params.moves.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "No moves specified".into(),
                    ));
                }
                for item in &params.moves {
                    if item.timeline_start.0 < 0 {
                        return Err(DispatchError::InvalidParameters(
                            "A clip cannot start before time zero".into(),
                        ));
                    }
                    let t_idx = find_track_index(&self.document, item.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if item.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters(
                            "Timeline clip does not exist".into(),
                        ));
                    }
                }

                self.checkpoint();
                for item in params.moves {
                    let t_idx = find_track_index(&self.document, item.track_id)?;
                    self.document.project.sequence.tracks[t_idx].clips[item.clip_index]
                        .timeline_start = item.timeline_start;
                }
                for track in &mut self.document.project.sequence.tracks {
                    track.clips.sort_by_key(|c| c.timeline_start);
                }

                Ok(json!({"moved": true}))
            })()),
            "timeline.move_clip" => Some((|| {
                let params: MoveClipParams = parse_params(params)?;
                if params.timeline_start.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "A clip cannot start before time zero".into(),
                    ));
                }
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                let target_track_index = params
                    .target_track_id
                    .map(|target_id| {
                        self.document
                            .project
                            .sequence
                            .tracks
                            .iter()
                            .position(|track| track.id == target_id)
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Timeline track {target_id} does not exist"
                                ))
                            })
                    })
                    .transpose()?
                    .unwrap_or(track_index);
                if self.document.project.sequence.tracks[track_index].locked
                    || self.document.project.sequence.tracks[target_track_index].locked
                {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                let media_id = self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index]
                    .media_id;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {media_id} does not exist"
                        ))
                    })?;
                let target_kind = &self.document.project.sequence.tracks[target_track_index].kind;
                let kind_matches = matches!(
                    (&media.kind, target_kind),
                    (MediaKind::Audio, TrackKind::Audio)
                        | (
                            MediaKind::Video
                                | MediaKind::Image
                                | MediaKind::Title
                                | MediaKind::Unknown,
                            TrackKind::Video
                        )
                );
                if !kind_matches {
                    return Err(DispatchError::InvalidParameters(
                        "Audio media must be placed on an audio track, and visual media on a video track".into(),
                    ));
                }
                self.checkpoint();
                let mut target_track_id = params.track_id;
                if track_index == target_track_index {
                    self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .timeline_start = params.timeline_start;
                } else {
                    let mut clip = self.document.project.sequence.tracks[track_index]
                        .clips
                        .remove(params.clip_index);
                    clip.timeline_start = params.timeline_start;
                    let target = &mut self.document.project.sequence.tracks[target_track_index];
                    target.clips.push(clip);
                    target.clips.sort_by_key(|item| item.timeline_start);
                    target_track_id = target.id;
                }
                let target = &mut self.document.project.sequence.tracks[target_track_index];
                if track_index == target_track_index {
                    target.clips.sort_by_key(|item| item.timeline_start);
                }
                let clip_index = target
                    .clips
                    .iter()
                    .position(|clip| {
                        clip.media_id == media_id && clip.timeline_start == params.timeline_start
                    })
                    .ok_or_else(|| {
                        DispatchError::Operation("Moved clip could not be found".into())
                    })?;
                Ok(json!({"moved": true, "track_id": target_track_id, "clip_index": clip_index}))
            })()),
            _ => None,
        }
    }
}
