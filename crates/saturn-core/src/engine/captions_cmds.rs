use super::*;
use crate::project::{CaptionStyle, CaptionTrack};

/// Returns the default (first) caption track id, creating the track when the
/// sequence has none — legacy `subtitle.*` commands and imports land there.
fn ensure_default_track(engine: &mut EditorEngine) -> u64 {
    if let Some(track) = engine.document.project.sequence.caption_tracks.first() {
        return track.id;
    }
    let track_id = engine.document.project.next_id;
    engine.document.project.next_id = track_id.saturating_add(1);
    engine
        .document
        .project
        .sequence
        .caption_tracks
        .push(CaptionTrack {
            id: track_id,
            name: "Captions".into(),
            language: "en".into(),
            cues: Vec::new(),
            styles: Vec::new(),
        });
    track_id
}

/// Allocates one project-wide entity id.
fn next_id(engine: &mut EditorEngine) -> u64 {
    let id = engine.document.project.next_id;
    engine.document.project.next_id = id.saturating_add(1);
    id
}

fn track_mut(engine: &mut EditorEngine, track_id: u64) -> Option<&mut CaptionTrack> {
    engine
        .document
        .project
        .sequence
        .caption_tracks
        .iter_mut()
        .find(|track| track.id == track_id)
}

fn track_exists(engine: &EditorEngine, track_id: u64) -> bool {
    engine
        .document
        .project
        .sequence
        .caption_tracks
        .iter()
        .any(|track| track.id == track_id)
}

/// Shared cue invariants (timing + text) used by every mutating caption command.
fn validate_cue_timing(start: Tick, end: Tick, text: &str) -> Result<(), DispatchError> {
    if start.0 < 0 || end.0 <= start.0 {
        return Err(DispatchError::InvalidParameters(
            "Caption cue start time must be non-negative and before end time".into(),
        ));
    }
    if text.trim().is_empty() {
        return Err(DispatchError::InvalidParameters(
            "Caption cue text cannot be empty".into(),
        ));
    }
    Ok(())
}

impl EditorEngine {
    pub(super) fn dispatch_captions(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            // Legacy-compatible cue surface (CLI/MCP parity): the default track.
            "subtitle.add" => Some((|| {
                let params: AddSubtitleParams = parse_params(params)?;
                validate_cue_timing(params.start_ticks, params.end_ticks, &params.text)?;
                let track_id = ensure_default_track(self);
                let cue_id = next_id(self);
                self.checkpoint();
                let Some(track) = track_mut(self, track_id) else {
                    unreachable!("default track exists");
                };
                track.cues.push(SubtitleItem {
                    id: cue_id,
                    start_ticks: params.start_ticks,
                    end_ticks: params.end_ticks,
                    text: params.text,
                    speaker: params.speaker,
                    style_id: params.style_id,
                });
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({"added": true, "id": cue_id, "track_id": track_id}))
            })()),
            "subtitle.remove" => Some((|| {
                let params: RemoveSubtitleParams = parse_params(params)?;
                let track_id = ensure_default_track(self);
                self.checkpoint();
                let track = track_mut(self, track_id).expect("default track exists");
                let initial = track.cues.len();
                track.cues.retain(|cue| cue.id != params.id);
                Ok(json!({"removed": track.cues.len() < initial}))
            })()),
            "subtitle.update" => Some((|| {
                let params: UpdateSubtitleParams = parse_params(params)?;
                let track_id = ensure_default_track(self);
                {
                    let Some(track) = track_mut(self, track_id) else {
                        unreachable!("default track exists");
                    };
                    let Some(cue) = track.cues.iter().find(|cue| cue.id == params.id) else {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Caption cue {} does not exist",
                            params.id
                        )));
                    };
                    let new_start = params.start_ticks.unwrap_or(cue.start_ticks);
                    let new_end = params.end_ticks.unwrap_or(cue.end_ticks);
                    let new_text = params.text.clone().unwrap_or_else(|| cue.text.clone());
                    validate_cue_timing(new_start, new_end, &new_text)?;
                }
                self.checkpoint();
                let track = track_mut(self, track_id).expect("default track exists");
                let cue = track
                    .cues
                    .iter_mut()
                    .find(|cue| cue.id == params.id)
                    .expect("verified above");
                cue.start_ticks = params.start_ticks.unwrap_or(cue.start_ticks);
                cue.end_ticks = params.end_ticks.unwrap_or(cue.end_ticks);
                if let Some(text) = params.text {
                    cue.text = text;
                }
                if let Some(speaker) = params.speaker {
                    cue.speaker = Some(speaker);
                }
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({"updated": true, "id": params.id}))
            })()),
            "subtitle.clear" => Some((|| {
                let track_id = ensure_default_track(self);
                self.checkpoint();
                let track = track_mut(self, track_id).expect("default track exists");
                track.cues.clear();
                Ok(json!({"cleared": true}))
            })()),
            "caption.track_add" => Some((|| {
                let params: CaptionTrackAddParams = parse_params(params)?;
                self.checkpoint();
                let track_id = next_id(self);
                self.document
                    .project
                    .sequence
                    .caption_tracks
                    .push(CaptionTrack {
                        id: track_id,
                        name: params.name.unwrap_or_else(|| "Captions".into()),
                        language: params.language.unwrap_or_else(|| "en".into()),
                        cues: Vec::new(),
                        styles: Vec::new(),
                    });
                Ok(json!({"added": true, "id": track_id}))
            })()),
            "caption.track_remove" => Some((|| {
                let params: CaptionTrackRemoveParams = parse_params(params)?;
                if self.document.project.sequence.caption_tracks.len() <= 1 {
                    return Err(DispatchError::InvalidParameters(
                        "The last caption track cannot be removed".into(),
                    ));
                }
                if !track_exists(self, params.id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.id
                    )));
                }
                self.checkpoint();
                self.document
                    .project
                    .sequence
                    .caption_tracks
                    .retain(|track| track.id != params.id);
                Ok(json!({"removed": true}))
            })()),
            "caption.track_rename" => Some((|| {
                let params: CaptionTrackRenameParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Caption track name cannot be empty".into(),
                    ));
                }
                if !track_exists(self, params.id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.id
                    )));
                }
                self.checkpoint();
                let track = track_mut(self, params.id).expect("verified above");
                track.name = params.name;
                Ok(json!({"renamed": true, "id": params.id}))
            })()),
            "caption.cue_add" => Some((|| {
                let params: CaptionCueAddParams = parse_params(params)?;
                validate_cue_timing(params.start_ticks, params.end_ticks, &params.text)?;
                let track_id = params.track_id.unwrap_or_else(|| ensure_default_track(self));
                if !track_exists(self, track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        track_id
                    )));
                }
                let cue_id = next_id(self);
                self.checkpoint();
                let track = track_mut(self, track_id).expect("verified above");
                track.cues.push(SubtitleItem {
                    id: cue_id,
                    start_ticks: params.start_ticks,
                    end_ticks: params.end_ticks,
                    text: params.text,
                    speaker: params.speaker,
                    style_id: params.style_id,
                });
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({"added": true, "id": cue_id, "track_id": track_id}))
            })()),
            "caption.cue_update" => Some((|| {
                let params: CaptionCueUpdateParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                {
                    let Some(track) = track_mut(self, params.track_id) else {
                        unreachable!("verified above");
                    };
                    let Some(cue) = track.cues.iter().find(|cue| cue.id == params.id) else {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Caption cue {} does not exist",
                            params.id
                        )));
                    };
                    let new_start = params.start_ticks.unwrap_or(cue.start_ticks);
                    let new_end = params.end_ticks.unwrap_or(cue.end_ticks);
                    let new_text = params.text.clone().unwrap_or_else(|| cue.text.clone());
                    validate_cue_timing(new_start, new_end, &new_text)?;
                }
                self.checkpoint();
                let track = track_mut(self, params.track_id).expect("verified above");
                let cue = track
                    .cues
                    .iter_mut()
                    .find(|cue| cue.id == params.id)
                    .expect("verified above");
                cue.start_ticks = params.start_ticks.unwrap_or(cue.start_ticks);
                cue.end_ticks = params.end_ticks.unwrap_or(cue.end_ticks);
                if let Some(text) = params.text {
                    cue.text = text;
                }
                if let Some(speaker) = params.speaker {
                    cue.speaker = speaker;
                }
                if let Some(style_id) = params.style_id {
                    cue.style_id = Some(style_id);
                }
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({"updated": true, "id": params.id}))
            })()),
            "caption.cue_remove" => Some((|| {
                let params: CaptionCueRemoveParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                self.checkpoint();
                let track = track_mut(self, params.track_id).expect("verified above");
                let before = track.cues.len();
                track.cues.retain(|cue| cue.id != params.id);
                Ok(json!({"removed": track.cues.len() < before}))
            })()),
            "caption.cue_nudge" => Some((|| {
                let params: CaptionCueNudgeParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                {
                    let Some(track) = track_mut(self, params.track_id) else {
                        unreachable!("verified above");
                    };
                    let Some(cue) = track.cues.iter().find(|cue| cue.id == params.id) else {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Caption cue {} does not exist",
                            params.id
                        )));
                    };
                    let new_start = Tick(cue.start_ticks.0 + params.delta_ticks);
                    let new_end = Tick(cue.end_ticks.0 + params.delta_ticks);
                    validate_cue_timing(new_start, new_end, &cue.text)?;
                }
                self.checkpoint();
                let track = track_mut(self, params.track_id).expect("verified above");
                let cue = track
                    .cues
                    .iter_mut()
                    .find(|cue| cue.id == params.id)
                    .expect("verified above");
                cue.start_ticks = Tick(cue.start_ticks.0 + params.delta_ticks);
                cue.end_ticks = Tick(cue.end_ticks.0 + params.delta_ticks);
                let (new_start, new_end) = (cue.start_ticks, cue.end_ticks);
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({
                    "nudged": true,
                    "id": params.id,
                    "start_ticks": new_start.0,
                    "end_ticks": new_end.0,
                }))
            })()),
            "caption.style_set" => Some((|| {
                let params: CaptionStyleSetParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                if let Some(style_id) = params.id {
                    {
                        let Some(track) = track_mut(self, params.track_id) else {
                            unreachable!("verified above");
                        };
                        let Some(_style) = track.styles.iter().find(|style| style.id == style_id)
                        else {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Caption style {} does not exist",
                                style_id
                            )));
                        };
                        if params.name.as_deref().map(str::trim) == Some("") {
                            return Err(DispatchError::InvalidParameters(
                                "Caption style name cannot be empty".into(),
                            ));
                        }
                    }
                    self.checkpoint();
                    let track = track_mut(self, params.track_id).expect("verified above");
                    let style = track
                        .styles
                        .iter_mut()
                        .find(|style| style.id == style_id)
                        .expect("verified above");
                    if let Some(name) = params.name {
                        style.name = name;
                    }
                    if let Some(font_family) = params.font_family {
                        style.font_family = font_family;
                    }
                    if let Some(font_size) = params.font_size {
                        style.font_size = font_size;
                    }
                    if let Some(fill_color) = params.fill_color {
                        style.fill_color = fill_color;
                    }
                    if let Some(stroke_color) = params.stroke_color {
                        style.stroke_color = Some(stroke_color);
                    }
                    if let Some(stroke_width) = params.stroke_width {
                        style.stroke_width = stroke_width;
                    }
                    if let Some(background_color) = params.background_color {
                        style.background_color = Some(background_color);
                    }
                    Ok(json!({"updated": true, "id": style_id}))
                } else {
                    let style_id = next_id(self);
                    self.checkpoint();
                    let track = track_mut(self, params.track_id).expect("verified above");
                    track.styles.push(CaptionStyle {
                        id: style_id,
                        name: params.name.unwrap_or_else(|| "Style".into()),
                        font_family: params.font_family.unwrap_or_default(),
                        font_size: params.font_size.unwrap_or(42.0),
                        fill_color: params.fill_color.unwrap_or_else(|| "#ffffff".into()),
                        stroke_color: params.stroke_color,
                        stroke_width: params.stroke_width.unwrap_or(0.0),
                        background_color: params.background_color,
                    });
                    Ok(json!({"added": true, "id": style_id}))
                }
            })()),
            "caption.import" => Some((|| {
                let params: CaptionImportParams = parse_params(params)?;
                let cues = crate::subtitles::detect_and_parse_subtitles(&params.content)
                    .map_err(DispatchError::Operation)?;
                if cues.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "The caption file contains no cues".into(),
                    ));
                }
                let track_id = params.track_id.unwrap_or_else(|| ensure_default_track(self));
                if !track_exists(self, track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        track_id
                    )));
                }
                self.checkpoint();
                let track = track_mut(self, track_id).expect("verified above");
                let imported = cues.len();
                track.cues = cues;
                Ok(json!({"imported": imported, "track_id": track_id}))
            })()),
            "caption.export" => Some((|| {
                let params: CaptionExportParams = parse_params(params)?;
                let track = match params.track_id {
                    Some(track_id) => {
                        let Some(track) = self
                            .document
                            .project
                            .sequence
                            .caption_tracks
                            .iter()
                            .find(|track| track.id == track_id)
                        else {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Caption track {} does not exist",
                                track_id
                            )));
                        };
                        track
                    }
                    None => self
                        .document
                        .project
                        .sequence
                        .caption_tracks
                        .first()
                        .ok_or_else(|| {
                            DispatchError::InvalidParameters(
                                "The project has no caption tracks".into(),
                            )
                        })?,
                };
                // Non-mutating: no checkpoint.
                let text = if params.format.eq_ignore_ascii_case("vtt") {
                    crate::subtitles::export_vtt(&track.cues)
                } else {
                    crate::subtitles::export_srt(&track.cues)
                };
                Ok(json!({
                    "format": params.format,
                    "content": text,
                    "cues": track.cues.len(),
                    "track_id": track.id,
                }))
            })()),
            _ => None,
        }
    }
}
