use super::*;
use crate::project::MarkerKind;

impl EditorEngine {
    pub(super) fn dispatch_markers(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "marker.add" => Some((|| {
                let params: AddMarkerParams = parse_params(params)?;
                if params.at.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Marker start must be non-negative".into(),
                    ));
                }
                let duration = params.duration.unwrap_or(Tick(0));
                if duration.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Marker duration must be non-negative".into(),
                    ));
                }
                let name = marker_name(&self.document, params.name);
                self.checkpoint();
                let marker_id = self.document.project.next_id;
                self.document.project.next_id = marker_id.saturating_add(1);
                self.document.project.sequence.markers.push(Marker {
                    id: marker_id,
                    name,
                    kind: params.kind,
                    start_ticks: params.at,
                    duration_ticks: duration,
                    color: params.color.trim().to_string(),
                    note: params.note.trim().to_string(),
                    audio_start_ticks: None,
                    video_start_ticks: None,
                });
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({
                    "added": true,
                    "id": marker_id,
                    "range": duration.0 > 0
                }))
            })()),
            "marker.add_range" => Some((|| {
                let params: AddRangeMarkerParams = parse_params(params)?;
                if params.start.0 < 0 || params.end.0 <= params.start.0 {
                    return Err(DispatchError::InvalidParameters(
                        "A range marker needs a non-negative start before its end".into(),
                    ));
                }
                let name = marker_name(&self.document, params.name);
                self.checkpoint();
                let marker_id = self.document.project.next_id;
                self.document.project.next_id = marker_id.saturating_add(1);
                self.document.project.sequence.markers.push(Marker {
                    id: marker_id,
                    name,
                    kind: params.kind,
                    start_ticks: params.start,
                    duration_ticks: Tick(params.end.0 - params.start.0),
                    color: params.color.trim().to_string(),
                    note: params.note.trim().to_string(),
                    audio_start_ticks: None,
                    video_start_ticks: None,
                });
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({
                    "added": true,
                    "id": marker_id,
                    "duration": params.end.0 - params.start.0
                }))
            })()),
            "marker.update" => Some((|| {
                let params: UpdateMarkerParams = parse_params(params)?;
                let Some(existing) = self
                    .document
                    .project
                    .sequence
                    .markers
                    .iter()
                    .find(|marker| marker.id == params.id)
                    .cloned()
                else {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Marker {} does not exist",
                        params.id
                    )));
                };
                let start = params.at.unwrap_or(existing.start_ticks);
                let duration = params.duration.unwrap_or(existing.duration_ticks);
                if start.0 < 0 || duration.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Marker timing must be non-negative".into(),
                    ));
                }
                if start.0.checked_add(duration.0).is_none() {
                    return Err(DispatchError::InvalidParameters(
                        "Marker timing overflows the timebase".into(),
                    ));
                }
                let name = params
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or(existing.name);
                self.checkpoint();
                let Some(marker) = self
                    .document
                    .project
                    .sequence
                    .markers
                    .iter_mut()
                    .find(|marker| marker.id == params.id)
                else {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Marker {} does not exist",
                        params.id
                    )));
                };
                marker.name = name;
                marker.start_ticks = start;
                marker.duration_ticks = duration;
                if let Some(kind) = params.kind {
                    marker.kind = kind;
                }
                if let Some(color) = params.color {
                    marker.color = color.trim().to_string();
                }
                if let Some(note) = params.note {
                    marker.note = note.trim().to_string();
                }
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"updated": true, "id": params.id}))
            })()),
            "marker.remove" => Some((|| {
                let params: MarkerIdParams = parse_params(params)?;
                self.checkpoint();
                let before = self.document.project.sequence.markers.len();
                self.document
                    .project
                    .sequence
                    .markers
                    .retain(|marker| marker.id != params.id);
                Ok(json!({
                    "removed": self.document.project.sequence.markers.len() < before,
                    "id": params.id
                }))
            })()),
            "marker.clear" => Some((|| {
                self.checkpoint();
                let cleared = self.document.project.sequence.markers.len();
                self.document.project.sequence.markers.clear();
                Ok(json!({"cleared": cleared}))
            })()),
            "marker.next" => Some((|| {
                let params: MarkerNavParams = parse_params(params)?;
                let markers = &self.document.project.sequence.markers;
                Ok(match crate::markers::next_marker(markers, params.from) {
                    Some(marker) => json!({
                        "target": marker.start_ticks.0,
                        "id": marker.id,
                        "name": marker.name,
                    }),
                    None => json!({"target": null}),
                })
            })()),
            "marker.previous" => Some((|| {
                let params: MarkerNavParams = parse_params(params)?;
                let markers = &self.document.project.sequence.markers;
                Ok(match crate::markers::previous_marker(markers, params.from) {
                    Some(marker) => json!({
                        "target": marker.start_ticks.0,
                        "id": marker.id,
                        "name": marker.name,
                    }),
                    None => json!({"target": null}),
                })
            })()),
            "marker.ripple" => Some((|| {
                let params: RippleMarkersParams = parse_params(params)?;
                if params.delta.0 == 0 {
                    return Ok(json!({"shifted": 0, "clamped": 0}));
                }
                self.checkpoint();
                let mut shifted = 0;
                let mut clamped = 0;
                for marker in self.document.project.sequence.markers.iter_mut() {
                    if marker.start_ticks.0 < params.from.0 {
                        continue;
                    }
                    let target = marker.start_ticks.0.saturating_add(params.delta.0);
                    if target < 0 {
                        marker.start_ticks = Tick(0);
                        clamped += 1;
                    } else {
                        marker.start_ticks = Tick(target);
                    }
                    shifted += 1;
                }
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"shifted": shifted, "clamped": clamped}))
            })()),
            "marker.split" => Some((|| {
                let params: MarkerSplitParams = parse_params(params)?;
                if params.at.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Split point must be non-negative".into(),
                    ));
                }
                let split_targets: Vec<(u64, Marker)> = self
                    .document
                    .project
                    .sequence
                    .markers
                    .iter()
                    .filter(|marker| marker.contains(params.at))
                    .map(|marker| {
                        let second = Marker {
                            id: 0,
                            name: marker.name.clone(),
                            kind: marker.kind,
                            start_ticks: params.at,
                            duration_ticks: Tick(marker.end_ticks().0 - params.at.0),
                            color: marker.color.clone(),
                            note: marker.note.clone(),
                            audio_start_ticks: None,
                            video_start_ticks: None,
                        };
                        (marker.id, second)
                    })
                    .collect();
                if split_targets.is_empty() {
                    return Ok(json!({"split": 0}));
                }
                self.checkpoint();
                for (original_id, mut second) in split_targets {
                    if let Some(original) = self
                        .document
                        .project
                        .sequence
                        .markers
                        .iter_mut()
                        .find(|marker| marker.id == original_id)
                    {
                        original.duration_ticks = Tick(params.at.0 - original.start_ticks.0);
                    }
                    let new_id = self.document.project.next_id;
                    self.document.project.next_id = new_id.saturating_add(1);
                    second.id = new_id;
                    self.document.project.sequence.markers.push(second);
                }
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"split": 1}))
            })()),
            "marker.export_csv" => Some((|| {
                let markers = &self.document.project.sequence.markers;
                Ok(json!({
                    "csv": crate::markers::export_markers_csv(markers),
                    "count": markers.len()
                }))
            })()),
            "marker.import_csv" => Some((|| {
                let params: MarkerImportParams = parse_params(params)?;
                let parsed = crate::markers::parse_markers_csv(&params.csv)
                    .map_err(DispatchError::InvalidParameters)?;
                if parsed.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "The marker CSV contains no rows".into(),
                    ));
                }
                self.checkpoint();
                if params.replace {
                    self.document.project.sequence.markers.clear();
                }
                let imported = parsed.len();
                for row in parsed {
                    let marker_id = self.document.project.next_id;
                    self.document.project.next_id = marker_id.saturating_add(1);
                    self.document.project.sequence.markers.push(Marker {
                        id: marker_id,
                        name: row.name,
                        kind: row.kind,
                        start_ticks: row.start_ticks,
                        duration_ticks: row.duration_ticks,
                        color: row.color,
                        note: row.note,
                        audio_start_ticks: None,
                        video_start_ticks: None,
                    });
                }
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"imported": imported, "replaced": params.replace}))
            })()),
            "marker.add_chapter" => Some((|| {
                let params: AddMarkerParams = parse_params(params)?;
                self.add_typed_marker(params, MarkerKind::Chapter)
            })()),
            "marker.add_flash_cue" => Some((|| {
                let params: AddMarkerParams = parse_params(params)?;
                self.add_typed_marker(params, MarkerKind::FlashCue)
            })()),
            "marker.add_range_in_out" => Some((|| {
                let sequence = &self.document.project.sequence;
                let (start, end) = match (sequence.mark_in, sequence.mark_out) {
                    (Some(mark_in), Some(mark_out)) if mark_out.0 > mark_in.0 => (mark_in, mark_out),
                    _ => {
                        return Err(DispatchError::Operation(
                            "Set both In and Out points before adding a range marker".into(),
                        ))
                    }
                };
                let name = marker_name(&self.document, None);
                self.checkpoint();
                let marker_id = self.document.project.next_id;
                self.document.project.next_id = marker_id.saturating_add(1);
                self.document.project.sequence.markers.push(Marker {
                    id: marker_id,
                    name,
                    kind: MarkerKind::Comment,
                    start_ticks: start,
                    duration_ticks: Tick(end.0 - start.0),
                    color: String::new(),
                    note: String::new(),
                    audio_start_ticks: None,
                    video_start_ticks: None,
                });
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"added": true, "id": marker_id, "start": start.0, "end": end.0}))
            })()),
            "marker.set_color_filter" => Some((|| {
                let params: SetMarkerColorFilterParams = parse_params(params)?;
                let mut colors: Vec<String> = Vec::new();
                for color in params.colors {
                    let normalized = color.trim().to_lowercase();
                    if !normalized.is_empty() && !colors.contains(&normalized) {
                        colors.push(normalized);
                    }
                }
                self.store_pref("marker_color_filter", json!(colors))?;
                Ok(json!({"filter": colors}))
            })()),
            "marker.show_all_colors" => Some((|| {
                self.store_pref("marker_color_filter", json!([]))?;
                Ok(json!({"filter": []}))
            })()),
            "marker.set_copy_paste_includes" => Some((|| {
                let params: SetMarkerCopyPasteParams = parse_params(params)?;
                self.store_pref("marker_copy_paste_includes", json!(params.enabled))?;
                Ok(json!({"copy_paste_includes": params.enabled}))
            })()),
            "marker.set_split_audio" => Some(self.set_marker_split(params, true, false)),
            "marker.set_split_video" => Some(self.set_marker_split(params, false, false)),
            "marker.set_split_audio_out" => Some(self.set_marker_split(params, true, true)),
            "marker.set_split_video_out" => Some(self.set_marker_split(params, false, true)),
            _ => None,
        }
    }

    /// Merge one preference into the shared "prefs" object so it is
    /// readable through `prefs.get` and survives restarts with the settings
    /// file. Shared by the marker, panel, and freeform preference keys.
    pub(in crate::engine) fn store_pref(&mut self, key: &str, value: Value) -> Result<(), DispatchError> {
        let mut prefs = self
            .settings
            .object("prefs")
            .as_object()
            .cloned()
            .unwrap_or_default();
        prefs.insert(key.into(), value);
        self.settings.store_object("prefs", Value::Object(prefs))
    }

    /// Shared add path for the chapter/flash-cue marker wrappers: the kind is
    /// fixed by the command, everything else follows `marker.add`.
    fn add_typed_marker(
        &mut self,
        params: AddMarkerParams,
        kind: MarkerKind,
    ) -> Result<Value, DispatchError> {
        if params.at.0 < 0 {
            return Err(DispatchError::InvalidParameters(
                "Marker start must be non-negative".into(),
            ));
        }
        let duration = params.duration.unwrap_or(Tick(0));
        if duration.0 < 0 {
            return Err(DispatchError::InvalidParameters(
                "Marker duration must be non-negative".into(),
            ));
        }
        let name = marker_name(&self.document, params.name);
        self.checkpoint();
        let marker_id = self.document.project.next_id;
        self.document.project.next_id = marker_id.saturating_add(1);
        self.document.project.sequence.markers.push(Marker {
            id: marker_id,
            name,
            kind,
            start_ticks: params.at,
            duration_ticks: duration,
            color: params.color.trim().to_string(),
            note: params.note.trim().to_string(),
            audio_start_ticks: None,
            video_start_ticks: None,
        });
        sort_markers(&mut self.document.project.sequence.markers);
        Ok(json!({
            "added": true,
            "id": marker_id,
            "kind": kind,
            "range": duration.0 > 0
        }))
    }

    /// Set a split marker's audio/video in-position, or (with `out`) pull the
    /// shared duration so the domain position out lands at `at`. The split
    /// domains share the marker duration; moving a domain out therefore
    /// resizes both, which is the documented divergence until per-domain
    /// durations are modelled.
    fn set_marker_split(
        &mut self,
        params: Value,
        audio: bool,
        out: bool,
    ) -> Result<Value, DispatchError> {
        let params: SetMarkerSplitParams = parse_params(params)?;
        if params.at.0 < 0 {
            return Err(DispatchError::InvalidParameters(
                "Split position must be non-negative".into(),
            ));
        }
        let out_delta: Option<i64> = {
            let marker = self
                .document
                .project
                .sequence
                .markers
                .iter()
                .find(|marker| marker.id == params.id)
                .ok_or_else(|| {
                    DispatchError::InvalidParameters(format!(
                        "Marker {} does not exist",
                        params.id
                    ))
                })?;
            if out {
                let position = if audio {
                    marker.audio_position().0
                } else {
                    marker.video_position().0
                };
                if params.at.0 <= position {
                    return Err(DispatchError::InvalidParameters(
                        "The split out position must land after the domain in position".into(),
                    ));
                }
                Some(params.at.0 - position)
            } else {
                None
            }
        };
        self.checkpoint();
        {
            let marker = self
                .document
                .project
                .sequence
                .markers
                .iter_mut()
                .find(|marker| marker.id == params.id)
                .expect("marker verified above");
            if let Some(delta) = out_delta {
                marker.duration_ticks = Tick(delta);
            } else if audio {
                marker.audio_start_ticks = Some(params.at);
            } else {
                marker.video_start_ticks = Some(params.at);
            }
        }
        Ok(json!({
            "updated": true,
            "id": params.id,
            "domain": if audio { "audio" } else { "video" },
            "at": params.at.0,
            "out": out,
        }))
    }
}

fn sort_markers(markers: &mut [Marker]) {
    markers.sort_by_key(|marker| (marker.start_ticks.0, marker.id));
}

fn marker_name(document: &ProjectDocument, requested: Option<String>) -> String {
    if let Some(name) = requested.map(|value| value.trim().to_string()) {
        if !name.is_empty() {
            return name;
        }
    }
    let markers = &document.project.sequence.markers;
    let mut index = markers.len() + 1;
    loop {
        let candidate = format!("Marker {index}");
        if markers.iter().all(|marker| marker.name != candidate) {
            return candidate;
        }
        index += 1;
    }
}
