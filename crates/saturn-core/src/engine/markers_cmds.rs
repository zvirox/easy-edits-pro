use super::*;

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
                    });
                }
                sort_markers(&mut self.document.project.sequence.markers);
                Ok(json!({"imported": imported, "replaced": params.replace}))
            })()),
            _ => None,
        }
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
