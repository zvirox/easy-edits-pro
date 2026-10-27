use super::*;

impl EditorEngine {
    pub(super) fn dispatch_effects(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "clip.add_effect" => Some((|| {
                let params: AddEffectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let effect_id = self.document.project.next_id;
                self.document.project.next_id = effect_id.saturating_add(1);
                let effect = match params.kind {
                    EffectKind::ColorCorrection => {
                        let mut inst = EffectInstance::new_color_correction(
                            effect_id,
                            ColorCorrectionParams::default(),
                        );
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::GaussianBlur => {
                        let mut inst = EffectInstance::new_gaussian_blur(
                            effect_id,
                            GaussianBlurParams::default(),
                        );
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::Crop => {
                        let mut inst = EffectInstance::new_crop(effect_id, CropParams::default());
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::Vignette => {
                        let mut inst =
                            EffectInstance::new_vignette(effect_id, VignetteParams::default());
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::ChromaKey => {
                        let mut inst =
                            EffectInstance::new_chroma_key(effect_id, ChromaKeyParams::default());
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::Lut => {
                        let mut inst = EffectInstance::new_lut(effect_id, LutParams::default());
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                    EffectKind::CustomShader => {
                        let mut inst = EffectInstance::new_custom_shader(
                            effect_id,
                            crate::project::CustomShaderParams::default(),
                        );
                        if let Some(name) = params.name {
                            inst.name = name;
                        }
                        inst
                    }
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .effects
                    .push(effect);
                Ok(json!({"added": true, "effect_id": effect_id}))
            })()),
            "clip.remove_effect" => Some((|| {
                let params: RemoveEffectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Ok(json!({"removed": false}));
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .effects
                    .remove(pos);
                Ok(json!({"removed": true}))
            })()),
            "clip.update_effect" => Some((|| {
                let params: UpdateEffectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Err(DispatchError::Operation("Effect could not be found".into()));
                };
                self.checkpoint();
                let target = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index]
                    .effects[pos];
                target.payload = params.payload.clone();
                if let EffectPayload::ColorCorrection(ref c) = params.payload {
                    let clip_ref = &mut self.document.project.sequence.tracks[track_index].clips
                        [params.clip_index];
                    clip_ref.color.exposure = c.exposure;
                    clip_ref.color.contrast = c.contrast;
                    clip_ref.color.saturation = c.saturation;
                    clip_ref.color.hue = c.hue;
                    clip_ref.color.wheels = c.wheels.clone();
                }
                Ok(json!({"updated": true}))
            })()),
            "clip.toggle_effect" => Some((|| {
                let params: ToggleEffectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Err(DispatchError::Operation("Effect could not be found".into()));
                };
                self.checkpoint();
                let target = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index]
                    .effects[pos];
                target.enabled = params.enabled.unwrap_or(!target.enabled);
                let enabled = target.enabled;
                Ok(json!({"updated": true, "enabled": enabled}))
            })()),
            "clip.reorder_effects" => Some((|| {
                let params: ReorderEffectsParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let total = clip.effects.len();
                if params.from_index >= total || params.to_index >= total {
                    return Err(DispatchError::InvalidParameters(
                        "Effect index out of bounds".into(),
                    ));
                }
                if params.from_index != params.to_index {
                    self.checkpoint();
                    let effect = self.document.project.sequence.tracks[track_index].clips
                        [params.clip_index]
                        .effects
                        .remove(params.from_index);
                    self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .effects
                        .insert(params.to_index, effect);
                }
                Ok(json!({"reordered": true}))
            })()),
            "timeline.add_transition" => Some((|| {
                let params: AddTransitionParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let duration = params.duration.unwrap_or(Tick(TICKS_PER_SECOND));
                let kind = params.kind.unwrap_or(TransitionKind::Crossfade);
                if duration.0 <= 0 || params.cut_at.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Transition duration and position must be positive".into(),
                    ));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let track = &mut self.document.project.sequence.tracks[track_index];
                track
                    .transitions
                    .retain(|t| (t.cut_at.0 - params.cut_at.0).abs() > TICKS_PER_SECOND / 10);
                track.transitions.push(Transition {
                    id,
                    kind,
                    cut_at: params.cut_at,
                    duration,
                });
                Ok(json!({
                    "transition_id": id,
                    "track_id": params.track_id,
                    "cut_at": params.cut_at,
                    "duration": duration,
                    "kind": kind,
                }))
            })()),
            "timeline.remove_transition" => Some((|| {
                let params: RemoveTransitionParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                let initial_count = track.transitions.len();
                track.transitions.retain(|t| t.id != params.transition_id);
                Ok(json!({"removed": track.transitions.len() < initial_count}))
            })()),
            "timeline.update_transition" => Some((|| {
                let params: UpdateTransitionParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let position = track
                    .transitions
                    .iter()
                    .position(|item| item.id == params.transition_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Transition {} does not exist",
                            params.transition_id
                        ))
                    })?;
                if params.duration.is_some_and(|duration| duration.0 <= 0) {
                    return Err(DispatchError::InvalidParameters(
                        "Transition duration must be positive".into(),
                    ));
                }
                if params.cut_at.is_some_and(|cut| cut.0 < 0) {
                    return Err(DispatchError::InvalidParameters(
                        "Transition position cannot be negative".into(),
                    ));
                }
                self.checkpoint();
                let transition =
                    &mut self.document.project.sequence.tracks[track_index].transitions[position];
                if let Some(kind) = params.kind {
                    transition.kind = kind;
                }
                if let Some(duration) = params.duration {
                    transition.duration = duration;
                }
                if let Some(cut_at) = params.cut_at {
                    transition.cut_at = cut_at;
                }
                Ok(json!({
                    "updated": true,
                    "transition_id": transition.id,
                    "kind": transition.kind,
                    "duration": transition.duration,
                    "cut_at": transition.cut_at,
                }))
            })()),
            "timeline.add_title" => Some((|| {
                let params: AddTitleParams = parse_params(params)?;
                let track_id = params.track_id.unwrap_or(1);
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|t| t.id == track_id && matches!(t.kind, TrackKind::Video))
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("Target video track does not exist".into())
                    })?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }

                self.checkpoint();
                let media_id = self.document.project.next_id;
                self.document.project.next_id = media_id.saturating_add(1);
                let duration = params.duration.unwrap_or(Tick(TICKS_PER_SECOND * 5));
                let title_props = TitleProperties {
                    text: params.text.clone(),
                    subtitle: params.subtitle,
                    style: params.style.unwrap_or_else(|| "lower_third".into()),
                    font_family: "Inter".into(),
                    font_size: 42.0,
                    color: "#ffffff".into(),
                    bg_color: Some("rgba(15, 23, 42, 0.75)".into()),
                };
                let media_item = MediaAsset {
                    id: media_id,
                    name: format!(
                        "Title: {}",
                        params.text.chars().take(20).collect::<String>()
                    ),
                    path: PathBuf::from(format!("title://{}", media_id)),
                    kind: MediaKind::Title,
                    duration: Some(duration),
                    width: Some(self.document.project.settings.width),
                    height: Some(self.document.project.settings.height),
                    title: Some(title_props),
                    ..Default::default()
                };
                self.document.project.media.push(media_item);

                let track = &self.document.project.sequence.tracks[track_index];
                let timeline_start = params.timeline_start.unwrap_or_else(|| {
                    track
                        .clips
                        .iter()
                        .map(|c| c.timeline_start.0 + c.duration.0)
                        .max()
                        .unwrap_or(0)
                        .into()
                });
                let clip = Clip {
                    media_id,
                    timeline_start,
                    source_in: Tick(0),
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
                };
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips.push(clip);
                track.clips.sort_by_key(|c| c.timeline_start);
                let clip_index = track
                    .clips
                    .iter()
                    .position(|c| c.media_id == media_id && c.timeline_start == timeline_start)
                    .unwrap_or(0);
                Ok(json!({
                    "media_id": media_id,
                    "track_id": track_id,
                    "clip_index": clip_index,
                }))
            })()),
            "title.update" => Some((|| {
                let params: UpdateTitleParams = parse_params(params)?;
                let media_index = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|m| m.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                if !matches!(
                    self.document.project.media[media_index].kind,
                    MediaKind::Title
                ) {
                    return Err(DispatchError::InvalidParameters(
                        "Media is not a title clip".into(),
                    ));
                }
                self.checkpoint();
                let media = &mut self.document.project.media[media_index];
                let mut title = media.title.clone().unwrap_or_default();
                if let Some(t) = params.text {
                    media.name = format!("Title: {}", t.chars().take(20).collect::<String>());
                    title.text = t;
                }
                if let Some(sub) = params.subtitle {
                    title.subtitle = Some(sub);
                }
                if let Some(st) = params.style {
                    title.style = st;
                }
                if let Some(sz) = params.font_size {
                    title.font_size = sz;
                }
                if let Some(col) = params.color {
                    title.color = col;
                }
                if let Some(bg) = params.bg_color {
                    title.bg_color = Some(bg);
                }
                media.title = Some(title.clone());
                Ok(json!({"media_id": params.media_id, "title": title}))
            })()),
            "multicam.create" => Some((|| {
                let params: CreateMulticamParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Multicam name cannot be empty".into(),
                    ));
                }
                if params.angles.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Multicam must contain at least one angle".into(),
                    ));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let active_angle_id = params.angles[0].id;
                let group = MultiCamClip {
                    id,
                    name: params.name,
                    angles: params.angles,
                    active_angle_id,
                    sync_mode: params.sync_mode,
                };
                self.document.project.sequence.multicam_clips.push(group);
                Ok(json!({"id": id, "active_angle_id": active_angle_id}))
            })()),
            "multicam.switch_angle" => Some((|| {
                let params: SwitchMulticamAngleParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let clip =
                    &self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                let group = if let Some(m_id) = clip.multicam_id {
                    self.document
                        .project
                        .sequence
                        .multicam_clips
                        .iter()
                        .find(|g| g.id == m_id)
                } else {
                    self.document
                        .project
                        .sequence
                        .multicam_clips
                        .iter()
                        .find(|g| g.angles.iter().any(|a| a.id == params.angle_id))
                }
                .ok_or_else(|| {
                    DispatchError::InvalidParameters("Multicam group not found".into())
                })?;
                let angle = group
                    .angles
                    .iter()
                    .find(|a| a.id == params.angle_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Angle {} not found in multicam",
                            params.angle_id
                        ))
                    })?;
                let group_id = group.id;
                let angle_media_id = angle.media_id;
                let sync_offset = angle.sync_offset_ticks;
                self.checkpoint();
                let clip_mut = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip_mut.multicam_id = Some(group_id);
                clip_mut.multicam_angle_id = Some(params.angle_id);
                clip_mut.media_id = angle_media_id;
                clip_mut.source_in = sync_offset;
                Ok(
                    json!({"switched": true, "media_id": angle_media_id, "angle_id": params.angle_id}),
                )
            })()),
            "multicam.sync" => Some((|| {
                let params: SyncMulticamParams = parse_params(params)?;
                let pos = self
                    .document
                    .project
                    .sequence
                    .multicam_clips
                    .iter()
                    .position(|g| g.id == params.multicam_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Multicam {} does not exist",
                            params.multicam_id
                        ))
                    })?;
                self.checkpoint();
                let group = &mut self.document.project.sequence.multicam_clips[pos];
                group.sync_mode = params.sync_mode;
                if let Some(offsets) = params.offsets {
                    for angle in &mut group.angles {
                        if let Some(&offset) = offsets.get(&angle.id) {
                            angle.sync_offset_ticks = Tick(offset.max(0));
                        }
                    }
                }
                Ok(json!({"synced": true, "multicam_id": params.multicam_id}))
            })()),
            "multicam.flatten" => Some((|| {
                let params: EditClipTargetParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let media_id = clip.media_id;
                let multicam_id = clip.multicam_id;
                let angle_id = clip.multicam_angle_id;
                let Some(group_id) = multicam_id else {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip is not a multicam clip".into(),
                    ));
                };
                let group = self
                    .document
                    .project
                    .sequence
                    .multicam_clips
                    .iter()
                    .find(|g| g.id == group_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("Multicam group not found".into())
                    })?;
                // A clip switched to an angle already carries that angle's
                // media; otherwise resolve the group's active angle.
                let resolved_media_id = if angle_id.is_some() {
                    media_id
                } else {
                    group
                        .angles
                        .iter()
                        .find(|a| a.id == group.active_angle_id)
                        .map(|a| a.media_id)
                        .unwrap_or(media_id)
                };
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.media_id = resolved_media_id;
                clip.multicam_id = None;
                clip.multicam_angle_id = None;
                Ok(json!({
                    "flattened": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "media_id": clip.media_id,
                }))
            })()),
            "clip.set_motion_tracking" => Some((|| {
                let params: SetMotionTrackingParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.tracking = Some(params.tracking);
                Ok(json!({"updated": true}))
            })()),
            "clip.pin_to_track" => Some((|| {
                let params: PinToTrackParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                if let Some(ref mut tr) = clip.tracking {
                    tr.pinned_clip_id = params.pinned_clip_id;
                } else {
                    let tr = MotionTrackingData {
                        id: 1,
                        name: "Track Point".into(),
                        mode: TrackingMode::Pin,
                        points: Vec::new(),
                        pinned_clip_id: params.pinned_clip_id,
                        smoothness: 0.5,
                    };
                    clip.tracking = Some(tr);
                }
                Ok(json!({"pinned": true, "pinned_clip_id": params.pinned_clip_id}))
            })()),

            "clip.set_speed_ramp" => Some((|| {
                let params: SetSpeedRampParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.speed_ramp = Some(params.speed_ramp);
                Ok(json!({"updated": true}))
            })()),
            "clip.set_motion_blur" => Some((|| {
                let params: SetMotionBlurParams = parse_params(params)?;
                if !params.motion_blur.shutter_angle_deg.is_finite()
                    || !(0.0..=360.0).contains(&params.motion_blur.shutter_angle_deg)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Shutter angle must be between 0 and 360 degrees".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.motion_blur = Some(params.motion_blur);
                Ok(json!({"updated": true}))
            })()),
            _ => None,
        }
    }
}
