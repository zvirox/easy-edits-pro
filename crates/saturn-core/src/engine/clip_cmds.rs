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
                volume_db: 0.0,
                enabled: true,
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
                {
                    let clip = find_clip(&self.document, track_index, params.clip_index)?;
                    let offset = params.at.0.saturating_sub(clip.timeline_start.0);
                    if offset <= 0 || offset >= clip.duration.0 {
                        return Err(DispatchError::InvalidParameters(
                            "Playhead must be inside the selected clip, away from its edges"
                                .into(),
                        ));
                    }
                }
                self.checkpoint();
                apply_clip_split(
                    &mut self.document,
                    track_index,
                    params.clip_index,
                    params.at,
                )?;
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
            "clip.set_volume_db" => Some((|| {
                let params: SetClipVolumeParams = parse_params(params)?;
                if !params.volume_db.is_finite() || !(-60.0..=12.0).contains(&params.volume_db) {
                    return Err(DispatchError::InvalidParameters(
                        "Clip volume must be between -60 and +12 dB".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.volume_db = params.volume_db;
                Ok(json!({
                    "clip_index": params.clip_index,
                    "track_id": params.track_id,
                    "volume_db": clip.volume_db,
                }))
            })()),
            "clip.set_enabled" => Some((|| {
                let params: SetClipEnabledParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.enabled = params.enabled.unwrap_or(!clip.enabled);
                Ok(json!({
                    "clip_index": params.clip_index,
                    "track_id": params.track_id,
                    "enabled": clip.enabled,
                }))
            })()),
            "timeline.replace_media" => Some((|| {
                let params: ReplaceMediaParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                if !self
                    .document
                    .project
                    .media
                    .iter()
                    .any(|item| item.id == params.media_id)
                {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Media item {} does not exist",
                        params.media_id
                    )));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.media_id = params.media_id;
                // The replacement media is not a multicam angle; drop stale
                // multicam linkage the same way a source relink would.
                clip.multicam_id = None;
                clip.multicam_angle_id = None;
                Ok(json!({
                    "replaced": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "media_id": clip.media_id,
                }))
            })()),
            "clip.make_subclip" => Some((|| {
                let params: MakeSubclipParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let source = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == clip.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline clip references missing media {}",
                            clip.media_id
                        ))
                    })?;
                let name = params.name.unwrap_or_else(|| {
                    format!("Subclip of {}", source.name)
                });
                if name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Subclip name cannot be empty".into(),
                    ));
                }
                let clip_duration = clip.duration;
                let clip_source_in = clip.source_in;
                let mut subclip = source.clone();
                subclip.id = 0;
                subclip.name = name;
                subclip.duration = Some(clip_duration);
                subclip.tags = vec!["subclip".into()];
                subclip.comment = format!(
                    "Made from clip at {} on track {}",
                    clip.timeline_start.0, params.track_id
                );
                subclip.source_mark_in = Some(clip_source_in);
                subclip.source_mark_out = Some(Tick(
                    clip_source_in.0.saturating_add(clip_duration.0),
                ));
                subclip.title = None;
                subclip.nested_sequence = None;
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                subclip.id = id;
                let subclip_id = subclip.id;
                let subclip_name = subclip.name.clone();
                self.document.project.media.push(subclip);
                Ok(json!({
                    "media_id": subclip_id,
                    "name": subclip_name,
                    "duration": clip_duration.0,
                }))
            })()),
            "clip.frame_fit" => Some((|| {
                let params: FrameFitParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let source = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == clip.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline clip references missing media {}",
                            clip.media_id
                        ))
                    })?;
                let (Some(media_w), Some(media_h)) = (source.width, source.height) else {
                    return Err(DispatchError::InvalidParameters(
                        "Media has no intrinsic dimensions for frame fit".into(),
                    ));
                };
                if media_w == 0 || media_h == 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Media dimensions must be positive".into(),
                    ));
                }
                // GES letterbox-fits media into the project frame, so a scale
                // multiplier of 1.0 is the "fit" state and filling needs the
                // cover factor over that fit.
                let settings = &self.document.project.settings;
                let media_aspect = media_w as f64 / media_h as f64;
                let frame_aspect = settings.width as f64 / settings.height as f64;
                let scale = match params.mode {
                    FrameFitMode::Fit | FrameFitMode::Scale100 => 1.0,
                    FrameFitMode::Fill => {
                        let raw = if media_aspect >= frame_aspect {
                            media_aspect / frame_aspect
                        } else {
                            frame_aspect / media_aspect
                        };
                        raw.clamp(0.01, 100.0)
                    }
                };
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.keyframes
                    .retain(|key| key.property != KeyframeProperty::Scale);
                clip.keyframes
                    .push(Keyframe::new(KeyframeProperty::Scale, Tick(0), scale));
                Ok(json!({
                    "mode": match params.mode {
                        FrameFitMode::Fill => "fill",
                        FrameFitMode::Fit => "fit",
                        FrameFitMode::Scale100 => "scale100",
                    },
                    "scale": scale,
                    "clip_index": params.clip_index,
                }))
            })()),
            _ => None,
        }
    }
}
