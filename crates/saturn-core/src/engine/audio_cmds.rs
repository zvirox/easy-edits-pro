use super::*;

impl EditorEngine {
    pub(super) fn dispatch_audio(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "track.set_gain_db" => Some((|| {
                let params: SetTrackGainParams = parse_params(params)?;
                if !params.gain_db.is_finite() || !(-60.0..=12.0).contains(&params.gain_db) {
                    return Err(DispatchError::InvalidParameters(
                        "Track gain must be between -60 and +12 dB".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].gain_db = params.gain_db;
                Ok(json!({"updated": true}))
            })()),
            "track.set_pan" => Some((|| {
                let params: SetTrackPanParams = parse_params(params)?;
                if !params.pan.is_finite() || !(-1.0..=1.0).contains(&params.pan) {
                    return Err(DispatchError::InvalidParameters(
                        "Track pan must be between -1.0 and +1.0".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].pan = params.pan;
                Ok(json!({"updated": true, "pan": params.pan}))
            })()),
            "track.set_muted" => Some((|| {
                let params: SetTrackMutedParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].muted = params.muted;
                Ok(json!({"updated": true, "muted": params.muted}))
            })()),
            "track.set_solo" => Some((|| {
                let params: SetTrackSoloParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].solo = params.solo;
                Ok(json!({"updated": true, "solo": params.solo}))
            })()),
            "track.set_locked" => Some((|| {
                let params: SetTrackLockedParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].locked = params.locked;
                Ok(json!({"updated": true, "locked": params.locked}))
            })()),
            "track.set_audio_processing" => Some((|| {
                let params: SetTrackAudioProcessingParams = parse_params(params)?;
                if !params.eq_low_gain.is_finite()
                    || !(-24.0..=24.0).contains(&params.eq_low_gain)
                    || !params.eq_mid_gain.is_finite()
                    || !(-24.0..=24.0).contains(&params.eq_mid_gain)
                    || !params.eq_high_gain.is_finite()
                    || !(-24.0..=24.0).contains(&params.eq_high_gain)
                    || !params.comp_threshold.is_finite()
                    || !(-80.0..=0.0).contains(&params.comp_threshold)
                    || !params.comp_ratio.is_finite()
                    || !(1.0..=50.0).contains(&params.comp_ratio)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Audio processing values are outside supported ranges".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].audio_processing =
                    TrackAudioProcessing {
                        eq_enabled: params.eq_enabled,
                        eq_low_gain: params.eq_low_gain,
                        eq_mid_gain: params.eq_mid_gain,
                        eq_highmid_gain: params.eq_highmid_gain,
                        eq_high_gain: params.eq_high_gain,
                        comp_enabled: params.comp_enabled,
                        comp_threshold: params.comp_threshold,
                        comp_ratio: params.comp_ratio,
                        comp_attack: params.comp_attack,
                        comp_release: params.comp_release,
                        denoise_enabled: params.denoise_enabled,
                        denoise_amount: params.denoise_amount,
                    };
                Ok(json!({"updated": true}))
            })()),
            "track.set_bus_route" => Some((|| {
                let params: SetTrackBusRouteParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].bus = params.bus;
                Ok(json!({"updated": true, "bus": params.bus}))
            })()),
            "track.set_automation" => Some((|| {
                let params: SetTrackAutomationParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].automation = params.automation;
                Ok(json!({"updated": true}))
            })()),
            "track.set_automation_mode" => Some((|| {
                let params: SetAutomationModeParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].automation.mode = params.mode;
                Ok(json!({"updated": true, "mode": params.mode}))
            })()),
            "track.set_automation_interpolation" => Some((|| {
                let params: SetAutomationInterpolationParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].automation.interpolation =
                    params.interpolation;
                Ok(json!({"updated": true, "interpolation": params.interpolation}))
            })()),
            "track.add_automation_point" => Some((|| {
                let params: AddAutomationPointParams = parse_params(params)?;
                let value = crate::project::clamp_automation_value(params.lane, params.value);
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index]
                    .automation
                    .upsert_point(params.lane, params.time, value);
                Ok(json!({"added": true, "value": value}))
            })()),
            "track.remove_automation_point" => Some((|| {
                let params: RemoveAutomationPointParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                let removed = self.document.project.sequence.tracks[track_index]
                    .automation
                    .remove_point_at(params.lane, params.time);
                Ok(json!({"removed": removed}))
            })()),
            "track.clear_automation" => Some((|| {
                let params: ClearAutomationParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                let automation =
                    &mut self.document.project.sequence.tracks[track_index].automation;
                let cleared = match params.lane {
                    None => {
                        let count = automation.volume_points.len() + automation.pan_points.len();
                        automation.volume_points.clear();
                        automation.pan_points.clear();
                        count
                    }
                    Some(AutomationLane::Volume) => {
                        let count = automation.volume_points.len();
                        automation.volume_points.clear();
                        count
                    }
                    Some(AutomationLane::Pan) => {
                        let count = automation.pan_points.len();
                        automation.pan_points.clear();
                        count
                    }
                };
                Ok(json!({"cleared": cleared}))
            })()),
            "bus.set_gain_db" => Some((|| {
                let params: SetBusGainParams = parse_params(params)?;
                if !params.gain_db.is_finite() || !(-60.0..=12.0).contains(&params.gain_db) {
                    return Err(DispatchError::InvalidParameters(
                        "Bus gain must be between -60 and +12 dB".into(),
                    ));
                }
                self.checkpoint();
                if let Some(bus) = self
                    .document
                    .project
                    .sequence
                    .buses
                    .iter_mut()
                    .find(|b| b.target == params.target)
                {
                    bus.gain_db = params.gain_db;
                }
                Ok(json!({"updated": true, "gain_db": params.gain_db}))
            })()),
            "speech.transcribe_to_subtitles" => Some((|| {
                let params: TranscribeToSubtitlesParams = parse_params(params)?;
                if params.segments.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "No speech segments provided".into(),
                    ));
                }
                self.checkpoint();
                let new_subs = crate::speech::speech_segments_to_subtitles(&params.segments);
                let count = new_subs.len();
                {
                    let sequence = &mut self.document.project.sequence;
                    if sequence.caption_tracks.is_empty() {
                        let track_id = self.document.project.next_id;
                        self.document.project.next_id = track_id.saturating_add(1);
                        sequence.caption_tracks.push(crate::project::CaptionTrack {
                            id: track_id,
                            name: "Captions".into(),
                            language: "en".into(),
                            visible: true,
                            cues: Vec::new(),
                            styles: Vec::new(),
                        });
                    }
                }
                let track = self
                    .document
                    .project
                    .sequence
                    .caption_tracks
                    .first_mut()
                    .expect("just ensured");
                if params.replace_existing.unwrap_or(true) {
                    track.cues = new_subs;
                } else {
                    track.cues.extend(new_subs);
                    track.cues.sort_by_key(|s| s.start_ticks);
                }
                Ok(json!({"transcribed": true, "subtitles_count": count}))
            })()),
            "speech.ripple_delete_silence" => Some((|| {
                let params: RippleDeleteSilenceParams = parse_params(params)?;
                let opts = params.options.unwrap_or_default();
                let total_dur = self.document.project.sequence.duration();
                let silences =
                    crate::speech::detect_silence_from_peaks(&params.peaks, total_dur, &opts);
                let ranges: Vec<(Tick, Tick)> = silences
                    .into_iter()
                    .map(|s| (s.start_ticks, s.end_ticks))
                    .collect();
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(
                    &mut self.document.project.sequence,
                    &ranges,
                )
                .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            })()),
            "speech.ripple_delete_fillers" => Some((|| {
                let params: RippleDeleteFillersParams = parse_params(params)?;
                let custom_slice: Option<Vec<&str>> = params
                    .custom_fillers
                    .as_ref()
                    .map(|v| v.iter().map(|s| s.as_str()).collect());
                let fillers = crate::speech::extract_filler_intervals(
                    &params.segments,
                    custom_slice.as_deref(),
                );
                let ranges: Vec<(Tick, Tick)> = fillers
                    .into_iter()
                    .map(|s| (s.start_ticks, s.end_ticks))
                    .collect();
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(
                    &mut self.document.project.sequence,
                    &ranges,
                )
                .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            })()),
            _ => None,
        }
    }
}
