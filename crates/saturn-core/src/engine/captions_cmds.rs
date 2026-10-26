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
            visible: true,
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
                        visible: true,
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
            "caption.list" => Some((|| {
                let params: CaptionListParams = parse_params(params)?;
                if let Some(track_id) = params.track_id {
                    if !track_exists(self, track_id) {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Caption track {} does not exist",
                            track_id
                        )));
                    }
                }
                let tracks: Vec<Value> = self
                    .document
                    .project
                    .sequence
                    .caption_tracks
                    .iter()
                    .filter(|track| params.track_id.is_none_or(|id| track.id == id))
                    .map(|track| {
                        json!({
                            "id": track.id,
                            "name": track.name,
                            "language": track.language,
                            "visible": track.visible,
                            "cues": track.cues.iter().map(|cue| json!({
                                "id": cue.id,
                                "start_ticks": cue.start_ticks.0,
                                "end_ticks": cue.end_ticks.0,
                                "text": cue.text,
                                "speaker": cue.speaker,
                            })).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                let cues: usize = tracks.iter().map(|track| track["cues"].as_array().map_or(0, Vec::len)).sum();
                Ok(json!({"tracks": tracks, "cues": cues}))
            })()),
            "caption.cue_delete" => Some((|| {
                let params: CaptionCueDeleteParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                if params.ids.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "cue_delete needs at least one cue id".into(),
                    ));
                }
                self.checkpoint();
                let track = track_mut(self, params.track_id).expect("verified above");
                let before = track.cues.len();
                let ids: std::collections::HashSet<u64> = params.ids.iter().copied().collect();
                track.cues.retain(|cue| !ids.contains(&cue.id));
                Ok(json!({"removed": before - track.cues.len()}))
            })()),
            "caption.cue_split" => Some((|| {
                let params: CaptionCueSplitParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                {
                    let track = track_mut(self, params.track_id)
                        .expect("verified above");
                    let Some(cue) = track.cues.iter().find(|cue| cue.id == params.id) else {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Caption cue {} does not exist",
                            params.id
                        )));
                    };
                    if params.at_ticks.0 <= cue.start_ticks.0
                        || params.at_ticks.0 >= cue.end_ticks.0
                    {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Split point {} is outside cue {} ({}..{})",
                            params.at_ticks.0, cue.id, cue.start_ticks.0, cue.end_ticks.0
                        )));
                    }
                    let total = cue.end_ticks.0 - cue.start_ticks.0;
                    let into = params.at_ticks.0 - cue.start_ticks.0;
                    let cut = ((cue.text.chars().count() as i64 * into) / total) as usize;
                    if cut == 0 || cut >= cue.text.chars().count() {
                        return Err(DispatchError::InvalidParameters(
                            "The split point does not divide the caption text".into(),
                        ));
                    }
                }
                self.checkpoint();
                let new_id = next_id(self);
                let track = track_mut(self, params.track_id).expect("verified above");
                let index = track
                    .cues
                    .iter()
                    .position(|cue| cue.id == params.id)
                    .expect("verified above");
                let original_end = track.cues[index].end_ticks;
                let first = &mut track.cues[index];
                let at = params.at_ticks.0;
                let total = first.end_ticks.0 - first.start_ticks.0;
                let into = at - first.start_ticks.0;
                let cut = (first.text.chars().count() as i64 * into / total) as usize;
                let second_chars: Vec<char> = first.text.chars().collect();
                let second_text = second_chars[cut..]
                    .iter()
                    .collect::<String>()
                    .trim_start()
                    .to_string();
                first.text = second_chars[..cut].iter().collect();
                first.end_ticks = Tick(at);
                let second = SubtitleItem {
                    id: new_id,
                    start_ticks: Tick(at),
                    end_ticks: original_end,
                    text: second_text,
                    speaker: track.cues[index].speaker.clone(),
                    style_id: track.cues[index].style_id,
                };
                let first_id = track.cues[index].id;
                track.cues.insert(index + 1, second);
                Ok(json!({"split": true, "first": first_id, "second": track.cues[index + 1].id}))
            })()),
            "caption.cue_merge" => Some((|| {
                let params: CaptionCueMergeParams = parse_params(params)?;
                if !track_exists(self, params.track_id) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Caption track {} does not exist",
                        params.track_id
                    )));
                }
                if params.ids.len() < 2 {
                    return Err(DispatchError::InvalidParameters(
                        "cue_merge needs at least two cue ids".into(),
                    ));
                }
                {
                    let track = track_mut(self, params.track_id).expect("verified above");
                    let mut found: Vec<&SubtitleItem> = Vec::new();
                    for id in &params.ids {
                        match track.cues.iter().find(|cue| &cue.id == id) {
                            Some(cue) => found.push(cue),
                            None => {
                                return Err(DispatchError::InvalidParameters(format!(
                                    "Caption cue {id} does not exist"
                                )));
                            }
                        }
                    }
                    found.sort_by_key(|cue| cue.start_ticks);
                    for pair in found.windows(2) {
                        if pair[1].start_ticks.0 < pair[0].end_ticks.0 {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Cues {} and {} overlap; merge needs distinct cues",
                                pair[0].id, pair[1].id
                            )));
                        }
                    }
                }
                self.checkpoint();
                let track = track_mut(self, params.track_id).expect("verified above");
                let mut merged: Vec<SubtitleItem> = Vec::new();
                let ids: std::collections::HashSet<u64> = params.ids.iter().copied().collect();
                for cue in track.cues.iter() {
                    if ids.contains(&cue.id) {
                        merged.push(cue.clone());
                    }
                }
                merged.sort_by_key(|cue| cue.start_ticks);
                let text = merged
                    .iter()
                    .map(|cue| cue.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ");
                let combined = SubtitleItem {
                    id: merged[0].id,
                    start_ticks: merged[0].start_ticks,
                    end_ticks: merged[merged.len() - 1].end_ticks,
                    text,
                    speaker: merged.iter().find_map(|cue| cue.speaker.clone()),
                    style_id: merged.iter().find_map(|cue| cue.style_id),
                };
                let merged_count = merged.len();
                track.cues.retain(|cue| !ids.contains(&cue.id));
                track.cues.push(combined);
                track.cues.sort_by_key(|cue| cue.start_ticks);
                Ok(json!({"merged": true, "id": merged[0].id, "count": merged_count}))
            })()),
            "caption.next" => Some((|| {
                let params: CaptionNavParams = parse_params(params)?;
                let from = params.from.0;
                let track_id = params.track_id;
                let target = self
                    .document
                    .project
                    .sequence
                    .caption_tracks
                    .iter()
                    .filter(|track| track_id.is_none_or(|id| track.id == id))
                    .flat_map(|track| {
                        let track_id = track.id;
                        track.cues.iter().map(move |cue| (track_id, cue))
                    })
                    .filter(|(_, cue)| cue.start_ticks.0 > from)
                    .min_by_key(|(_, cue)| cue.start_ticks.0);
                Ok(match target {
                    Some((track_id, cue)) => json!({
                        "target": cue.start_ticks.0,
                        "id": cue.id,
                        "track_id": track_id,
                    }),
                    None => json!({"target": null}),
                })
            })()),
            "caption.previous" => Some((|| {
                let params: CaptionNavParams = parse_params(params)?;
                let from = params.from.0;
                let track_id = params.track_id;
                let target = self
                    .document
                    .project
                    .sequence
                    .caption_tracks
                    .iter()
                    .filter(|track| track_id.is_none_or(|id| track.id == id))
                    .flat_map(|track| {
                        let track_id = track.id;
                        track.cues.iter().map(move |cue| (track_id, cue))
                    })
                    .filter(|(_, cue)| cue.start_ticks.0 < from)
                    .max_by_key(|(_, cue)| cue.start_ticks.0);
                Ok(match target {
                    Some((track_id, cue)) => json!({
                        "target": cue.start_ticks.0,
                        "id": cue.id,
                        "track_id": track_id,
                    }),
                    None => json!({"target": null}),
                })
            })()),
            "caption.tracks_hide" => Some((|| {
                let params: CaptionTracksVisibilityParams = parse_params(params)?;
                self.checkpoint();
                let mut changed = 0;
                for track in self.document.project.sequence.caption_tracks.iter_mut() {
                    if params
                        .track_ids
                        .as_ref()
                        .is_some_and(|ids| !ids.contains(&track.id))
                    {
                        continue;
                    }
                    if track.visible {
                        track.visible = false;
                        changed += 1;
                    }
                }
                Ok(json!({"hidden": changed}))
            })()),
            "caption.tracks_show" => Some((|| {
                let params: CaptionTracksVisibilityParams = parse_params(params)?;
                self.checkpoint();
                let mut changed = 0;
                for track in self.document.project.sequence.caption_tracks.iter_mut() {
                    if params
                        .track_ids
                        .as_ref()
                        .is_some_and(|ids| !ids.contains(&track.id))
                    {
                        continue;
                    }
                    if !track.visible {
                        track.visible = true;
                        changed += 1;
                    }
                }
                Ok(json!({"shown": changed}))
            })()),
            _ => None,
        }
    }
}
