use super::*;

impl EditorEngine {
    pub(super) fn dispatch_keyframe(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "clip.set_keyframe" => Some((|| {
                let params: SetKeyframeParams = parse_params(params)?;
                if !params.value.is_finite()
                    || !valid_keyframe_value(&params.property, params.value)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe value is outside the supported range".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                if params.time.0 < 0 || params.time.0 > clip.duration.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe time must be within the selected clip".into(),
                    ));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let interp = params.interpolation.unwrap_or_default();
                if let Some(existing) = clip.keyframes.iter_mut().find(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) {
                    existing.value = params.value;
                    if params.interpolation.is_some() {
                        existing.interpolation = interp;
                        existing.handle_in_x = params.handle_in_x;
                        existing.handle_in_y = params.handle_in_y;
                        existing.handle_out_x = params.handle_out_x;
                        existing.handle_out_y = params.handle_out_y;
                    }
                } else {
                    clip.keyframes.push(Keyframe {
                        property: params.property,
                        time: params.time,
                        value: params.value,
                        interpolation: interp,
                        handle_in_x: params.handle_in_x,
                        handle_in_y: params.handle_in_y,
                        handle_out_x: params.handle_out_x,
                        handle_out_y: params.handle_out_y,
                    });
                    clip.keyframes.sort_by_key(|keyframe| keyframe.time);
                }
                Ok(json!({"updated": true, "keyframes": clip.keyframes.len()}))
            })()),
            "clip.remove_keyframe" => Some((|| {
                let params: RemoveKeyframeParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(index) = clip.keyframes.iter().position(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) else {
                    return Ok(json!({"removed": false}));
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .keyframes
                    .remove(index);
                Ok(json!({"removed": true}))
            })()),
            _ => None,
        }
    }
}
