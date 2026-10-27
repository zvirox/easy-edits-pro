//! Edit-family clip commands: FC-style label colours, the attribute
//! clipboard (copy/paste/remove attributes), duplicate, consolidate
//! duplicates, ripple-delete-head, and unused-media removal.
//!
//! The label colours are engine-validated against `LABEL_COLORS`; the
//! attribute clipboard is editor state (not document state), mirroring the
//! trim family's `TrimState`.

use super::*;

/// The sixteen reference-editor label colour names, shared by the engine validator and the
/// command catalogue (`edit.label.<color>`).
pub const LABEL_COLORS: [&str; 16] = [
    "blue", "brown", "caribbean", "cerulean", "forest", "green", "iris", "lavender", "magenta",
    "mango", "purple", "rose", "tan", "teal", "violet", "yellow",
];

/// Attributes transferred by `edit.copy_attributes` / `edit.paste_attributes`.
#[derive(Clone)]
pub(in crate::engine) struct ClipAttributes {
    color: ColorAdjustments,
    effects: Vec<EffectInstance>,
    speed: f64,
    speed_ramp: Option<SpeedRamp>,
    motion_blur: Option<MotionBlurParams>,
}

impl ClipAttributes {
    fn from_clip(clip: &Clip) -> Self {
        Self {
            color: clip.color.clone(),
            effects: clip.effects.clone(),
            speed: clip.speed,
            speed_ramp: clip.speed_ramp.clone(),
            motion_blur: clip.motion_blur.clone(),
        }
    }

    fn apply_to(&self, clip: &mut Clip) {
        clip.color = self.color.clone();
        clip.effects = self.effects.clone();
        clip.speed = self.speed;
        clip.speed_ramp = self.speed_ramp.clone();
        clip.motion_blur = self.motion_blur.clone();
    }
}

/// Editor-held state for the edit family: the attribute clipboard.
#[derive(Clone, Default)]
pub(in crate::engine) struct EditState {
    attributes: Option<ClipAttributes>,
}

impl EditorEngine {
    pub(super) fn dispatch_edit_labels(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if let Some(color) = id.strip_prefix("edit.label.") {
            return Some(self.set_label(color, params));
        }
        match id {
            "edit.duplicate" => Some((|| {
                let params: EditClipTargetParams = parse_params(params)?;
                self.duplicate_clip(params)
            })()),
            "edit.consolidate_duplicates" => Some(self.consolidate_duplicates()),
            "edit.copy_attributes" => Some((|| {
                let params: EditClipTargetParams = parse_params(params)?;
                self.copy_attributes(params)
            })()),
            "edit.paste_attributes" => Some((|| {
                let params: EditClipTargetParams = parse_params(params)?;
                self.paste_attributes(params)
            })()),
            "edit.remove_attributes" => Some((|| {
                let params: EditClipTargetParams = parse_params(params)?;
                self.remove_attributes(params)
            })()),
            "edit.ripple_delete_head" => Some((|| {
                let params: RippleDeleteHeadParams = parse_params(params)?;
                self.ripple_delete_head(params)
            })()),
            "edit.remove_unused_media" => Some(self.remove_unused_media()),
            _ => None,
        }
    }

    fn set_label(
        &mut self,
        color: &str,
        params: Value,
    ) -> Result<Value, DispatchError> {
        if !LABEL_COLORS.contains(&color) {
            return Err(DispatchError::InvalidParameters(format!(
                "Unknown label colour '{color}'"
            )));
        }
        let params: EditLabelTargetsParams = parse_params(params)?;
        if params.clips.is_empty() {
            return Err(DispatchError::InvalidParameters(
                "Select one or more timeline clips".into(),
            ));
        }
        for reference in &params.clips {
            let track_index = find_track_index(&self.document, reference.track_id)?;
            if self.document.project.sequence.tracks[track_index].locked {
                return Err(DispatchError::Operation("Track is locked".into()));
            }
            let clips = &self.document.project.sequence.tracks[track_index].clips;
            if clips.get(reference.clip_index).is_none() {
                return Err(DispatchError::InvalidParameters(
                    "Timeline clip does not exist".into(),
                ));
            }
        }
        self.checkpoint();
        let mut labelled = 0;
        for reference in &params.clips {
            let track_index = find_track_index(&self.document, reference.track_id)?;
            let clips = &mut self.document.project.sequence.tracks[track_index].clips;
            if let Some(clip) = clips.get_mut(reference.clip_index) {
                clip.label = color.to_owned();
                labelled += 1;
            }
        }
        Ok(json!({"labelled": labelled, "color": color}))
    }

    fn duplicate_clip(
        &mut self,
        params: EditClipTargetParams,
    ) -> Result<Value, DispatchError> {
        let track_index = find_track_index(&self.document, params.track_id)?;
        if self.document.project.sequence.tracks[track_index].locked {
            return Err(DispatchError::Operation("Track is locked".into()));
        }
        let source = self.document.project.sequence.tracks[track_index]
            .clips
            .get(params.clip_index)
            .cloned()
            .ok_or_else(|| {
                DispatchError::InvalidParameters("Timeline clip does not exist".into())
            })?;
        let new_start = Tick(
            source
                .timeline_start
                .0
                .saturating_add(source.duration.0),
        );
        self.checkpoint();
        let clips = &mut self.document.project.sequence.tracks[track_index].clips;
        let duplicate = Clip {
            timeline_start: new_start,
            ..source
        };
        clips.push(duplicate);
        clips.sort_by_key(|item| item.timeline_start);
        Ok(json!({
            "duplicated": true,
            "timeline_start": new_start.0,
        }))
    }

    fn consolidate_duplicates(&mut self) -> Result<Value, DispatchError> {
        // Group clips on each track by identical source usage
        // (media_id + source_in + duration); remove every later copy,
        // keeping the earliest placement.
        let sequence = &self.document.project.sequence;
        let mut doomed = Vec::<(usize, usize)>::new();
        let mut groups = 0usize;
        for (track_index, track) in sequence.tracks.iter().enumerate() {
            if track.locked {
                continue;
            }
            let mut seen = std::collections::HashMap::<
                (u64, i64, i64),
                Vec<(usize, i64)>,
            >::new();
            for (clip_index, clip) in track.clips.iter().enumerate() {
                let key = (clip.media_id, clip.source_in.0, clip.duration.0);
                seen.entry(key)
                    .or_default()
                    .push((clip_index, clip.timeline_start.0));
            }
            for occurrences in seen.values() {
                if occurrences.len() < 2 {
                    continue;
                }
                groups += 1;
                let keep = occurrences
                    .iter()
                    .min_by_key(|(_, start)| *start)
                    .map(|(index, _)| *index)
                    .unwrap_or(0);
                for (clip_index, _) in occurrences {
                    if *clip_index != keep {
                        doomed.push((track_index, *clip_index));
                    }
                }
            }
        }
        if doomed.is_empty() {
            return Ok(json!({"removed": 0, "groups": 0}));
        }
        self.checkpoint();
        let mut removed = 0;
        let mut by_track = std::collections::HashMap::<usize, Vec<usize>>::new();
        for (track_index, clip_index) in doomed {
            by_track.entry(track_index).or_default().push(clip_index);
        }
        let mut tracks: Vec<usize> = by_track.keys().copied().collect();
        tracks.sort_unstable();
        for track_index in tracks {
            let mut indices = by_track.remove(&track_index).unwrap_or_default();
            indices.sort_unstable_by(|left, right| right.cmp(left));
            let clips = &mut self.document.project.sequence.tracks[track_index].clips;
            for index in indices {
                if clips.get(index).is_some() {
                    clips.remove(index);
                    removed += 1;
                }
            }
            clips.sort_by_key(|item| item.timeline_start);
        }
        Ok(json!({"removed": removed, "groups": groups}))
    }

    fn copy_attributes(
        &mut self,
        params: EditClipTargetParams,
    ) -> Result<Value, DispatchError> {
        let track_index = find_track_index(&self.document, params.track_id)?;
        let clip = self.document.project.sequence.tracks[track_index]
            .clips
            .get(params.clip_index)
            .ok_or_else(|| {
                DispatchError::InvalidParameters("Timeline clip does not exist".into())
            })?;
        let snapshot = ClipAttributes::from_clip(clip);
        let effects = snapshot.effects.len();
        self.edit.attributes = Some(snapshot);
        Ok(json!({"copied": true, "effects": effects}))
    }

    fn paste_attributes(
        &mut self,
        params: EditClipTargetParams,
    ) -> Result<Value, DispatchError> {
        let attributes = match self.edit.attributes.clone() {
            Some(attributes) => attributes,
            None => {
                return Err(DispatchError::Operation(
                    "No copied attributes; copy attributes from a clip first".into(),
                ))
            }
        };
        let track_index = find_track_index(&self.document, params.track_id)?;
        if self.document.project.sequence.tracks[track_index].locked {
            return Err(DispatchError::Operation("Track is locked".into()));
        }
        if self.document.project.sequence.tracks[track_index]
            .clips
            .get(params.clip_index)
            .is_none()
        {
            return Err(DispatchError::InvalidParameters(
                "Timeline clip does not exist".into(),
            ));
        }
        self.checkpoint();
        let clips = &mut self.document.project.sequence.tracks[track_index].clips;
        if let Some(clip) = clips.get_mut(params.clip_index) {
            attributes.apply_to(clip);
        }
        Ok(json!({"pasted": true}))
    }

    fn remove_attributes(
        &mut self,
        params: EditClipTargetParams,
    ) -> Result<Value, DispatchError> {
        let track_index = find_track_index(&self.document, params.track_id)?;
        if self.document.project.sequence.tracks[track_index].locked {
            return Err(DispatchError::Operation("Track is locked".into()));
        }
        if self.document.project.sequence.tracks[track_index]
            .clips
            .get(params.clip_index)
            .is_none()
        {
            return Err(DispatchError::InvalidParameters(
                "Timeline clip does not exist".into(),
            ));
        }
        self.checkpoint();
        let clips = &mut self.document.project.sequence.tracks[track_index].clips;
        if let Some(clip) = clips.get_mut(params.clip_index) {
            clip.color = ColorAdjustments::default();
            clip.effects.clear();
            clip.speed = 1.0;
            clip.speed_ramp = None;
            clip.motion_blur = None;
        }
        Ok(json!({"removed": true}))
    }

    fn ripple_delete_head(
        &mut self,
        params: RippleDeleteHeadParams,
    ) -> Result<Value, DispatchError> {
        if params.frames < 1 {
            return Err(DispatchError::InvalidParameters(
                "Frame count must be at least 1".into(),
            ));
        }
        let frame_ticks = self
            .document
            .project
            .settings
            .frame_rate
            .frame_ticks()
            .ok_or_else(|| {
                DispatchError::Operation(
                    "Project frame rate does not map to whole-frame ticks".into(),
                )
            })?;
        let head = frame_ticks
            .checked_mul(params.frames)
            .ok_or_else(|| DispatchError::InvalidParameters("Frame range overflows".into()))?;
        let track_index = find_track_index(&self.document, params.track_id)?;
        if self.document.project.sequence.tracks[track_index].locked {
            return Err(DispatchError::Operation("Track is locked".into()));
        }
        let clips = &self.document.project.sequence.tracks[track_index].clips;
        let clip = clips.get(params.clip_index).ok_or_else(|| {
            DispatchError::InvalidParameters("Timeline clip does not exist".into())
        })?;
        let start = clip.timeline_start.0;
        let source_in = clip.source_in.0;
        if clip.duration.0 <= head {
            return Err(DispatchError::Operation(
                "Removing that many head frames would erase the clip".into(),
            ));
        }
        self.checkpoint();
        let clips = &mut self.document.project.sequence.tracks[track_index].clips;
        if let Some(clip) = clips.get_mut(params.clip_index) {
            clip.source_in = Tick(source_in.saturating_add(head));
            clip.duration = Tick(clip.duration.0 - head);
        }
        // Ripple-close: everything on the track that starts after the
        // removed head range shifts left by `head`.
        for later in clips
            .iter_mut()
            .filter(|item| item.timeline_start.0 >= start.saturating_add(head))
        {
            later.timeline_start.0 = later.timeline_start.0.saturating_sub(head);
        }
        clips.sort_by_key(|item| item.timeline_start);
        Ok(json!({"removed_ticks": head, "frames": params.frames}))
    }

    fn remove_unused_media(&mut self) -> Result<Value, DispatchError> {
        let mut used = std::collections::HashSet::<u64>::new();
        for track in &self.document.project.sequence.tracks {
            for clip in &track.clips {
                used.insert(clip.media_id);
            }
        }
        if self.document.project.media.iter().all(|asset| used.contains(&asset.id)) {
            return Ok(json!({"removed": 0, "names": []}));
        }
        self.checkpoint();
        let mut names = Vec::new();
        self.document.project.media.retain(|asset| {
            if used.contains(&asset.id) {
                true
            } else {
                names.push(asset.name.clone());
                false
            }
        });
        let removed = names.len();
        Ok(json!({"removed": removed, "names": names}))
    }
}
