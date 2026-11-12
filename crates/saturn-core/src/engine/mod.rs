// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use crate::actor::{ActorContext, ActorId, SessionViewState};
use crate::command::{find_command, status, CommandRequest, CommandStatus};
use crate::journal::ChangeJournal;
use crate::project::{
    AutomationInterpolation, AutomationLane, AutomationMode, BusTarget,
    ChromaKeyParams, Clip, ColorAdjustments, ColorCorrectionParams, ColorNode, ColorNodeKind,
    ColorWheels, CompositingNode, CompositingNodeKind, CropParams, CustomShaderParams,
    EffectInstance, EffectKind, EffectPayload, GaussianBlurParams, InterpolationKind, Keyframe,
    KeyframeProperty, LutParams, Marker, MediaAsset, MediaKind, MotionBlurParams,
    MotionTrackingData, MultiCamAngle, MultiCamClip, MultiCamSyncMode, NodeConnection, NodeGraph,
    ProjectDocument, RotoSplineMask, Sequence, SpeedRamp, SubtitleItem, TitleProperties, Track,
    TrackAudioProcessing, TrackAutomation, TrackGroup, TrackKind, TrackingMode, Transition,
    TransitionKind, VignetteParams, SUPPORTED_PRINT_STOCKS, SHOT_MATCH_ORIGIN, PowerWindow,
    WindowShape, WindowTransform, default_payload, validate_payload,
};
use crate::time::{FrameRate, Tick, TICKS_PER_SECOND};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

mod audio_cmds;
mod audio_edit_cmds;
mod asymmetric_trim_cmd;
mod match_cut_cmd;
mod beat_grid_cmd;
mod branch_cmds;
mod brief_cmds;
mod clip_advanced_cmds;
mod clip_cmds;
mod color_cmds;
mod color_node_cmds;
mod compound_cmds;
mod delivery_cmds;
mod edit_cmds;
mod dsp_cmds;
mod edit_labels_cmds;
mod effects_cmds;
pub(crate) mod helpers;
mod history;
mod keyframe_cmds;
mod lock_cmds;
mod markers_cmds;
mod media_cmds;
mod review_cmds;
mod ocio_cmds;
mod nodegraph_cmds;
mod export_cmds;
mod params;
mod project_cmds;
mod project_freeform_cmds;
mod project_panel_cmds;
mod sequence_cmds;
mod settings_cmds;
mod tasksheet_cmds;
mod tasksheet_gates;
mod captions_cmds;
mod graphics_cmds;
mod timeline_cmds;
mod timeline_targeting;
mod trim_cmds;
mod viewport_cmds;
use helpers::*;
use params::*;

#[derive(Debug)]
pub enum DispatchError {
    UnknownCommand(String),
    /// The command is declared `ui_only` and must be handled by the frontend.
    UiOnly(String),
    Disabled {
        id: String,
        reason: String,
    },
    InvalidParameters(String),
    Operation(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(id) => write!(f, "Unknown command `{id}`"),
            Self::UiOnly(id) => write!(f, "Command `{id}` is handled by the interface"),
            Self::Disabled { id, reason } => write!(f, "Command `{id}` is disabled: {reason}"),
            Self::InvalidParameters(error) | Self::Operation(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for DispatchError {}

#[derive(Clone)]
pub struct EditorEngine {
    document: ProjectDocument,
    current_path: Option<PathBuf>,
    undo: Vec<ProjectDocument>,
    redo: Vec<ProjectDocument>,
    /// Entry labels parallel to `undo`/`redo`: the command id that produced
    /// each snapshot, newest last.
    undo_labels: Vec<String>,
    redo_labels: Vec<String>,
    /// Command id of the dispatch in flight; consumed by `checkpoint`.
    pending_label: Option<String>,
    saved_snapshot: String,
    gesture_depth: u32,
    gesture_checkpointed: bool,
    settings: settings_cmds::SettingsStore,
    /// Trim family state: selected edit point + ripple flag (editor state,
    /// not document state).
    trim: trim_cmds::TrimState,
    /// Edit family state: the attribute clipboard (editor state, not
    /// document state).
    edit: edit_labels_cmds::EditState,
    /// Timeline targeting family state: the targeted video/audio track
    /// ordinals (editor state, not document state).
    targeting: timeline_targeting::TargetingState,
    /// Unified playhead/selection/export-queue state (AI-native plan Packet
    /// 0.1; editor state, not document state).
    viewport: crate::viewport_state::UnifiedViewportState,
    /// Per-actor session view states (AF2).
    pub sessions: std::collections::HashMap<String, SessionViewState>,
    /// Project-wide change journal and attribution records (AF2).
    pub journal: ChangeJournal,
    /// Stopped sessions fenced against mutation (AF2).
    pub stopped_sessions: std::collections::HashSet<String>,
    /// Active actor context during command execution (AF2).
    current_context: Option<ActorContext>,
    /// Lazily-built shaping context (system font scan is expensive); shared
    /// so engine clones keep one font database.
    text: std::sync::Arc<
        std::sync::Mutex<Option<saturn_text::TextContext>>,
    >,
}

impl Default for EditorEngine {
    fn default() -> Self {
        let document = ProjectDocument::default();
        let saved_snapshot = serde_json::to_string(&document).unwrap_or_default();
        Self {
            document,
            current_path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            undo_labels: Vec::new(),
            redo_labels: Vec::new(),
            pending_label: None,
            saved_snapshot,
            gesture_depth: 0,
            gesture_checkpointed: false,
            settings: settings_cmds::SettingsStore::default(),
            trim: trim_cmds::TrimState::default(),
            edit: edit_labels_cmds::EditState::default(),
            targeting: timeline_targeting::TargetingState::new(),
            viewport: crate::viewport_state::UnifiedViewportState::new(),
            sessions: std::collections::HashMap::new(),
            journal: ChangeJournal::new(),
            stopped_sessions: std::collections::HashSet::new(),
            current_context: None,
            text: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl EditorEngine {
    pub fn document(&self) -> &ProjectDocument {
        &self.document
    }
    pub fn current_path(&self) -> Option<&std::path::Path> {
        self.current_path.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        serde_json::to_string(&self.document)
            .map(|snapshot| snapshot != self.saved_snapshot)
            .unwrap_or(true)
    }

    pub fn restore_recovery(
        &mut self,
        document: ProjectDocument,
        source_path: Option<PathBuf>,
    ) -> Result<(), DispatchError> {
        document.validate().map_err(DispatchError::Operation)?;
        self.document = document;
        self.current_path = source_path;
        self.undo.clear();
        self.redo.clear();
        self.undo_labels.clear();
        self.redo_labels.clear();
        self.saved_snapshot.clear();
        Ok(())
    }

    /// Open an undo gesture. Mutating commands executed until `end_gesture`
    /// collapse into the first history entry taken inside the gesture.
    pub fn begin_gesture(&mut self) {
        if self.gesture_depth == 0 {
            self.gesture_checkpointed = false;
        }
        self.gesture_depth = self.gesture_depth.saturating_add(1);
    }

    /// Close an undo gesture. Unbalanced calls are ignored.
    pub fn end_gesture(&mut self) {
        self.gesture_depth = self.gesture_depth.saturating_sub(1);
    }

    pub fn in_gesture(&self) -> bool {
        self.gesture_depth > 0
    }

    pub fn command_status(&self, id: &str) -> Result<CommandStatus, DispatchError> {
        let command =
            find_command(id).ok_or_else(|| DispatchError::UnknownCommand(id.to_owned()))?;
        if command.ui_only {
            return Ok(status(true, ""));
        }
        let result = match id {
            "edit.undo" => status(!self.undo.is_empty(), "Nothing to undo"),
            "edit.redo" => status(!self.redo.is_empty(), "Nothing to redo"),
            _ => status(true, ""),
        };
        Ok(result)
    }

    pub fn current_context(&self) -> Option<&ActorContext> {
        self.current_context.as_ref()
    }

    pub fn current_actor(&self) -> ActorId {
        self.current_context
            .as_ref()
            .map(|c| c.actor.clone())
            .unwrap_or(ActorId::Human)
    }

    pub fn current_session_id(&self) -> String {
        self.current_context
            .as_ref()
            .map(|c| c.session_id.clone())
            .unwrap_or_else(|| "human-gui".to_string())
    }

    pub fn current_branch(&self) -> String {
        self.current_context
            .as_ref()
            .map(|c| c.branch.clone())
            .unwrap_or_else(|| crate::branch::MAIN_BRANCH.to_string())
    }

    pub fn check_actor_context(&self, ctx: &ActorContext) -> Result<(), DispatchError> {
        if self.stopped_sessions.contains(&ctx.session_id) {
            return Err(DispatchError::Operation(format!(
                "SessionStopped: session '{}' has been stopped. Subsequent calls are rejected until resumed by human authority.",
                ctx.session_id
            )));
        }
        if let Some(session) = self.sessions.get(&ctx.session_id) {
            if session.is_stopped {
                return Err(DispatchError::Operation(format!(
                    "SessionStopped: session '{}' has been stopped. Subsequent calls are rejected until resumed by human authority.",
                    ctx.session_id
                )));
            }
            if ctx.generation < session.generation {
                return Err(DispatchError::Operation(format!(
                    "GenerationStale: session '{}' request generation {} is older than active fenced generation {}.",
                    ctx.session_id, ctx.generation, session.generation
                )));
            }
        }
        Ok(())
    }

    pub fn get_or_create_session(&mut self, ctx: &ActorContext) -> &mut SessionViewState {
        self.sessions
            .entry(ctx.session_id.clone())
            .or_insert_with(|| SessionViewState::new(ctx.actor.clone(), ctx.session_id.clone()))
    }

    pub fn stop_session(&mut self, session_id: &str) -> u64 {
        self.stopped_sessions.insert(session_id.to_string());
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.stop()
        } else {
            1
        }
    }

    pub fn resume_session(&mut self, session_id: &str, caller: &ActorId) -> Result<u64, DispatchError> {
        if !caller.is_human() {
            return Err(DispatchError::Operation(
                "PermissionDenied: only human authority can resume stopped sessions".into(),
            ));
        }
        self.stopped_sessions.remove(session_id);
        if let Some(session) = self.sessions.get_mut(session_id) {
            Ok(session.resume())
        } else {
            Ok(1)
        }
    }

    pub fn is_session_stopped(&self, session_id: &str) -> bool {
        self.stopped_sessions.contains(session_id)
            || self.sessions.get(session_id).map(|s| s.is_stopped).unwrap_or(false)
    }

    pub fn session_viewport_snapshot(&self, ctx: &ActorContext) -> Value {
        if ctx.actor.is_human() {
            let (video, audio) = self.targeting.snapshot();
            self.viewport.snapshot(&video, &audio)
        } else if let Some(session) = self.sessions.get(&ctx.session_id) {
            json!({
                "actor": session.actor,
                "session_id": session.session_id,
                "playhead_tick": session.playhead.0,
                "selected_clips": session.selected_clips,
                "selected_entity_ids": session.selected_entity_ids,
                "active_branch": session.active_branch,
                "generation": session.generation,
                "is_stopped": session.is_stopped,
            })
        } else {
            json!({
                "actor": ctx.actor,
                "session_id": ctx.session_id,
                "playhead_tick": 0,
                "selected_clips": [],
                "selected_entity_ids": [],
                "active_branch": ctx.branch,
                "generation": ctx.generation,
                "is_stopped": false,
            })
        }
    }

    fn dispatch_agent_viewport(
        &mut self,
        id: &str,
        params: Value,
        ctx: &ActorContext,
    ) -> Option<Result<Value, DispatchError>> {
        let frame = crate::viewport_state::frame_ticks(&self.document.project.settings.frame_rate);
        let duration = crate::viewport_state::sequence_duration(&self.document.project.sequence);

        if id == "viewport.inspect" {
            let session = self.get_or_create_session(ctx);
            return Some(Ok(json!({
                "viewport": {
                    "actor": session.actor,
                    "session_id": session.session_id,
                    "playhead_tick": session.playhead.0,
                    "selected_clips": session.selected_clips,
                    "selected_entity_ids": session.selected_entity_ids,
                    "active_branch": session.active_branch,
                    "generation": session.generation,
                    "is_stopped": session.is_stopped,
                }
            })));
        }

        if id.starts_with("playhead.") {
            let session = self.get_or_create_session(ctx);
            match id {
                "playhead.start" => {
                    session.playhead = Tick(0);
                }
                "playhead.end" => {
                    session.playhead = duration;
                }
                "playhead.nudge_forward_1" | "playhead.nudge_forward_5" => {
                    let step = if id.ends_with("5") { 5 } else { 1 };
                    let target = session.playhead.0.saturating_add(frame * step);
                    session.playhead = Tick(target.min(duration.0));
                }
                "playhead.nudge_back_1" | "playhead.nudge_back_5" => {
                    let step = if id.ends_with("5") { 5 } else { 1 };
                    let target = session.playhead.0.saturating_sub(frame * step);
                    session.playhead = Tick(target.max(0));
                }
                _ => {
                    if let Some(tick) = params.get("tick").and_then(Value::as_i64) {
                        session.playhead = Tick(tick.clamp(0, duration.0));
                    }
                }
            }
            return Some(Ok(json!({
                "playhead_tick": session.playhead.0,
                "session_id": session.session_id,
            })));
        }

        if id == "timeline.select" || id == "timeline.select_clip" {
            if let (Some(track_id), Some(clip_index)) = (
                params.get("track_id").and_then(Value::as_u64),
                params.get("clip_index").and_then(Value::as_u64),
            ) {
                let entity_id = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .find(|t| t.id == track_id)
                    .and_then(|t| t.clips.get(clip_index as usize))
                    .map(|c| c.id);
                let session = self.get_or_create_session(ctx);
                let clip_ref = crate::viewport_state::SelectedClipRef {
                    track_id,
                    clip_index: clip_index as usize,
                };
                session.selected_clips = vec![clip_ref];
                session.selected_entity_ids = entity_id.map(|id| vec![id]).unwrap_or_default();
                return Some(Ok(json!({
                    "selected_clips": session.selected_clips,
                    "selected_entity_ids": session.selected_entity_ids,
                })));
            }
        }

        if id == "timeline.select_none" || id == "edit.select_none" {
            let session = self.get_or_create_session(ctx);
            session.selected_clips.clear();
            session.selected_entity_ids.clear();
            return Some(Ok(json!({
                "selected_clips": [],
                "selected_entity_ids": [],
            })));
        }

        None
    }

    fn extract_affected_entities(&self, _id: &str, params: &Value) -> Vec<u64> {
        let mut ids = Vec::new();
        if let Some(cid) = params.get("clip_id").and_then(Value::as_u64) {
            ids.push(cid);
        }
        if let (Some(track_id), Some(clip_index)) = (
            params.get("track_id").and_then(Value::as_u64),
            params.get("clip_index").and_then(Value::as_u64),
        ) {
            if let Some(track) = self.document.project.sequence.tracks.iter().find(|t| t.id == track_id) {
                if let Some(clip) = track.clips.get(clip_index as usize) {
                    if !ids.contains(&clip.id) {
                        ids.push(clip.id);
                    }
                }
            }
        }
        if let Some(moves) = params.get("moves").and_then(Value::as_array) {
            for m in moves {
                if let (Some(track_id), Some(clip_index)) = (
                    m.get("track_id").and_then(Value::as_u64),
                    m.get("clip_index").and_then(Value::as_u64),
                ) {
                    if let Some(track) = self.document.project.sequence.tracks.iter().find(|t| t.id == track_id) {
                        if let Some(clip) = track.clips.get(clip_index as usize) {
                            if !ids.contains(&clip.id) {
                                ids.push(clip.id);
                            }
                        }
                    }
                }
            }
        }
        ids
    }

    pub fn undo_change(&mut self, change_id: u64, ctx: &ActorContext) -> Result<Value, DispatchError> {
        self.check_actor_context(ctx)?;
        let record = self
            .journal
            .check_undo_allowed(change_id)
            .map_err(DispatchError::Operation)?
            .clone();

        for entity_id in &record.affected_entity_ids {
            self.document
                .locks
                .check_clip_mutation(*entity_id, &ctx.actor)
                .map_err(DispatchError::Operation)?;
        }

        let inverse_val = record.inverse_state.ok_or_else(|| {
            DispatchError::Operation(format!("Change {change_id} has no inverse state"))
        })?;

        let restored_doc: ProjectDocument = serde_json::from_value(inverse_val)
            .map_err(|e| DispatchError::Operation(format!("Failed to deserialize inverse state: {e}")))?;

        self.checkpoint();
        self.document = restored_doc;

        let undo_change_id = self.journal.record_change(
            0,
            ctx.actor.clone(),
            &ctx.session_id,
            &ctx.branch,
            format!("undo:{}", record.command_id),
            record.affected_entity_ids.clone(),
            None,
        );

        Ok(json!({
            "undone_change_id": change_id,
            "undo_change_id": undo_change_id,
            "command_id": record.command_id,
            "affected_entities": record.affected_entity_ids,
        }))
    }

    pub fn execute_transaction(
        &mut self,
        steps: Vec<CommandRequest>,
        ctx: &ActorContext,
    ) -> Result<Value, DispatchError> {
        self.check_actor_context(ctx)?;
        if steps.is_empty() {
            return Ok(json!({ "executed": 0, "results": [] }));
        }

        if steps.len() > 100 {
            return Err(DispatchError::InvalidParameters(
                "Transaction step limit exceeded (max 100)".into(),
            ));
        }

        for step in &steps {
            if step.id.starts_with("export.")
                || step.id == "project.save"
                || step.id == "project.open"
                || step.id == "project.new"
                || step.id.starts_with("project.branch_")
                || step.id.starts_with("dsp.")
            {
                return Err(DispatchError::Operation(format!(
                    "Command '{}' is not supported in atomic transactions: external/irreversible effects",
                    step.id
                )));
            }
        }

        let doc_backup = self.document.clone();
        let targeting_backup = self.targeting.clone();
        let trim_backup = self.trim.clone();
        let edit_backup = self.edit.clone();
        let transaction_id = self.journal.new_transaction_id();

        let mut results = Vec::new();
        self.begin_gesture();

        let prev_context = self.current_context.replace(ctx.clone());

        for (index, step) in steps.into_iter().enumerate() {
            if let Err(e) = self.check_actor_context(ctx) {
                self.document = doc_backup;
                self.targeting = targeting_backup;
                self.trim = trim_backup;
                self.edit = edit_backup;
                self.end_gesture();
                self.current_context = prev_context;
                return Err(e);
            }

            match self.execute_internal(step.clone()) {
                Ok(val) => {
                    results.push(val);
                }
                Err(err) => {
                    self.document = doc_backup;
                    self.targeting = targeting_backup;
                    self.trim = trim_backup;
                    self.edit = edit_backup;
                    self.end_gesture();
                    self.current_context = prev_context;
                    return Err(DispatchError::Operation(format!(
                        "Transaction aborted at step {} ('{}'): {}. All changes rolled back.",
                        index, step.id, err
                    )));
                }
            }
        }

        self.current_context = prev_context;
        self.end_gesture();

        if let Err(val_err) = self.document.validate() {
            self.document = doc_backup;
            self.targeting = targeting_backup;
            self.trim = trim_backup;
            self.edit = edit_backup;
            return Err(DispatchError::Operation(format!(
                "Transaction validation failed: {val_err}. All changes rolled back."
            )));
        }

        Ok(json!({
            "transaction_id": transaction_id,
            "executed": results.len(),
            "results": results
        }))
    }

    pub fn execute(&mut self, request: CommandRequest) -> Result<Value, DispatchError> {
        self.execute_with_context(request, &ActorContext::human())
    }

    pub fn execute_with_context(
        &mut self,
        request: CommandRequest,
        ctx: &ActorContext,
    ) -> Result<Value, DispatchError> {
        self.check_actor_context(ctx)?;
        self.get_or_create_session(ctx);

        if let Some(ref r) = request.rationale {
            if r.chars().count() > crate::brief::MAX_RATIONALE_LEN {
                return Err(DispatchError::InvalidParameters(format!(
                    "Rationale exceeds {} characters",
                    crate::brief::MAX_RATIONALE_LEN
                )));
            }
        }

        if !ctx.actor.is_human() {
            if let Some(res) = self.dispatch_agent_viewport(&request.id, request.params.clone(), ctx) {
                return res;
            }
        }

        self.check_agent_tasksheet_gates(&request, ctx)?;

        let prev_context = self.current_context.replace(ctx.clone());
        let result = self.execute_internal(request);
        self.current_context = prev_context;
        result
    }

    fn execute_internal(&mut self, request: CommandRequest) -> Result<Value, DispatchError> {
        let command = find_command(&request.id)
            .ok_or_else(|| DispatchError::UnknownCommand(request.id.clone()))?;
        if command.ui_only {
            return Err(DispatchError::UiOnly(request.id));
        }
        self.pending_label = Some(command.id.to_owned());
        let availability = self.command_status(command.id)?;
        if !availability.enabled {
            return Err(DispatchError::Disabled {
                id: request.id,
                reason: availability.reason.unwrap_or_default(),
            });
        }

        let before_doc = if command.mutating {
            Some(self.document.clone())
        } else {
            None
        };
        let affected = self.extract_affected_entities(command.id, &request.params);

        let result = (|| {
            if let Some(result) = self.dispatch_project(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_brief(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_tasksheet(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_tasksheet_gates(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_review(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_branch(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_lock(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_project_freeform(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_sequence(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_media(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_history(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_edit(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_settings(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_export(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_delivery(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_clip(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_clip_advanced(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_trim(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_markers(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_timeline(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_color(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_ocio(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_color_nodes(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_asymmetric_trim(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_match_cut(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_beat_grid(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_audio_edit(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_audio(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_dsp(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_keyframe(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_effects(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_nodegraph(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_captions(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_graphics(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_compound(command.id, request.params.clone()) {
                return result;
            }
            if let Some(result) = self.dispatch_viewport(command.id, request.params.clone()) {
                return result;
            }
            Err(DispatchError::UnknownCommand(request.id))
        })()?;

        if command.mutating {
            self.record_mutating_change(
                command.id,
                affected,
                before_doc,
                request.rationale,
                request.confidence,
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests;
