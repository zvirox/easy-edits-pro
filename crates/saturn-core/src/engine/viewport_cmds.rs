//! Viewport family: playhead navigation, clip/gap/bin selection and the
//! export-queue state, all owned by [`UnifiedViewportState`](crate::viewport_state::UnifiedViewportState)
//! (AI-native plan Packet 0.1).
//!
//! These commands were previously `ui_only`. They now mutate engine-held
//! state and return a `viewport` snapshot so MCP, the CLI and the desktop UI
//! read the same deterministic surface. This state is *not* undo content:
//! handlers never call `checkpoint`.

use super::*;
use crate::viewport_state::{
    find_clip_at_playhead, sequence_duration, edit_points, frame_ticks, ExportItemState,
    ExportQueueItem, MAX_EXPORT_QUEUE_ITEMS, SelectedClipRef, SelectedGap, UnifiedViewportState,
};

/// Strict empty-parameter payload: commands whose catalogue signature is `{}`
/// reject junk keys instead of silently ignoring them (agent-facing
/// determinism, AI-native plan Packet 0.1).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NoParams {}

fn parse_no_params(params: Value) -> Result<(), DispatchError> {
    parse_params::<NoParams>(params).map(|_| ())
}

impl EditorEngine {
    fn viewport_snapshot(&self) -> Value {
        let (video, audio) = self.targeting.snapshot();
        self.viewport.snapshot(&video, &audio)
    }

    fn clamp_playhead(&mut self, tick: Tick) {
        self.viewport.set_playhead(tick, sequence_duration(&self.document.project.sequence));
    }

    /// Last selected clip (the active one), validated against the document
    /// every time: selection references can go stale after edits.
    fn active_selected_clip(&self) -> Result<(u64, usize), DispatchError> {
        let Some(selected) = self.viewport.selected_clips.last().copied() else {
            return Err(DispatchError::Operation("No clip is selected".into()));
        };
        find_track_index(&self.document, selected.track_id)?;
        let track = &self.document.project.sequence
            .tracks
            .iter()
            .find(|track| track.id == selected.track_id)
            .expect("verified above");
        if selected.clip_index >= track.clips.len() {
            return Err(DispatchError::Operation(format!(
                "Clip index {} does not exist on track {}",
                selected.clip_index, selected.track_id
            )));
        }
        Ok((selected.track_id, selected.clip_index))
    }

    pub(super) fn dispatch_viewport(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        // Playhead navigation ------------------------------------------------
        if id.starts_with("playhead.") {
            return Some(self.dispatch_playhead(id, params));
        }
        if id.starts_with("workspace.") {
            return Some(self.dispatch_workspace(id, params));
        }
        // Selection ----------------------------------------------------------
        match id {
            "timeline.select" | "timeline.selectNextClip" | "timeline.selectPrevClip"
            | "timeline.selectGap" => Some(self.dispatch_timeline_selection(id, params)),
            "edit.select_all"
            | "edit.select_all_matching"
            | "edit.select_clip_at_playhead"
            | "edit.select_label_group" => Some(self.dispatch_edit_selection(id, params)),
            "project.select" | "project.selectAll" | "sequence.selection_follows_playhead" => {
                Some(self.dispatch_bin_selection(id, params))
            }
            // Export queue -------------------------------------------------------
            "viewport.inspect" => {
                let _ = parse_params::<serde_json::Value>(params).ok()?;
                Some(Ok(json!({ "viewport": self.viewport_snapshot() })))
            }
            "export.queue.add" | "export.queue.list" | "export.queue.cancel"
            | "export.queue.remove" | "export.queue.clear" | "export.queue.move"
            | "export.queue.retry" | "export.queue.start" | "export.queue.stop" => {
                Some(self.dispatch_export_queue(id, params))
            }
            _ => None,
        }
    }

    /// `workspace.*` switches: the active workspace is engine state now, so
    /// MCP/CLI agents and the desktop UI park on the same workspace.
    fn dispatch_workspace(&mut self, id: &str, params: Value) -> Result<Value, DispatchError> {
        let _ = parse_no_params(params)?;
        let workspace = id
            .strip_prefix("workspace.")
            .ok_or_else(|| DispatchError::UnknownCommand(id.to_owned()))?;
        let workspace = UnifiedViewportState::validate_workspace(workspace)
            .map_err(DispatchError::InvalidParameters)?;
        self.viewport.active_workspace = workspace;
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }

    fn dispatch_playhead(&mut self, id: &str, params: Value) -> Result<Value, DispatchError> {        let frame = frame_ticks(&self.document.project.settings.frame_rate);
        let duration = sequence_duration(&self.document.project.sequence);
        match id {
            "playhead.start" => {
                let _ = parse_no_params(params)?;
                self.clamp_playhead(Tick(0));
            }
            "playhead.end" => {
                let _ = parse_no_params(params)?;
                self.clamp_playhead(duration);
            }
            "playhead.nudge_forward_1" | "playhead.nudge_forward_5" => {
                let step = if id.ends_with("5") { 5 } else { 1 };
                let _ = parse_no_params(params)?;
                let target = self.viewport.playhead_tick.checked_add(frame * step).ok_or_else(|| {
                    DispatchError::InvalidParameters("Playhead nudge would overflow".into())
                })?;
                self.clamp_playhead(target);
            }
            "playhead.nudge_back_1" | "playhead.nudge_back_5" => {
                let step = if id.ends_with("5") { 5 } else { 1 };
                let _ = parse_no_params(params)?;
                let target = self.viewport.playhead_tick.checked_sub(frame * step).unwrap_or(Tick(0));
                self.clamp_playhead(target);
            }
            "playhead.next_edit" => {
                let _ = parse_no_params(params)?;
                let current = self.viewport.playhead_tick.0;
                let next = edit_points(&self.document.project.sequence)
                    .into_iter()
                    .find(|tick| *tick > current + 1)
                    .ok_or_else(|| {
                        DispatchError::Operation("No edit point after the playhead".into())
                    })?;
                self.clamp_playhead(Tick(next));
            }
            "playhead.previous_edit" => {
                let _ = parse_no_params(params)?;
                let current = self.viewport.playhead_tick.0;
                let previous = edit_points(&self.document.project.sequence)
                    .into_iter()
                    .rev()
                    .find(|tick| *tick < current - 1)
                    .ok_or_else(|| {
                        DispatchError::Operation("No edit point before the playhead".into())
                    })?;
                self.clamp_playhead(Tick(previous.max(0)));
            }
            "playhead.go_to_in" => {
                let _ = parse_no_params(params)?;
                let mark_in = self
                    .document
                    .project
                    .sequence
                    .mark_in
                    .ok_or_else(|| DispatchError::Operation("No mark in point set".into()))?;
                self.clamp_playhead(mark_in);
            }
            "playhead.go_to_out" => {
                let _ = parse_no_params(params)?;
                let mark_out = self
                    .document
                    .project
                    .sequence
                    .mark_out
                    .ok_or_else(|| DispatchError::Operation("No mark out point set".into()))?;
                self.clamp_playhead(mark_out);
            }
            "playhead.go_to_clip_start" | "playhead.go_to_clip_end" => {
                let _ = parse_no_params(params)?;
                let (track_id, clip_index) = self.active_selected_clip()?;
                let track = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .expect("verified above");
                let clip = &track.clips[clip_index];
                if id.ends_with("start") {
                    self.clamp_playhead(clip.timeline_start);
                } else {
                    self.clamp_playhead(clip.timeline_start.saturating_add(clip.duration.0));
                }
            }
            "playhead.go_to_timecode" => {
                let params: GoToTimecodeParams = parse_params(params)?;
                let target = crate::interop::timecode_to_ticks(&params.timecode, self.document.project.settings.frame_rate)
                    .map_err(DispatchError::InvalidParameters)?;
                self.clamp_playhead(target);
            }
            _ => return Err(DispatchError::UnknownCommand(id.to_owned())),
        }
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }

    fn dispatch_timeline_selection(
        &mut self,
        id: &str,
        params: Value,
    ) -> Result<Value, DispatchError> {
        match id {
            "timeline.select" => {
                let params: TimelineSelectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let track = &self.document.project.sequence.tracks[track_index];
                if params.clip_index >= track.clips.len() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Clip index {} does not exist on track {}",
                        params.clip_index, params.track_id
                    )));
                }
                self.viewport.selected_gap = None;
                self.viewport.selected_clips = vec![SelectedClipRef {
                    track_id: params.track_id,
                    clip_index: params.clip_index,
                }];
            }
            "timeline.selectGap" => {
                let default_tick = Tick(self.viewport.playhead_tick.0);
                let params: TimelineGapSelectParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let start = params.start_tick.unwrap_or(default_tick);
                let end = params.end_tick.unwrap_or_else(|| params.start_tick.unwrap_or(default_tick));
                if end.0 <= start.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Selected gap end must be after its start".into(),
                    ));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                // The gap must be empty on the selected track.
                let overlaps = track.clips.iter().any(|clip| {
                    let clip_end = clip.timeline_start.saturating_add(clip.duration.0).0;
                    start.0 < clip_end && end.0 > clip.timeline_start.0
                });
                if overlaps {
                    return Err(DispatchError::Operation(
                        "Selected range contains clips; use timeline.select instead".into(),
                    ));
                }
                self.viewport.selected_clips.clear();
                self.viewport.selected_gap = Some(SelectedGap {
                    track_id: params.track_id,
                    start_tick: start,
                    end_tick: end,
                });
            }
            "timeline.selectNextClip" | "timeline.selectPrevClip" => {
                let _ = parse_no_params(params)?;
                let forward = id.ends_with("NextClip");
                let (track_id, clip_index) = self.active_selected_clip()?;
                let next_index = if forward {
                    clip_index.checked_add(1)
                } else {
                    clip_index.checked_sub(1)
                };
                let track = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .expect("verified above");
                let next_index = next_index.filter(|index| *index < track.clips.len()).ok_or_else(|| {
                    DispatchError::Operation(if forward {
                        "No next clip on the selected track".into()
                    } else {
                        "No previous clip on the selected track".into()
                    })
                })?;
                self.viewport.selected_gap = None;
                self.viewport.selected_clips = vec![SelectedClipRef {
                    track_id,
                    clip_index: next_index,
                }];
            }
            _ => return Err(DispatchError::UnknownCommand(id.to_owned())),
        }
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }

    fn dispatch_edit_selection(
        &mut self,
        id: &str,
        params: Value,
    ) -> Result<Value, DispatchError> {
        match id {
            "edit.select_all" => {
                let _ = parse_no_params(params)?;
                self.viewport.selected_clips = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .flat_map(|track| {
                        (0..track.clips.len()).map(|clip_index| SelectedClipRef {
                            track_id: track.id,
                            clip_index,
                        })
                    })
                    .collect();
                self.viewport.selected_gap = None;
            }
            "edit.select_all_matching" => {
                let params: SelectMatchingParams = parse_params(params)?;
                let query = params.query.trim().to_lowercase();
                if query.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "select_all_matching requires a non-empty `query`".into(),
                    ));
                }
                let matched: Vec<SelectedClipRef> = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .flat_map(|track| track.clips.iter().enumerate().map(move |(clip_index, clip)| {
                        (track.id, clip_index, clip)
                    }))
                    .filter(|(_, _, clip)| clip.label.to_lowercase().contains(&query))
                    .map(|(track_id, clip_index, _)| SelectedClipRef { track_id, clip_index })
                    .collect();
                if matched.is_empty() {
                    return Err(DispatchError::Operation(format!(
                        "No clips match label `{}`",
                        params.query
                    )));
                }
                self.viewport.selected_gap = None;
                self.viewport.selected_clips = matched;
            }
            "edit.select_clip_at_playhead" => {
                let _ = parse_no_params(params)?;
                let selected = find_clip_at_playhead(
                    &self.document.project.sequence,
                    self.viewport.playhead_tick,
                )
                .ok_or_else(|| {
                    DispatchError::Operation("No enabled video clip under the playhead".into())
                })?;
                self.viewport.selected_gap = None;
                self.viewport.selected_clips = vec![selected];
            }
            "edit.select_label_group" => {
                let params: SelectLabelGroupParams = parse_params(params)?;
                let label = params.label.trim().to_string();
                if label.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "select_label_group requires a non-empty `label`".into(),
                    ));
                }
                let matched: Vec<SelectedClipRef> = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .flat_map(|track| track.clips.iter().enumerate().map(move |(clip_index, clip)| {
                        (track.id, clip_index, clip)
                    }))
                    .filter(|(_, _, clip)| clip.label == label)
                    .map(|(track_id, clip_index, _)| SelectedClipRef { track_id, clip_index })
                    .collect();
                if matched.is_empty() {
                    return Err(DispatchError::Operation(format!(
                        "No clips carry label `{label}`"
                    )));
                }
                self.viewport.selected_gap = None;
                self.viewport.selected_clips = matched;
            }
            _ => return Err(DispatchError::UnknownCommand(id.to_owned())),
        }
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }

    fn dispatch_bin_selection(
        &mut self,
        id: &str,
        params: Value,
    ) -> Result<Value, DispatchError> {
        match id {
            "project.select" => {
                let params: MediaSelectParams = parse_params(params)?;
                let media_ids = params.media_ids();
                for media_id in &media_ids {
                    if !self.document.project.media.iter().any(|asset| asset.id == *media_id) {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Media {media_id} does not exist"
                        )));
                    }
                }
                self.viewport.selected_media_ids = media_ids;
            }
            "project.selectAll" => {
                let _ = parse_no_params(params)?;
                self.viewport.selected_media_ids =
                    self.document.project.media.iter().map(|asset| asset.id).collect();
            }
            "sequence.selection_follows_playhead" => {
                let params: SelectionFollowsParams = parse_params(params)?;
                self.viewport.selection_follows_playhead = params.enabled;
            }
            _ => return Err(DispatchError::UnknownCommand(id.to_owned())),
        }
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }

    fn dispatch_export_queue(
        &mut self,
        id: &str,
        params: Value,
    ) -> Result<Value, DispatchError> {
        match id {
            "export.queue.add" => {
                let params: QueueAddParams = parse_params(params)?;
                let output_path = params.output_path.trim().to_string();
                if output_path.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "export.queue.add requires a non-empty `output_path`".into(),
                    ));
                }
                if self.viewport.export_queue.len() >= MAX_EXPORT_QUEUE_ITEMS {
                    return Err(DispatchError::Operation("Export queue is full".into()));
                }
                let item = ExportQueueItem::new(
                    self.viewport.next_export_item_id,
                    output_path,
                    params.settings.unwrap_or_else(|| json!({})),
                );
                self.viewport.next_export_item_id += 1;
                self.viewport.export_queue.push(item);
            }
            "export.queue.list" => {
                let _ = parse_no_params(params)?;
                let snapshot = self.viewport_snapshot();
                let queue = snapshot.get("export_queue").cloned().unwrap_or(Value::Null);
                let paused = snapshot
                    .get("export_queue_paused")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                return Ok(json!({ "paused": paused, "items": queue }));
            }
            "export.queue.cancel" | "export.queue.remove" | "export.queue.retry"
            | "export.queue.move" => {
                if id == "export.queue.move" {
                    let params: QueueMoveParams = parse_params(params)?;
                    let position =
                        self.viewport
                            .export_queue
                            .iter()
                            .position(|item| item.id == params.id)
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Export queue item {} does not exist",
                                    params.id
                                ))
                            })?;
                    let to = params.position.min(self.viewport.export_queue.len() - 1);
                    let item = self.viewport.export_queue.remove(position);
                    self.viewport.export_queue.insert(to, item);
                } else {
                    let params: QueueIdParams = parse_params(params)?;
                    let position =
                        self.viewport
                            .export_queue
                            .iter()
                            .position(|item| item.id == params.id)
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Export queue item {} does not exist",
                                    params.id
                                ))
                            })?;
                    match id {
                        "export.queue.cancel" => {
                            let item = &mut self.viewport.export_queue[position];
                            match item.state {
                                ExportItemState::Queued
                                | ExportItemState::Running
                                | ExportItemState::Paused => item.state = ExportItemState::Cancelled,
                                _ => {
                                    return Err(DispatchError::Operation(
                                        "Only queued, running or paused items can be cancelled".into(),
                                    ));
                                }
                            }
                        }
                        "export.queue.remove" => {
                            self.viewport.export_queue.remove(position);
                        }
                        "export.queue.retry" => {
                            let item = &mut self.viewport.export_queue[position];
                            match item.state {
                                ExportItemState::Failed | ExportItemState::Cancelled => {
                                    item.state = ExportItemState::Queued;
                                    item.error = None;
                                }
                                _ => {
                                    return Err(DispatchError::Operation(
                                        "Only failed or cancelled items can be retried".into(),
                                    ));
                                }
                            }
                        }
                        _ => unreachable!("matched above"),
                    }
                }
            }
            "export.queue.clear" => {
                let _ = parse_no_params(params)?;
                self.viewport.export_queue.clear();
            }
            "export.queue.start" => {
                let _ = parse_no_params(params)?;
                self.viewport.export_queue_paused = false;
            }
            "export.queue.stop" => {
                let _ = parse_no_params(params)?;
                self.viewport.export_queue_paused = true;
            }
            _ => return Err(DispatchError::UnknownCommand(id.to_owned())),
        }
        Ok(json!({ "viewport": self.viewport_snapshot() }))
    }
}

#[derive(Deserialize)]
struct GoToTimecodeParams {
    timecode: String,
}
#[derive(Deserialize)]
struct TimelineSelectParams {
    track_id: u64,
    clip_index: usize,
}
#[derive(Deserialize)]
struct TimelineGapSelectParams {
    track_id: u64,
    start_tick: Option<Tick>,
    end_tick: Option<Tick>,
}
#[derive(Deserialize)]
struct SelectMatchingParams {
    query: String,
}
#[derive(Deserialize)]
struct SelectLabelGroupParams {
    label: String,
}
#[derive(Deserialize)]
struct MediaSelectParams {
    #[serde(default)]
    media_ids: Vec<u64>,
    media_id: Option<u64>,
}
// Accept the singular form as a convenience; merge it into the list.
impl MediaSelectParams {
    fn media_ids(self) -> Vec<u64> {
        if !self.media_ids.is_empty() {
            self.media_ids
        } else {
            self.media_id.map(|id| vec![id]).unwrap_or_default()
        }
    }
}
#[derive(Deserialize)]
struct SelectionFollowsParams {
    enabled: bool,
}
#[derive(Deserialize)]
struct QueueAddParams {
    output_path: String,
    #[serde(default)]
    settings: Option<Value>,
}
#[derive(Deserialize)]
struct QueueIdParams {
    id: u64,
}
#[derive(Deserialize)]
struct QueueMoveParams {
    id: u64,
    #[serde(default)]
    position: usize,
}
