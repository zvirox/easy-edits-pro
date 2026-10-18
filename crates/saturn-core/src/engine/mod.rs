use crate::command::{status, CommandRequest, CommandStatus, COMMANDS};
use crate::project::{
    AudioPluginInstance, BusTarget, ChromaKeyParams, Clip, ColorAdjustments, ColorCorrectionParams,
    ColorWheels, CompositingNode, CompositingNodeKind, CropParams, CustomShaderParams,
    EffectInstance, EffectKind, EffectPayload, GaussianBlurParams, InterpolationKind, Keyframe,
    KeyframeProperty, LockTargetKind, LutParams, MediaAsset, MediaKind, MotionBlurParams,
    MotionTrackingData, MultiCamAngle, MultiCamClip, MultiCamSyncMode, NodeConnection, NodeGraph,
    ProjectDocument, RotoSplineMask, Sequence, SpeedRamp, SubtitleItem, TimelineChangeEntry,
    TitleProperties, Track, TrackAudioProcessing, TrackAutomation, TrackGroup, TrackKind,
    TrackingMode, Transition, TransitionKind, UserLock, VignetteParams,
};
use crate::time::{FrameRate, Tick, TICKS_PER_SECOND};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

mod audio_cmds;
mod clip_advanced_cmds;
mod clip_cmds;
mod collab_cmds;
mod color_cmds;
mod compound_cmds;
mod effects_cmds;
mod helpers;
mod history;
mod keyframe_cmds;
mod media_cmds;
mod nodegraph_cmds;
mod params;
mod project_cmds;
mod subtitles_cmds;
mod timeline_cmds;
use helpers::*;
use params::*;

#[derive(Debug)]
pub enum DispatchError {
    UnknownCommand(String),
    Disabled { id: String, reason: String },
    InvalidParameters(String),
    Operation(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(id) => write!(f, "Unknown command `{id}`"),
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
    saved_snapshot: String,
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
            saved_snapshot,
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
        self.saved_snapshot.clear();
        Ok(())
    }

    pub fn command_status(&self, id: &str) -> Result<CommandStatus, DispatchError> {
        if !COMMANDS.iter().any(|command| command.id == id) {
            return Err(DispatchError::UnknownCommand(id.to_owned()));
        }
        let result = match id {
            "edit.undo" => status(!self.undo.is_empty(), "Nothing to undo"),
            "edit.redo" => status(!self.redo.is_empty(), "Nothing to redo"),
            _ => status(true, ""),
        };
        Ok(result)
    }

    pub fn execute(&mut self, request: CommandRequest) -> Result<Value, DispatchError> {
        let command = COMMANDS
            .iter()
            .find(|command| command.id == request.id.as_str())
            .ok_or_else(|| DispatchError::UnknownCommand(request.id.clone()))?;
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
        if let Some(result) = self.dispatch_media(command.id, request.params.clone()) {
            return result;
        }
        if let Some(result) = self.dispatch_history(command.id, request.params.clone()) {
            return result;
        }
        if let Some(result) = self.dispatch_clip(command.id, request.params.clone()) {
            return result;
        }
        if let Some(result) = self.dispatch_clip_advanced(command.id, request.params.clone()) {
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
        if let Some(result) = self.dispatch_subtitles(command.id, request.params.clone()) {
            return result;
        }
        if let Some(result) = self.dispatch_collab(command.id, request.params.clone()) {
            return result;
        }
        if let Some(result) = self.dispatch_compound(command.id, request.params.clone()) {
            return result;
        }
        Err(DispatchError::UnknownCommand(request.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MediaKind;

    #[test]
    fn commands_report_reasons_and_undo_redo_project_mutations() {
        let mut engine = EditorEngine::default();
        assert_eq!(
            engine
                .command_status("edit.undo")
                .unwrap()
                .reason
                .as_deref(),
            Some("Nothing to undo")
        );
        engine
            .execute(CommandRequest::new(
                "project.add_media",
                json!({
                    "name":"clip.mp4", "path":"/clips/clip.mp4", "kind": MediaKind::Video
                }),
            ))
            .unwrap();
        assert_eq!(engine.document().project.media.len(), 1);
        engine
            .execute(CommandRequest::new("edit.undo", Value::Null))
            .unwrap();
        assert!(engine.document().project.media.is_empty());
        engine
            .execute(CommandRequest::new("edit.redo", Value::Null))
            .unwrap();
        assert_eq!(engine.document().project.media[0].name, "clip.mp4");
    }
}
