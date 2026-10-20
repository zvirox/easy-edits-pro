use super::*;

impl EditorEngine {
    pub(super) fn dispatch_color(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "clip.set_color" => Some((|| {
                let params: SetColorParams = parse_params(params)?;
                if !params.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&params.exposure)
                    || !params.contrast.is_finite()
                    || !(0.0..=2.0).contains(&params.contrast)
                    || !params.saturation.is_finite()
                    || !(0.0..=2.0).contains(&params.saturation)
                    || !params.hue.is_finite()
                    || !(-1.0..=1.0).contains(&params.hue)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Color values must be exposure -5..5, contrast 0..2, saturation 0..2, and hue -1..1"
                            .into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.color.exposure = params.exposure;
                clip.color.contrast = params.contrast;
                clip.color.saturation = params.saturation;
                clip.color.hue = params.hue;
                if let Some(existing) = clip.effects.iter_mut().find_map(|e| match &mut e.payload {
                    EffectPayload::ColorCorrection(c) => Some(c),
                    _ => None,
                }) {
                    existing.exposure = params.exposure;
                    existing.contrast = params.contrast;
                    existing.saturation = params.saturation;
                    existing.hue = params.hue;
                }
                Ok(json!({"updated": true}))
            })()),
            "clip.set_color_wheels" => Some((|| {
                let params: SetColorWheelsParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.color.wheels = params.wheels.clone();
                if let Some(existing) = clip.effects.iter_mut().find_map(|e| match &mut e.payload {
                    EffectPayload::ColorCorrection(c) => Some(c),
                    _ => None,
                }) {
                    existing.wheels = params.wheels;
                }
                Ok(json!({"updated": true}))
            })()),
            "clip.update_rgb_curves" => Some((|| {
                let params: UpdateRgbCurvesParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.color.curves = Some(params.curves.clone());
                Ok(json!({"updated": true, "curves": params.curves}))
            })()),
            "clip.update_hsl_qualifier" => Some((|| {
                let params: UpdateHslQualifierParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                clip.color.qualifier = Some(params.qualifier.clone());
                Ok(json!({"updated": true, "qualifier": params.qualifier}))
            })()),
            "clip.add_custom_shader" => Some((|| {
                let params: AddCustomShaderParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let effect_id = self.document.project.next_id;
                self.document.project.next_id = effect_id.saturating_add(1);
                let shader_params = crate::project::CustomShaderParams {
                    name: params.name.unwrap_or_else(|| "Custom Shader".into()),
                    shader_code: params
                        .shader_code
                        .unwrap_or_else(|| CustomShaderParams::default().shader_code),
                    preset_id: params.preset_id,
                    uniforms: params.uniforms.unwrap_or_default(),
                };
                let effect = EffectInstance::new_custom_shader(effect_id, shader_params);
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .effects
                    .push(effect);
                Ok(json!({"added": true, "effect_id": effect_id}))
            })()),
            "clip.update_custom_shader" => Some((|| {
                let params: UpdateCustomShaderParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let effect = clip
                    .effects
                    .iter_mut()
                    .find(|e| e.id == params.effect_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Effect {} not found",
                            params.effect_id
                        ))
                    })?;
                match &mut effect.payload {
                    EffectPayload::CustomShader(p) => {
                        if let Some(n) = params.name {
                            effect.name = n.clone();
                            p.name = n;
                        }
                        if let Some(c) = params.shader_code {
                            p.shader_code = c;
                        }
                        if let Some(pid) = params.preset_id {
                            p.preset_id = Some(pid);
                        }
                        if let Some(u) = params.uniforms {
                            p.uniforms = u;
                        }
                    }
                    _ => {
                        return Err(DispatchError::InvalidParameters(
                            "Effect is not a custom shader".into(),
                        ));
                    }
                }
                Ok(json!({"updated": true, "effect_id": params.effect_id}))
            })()),
            _ => None,
        }
    }
}
