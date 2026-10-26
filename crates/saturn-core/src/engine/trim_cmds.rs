use super::*;

/// The edit point the trim family operates on: a clip edge plus its track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct TrimSelection {
    pub(super) track_id: u64,
    pub(super) clip_index: usize,
    pub(super) edge: TrimEdge,
}

/// Engine-held trim state: the selected edit point and the trim type flag
/// (ripple versus overwrite). Both are editor state, not document state, so
/// they stay out of undo snapshots.
#[derive(Clone, Default)]
pub(super) struct TrimState {
    pub(super) selection: Option<TrimSelection>,
    pub(super) ripple: bool,
}

/// One editable clip boundary, in timeline ticks.
#[derive(Clone, Copy, Debug)]
struct EditPoint {
    track_id: u64,
    track_index: usize,
    clip_index: usize,
    edge: TrimEdge,
    tick: i64,
}

fn collect_track_edit_points(document: &ProjectDocument, track_index: usize) -> Vec<EditPoint> {
    let track = &document.project.sequence.tracks[track_index];
    track
        .clips
        .iter()
        .enumerate()
        .flat_map(|(clip_index, clip)| {
            let start = clip.timeline_start.0;
            let end = start.saturating_add(clip.duration.0);
            [
                EditPoint {
                    track_id: track.id,
                    track_index,
                    clip_index,
                    edge: TrimEdge::Start,
                    tick: start,
                },
                EditPoint {
                    track_id: track.id,
                    track_index,
                    clip_index,
                    edge: TrimEdge::End,
                    tick: end,
                },
            ]
        })
        .collect()
}

fn collect_all_edit_points(document: &ProjectDocument) -> Result<Vec<EditPoint>, DispatchError> {
    let mut points = Vec::new();
    for track_index in 0..document.project.sequence.tracks.len() {
        points.extend(collect_track_edit_points(document, track_index));
    }
    if points.is_empty() {
        return Err(DispatchError::Operation(
            "Timeline has no edit points".into(),
        ));
    }
    points.sort_by_key(|point| (point.tick, point.track_index, point.clip_index));
    Ok(points)
}

/// Delta that moves an edit point boundary to the given absolute tick.
fn edit_tick(document: &ProjectDocument, selection: TrimSelection) -> Result<i64, DispatchError> {
    let track_index = find_track_index(document, selection.track_id)?;
    let clip = find_clip(document, track_index, selection.clip_index)?;
    Ok(match selection.edge {
        TrimEdge::Start => clip.timeline_start.0,
        TrimEdge::End => clip
            .timeline_start
            .0
            .saturating_add(clip.duration.0),
    })
}

impl EditorEngine {
    pub(super) fn dispatch_trim(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "trim.selectEditPoint" => Some((|| {
                let params: TrimSelectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let selection = TrimSelection {
                    track_id: params.track_id,
                    clip_index: params.clip_index,
                    edge: params.edge,
                };
                self.trim.selection = Some(selection);
                Ok(json!({
                    "selected": true,
                    "track_id": selection.track_id,
                    "clip_index": selection.clip_index,
                    "edge": selection.edge,
                }))
            })()),
            "trim.selectNearest" => Some((|| {
                let params: TrimOptionalAtParams = parse_optional_at(params)?;
                let at = params.at.map(|tick| tick.0).unwrap_or(0);
                let points = collect_all_edit_points(&self.document)?;
                let best = *points
                    .iter()
                    .min_by_key(|point| (point.tick - at).abs())
                    .expect("collect_all_edit_points errors on empty");
                let selection = TrimSelection {
                    track_id: best.track_id,
                    clip_index: best.clip_index,
                    edge: best.edge,
                };
                self.trim.selection = Some(selection);
                Ok(json!({
                    "selected": true,
                    "track_id": selection.track_id,
                    "clip_index": selection.clip_index,
                    "edge": selection.edge,
                    "tick": best.tick,
                }))
            })()),
            "trim.clear" => Some((|| {
                let previous = self.trim.selection.take();
                Ok(json!({
                    "cleared": previous.is_some(),
                    "selection": previous,
                }))
            })()),
            "trim.monitor" => Some((|| {
                let params: TrimOptionalAtParams = parse_optional_at(params)?;
                let at = params.at.map(|tick| tick.0).unwrap_or(0);
                let selection = self.trim.selection;
                let mut selection_info = Value::Null;
                if let Some(selected) = selection {
                    let track_index = find_track_index(&self.document, selected.track_id)?;
                    let track = &self.document.project.sequence.tracks[track_index];
                    let clip = &track.clips[selected.clip_index];
                    selection_info = json!({
                        "track_id": selected.track_id,
                        "track_name": track.name,
                        "clip_index": selected.clip_index,
                        "edge": selected.edge,
                        "timeline_start": clip.timeline_start.0,
                        "duration": clip.duration.0,
                        "source_in": clip.source_in.0,
                    });
                }
                let neighbors = collect_all_edit_points(&self.document)?;
                let before = neighbors
                    .iter()
                    .rev()
                    .find(|point| point.tick <= at)
                    .map(|point| json!({
                        "track_id": point.track_id,
                        "clip_index": point.clip_index,
                        "edge": point.edge,
                        "tick": point.tick,
                    }));
                let after = neighbors
                    .iter()
                    .find(|point| point.tick > at)
                    .map(|point| json!({
                        "track_id": point.track_id,
                        "clip_index": point.clip_index,
                        "edge": point.edge,
                        "tick": point.tick,
                    }));
                Ok(json!({
                    "selection": selection_info,
                    "ripple": self.trim.ripple,
                    "at": at,
                    "previous_edit": before,
                    "next_edit": after,
                }))
            })()),
            "trim.edit" => Some((|| {
                let selected = self.require_selection()?;
                let tick = edit_tick(&self.document, selected)?;
                Ok(json!({
                    "editing": true,
                    "track_id": selected.track_id,
                    "clip_index": selected.clip_index,
                    "edge": selected.edge,
                    "cut": tick,
                    "ripple": self.trim.ripple,
                }))
            })()),
            "trim.forward" => Some(self.trim_by_frames(1, None)),
            "trim.backward" => Some(self.trim_by_frames(-1, None)),
            "trim.forwardMany" => Some(self.trim_by_frames(10, None)),
            "trim.backwardMany" => Some(self.trim_by_frames(-10, None)),
            "trim.nudge" => Some((|| {
                let params: TrimNudgeParams = parse_params(params)?;
                self.trim_by_frames(params.frames, params.ripple)
            })()),
            "trim.extendToPlayhead" => Some((|| {
                let params: TrimAtParams = parse_params(params)?;
                let selected = self.require_selection()?;
                let tick = edit_tick(&self.document, selected)?;
                let delta = params.at.0 - tick;
                if delta == 0 {
                    return Ok(json!({
                        "trimmed": false,
                        "reason": "Edit point already at the playhead",
                    }));
                }
                self.trim_apply(selected, delta, self.trim.ripple)
            })()),
            "trim.extendNextEdit" => Some((|| {
                let params: TrimAtParams = parse_params(params)?;
                self.trim_neighbor_edit(params.at.0, true, self.trim.ripple)
            })()),
            "trim.extendPreviousEdit" => Some((|| {
                let params: TrimAtParams = parse_params(params)?;
                self.trim_neighbor_edit(params.at.0, false, self.trim.ripple)
            })()),
            "trim.next" => Some((|| {
                let params: TrimOptionalAtParams = parse_optional_at(params)?;
                let at = params.at.map(|tick| tick.0).unwrap_or(0);
                self.trim_neighbor_edit(at, true, self.trim.ripple)
            })()),
            "trim.previous" => Some((|| {
                let params: TrimOptionalAtParams = parse_optional_at(params)?;
                let at = params.at.map(|tick| tick.0).unwrap_or(0);
                self.trim_neighbor_edit(at, false, self.trim.ripple)
            })()),
            "trim.rippleNext" => Some((|| {
                let params: TrimAtParams = parse_params(params)?;
                self.trim_neighbor_edit(params.at.0, true, true)
            })()),
            "trim.ripplePrevious" => Some((|| {
                let params: TrimAtParams = parse_params(params)?;
                self.trim_neighbor_edit(params.at.0, false, true)
            })()),
            "trim.toggleType" => Some((|| {
                self.trim.ripple = !self.trim.ripple;
                Ok(json!({"ripple": self.trim.ripple}))
            })()),
            "trim.applyDefaultTransition" => Some((|| {
                let params: TrimTransitionParams = parse_transition_params(params)?;
                let selected = self.require_selection()?;
                let cut = edit_tick(&self.document, selected)?;
                self.execute(CommandRequest::new(
                    "timeline.add_transition",
                    json!({
                        "track_id": selected.track_id,
                        "cut_at": cut,
                        "duration": params.duration,
                        "kind": params.kind,
                    }),
                ))
            })()),
            _ => None,
        }
    }

    fn require_selection(&self) -> Result<TrimSelection, DispatchError> {
        self.trim.selection.ok_or_else(|| {
            DispatchError::Operation(
                "No edit point selected - use Select Edit Point first".into(),
            )
        })
    }

    fn trim_frame_ticks(&self) -> Result<i64, DispatchError> {
        self.document
            .project
            .settings
            .frame_rate
            .frame_ticks()
            .ok_or_else(|| {
                DispatchError::Operation(
                    "Project frame rate does not map to whole-frame ticks".into(),
                )
            })
    }

    /// Selected edit point + signed frame count -> one trim through
    /// `trim_clip_core`. `ripple_override` replaces the trim type flag when
    /// given (trim.nudge).
    fn trim_by_frames(
        &mut self,
        frames: i64,
        ripple_override: Option<bool>,
    ) -> Result<Value, DispatchError> {
        let selected = self.require_selection()?;
        let frame_ticks = self.trim_frame_ticks()?;
        let delta = frames.saturating_mul(frame_ticks);
        let ripple = ripple_override.unwrap_or(self.trim.ripple);
        self.trim_apply(selected, delta, ripple)
    }

    /// Run one trim against the selection and echo it in the result.
    fn trim_apply(
        &mut self,
        selected: TrimSelection,
        delta: i64,
        ripple: bool,
    ) -> Result<Value, DispatchError> {
        let mut result = self.trim_clip_core(
            selected.track_id,
            selected.clip_index,
            selected.edge,
            Tick(delta),
            ripple,
        )?;
        if let Value::Object(ref mut map) = result {
            map.insert(
                "selection".into(),
                serde_json::to_value(selected).unwrap_or(Value::Null),
            );
        }
        Ok(result)
    }

    /// Find the next (or previous) edit boundary relative to the playhead,
    /// select it, and trim it onto the playhead.
    fn trim_neighbor_edit(&mut self, at: i64, forward: bool, ripple: bool) -> Result<Value, DispatchError> {
        let points = collect_all_edit_points(&self.document)?;
        let target = if forward {
            points
                .iter()
                .find(|point| point.tick > at)
                .copied()
        } else {
            // Scan from the end so ties resolve to the boundary immediately
            // left of the playhead (the last point in ascending order).
            points
                .iter()
                .rev()
                .find(|point| point.tick < at)
                .copied()
        };
        let target = target.ok_or_else(|| {
            DispatchError::Operation(if forward {
                "No edit point after the playhead".into()
            } else {
                "No edit point before the playhead".into()
            })
        })?;
        let selection = TrimSelection {
            track_id: target.track_id,
            clip_index: target.clip_index,
            edge: target.edge,
        };
        self.trim.selection = Some(selection);
        let delta = at - target.tick;
        if delta == 0 {
            return Ok(json!({
                "trimmed": false,
                "reason": "Edit point already at the playhead",
                "selection": selection,
            }));
        }
        self.trim_apply(selection, delta, ripple)
    }
}

/// `parse_params` for payloads that are entirely optional: a bare `null`
/// (dispatch without params) means "all defaults".
fn parse_optional_at(params: Value) -> Result<TrimOptionalAtParams, DispatchError> {
    if params.is_null() {
        Ok(TrimOptionalAtParams { at: None })
    } else {
        parse_params(params)
    }
}

fn parse_transition_params(params: Value) -> Result<TrimTransitionParams, DispatchError> {
    if params.is_null() {
        Ok(TrimTransitionParams {
            duration: None,
            kind: None,
        })
    } else {
        parse_params(params)
    }
}
