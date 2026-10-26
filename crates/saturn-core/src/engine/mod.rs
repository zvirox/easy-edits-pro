use crate::command::{find_command, status, CommandRequest, CommandStatus};
use crate::project::{
    AutomationInterpolation, AutomationLane, AutomationMode, BusTarget,
    ChromaKeyParams, Clip, ColorAdjustments, ColorCorrectionParams, ColorWheels, CompositingNode,
    CompositingNodeKind, CropParams, CustomShaderParams, EffectInstance, EffectKind, EffectPayload,
    GaussianBlurParams, InterpolationKind, Keyframe, KeyframeProperty, LutParams,
    Marker, MediaAsset, MediaKind, MotionBlurParams, MotionTrackingData, MultiCamAngle,
    MultiCamClip, MultiCamSyncMode, NodeConnection, NodeGraph, ProjectDocument, RotoSplineMask,
    Sequence, SpeedRamp, SubtitleItem, TitleProperties, Track,
    TrackAudioProcessing, TrackAutomation, TrackGroup, TrackKind, TrackingMode, Transition,
    TransitionKind, VignetteParams,
};
use crate::time::{FrameRate, Tick, TICKS_PER_SECOND};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

mod audio_cmds;
mod clip_advanced_cmds;
mod clip_cmds;
mod color_cmds;
mod compound_cmds;
mod edit_cmds;
mod effects_cmds;
mod helpers;
mod history;
mod keyframe_cmds;
mod markers_cmds;
mod media_cmds;
mod nodegraph_cmds;
mod export_cmds;
mod params;
mod project_cmds;
mod sequence_cmds;
mod settings_cmds;
mod captions_cmds;
mod graphics_cmds;
mod timeline_cmds;
mod trim_cmds;
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

    pub fn execute(&mut self, request: CommandRequest) -> Result<Value, DispatchError> {
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

        if let Some(result) = self.dispatch_project(command.id, request.params.clone()) {
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
        if let Some(result) = self.dispatch_audio(command.id, request.params.clone()) {
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
        Err(DispatchError::UnknownCommand(request.id))
    }
}

#[cfg(test)]
mod tests;
