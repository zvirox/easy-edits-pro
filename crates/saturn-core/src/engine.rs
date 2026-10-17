use crate::command::{COMMANDS, CommandRequest, CommandStatus, status};
use crate::project::{
    AudioPluginInstance, BusTarget, Clip, ColorAdjustments, ColorCorrectionParams,
    ColorWheels, CompositingNode, CompositingNodeKind, CropParams, EffectInstance,
    EffectKind, EffectPayload, GaussianBlurParams, InterpolationKind, Keyframe,
    KeyframeProperty, LockTargetKind, LutParams, MediaAsset, MediaKind,
    MotionBlurParams, MotionTrackingData, MultiCamAngle, MultiCamClip, MultiCamSyncMode,
    NodeConnection, NodeGraph, ProjectDocument, RotoSplineMask, SpeedRamp,
    SubtitleItem, TimelineChangeEntry, TitleProperties, TrackAudioProcessing,
    TrackAutomation, TrackKind, TrackingMode, Transition, TransitionKind,
    UserLock, VignetteParams, ChromaKeyParams,
};
use crate::time::{FrameRate, TICKS_PER_SECOND, Tick};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

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

#[derive(Deserialize)]
struct NewProjectParams {
    #[serde(default = "default_project_name")]
    name: String,
    #[serde(default = "default_sequence_name")]
    sequence_name: String,
    #[serde(default = "default_project_width")]
    width: u32,
    #[serde(default = "default_project_height")]
    height: u32,
    #[serde(default = "default_project_frame_rate")]
    frame_rate: FrameRate,
}
fn default_project_name() -> String {
    "Untitled project".into()
}
fn default_sequence_name() -> String {
    "Sequence 1".into()
}
fn default_project_width() -> u32 {
    1920
}
fn default_project_height() -> u32 {
    1080
}
fn default_project_frame_rate() -> FrameRate {
    FrameRate::FPS_30
}

#[derive(Deserialize)]
struct AddMediaParams {
    name: String,
    path: PathBuf,
    #[serde(default)]
    kind: crate::project::MediaKind,
    #[serde(default)]
    duration: Option<crate::time::Tick>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    title: Option<TitleProperties>,
    #[serde(default)]
    waveform: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct UpdateMediaParams {
    path: PathBuf,
    #[serde(default)]
    kind: crate::project::MediaKind,
    #[serde(default)]
    duration: Option<crate::time::Tick>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    waveform: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct AddTransitionParams {
    track_id: u64,
    cut_at: Tick,
    #[serde(default)]
    duration: Option<Tick>,
    #[serde(default)]
    kind: Option<TransitionKind>,
}

#[derive(Deserialize)]
struct RemoveTransitionParams {
    track_id: u64,
    transition_id: u64,
}

#[derive(Deserialize)]
struct AddTitleParams {
    text: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    duration: Option<Tick>,
    #[serde(default)]
    track_id: Option<u64>,
    #[serde(default)]
    timeline_start: Option<Tick>,
}

#[derive(Deserialize)]
struct UpdateTitleParams {
    media_id: u64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    font_size: Option<f64>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    bg_color: Option<String>,
}


#[derive(Deserialize)]
struct RelinkMediaParams {
    media_id: u64,
    path: PathBuf,
}

#[derive(Deserialize)]
struct FilePathParams {
    path: PathBuf,
}

#[derive(Deserialize)]
struct AddClipParams {
    media_id: u64,
    #[serde(default)]
    track_id: Option<u64>,
    #[serde(default)]
    timeline_start: Option<Tick>,
    #[serde(default)]
    source_in: Tick,
    #[serde(default)]
    duration: Option<Tick>,
}

#[derive(Deserialize)]
struct RemoveClipParams {
    track_id: u64,
    clip_index: usize,
}

#[derive(Deserialize)]
struct ClipReferenceParams {
    track_id: u64,
    clip_index: usize,
}

#[derive(Deserialize)]
struct RemoveClipsParams {
    clips: Vec<ClipReferenceParams>,
    #[serde(default)]
    ripple: bool,
}

#[derive(Deserialize)]
struct MoveClipParams {
    track_id: u64,
    clip_index: usize,
    #[serde(default)]
    target_track_id: Option<u64>,
    timeline_start: Tick,
}

#[derive(Deserialize)]
struct SplitClipParams {
    track_id: u64,
    clip_index: usize,
    at: Tick,
}

#[derive(Deserialize)]
struct SlipClipParams {
    track_id: u64,
    clip_index: usize,
    delta: Tick,
}

#[derive(Deserialize)]
struct RollEditParams {
    track_id: u64,
    left_clip_index: usize,
    delta: Tick,
}

#[derive(Deserialize)]
struct SlideClipParams {
    track_id: u64,
    clip_index: usize,
    delta: Tick,
}

#[derive(Deserialize)]
struct RateStretchParams {
    track_id: u64,
    clip_index: usize,
    #[serde(default)]
    new_duration: Option<Tick>,
    #[serde(default)]
    speed: Option<f64>,
}

#[derive(Deserialize)]
struct LinkClipsParams {
    clips: Vec<ClipReferenceParams>,
}

#[derive(Deserialize)]
struct UnlinkClipsParams {
    clips: Vec<ClipReferenceParams>,
}

#[derive(Deserialize)]
struct MoveClipBatchItem {
    track_id: u64,
    clip_index: usize,
    #[serde(default)]
    #[allow(dead_code)]
    target_track_id: Option<u64>,
    timeline_start: Tick,
}

#[derive(Deserialize)]
struct MoveClipsBatchParams {
    moves: Vec<MoveClipBatchItem>,
}

#[derive(Deserialize)]
struct SetMarkParams {
    at: Tick,
}

#[derive(Deserialize)]
struct SetColorParams {
    track_id: u64,
    clip_index: usize,
    exposure: f64,
    contrast: f64,
    saturation: f64,
    #[serde(default)]
    hue: f64,
}

#[derive(Deserialize)]
struct SetTrackAudioProcessingParams {
    track_id: u64,
    #[serde(default)]
    eq_enabled: bool,
    #[serde(default)]
    eq_low_gain: f64,
    #[serde(default)]
    eq_mid_gain: f64,
    #[serde(default)]
    eq_high_gain: f64,
    #[serde(default)]
    comp_enabled: bool,
    #[serde(default = "default_threshold")]
    comp_threshold: f64,
    #[serde(default = "default_ratio")]
    comp_ratio: f64,
    #[serde(default = "default_attack")]
    comp_attack: f64,
    #[serde(default = "default_release")]
    comp_release: f64,
    #[serde(default)]
    denoise_enabled: bool,
    #[serde(default = "default_denoise_amount")]
    denoise_amount: f64,
}

fn default_threshold() -> f64 { -24.0 }
fn default_ratio() -> f64 { 4.0 }
fn default_attack() -> f64 { 0.005 }
fn default_release() -> f64 { 0.1 }
fn default_denoise_amount() -> f64 { 0.5 }

#[derive(Deserialize)]
struct SetTrackGainParams {
    track_id: u64,
    gain_db: f64,
}

#[derive(Deserialize)]
struct SetTrackPanParams {
    track_id: u64,
    pan: f64,
}

#[derive(Deserialize)]
struct SetTrackMutedParams {
    track_id: u64,
    muted: bool,
}

#[derive(Deserialize)]
struct SetTrackSoloParams {
    track_id: u64,
    solo: bool,
}

#[derive(Deserialize)]
struct SetTrackLockedParams {
    track_id: u64,
    locked: bool,
}

#[derive(Deserialize)]
struct SetKeyframeParams {
    track_id: u64,
    clip_index: usize,
    property: KeyframeProperty,
    time: Tick,
    value: f64,
    #[serde(default)]
    interpolation: Option<InterpolationKind>,
    #[serde(default)]
    handle_in_x: Option<f64>,
    #[serde(default)]
    handle_in_y: Option<f64>,
    #[serde(default)]
    handle_out_x: Option<f64>,
    #[serde(default)]
    handle_out_y: Option<f64>,
}

#[derive(Deserialize)]
struct AddSubtitleParams {
    start_ticks: Tick,
    end_ticks: Tick,
    text: String,
}

#[derive(Deserialize)]
struct RemoveSubtitleParams {
    id: u64,
}

#[derive(Deserialize)]
struct UpdateSubtitleParams {
    id: u64,
    #[serde(default)]
    start_ticks: Option<Tick>,
    #[serde(default)]
    end_ticks: Option<Tick>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct RemoveKeyframeParams {
    track_id: u64,
    clip_index: usize,
    property: KeyframeProperty,
    time: Tick,
}

#[derive(Deserialize)]
struct AddEffectParams {
    track_id: u64,
    clip_index: usize,
    kind: EffectKind,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct RemoveEffectParams {
    track_id: u64,
    clip_index: usize,
    effect_id: u64,
}

#[derive(Deserialize)]
struct UpdateEffectParams {
    track_id: u64,
    clip_index: usize,
    effect_id: u64,
    payload: EffectPayload,
}

#[derive(Deserialize)]
struct ToggleEffectParams {
    track_id: u64,
    clip_index: usize,
    effect_id: u64,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct ReorderEffectsParams {
    track_id: u64,
    clip_index: usize,
    from_index: usize,
    to_index: usize,
}

#[derive(Deserialize)]
struct SetColorWheelsParams {
    track_id: u64,
    clip_index: usize,
    wheels: ColorWheels,
}

#[derive(Deserialize)]
struct CreateMulticamParams {
    name: String,
    angles: Vec<MultiCamAngle>,
    #[serde(default)]
    sync_mode: MultiCamSyncMode,
}

#[derive(Deserialize)]
struct SwitchMulticamAngleParams {
    track_id: u64,
    clip_index: usize,
    angle_id: u64,
}

#[derive(Deserialize)]
struct SyncMulticamParams {
    multicam_id: u64,
    sync_mode: MultiCamSyncMode,
    #[serde(default)]
    offsets: Option<std::collections::HashMap<u64, i64>>,
}

#[derive(Deserialize)]
struct SetMotionTrackingParams {
    track_id: u64,
    clip_index: usize,
    tracking: MotionTrackingData,
}

#[derive(Deserialize)]
struct PinToTrackParams {
    track_id: u64,
    clip_index: usize,
    pinned_clip_id: Option<u64>,
}

#[derive(Deserialize)]
struct SetTrackBusRouteParams {
    track_id: u64,
    bus: BusTarget,
}

#[derive(Deserialize)]
struct SetTrackAutomationParams {
    track_id: u64,
    automation: TrackAutomation,
}

#[derive(Deserialize)]
struct AddAudioPluginParams {
    track_id: u64,
    plugin: AudioPluginInstance,
}

#[derive(Deserialize)]
struct ToggleAudioPluginParams {
    track_id: u64,
    plugin_id: u64,
    enabled: bool,
}

#[derive(Deserialize)]
struct SetBusGainParams {
    target: BusTarget,
    gain_db: f64,
}

#[derive(Deserialize)]
struct AddNodeParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    kind: CompositingNodeKind,
    #[serde(default = "default_node_title")]
    title: String,
    #[serde(default)]
    pos_x: f64,
    #[serde(default)]
    pos_y: f64,
    #[serde(default)]
    params: Value,
}

fn default_node_title() -> String { "New Node".into() }

#[derive(Deserialize)]
struct RemoveNodeParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    node_id: u64,
}

#[derive(Deserialize)]
struct ConnectNodesParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    from_node: u64,
    from_socket: String,
    to_node: u64,
    to_socket: String,
}

#[derive(Deserialize)]
struct DisconnectNodesParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    from_node: u64,
    from_socket: String,
    to_node: u64,
    to_socket: String,
}

#[derive(Deserialize)]
struct UpdateNodeParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    node_id: u64,
    pos_x: Option<f64>,
    pos_y: Option<f64>,
    enabled: Option<bool>,
    params: Option<Value>,
}

#[derive(Deserialize)]
struct AddMaskParams {
    track_id: Option<u64>,
    clip_index: Option<usize>,
    mask: RotoSplineMask,
}

#[derive(Deserialize)]
struct SetSpeedRampParams {
    track_id: u64,
    clip_index: usize,
    speed_ramp: SpeedRamp,
}

#[derive(Deserialize)]
struct SetMotionBlurParams {
    track_id: u64,
    clip_index: usize,
    motion_blur: MotionBlurParams,
}

#[derive(Deserialize)]
struct AcquireLockParams {
    user_id: String,
    user_name: String,
    user_color: Option<String>,
    target_kind: LockTargetKind,
    target_id: u64,
}

#[derive(Deserialize)]
struct ReleaseLockParams {
    user_id: String,
    target_kind: LockTargetKind,
    target_id: u64,
}

#[derive(Deserialize)]
struct RecordChangeParams {
    user_id: String,
    user_name: String,
    action: String,
    summary: String,
}

#[derive(Deserialize)]
struct TranscribeToSubtitlesParams {
    segments: Vec<crate::speech::SpeechSegment>,
    replace_existing: Option<bool>,
}

#[derive(Deserialize)]
struct RippleDeleteSilenceParams {
    peaks: Vec<u8>,
    options: Option<crate::speech::SilenceDetectOptions>,
}

#[derive(Deserialize)]
struct RippleDeleteFillersParams {
    segments: Vec<crate::speech::SpeechSegment>,
    custom_fillers: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RippleDeleteRangesParams {
    ranges: Vec<(Tick, Tick)>,
}

#[derive(Deserialize)]
struct ImportEdlParams {
    edl: String,
    project_name: Option<String>,
}

#[derive(Deserialize)]
struct ImportXmlParams {
    xml: String,
}

fn get_or_create_node_graph<'a>(
    doc: &'a mut ProjectDocument,
    track_id: Option<u64>,
    clip_index: Option<usize>,
) -> Result<&'a mut NodeGraph, DispatchError> {
    if let (Some(t_id), Some(c_idx)) = (track_id, clip_index) {
        let track_idx = find_track_index(doc, t_id)?;
        find_clip(doc, track_idx, c_idx)?;
        let clip_mut = &mut doc.project.sequence.tracks[track_idx].clips[c_idx];
        if clip_mut.node_graph.is_none() {
            clip_mut.node_graph = Some(NodeGraph::default());
        }
        Ok(clip_mut.node_graph.as_mut().unwrap())
    } else {
        if doc.project.sequence.node_graph.is_none() {
            doc.project.sequence.node_graph = Some(NodeGraph::default());
        }
        Ok(doc.project.sequence.node_graph.as_mut().unwrap())
    }
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

        match command.id {
            "project.new" => {
                let params: NewProjectParams = parse_params(request.params)?;
                if params.name.trim().is_empty() || params.sequence_name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Project and timeline names cannot be empty".into(),
                    ));
                }
                let mut document = ProjectDocument::new(params.name.trim());
                document.project.settings.width = params.width;
                document.project.settings.height = params.height;
                document.project.settings.frame_rate = params.frame_rate;
                document.project.sequence.name = params.sequence_name.trim().to_owned();
                document.view.active_sequence = Some(params.sequence_name.trim().to_owned());
                document
                    .validate()
                    .map_err(DispatchError::InvalidParameters)?;
                self.checkpoint();
                self.document = document;
                self.current_path = None;
                Ok(json!({"name": self.document.project.name}))
            }
            "project.add_media" => {
                let params: AddMediaParams = parse_params(request.params)?;
                if let Some(existing) = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.path == params.path)
                {
                    return Ok(json!({"media_id": existing.id, "already_present": true}));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let item = MediaAsset {
                    id,
                    name: params.name,
                    path: params.path,
                    kind: params.kind,
                    duration: params.duration,
                    width: params.width,
                    height: params.height,
                    title: params.title,
                    waveform: params.waveform,
                };
                self.document.project.media.push(item);
                Ok(json!({"media_id": id}))
            }
            "project.update_media" => {
                let params: UpdateMediaParams = parse_params(request.params)?;
                let Some(item) = self
                    .document
                    .project
                    .media
                    .iter_mut()
                    .find(|item| item.path == params.path)
                else {
                    return Ok(json!({"updated": false}));
                };
                item.kind = params.kind;
                item.duration = params.duration;
                item.width = params.width;
                item.height = params.height;
                if params.waveform.is_some() {
                    item.waveform = params.waveform;
                }
                Ok(json!({"updated": true, "media_id": item.id}))
            }
            "project.relink_media" => {
                let params: RelinkMediaParams = parse_params(request.params)?;
                if !params.path.is_file() {
                    return Err(DispatchError::InvalidParameters("The replacement media file does not exist".into()));
                }
                let Some(index) = self.document.project.media.iter().position(|item| item.id == params.media_id) else {
                    return Err(DispatchError::InvalidParameters(format!("Media item {} does not exist", params.media_id)));
                };
                self.checkpoint();
                let media = &mut self.document.project.media[index];
                media.path = params.path;
                Ok(json!({"relinked": true, "media_id": media.id, "path": media.path}))
            }
            "file.save" => {
                let params: FilePathParams = parse_params(request.params)?;
                self.document
                    .save_atomic(&params.path)
                    .map_err(DispatchError::Operation)?;
                self.current_path = Some(params.path.clone());
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"path": params.path}))
            }
            "file.open" => {
                let params: FilePathParams = parse_params(request.params)?;
                let loaded =
                    ProjectDocument::load(&params.path).map_err(DispatchError::Operation)?;
                self.document = loaded;
                self.current_path = Some(params.path.clone());
                self.undo.clear();
                self.redo.clear();
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"name": self.document.project.name, "path": params.path}))
            }
            "edit.undo" => {
                let previous = self.undo.pop().expect("availability checked");
                self.redo.push(self.document.clone());
                self.document = previous;
                Ok(json!({"undone": true}))
            }
            "edit.redo" => {
                let next = self.redo.pop().expect("availability checked");
                self.undo.push(self.document.clone());
                self.document = next;
                Ok(json!({"redone": true}))
            }
            "project.inspect" => serde_json::to_value(&self.document)
                .map_err(|e| DispatchError::Operation(e.to_string())),
            "project.status" => Ok(json!({
                "document": &self.document,
                "path": self.current_path.as_ref(),
                "is_dirty": self.is_dirty(),
            })),
            "media.list" => serde_json::to_value(&self.document.project.media)
                .map_err(|e| DispatchError::Operation(e.to_string())),
            "timeline.add_clip" => {
                let params: AddClipParams = parse_params(request.params)?;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                let default_kind = match &media.kind {
                    MediaKind::Audio => TrackKind::Audio,
                    _ => TrackKind::Video,
                };
                let media_duration = media.duration;
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| {
                        params
                            .track_id
                            .map(|track_id| track.id == track_id)
                            .unwrap_or_else(|| {
                                std::mem::discriminant(&track.kind)
                                    == std::mem::discriminant(&default_kind)
                            })
                    })
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("No matching timeline track exists".into())
                    })?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let duration = params
                    .duration
                    .or(media_duration)
                    .unwrap_or(Tick(TICKS_PER_SECOND * 5));
                if duration.0 <= 0 || params.source_in.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Clip source and duration must be positive".into(),
                    ));
                }
                if params.timeline_start.is_some_and(|start| start.0 < 0) {
                    return Err(DispatchError::InvalidParameters(
                        "Clip timeline position cannot be negative".into(),
                    ));
                }
                if media_duration
                    .is_some_and(|length| params.source_in.0.saturating_add(duration.0) > length.0)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Clip range exceeds the media duration".into(),
                    ));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let timeline_start = params.timeline_start.map(|start| start.0).unwrap_or_else(|| {
                    track
                        .clips
                        .iter()
                        .filter_map(|clip| clip.timeline_start.0.checked_add(clip.duration.0))
                        .max()
                        .unwrap_or(0)
                });
                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips.push(Clip {
                    media_id: params.media_id,
                    timeline_start: Tick(timeline_start),
                    source_in: params.source_in,
                    duration,
                    speed: 1.0,
                    group_id: None,
                    color: ColorAdjustments::default(),
                    effects: Vec::new(),
                    keyframes: Vec::new(),
                    multicam_id: None,
                    multicam_angle_id: None,
                    tracking: None,
                    node_graph: None,
                    speed_ramp: None,
                    motion_blur: None,
                });
                Ok(json!({"track_id": track.id, "clip_index": track.clips.len() - 1}))
            }
            "timeline.remove_clip" => {
                let params: RemoveClipParams = parse_params(request.params)?;
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                self.checkpoint();
                self.document.project.sequence.tracks[track_index]
                    .clips
                    .remove(params.clip_index);
                Ok(json!({"removed": true}))
            }
            "timeline.ripple_delete" => {
                let params: RemoveClipParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clips = &self.document.project.sequence.tracks[track_index].clips;
                let clip = clips.get(params.clip_index).ok_or_else(|| {
                    DispatchError::InvalidParameters("Timeline clip does not exist".into())
                })?;
                let start = clip.timeline_start.0;
                let end = start.saturating_add(clip.duration.0);
                let duration = clip.duration.0;
                self.checkpoint();
                let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                clips.remove(params.clip_index);
                for later in clips.iter_mut().filter(|item| item.timeline_start.0 >= end) {
                    later.timeline_start.0 = later.timeline_start.0.saturating_sub(duration);
                }
                clips.sort_by_key(|item| item.timeline_start);
                Ok(json!({"removed": true, "rippled_ticks": duration, "start": start}))
            }
            "timeline.ripple_delete_batch" => {
                let params: RemoveClipsParams = parse_params(request.params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::InvalidParameters("Select one or more timeline clips".into()));
                }
                let mut removals = std::collections::BTreeMap::<usize, Vec<usize>>::new();
                for reference in &params.clips {
                    let track_index = find_track_index(&self.document, reference.track_id)?;
                    if self.document.project.sequence.tracks[track_index].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if reference.clip_index >= self.document.project.sequence.tracks[track_index].clips.len() {
                        return Err(DispatchError::InvalidParameters("Timeline clip does not exist".into()));
                    }
                    let indices = removals.entry(track_index).or_default();
                    if !indices.contains(&reference.clip_index) { indices.push(reference.clip_index); }
                }
                self.checkpoint();
                let mut removed = 0;
                for (track_index, indices) in removals {
                    let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                    let mut indices = indices;
                    indices.sort_unstable_by(|left, right| right.cmp(left));
                    for index in indices {
                        let clip = clips.remove(index);
                        let end = clip.timeline_start.0.saturating_add(clip.duration.0);
                        if params.ripple {
                            for later in clips.iter_mut().filter(|item| item.timeline_start.0 >= end) {
                                later.timeline_start.0 = later.timeline_start.0.saturating_sub(clip.duration.0);
                            }
                        }
                        removed += 1;
                    }
                    clips.sort_by_key(|item| item.timeline_start);
                }
                Ok(json!({"removed": removed, "ripple": params.ripple}))
            }
            "timeline.split_clip" => {
                let params: SplitClipParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let clip = track.clips.get(params.clip_index).ok_or_else(|| {
                    DispatchError::InvalidParameters("Timeline clip does not exist".into())
                })?;
                let split_offset = params.at.0.saturating_sub(clip.timeline_start.0);
                if split_offset <= 0 || split_offset >= clip.duration.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Playhead must be inside the selected clip, away from its edges".into(),
                    ));
                }
                let mut left = clip.clone();
                let mut right = clip.clone();
                left.duration = Tick(split_offset);
                right.timeline_start = params.at;
                right.source_in.0 = right.source_in.0.saturating_add(split_offset);
                right.duration.0 = clip.duration.0 - split_offset;
                let mut properties = Vec::new();
                for key in &clip.keyframes {
                    if !properties.contains(&key.property) { properties.push(key.property.clone()); }
                }
                for property in &properties {
                    if let Some(value) = keyframe_value_at(&clip.keyframes, property, split_offset) {
                        if !left.keyframes.iter().any(|key| &key.property == property && key.time.0 == split_offset) {
                            left.keyframes.push(Keyframe::new(property.clone(), Tick(split_offset), value));
                        }
                    }
                }
                left.keyframes.retain(|key| key.time.0 <= split_offset);
                right.keyframes.retain(|key| key.time.0 > split_offset);
                for key in &mut right.keyframes {
                    key.time.0 -= split_offset;
                }
                for property in properties {
                    if let Some(value) = keyframe_value_at(&clip.keyframes, &property, split_offset) {
                        right.keyframes.push(Keyframe::new(property, Tick(0), value));
                    }
                }
                self.checkpoint();
                let clips = &mut self.document.project.sequence.tracks[track_index].clips;
                clips[params.clip_index] = left;
                clips.insert(params.clip_index + 1, right);
                Ok(json!({"split": true, "track_id": params.track_id, "left_index": params.clip_index, "right_index": params.clip_index + 1}))
            }
            "timeline.slip_clip" => {
                let params: SlipClipParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let next_source_in = clip.source_in.0.saturating_add(params.delta.0);
                if next_source_in < 0 {
                    return Err(DispatchError::InvalidParameters("Slip cannot move the source before its beginning".into()));
                }
                let media_id = clip.media_id;
                let duration = clip.duration.0;
                let media_duration = self.document.project.media.iter().find(|media| media.id == media_id).and_then(|media| media.duration);
                if media_duration.is_some_and(|length| next_source_in.saturating_add(duration) > length.0) {
                    return Err(DispatchError::InvalidParameters("Slip cannot move the source beyond the media duration".into()));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.source_in = Tick(next_source_in);
                Ok(json!({"slipped": true, "source_in": clip.source_in}))
            }
            "timeline.roll_edit" => {
                let params: RollEditParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                if params.left_clip_index + 1 >= track.clips.len() {
                    return Err(DispatchError::InvalidParameters("Rolling edit requires two adjacent clips".into()));
                }
                let left = &track.clips[params.left_clip_index];
                let right = &track.clips[params.left_clip_index + 1];
                let delta = params.delta.0;

                let new_left_dur = left.duration.0.saturating_add(delta);
                let new_right_start = right.timeline_start.0.saturating_add(delta);
                let new_right_source_in = right.source_in.0.saturating_add(delta);
                let new_right_dur = right.duration.0.saturating_sub(delta);

                if new_left_dur <= 0 || new_right_dur <= 0 {
                    return Err(DispatchError::InvalidParameters("Rolling edit exceeds clip duration limit".into()));
                }
                if new_right_source_in < 0 {
                    return Err(DispatchError::InvalidParameters("Right clip cannot roll before media beginning".into()));
                }

                let left_media = self.document.project.media.iter().find(|m| m.id == left.media_id);
                if let Some(left_med) = left_media {
                    if let Some(med_dur) = left_med.duration {
                        if left.source_in.0.saturating_add(new_left_dur) > med_dur.0 {
                            return Err(DispatchError::InvalidParameters("Left clip cannot roll beyond media duration".into()));
                        }
                    }
                }
                let right_media = self.document.project.media.iter().find(|m| m.id == right.media_id);
                if let Some(right_med) = right_media {
                    if let Some(med_dur) = right_med.duration {
                        if new_right_source_in.saturating_add(new_right_dur) > med_dur.0 {
                            return Err(DispatchError::InvalidParameters("Right clip cannot roll beyond media duration".into()));
                        }
                    }
                }

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips[params.left_clip_index].duration = Tick(new_left_dur);
                track.clips[params.left_clip_index + 1].timeline_start = Tick(new_right_start);
                track.clips[params.left_clip_index + 1].source_in = Tick(new_right_source_in);
                track.clips[params.left_clip_index + 1].duration = Tick(new_right_dur);

                for key in &mut track.clips[params.left_clip_index + 1].keyframes {
                    key.time.0 = key.time.0.saturating_sub(delta);
                }
                track.clips[params.left_clip_index + 1].keyframes.retain(|k| k.time.0 >= 0 && k.time.0 <= new_right_dur);
                track.clips[params.left_clip_index].keyframes.retain(|k| k.time.0 <= new_left_dur);

                Ok(json!({
                    "rolled": true,
                    "track_id": params.track_id,
                    "left_index": params.left_clip_index,
                    "delta": delta,
                    "edit_point": new_right_start,
                }))
            }
            "timeline.slide_clip" => {
                let params: SlideClipParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                if params.clip_index == 0 || params.clip_index + 1 >= track.clips.len() {
                    return Err(DispatchError::InvalidParameters(
                        "Slide edit requires a clip situated between preceding and succeeding clips".into()
                    ));
                }
                let delta = params.delta.0;
                let clip_a = &track.clips[params.clip_index - 1];
                let clip_b = &track.clips[params.clip_index];
                let clip_c = &track.clips[params.clip_index + 1];

                let new_a_dur = clip_a.duration.0.saturating_add(delta);
                let new_b_start = clip_b.timeline_start.0.saturating_add(delta);
                let new_c_start = clip_c.timeline_start.0.saturating_add(delta);
                let new_c_source_in = clip_c.source_in.0.saturating_add(delta);
                let new_c_dur = clip_c.duration.0.saturating_sub(delta);

                if new_a_dur <= 0 || new_c_dur <= 0 || new_b_start < 0 {
                    return Err(DispatchError::InvalidParameters("Slide edit exceeds clip duration limit".into()));
                }
                if new_c_source_in < 0 {
                    return Err(DispatchError::InvalidParameters("Succeeding clip source cannot start before media beginning".into()));
                }

                let media_a = self.document.project.media.iter().find(|m| m.id == clip_a.media_id);
                if let Some(med_a) = media_a {
                    if let Some(dur) = med_a.duration {
                        if clip_a.source_in.0.saturating_add(new_a_dur) > dur.0 {
                            return Err(DispatchError::InvalidParameters("Preceding clip exceeds media duration".into()));
                        }
                    }
                }
                let media_c = self.document.project.media.iter().find(|m| m.id == clip_c.media_id);
                if let Some(med_c) = media_c {
                    if let Some(dur) = med_c.duration {
                        if new_c_source_in.saturating_add(new_c_dur) > dur.0 {
                            return Err(DispatchError::InvalidParameters("Succeeding clip exceeds media duration".into()));
                        }
                    }
                }

                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips[params.clip_index - 1].duration = Tick(new_a_dur);
                track.clips[params.clip_index].timeline_start = Tick(new_b_start);
                track.clips[params.clip_index + 1].timeline_start = Tick(new_c_start);
                track.clips[params.clip_index + 1].source_in = Tick(new_c_source_in);
                track.clips[params.clip_index + 1].duration = Tick(new_c_dur);

                for key in &mut track.clips[params.clip_index + 1].keyframes {
                    key.time.0 = key.time.0.saturating_sub(delta);
                }
                track.clips[params.clip_index + 1].keyframes.retain(|k| k.time.0 >= 0 && k.time.0 <= new_c_dur);
                track.clips[params.clip_index - 1].keyframes.retain(|k| k.time.0 <= new_a_dur);

                Ok(json!({
                    "slid": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "delta": delta,
                }))
            }
            "timeline.rate_stretch" => {
                let params: RateStretchParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let old_dur = clip.duration.0 as f64;
                let old_speed = if clip.speed > 0.0 { clip.speed } else { 1.0 };

                let (new_duration, new_speed) = if let Some(speed) = params.speed {
                    if speed < 0.05 || speed > 20.0 {
                        return Err(DispatchError::InvalidParameters("Speed must be between 0.05x and 20.0x".into()));
                    }
                    let new_dur = ((old_dur * old_speed) / speed).round() as i64;
                    (Tick(new_dur.max(1)), (speed * 1000.0).round() / 1000.0)
                } else if let Some(new_dur) = params.new_duration {
                    if new_dur.0 <= 0 {
                        return Err(DispatchError::InvalidParameters("Duration must be positive".into()));
                    }
                    let calc_speed = (old_dur * old_speed) / (new_dur.0 as f64);
                    let clamped = calc_speed.clamp(0.05, 20.0);
                    (new_dur, (clamped * 1000.0).round() / 1000.0)
                } else {
                    return Err(DispatchError::InvalidParameters("Either duration or speed must be specified".into()));
                };

                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                if old_dur > 0.0 {
                    let ratio = new_duration.0 as f64 / old_dur;
                    for key in &mut clip.keyframes {
                        key.time.0 = ((key.time.0 as f64) * ratio).round() as i64;
                    }
                }
                clip.duration = new_duration;
                clip.speed = new_speed;

                Ok(json!({
                    "stretched": true,
                    "track_id": params.track_id,
                    "clip_index": params.clip_index,
                    "duration": new_duration,
                    "speed": new_speed,
                }))
            }
            "timeline.link_clips" => {
                let params: LinkClipsParams = parse_params(request.params)?;
                if params.clips.len() < 2 {
                    return Err(DispatchError::InvalidParameters("Select at least two clips to link".into()));
                }
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if r.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters("Timeline clip does not exist".into()));
                    }
                }
                let max_id = self.document.project.sequence.tracks.iter()
                    .flat_map(|t| t.clips.iter())
                    .filter_map(|c| c.group_id)
                    .max()
                    .unwrap_or(0);
                let new_group_id = max_id + 1;

                self.checkpoint();
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    self.document.project.sequence.tracks[t_idx].clips[r.clip_index].group_id = Some(new_group_id);
                }

                Ok(json!({
                    "linked": true,
                    "group_id": new_group_id,
                    "count": params.clips.len(),
                }))
            }
            "timeline.unlink_clips" => {
                let params: UnlinkClipsParams = parse_params(request.params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::InvalidParameters("Select clips to unlink".into()));
                }
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if r.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters("Timeline clip does not exist".into()));
                    }
                }

                let mut groups_to_clear = std::collections::HashSet::new();
                for r in &params.clips {
                    let t_idx = find_track_index(&self.document, r.track_id)?;
                    if let Some(gid) = self.document.project.sequence.tracks[t_idx].clips[r.clip_index].group_id {
                        groups_to_clear.insert(gid);
                    }
                }

                self.checkpoint();
                let mut count = 0;
                for track in &mut self.document.project.sequence.tracks {
                    for clip in &mut track.clips {
                        if let Some(gid) = clip.group_id {
                            if groups_to_clear.contains(&gid) {
                                clip.group_id = None;
                                count += 1;
                            }
                        }
                    }
                }

                Ok(json!({
                    "unlinked": true,
                    "count": count,
                }))
            }
            "timeline.move_clips_batch" => {
                let params: MoveClipsBatchParams = parse_params(request.params)?;
                if params.moves.is_empty() {
                    return Err(DispatchError::InvalidParameters("No moves specified".into()));
                }
                for item in &params.moves {
                    if item.timeline_start.0 < 0 {
                        return Err(DispatchError::InvalidParameters("A clip cannot start before time zero".into()));
                    }
                    let t_idx = find_track_index(&self.document, item.track_id)?;
                    if self.document.project.sequence.tracks[t_idx].locked {
                        return Err(DispatchError::Operation("Track is locked".into()));
                    }
                    if item.clip_index >= self.document.project.sequence.tracks[t_idx].clips.len() {
                        return Err(DispatchError::InvalidParameters("Timeline clip does not exist".into()));
                    }
                }

                self.checkpoint();
                for item in params.moves {
                    let t_idx = find_track_index(&self.document, item.track_id)?;
                    self.document.project.sequence.tracks[t_idx].clips[item.clip_index].timeline_start = item.timeline_start;
                }
                for track in &mut self.document.project.sequence.tracks {
                    track.clips.sort_by_key(|c| c.timeline_start);
                }

                Ok(json!({"moved": true}))
            }
            "timeline.set_mark_in" | "timeline.set_mark_out" => {
                let params: SetMarkParams = parse_params(request.params)?;
                if params.at.0 < 0 {
                    return Err(DispatchError::InvalidParameters("Sequence marks cannot be before time zero".into()));
                }
                let sequence = &self.document.project.sequence;
                let (other, label) = if command.id == "timeline.set_mark_in" {
                    (sequence.mark_out, "In")
                } else {
                    (sequence.mark_in, "Out")
                };
                if let Some(other) = other {
                    let valid = if label == "In" { params.at.0 < other.0 } else { params.at.0 > other.0 };
                    if !valid {
                        return Err(DispatchError::InvalidParameters("The In point must be before the Out point".into()));
                    }
                }
                self.checkpoint();
                if label == "In" {
                    self.document.project.sequence.mark_in = Some(params.at);
                } else {
                    self.document.project.sequence.mark_out = Some(params.at);
                }
                Ok(json!({"mark": label, "at": params.at}))
            }
            "timeline.clear_marks" => {
                self.checkpoint();
                self.document.project.sequence.mark_in = None;
                self.document.project.sequence.mark_out = None;
                Ok(json!({"cleared": true}))
            }
            "timeline.move_clip" => {
                let params: MoveClipParams = parse_params(request.params)?;
                if params.timeline_start.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "A clip cannot start before time zero".into(),
                    ));
                }
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                let target_track_index = params
                    .target_track_id
                    .map(|target_id| {
                        self.document
                            .project
                            .sequence
                            .tracks
                            .iter()
                            .position(|track| track.id == target_id)
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Timeline track {target_id} does not exist"
                                ))
                            })
                    })
                    .transpose()?
                    .unwrap_or(track_index);
                if self.document.project.sequence.tracks[track_index].locked
                    || self.document.project.sequence.tracks[target_track_index].locked
                {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                let media_id = self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index]
                    .media_id;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {media_id} does not exist"
                        ))
                    })?;
                let target_kind = &self.document.project.sequence.tracks[target_track_index].kind;
                let kind_matches = matches!(
                    (&media.kind, target_kind),
                    (MediaKind::Audio, TrackKind::Audio)
                        | (
                            MediaKind::Video | MediaKind::Image | MediaKind::Title | MediaKind::Unknown,
                            TrackKind::Video
                        )
                );
                if !kind_matches {
                    return Err(DispatchError::InvalidParameters(
                        "Audio media must be placed on an audio track, and visual media on a video track".into(),
                    ));
                }
                self.checkpoint();
                let mut target_track_id = params.track_id;
                if track_index == target_track_index {
                    self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .timeline_start = params.timeline_start;
                } else {
                    let mut clip = self.document.project.sequence.tracks[track_index]
                        .clips
                        .remove(params.clip_index);
                    clip.timeline_start = params.timeline_start;
                    let target = &mut self.document.project.sequence.tracks[target_track_index];
                    target.clips.push(clip);
                    target.clips.sort_by_key(|item| item.timeline_start);
                    target_track_id = target.id;
                }
                let target = &mut self.document.project.sequence.tracks[target_track_index];
                if track_index == target_track_index {
                    target.clips.sort_by_key(|item| item.timeline_start);
                }
                let clip_index = target
                    .clips
                    .iter()
                    .position(|clip| {
                        clip.media_id == media_id && clip.timeline_start == params.timeline_start
                    })
                    .ok_or_else(|| {
                        DispatchError::Operation("Moved clip could not be found".into())
                    })?;
                Ok(json!({"moved": true, "track_id": target_track_id, "clip_index": clip_index}))
            }
            "clip.set_color" => {
                let params: SetColorParams = parse_params(request.params)?;
                if !params.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&params.exposure)
                    || !params.contrast.is_finite()
                    || !(0.0..=2.0).contains(&params.contrast)
                    || !params.saturation.is_finite()
                    || !(0.0..=2.0).contains(&params.saturation)
                    || !params.hue.is_finite()
                    || !(-1.0..=1.0).contains(&params.hue)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Color values must be exposure -5..5, contrast 0..2, saturation 0..2, and hue -1..1"
                            .into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.color.exposure = params.exposure;
                clip.color.contrast = params.contrast;
                clip.color.saturation = params.saturation;
                clip.color.hue = params.hue;
                if let Some(existing) = clip.effects.iter_mut().find_map(|e| match &mut e.payload {
                    EffectPayload::ColorCorrection(c) => Some(c),
                    _ => None,
                }) {
                    existing.exposure = params.exposure;
                    existing.contrast = params.contrast;
                    existing.saturation = params.saturation;
                    existing.hue = params.hue;
                }
                Ok(json!({"updated": true}))
            }
            "track.set_gain_db" => {
                let params: SetTrackGainParams = parse_params(request.params)?;
                if !params.gain_db.is_finite() || !(-60.0..=12.0).contains(&params.gain_db) {
                    return Err(DispatchError::InvalidParameters(
                        "Track gain must be between -60 and +12 dB".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].gain_db = params.gain_db;
                Ok(json!({"updated": true}))
            }
            "track.set_pan" => {
                let params: SetTrackPanParams = parse_params(request.params)?;
                if !params.pan.is_finite() || !(-1.0..=1.0).contains(&params.pan) {
                    return Err(DispatchError::InvalidParameters(
                        "Track pan must be between -1.0 and +1.0".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].pan = params.pan;
                Ok(json!({"updated": true, "pan": params.pan}))
            }
            "track.set_muted" => {
                let params: SetTrackMutedParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].muted = params.muted;
                Ok(json!({"updated": true, "muted": params.muted}))
            }
            "track.set_solo" => {
                let params: SetTrackSoloParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].solo = params.solo;
                Ok(json!({"updated": true, "solo": params.solo}))
            }
            "track.set_locked" => {
                let params: SetTrackLockedParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].locked = params.locked;
                Ok(json!({"updated": true, "locked": params.locked}))
            }
            "track.set_audio_processing" => {
                let params: SetTrackAudioProcessingParams = parse_params(request.params)?;
                if !params.eq_low_gain.is_finite() || !(-24.0..=24.0).contains(&params.eq_low_gain)
                    || !params.eq_mid_gain.is_finite() || !(-24.0..=24.0).contains(&params.eq_mid_gain)
                    || !params.eq_high_gain.is_finite() || !(-24.0..=24.0).contains(&params.eq_high_gain)
                    || !params.comp_threshold.is_finite() || !(-80.0..=0.0).contains(&params.comp_threshold)
                    || !params.comp_ratio.is_finite() || !(1.0..=50.0).contains(&params.comp_ratio)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Audio processing values are outside supported ranges".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].audio_processing = TrackAudioProcessing {
                    eq_enabled: params.eq_enabled,
                    eq_low_gain: params.eq_low_gain,
                    eq_mid_gain: params.eq_mid_gain,
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
            }
            "clip.set_keyframe" => {
                let params: SetKeyframeParams = parse_params(request.params)?;
                if !params.value.is_finite()
                    || !valid_keyframe_value(&params.property, params.value)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe value is outside the supported range".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                if params.time.0 < 0 || params.time.0 > clip.duration.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe time must be within the selected clip".into(),
                    ));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                let interp = params.interpolation.unwrap_or_default();
                if let Some(existing) = clip.keyframes.iter_mut().find(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) {
                    existing.value = params.value;
                    if params.interpolation.is_some() {
                        existing.interpolation = interp;
                        existing.handle_in_x = params.handle_in_x;
                        existing.handle_in_y = params.handle_in_y;
                        existing.handle_out_x = params.handle_out_x;
                        existing.handle_out_y = params.handle_out_y;
                    }
                } else {
                    clip.keyframes.push(Keyframe {
                        property: params.property,
                        time: params.time,
                        value: params.value,
                        interpolation: interp,
                        handle_in_x: params.handle_in_x,
                        handle_in_y: params.handle_in_y,
                        handle_out_x: params.handle_out_x,
                        handle_out_y: params.handle_out_y,
                    });
                    clip.keyframes.sort_by_key(|keyframe| keyframe.time);
                }
                Ok(json!({"updated": true, "keyframes": clip.keyframes.len()}))
            }
            "clip.remove_keyframe" => {
                let params: RemoveKeyframeParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(index) = clip.keyframes.iter().position(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) else {
                    return Ok(json!({"removed": false}));
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .keyframes
                    .remove(index);
                Ok(json!({"removed": true}))
            }
            "clip.add_effect" => {
                let params: AddEffectParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let effect_id = self.document.project.next_id;
                self.document.project.next_id = effect_id.saturating_add(1);
                let effect = match params.kind {
                    EffectKind::ColorCorrection => {
                        let mut inst = EffectInstance::new_color_correction(effect_id, ColorCorrectionParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                    EffectKind::GaussianBlur => {
                        let mut inst = EffectInstance::new_gaussian_blur(effect_id, GaussianBlurParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                    EffectKind::Crop => {
                        let mut inst = EffectInstance::new_crop(effect_id, CropParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                    EffectKind::Vignette => {
                        let mut inst = EffectInstance::new_vignette(effect_id, VignetteParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                    EffectKind::ChromaKey => {
                        let mut inst = EffectInstance::new_chroma_key(effect_id, ChromaKeyParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                    EffectKind::Lut => {
                        let mut inst = EffectInstance::new_lut(effect_id, LutParams::default());
                        if let Some(name) = params.name { inst.name = name; }
                        inst
                    }
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .effects
                    .push(effect);
                Ok(json!({"added": true, "effect_id": effect_id}))
            }
            "clip.remove_effect" => {
                let params: RemoveEffectParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Ok(json!({"removed": false}));
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .effects
                    .remove(pos);
                Ok(json!({"removed": true}))
            }
            "clip.update_effect" => {
                let params: UpdateEffectParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Err(DispatchError::Operation("Effect could not be found".into()));
                };
                self.checkpoint();
                let target = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index].effects[pos];
                target.payload = params.payload.clone();
                if let EffectPayload::ColorCorrection(ref c) = params.payload {
                    let clip_ref = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                    clip_ref.color.exposure = c.exposure;
                    clip_ref.color.contrast = c.contrast;
                    clip_ref.color.saturation = c.saturation;
                    clip_ref.color.hue = c.hue;
                    clip_ref.color.wheels = c.wheels.clone();
                }
                Ok(json!({"updated": true}))
            }
            "clip.toggle_effect" => {
                let params: ToggleEffectParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(pos) = clip.effects.iter().position(|e| e.id == params.effect_id) else {
                    return Err(DispatchError::Operation("Effect could not be found".into()));
                };
                self.checkpoint();
                let target = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index].effects[pos];
                target.enabled = params.enabled.unwrap_or(!target.enabled);
                let enabled = target.enabled;
                Ok(json!({"updated": true, "enabled": enabled}))
            }
            "clip.reorder_effects" => {
                let params: ReorderEffectsParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let total = clip.effects.len();
                if params.from_index >= total || params.to_index >= total {
                    return Err(DispatchError::InvalidParameters("Effect index out of bounds".into()));
                }
                if params.from_index != params.to_index {
                    self.checkpoint();
                    let effect = self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .effects
                        .remove(params.from_index);
                    self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .effects
                        .insert(params.to_index, effect);
                }
                Ok(json!({"reordered": true}))
            }
            "timeline.add_transition" => {
                let params: AddTransitionParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                let duration = params.duration.unwrap_or(Tick(TICKS_PER_SECOND));
                let kind = params.kind.unwrap_or(TransitionKind::Crossfade);
                if duration.0 <= 0 || params.cut_at.0 < 0 {
                    return Err(DispatchError::InvalidParameters("Transition duration and position must be positive".into()));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.transitions.retain(|t| (t.cut_at.0 - params.cut_at.0).abs() > TICKS_PER_SECOND / 10);
                track.transitions.push(Transition {
                    id,
                    kind,
                    cut_at: params.cut_at,
                    duration,
                });
                Ok(json!({
                    "transition_id": id,
                    "track_id": params.track_id,
                    "cut_at": params.cut_at,
                    "duration": duration,
                    "kind": kind,
                }))
            }
            "timeline.remove_transition" => {
                let params: RemoveTransitionParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }
                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                let initial_count = track.transitions.len();
                track.transitions.retain(|t| t.id != params.transition_id);
                Ok(json!({"removed": track.transitions.len() < initial_count}))
            }
            "timeline.add_title" => {
                let params: AddTitleParams = parse_params(request.params)?;
                let track_id = params.track_id.unwrap_or(1);
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|t| t.id == track_id && matches!(t.kind, TrackKind::Video))
                    .ok_or_else(|| DispatchError::InvalidParameters("Target video track does not exist".into()))?;
                if self.document.project.sequence.tracks[track_index].locked {
                    return Err(DispatchError::Operation("Track is locked".into()));
                }

                self.checkpoint();
                let media_id = self.document.project.next_id;
                self.document.project.next_id = media_id.saturating_add(1);
                let duration = params.duration.unwrap_or(Tick(TICKS_PER_SECOND * 5));
                let title_props = TitleProperties {
                    text: params.text.clone(),
                    subtitle: params.subtitle,
                    style: params.style.unwrap_or_else(|| "lower_third".into()),
                    font_family: "Inter".into(),
                    font_size: 42.0,
                    color: "#ffffff".into(),
                    bg_color: Some("rgba(15, 23, 42, 0.75)".into()),
                };
                let media_item = MediaAsset {
                    id: media_id,
                    name: format!("Title: {}", params.text.chars().take(20).collect::<String>()),
                    path: PathBuf::from(format!("title://{}", media_id)),
                    kind: MediaKind::Title,
                    duration: Some(duration),
                    width: Some(self.document.project.settings.width),
                    height: Some(self.document.project.settings.height),
                    title: Some(title_props),
                    waveform: None,
                };
                self.document.project.media.push(media_item);

                let track = &self.document.project.sequence.tracks[track_index];
                let timeline_start = params.timeline_start.unwrap_or_else(|| {
                    track.clips.iter().map(|c| c.timeline_start.0 + c.duration.0).max().unwrap_or(0).into()
                });
                let clip = Clip {
                    media_id,
                    timeline_start,
                    source_in: Tick(0),
                    duration,
                    speed: 1.0,
                    group_id: None,
                    color: ColorAdjustments::default(),
                    effects: Vec::new(),
                    keyframes: Vec::new(),
                    multicam_id: None,
                    multicam_angle_id: None,
                    tracking: None,
                    node_graph: None,
                    speed_ramp: None,
                    motion_blur: None,
                };
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips.push(clip);
                track.clips.sort_by_key(|c| c.timeline_start);
                let clip_index = track.clips.iter().position(|c| c.media_id == media_id && c.timeline_start == timeline_start).unwrap_or(0);
                Ok(json!({
                    "media_id": media_id,
                    "track_id": track_id,
                    "clip_index": clip_index,
                }))
            }
            "title.update" => {
                let params: UpdateTitleParams = parse_params(request.params)?;
                let media_index = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|m| m.id == params.media_id)
                    .ok_or_else(|| DispatchError::InvalidParameters(format!("Media item {} does not exist", params.media_id)))?;
                if !matches!(self.document.project.media[media_index].kind, MediaKind::Title) {
                    return Err(DispatchError::InvalidParameters("Media is not a title clip".into()));
                }
                self.checkpoint();
                let media = &mut self.document.project.media[media_index];
                let mut title = media.title.clone().unwrap_or_default();
                if let Some(t) = params.text {
                    media.name = format!("Title: {}", t.chars().take(20).collect::<String>());
                    title.text = t;
                }
                if let Some(sub) = params.subtitle { title.subtitle = Some(sub); }
                if let Some(st) = params.style { title.style = st; }
                if let Some(sz) = params.font_size { title.font_size = sz; }
                if let Some(col) = params.color { title.color = col; }
                if let Some(bg) = params.bg_color { title.bg_color = Some(bg); }
                media.title = Some(title.clone());
                Ok(json!({"media_id": params.media_id, "title": title}))
            }

            "subtitle.add" => {
                let params: AddSubtitleParams = parse_params(request.params)?;
                if params.start_ticks.0 < 0 || params.end_ticks.0 <= params.start_ticks.0 {
                    return Err(DispatchError::InvalidParameters("Subtitle start time must be non-negative and before end time".into()));
                }
                if params.text.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters("Subtitle text cannot be empty".into()));
                }
                self.checkpoint();
                let sub_id = self.document.project.next_id;
                self.document.project.next_id = sub_id.saturating_add(1);
                self.document.project.sequence.subtitles.push(SubtitleItem {
                    id: sub_id,
                    start_ticks: params.start_ticks,
                    end_ticks: params.end_ticks,
                    text: params.text,
                });
                self.document.project.sequence.subtitles.sort_by_key(|s| s.start_ticks);
                Ok(json!({"added": true, "id": sub_id}))
            }
            "subtitle.remove" => {
                let params: RemoveSubtitleParams = parse_params(request.params)?;
                let initial = self.document.project.sequence.subtitles.len();
                self.checkpoint();
                self.document.project.sequence.subtitles.retain(|s| s.id != params.id);
                Ok(json!({"removed": self.document.project.sequence.subtitles.len() < initial}))
            }
            "subtitle.update" => {
                let params: UpdateSubtitleParams = parse_params(request.params)?;
                let Some(sub) = self.document.project.sequence.subtitles.iter_mut().find(|s| s.id == params.id) else {
                    return Err(DispatchError::InvalidParameters(format!("Subtitle {} does not exist", params.id)));
                };
                let new_start = params.start_ticks.unwrap_or(sub.start_ticks);
                let new_end = params.end_ticks.unwrap_or(sub.end_ticks);
                if new_start.0 < 0 || new_end.0 <= new_start.0 {
                    return Err(DispatchError::InvalidParameters("Invalid subtitle timing: start must precede end".into()));
                }
                self.checkpoint();
                let sub = self.document.project.sequence.subtitles.iter_mut().find(|s| s.id == params.id).unwrap();
                sub.start_ticks = new_start;
                sub.end_ticks = new_end;
                if let Some(t) = params.text {
                    sub.text = t;
                }
                self.document.project.sequence.subtitles.sort_by_key(|s| s.start_ticks);
                Ok(json!({"updated": true, "id": params.id}))
            }
            "subtitle.clear" => {
                self.checkpoint();
                self.document.project.sequence.subtitles.clear();
                Ok(json!({"cleared": true}))
            }
            "clip.set_color_wheels" => {
                let params: SetColorWheelsParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.color.wheels = params.wheels.clone();
                if let Some(existing) = clip.effects.iter_mut().find_map(|e| match &mut e.payload {
                    EffectPayload::ColorCorrection(c) => Some(c),
                    _ => None,
                }) {
                    existing.wheels = params.wheels;
                }
                Ok(json!({"updated": true}))
            }
            "multicam.create" => {
                let params: CreateMulticamParams = parse_params(request.params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters("Multicam name cannot be empty".into()));
                }
                if params.angles.is_empty() {
                    return Err(DispatchError::InvalidParameters("Multicam must contain at least one angle".into()));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let active_angle_id = params.angles[0].id;
                let group = MultiCamClip {
                    id,
                    name: params.name,
                    angles: params.angles,
                    active_angle_id,
                    sync_mode: params.sync_mode,
                };
                self.document.project.sequence.multicam_clips.push(group);
                Ok(json!({"id": id, "active_angle_id": active_angle_id}))
            }
            "multicam.switch_angle" => {
                let params: SwitchMulticamAngleParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                let clip = &self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                let group = if let Some(m_id) = clip.multicam_id {
                    self.document.project.sequence.multicam_clips.iter().find(|g| g.id == m_id)
                } else {
                    self.document.project.sequence.multicam_clips.iter().find(|g| g.angles.iter().any(|a| a.id == params.angle_id))
                }.ok_or_else(|| DispatchError::InvalidParameters("Multicam group not found".into()))?;
                let angle = group.angles.iter().find(|a| a.id == params.angle_id)
                    .ok_or_else(|| DispatchError::InvalidParameters(format!("Angle {} not found in multicam", params.angle_id)))?;
                let group_id = group.id;
                let angle_media_id = angle.media_id;
                let sync_offset = angle.sync_offset_ticks;
                self.checkpoint();
                let clip_mut = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip_mut.multicam_id = Some(group_id);
                clip_mut.multicam_angle_id = Some(params.angle_id);
                clip_mut.media_id = angle_media_id;
                clip_mut.source_in = sync_offset;
                Ok(json!({"switched": true, "media_id": angle_media_id, "angle_id": params.angle_id}))
            }
            "multicam.sync" => {
                let params: SyncMulticamParams = parse_params(request.params)?;
                let pos = self.document.project.sequence.multicam_clips.iter()
                    .position(|g| g.id == params.multicam_id)
                    .ok_or_else(|| DispatchError::InvalidParameters(format!("Multicam {} does not exist", params.multicam_id)))?;
                self.checkpoint();
                let group = &mut self.document.project.sequence.multicam_clips[pos];
                group.sync_mode = params.sync_mode;
                if let Some(offsets) = params.offsets {
                    for angle in &mut group.angles {
                        if let Some(&offset) = offsets.get(&angle.id) {
                            angle.sync_offset_ticks = Tick(offset.max(0));
                        }
                    }
                }
                Ok(json!({"synced": true, "multicam_id": params.multicam_id}))
            }
            "clip.set_motion_tracking" => {
                let params: SetMotionTrackingParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.tracking = Some(params.tracking);
                Ok(json!({"updated": true}))
            }
            "clip.pin_to_track" => {
                let params: PinToTrackParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                if let Some(ref mut tr) = clip.tracking {
                    tr.pinned_clip_id = params.pinned_clip_id;
                } else {
                    let tr = MotionTrackingData {
                        id: 1,
                        name: "Track Point".into(),
                        mode: TrackingMode::Pin,
                        points: Vec::new(),
                        pinned_clip_id: params.pinned_clip_id,
                        smoothness: 0.5,
                    };
                    clip.tracking = Some(tr);
                }
                Ok(json!({"pinned": true, "pinned_clip_id": params.pinned_clip_id}))
            }
            "track.set_bus_route" => {
                let params: SetTrackBusRouteParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].bus = params.bus;
                Ok(json!({"updated": true, "bus": params.bus}))
            }
            "track.set_automation" => {
                let params: SetTrackAutomationParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].automation = params.automation;
                Ok(json!({"updated": true}))
            }
            "track.add_audio_plugin" => {
                let params: AddAudioPluginParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].plugins.push(params.plugin);
                Ok(json!({"added": true}))
            }
            "track.toggle_audio_plugin" => {
                let params: ToggleAudioPluginParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                if let Some(p) = self.document.project.sequence.tracks[track_index].plugins.iter_mut().find(|p| p.id == params.plugin_id) {
                    p.enabled = params.enabled;
                }
                Ok(json!({"updated": true}))
            }
            "bus.set_gain_db" => {
                let params: SetBusGainParams = parse_params(request.params)?;
                if !params.gain_db.is_finite() || !(-60.0..=12.0).contains(&params.gain_db) {
                    return Err(DispatchError::InvalidParameters("Bus gain must be between -60 and +12 dB".into()));
                }
                self.checkpoint();
                if let Some(bus) = self.document.project.sequence.buses.iter_mut().find(|b| b.target == params.target) {
                    bus.gain_db = params.gain_db;
                }
                Ok(json!({"updated": true, "gain_db": params.gain_db}))
            }
            "nodegraph.add_node" => {
                let params: AddNodeParams = parse_params(request.params)?;
                self.checkpoint();
                let node_id = self.document.project.next_id;
                self.document.project.next_id = node_id.saturating_add(1);
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                graph.nodes.push(CompositingNode {
                    id: node_id,
                    kind: params.kind,
                    title: params.title,
                    pos_x: params.pos_x,
                    pos_y: params.pos_y,
                    params: params.params,
                    enabled: true,
                });
                Ok(json!({"added": true, "node_id": node_id}))
            }
            "nodegraph.remove_node" => {
                let params: RemoveNodeParams = parse_params(request.params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                let initial_len = graph.nodes.len();
                graph.nodes.retain(|n| n.id != params.node_id);
                let removed = graph.nodes.len() < initial_len;
                if removed {
                    graph.connections.retain(|c| c.from_node != params.node_id && c.to_node != params.node_id);
                }
                Ok(json!({"removed": removed}))
            }
            "nodegraph.connect" => {
                let params: ConnectNodesParams = parse_params(request.params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                if !graph.connections.iter().any(|c| {
                    c.from_node == params.from_node
                        && c.from_socket == params.from_socket
                        && c.to_node == params.to_node
                        && c.to_socket == params.to_socket
                }) {
                    graph.connections.push(NodeConnection {
                        from_node: params.from_node,
                        from_socket: params.from_socket,
                        to_node: params.to_node,
                        to_socket: params.to_socket,
                    });
                }
                Ok(json!({"connected": true}))
            }
            "nodegraph.disconnect" => {
                let params: DisconnectNodesParams = parse_params(request.params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                let initial_len = graph.connections.len();
                graph.connections.retain(|c| !(
                    c.from_node == params.from_node
                        && c.from_socket == params.from_socket
                        && c.to_node == params.to_node
                        && c.to_socket == params.to_socket
                ));
                Ok(json!({"disconnected": graph.connections.len() < initial_len}))
            }
            "nodegraph.update_node" => {
                let params: UpdateNodeParams = parse_params(request.params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                let node = graph.nodes.iter_mut().find(|n| n.id == params.node_id)
                    .ok_or_else(|| DispatchError::Operation(format!("Node {} not found", params.node_id)))?;
                if let Some(x) = params.pos_x { node.pos_x = x; }
                if let Some(y) = params.pos_y { node.pos_y = y; }
                if let Some(e) = params.enabled { node.enabled = e; }
                if let Some(p) = params.params { node.params = p; }
                Ok(json!({"updated": true, "node_id": params.node_id}))
            }
            "nodegraph.add_mask" => {
                let params: AddMaskParams = parse_params(request.params)?;
                self.checkpoint();
                let mask_id = if params.mask.id == 0 {
                    let id = self.document.project.next_id;
                    self.document.project.next_id = id.saturating_add(1);
                    id
                } else {
                    params.mask.id
                };
                let mut mask = params.mask;
                mask.id = mask_id;
                let graph = get_or_create_node_graph(&mut self.document, params.track_id, params.clip_index)?;
                graph.masks.push(mask);
                Ok(json!({"added": true, "mask_id": mask_id}))
            }
            "clip.set_speed_ramp" => {
                let params: SetSpeedRampParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.speed_ramp = Some(params.speed_ramp);
                Ok(json!({"updated": true}))
            }
            "clip.set_motion_blur" => {
                let params: SetMotionBlurParams = parse_params(request.params)?;
                if !params.motion_blur.shutter_angle_deg.is_finite()
                    || !(0.0..=360.0).contains(&params.motion_blur.shutter_angle_deg)
                {
                    return Err(DispatchError::InvalidParameters("Shutter angle must be between 0 and 360 degrees".into()));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips[params.clip_index];
                clip.motion_blur = Some(params.motion_blur);
                Ok(json!({"updated": true}))
            }
            "collab.acquire_lock" => {
                let params: AcquireLockParams = parse_params(request.params)?;
                self.checkpoint();
                let collab = &mut self.document.project.collaboration;
                if let Some(existing) = collab.locks.iter().find(|l| l.target_kind == params.target_kind && l.target_id == params.target_id) {
                    if existing.user_id != params.user_id {
                        return Ok(json!({
                            "acquired": false,
                            "locked_by": existing.user_name,
                            "user_id": existing.user_id
                        }));
                    }
                }
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let lock = UserLock {
                    user_id: params.user_id.clone(),
                    user_name: params.user_name.clone(),
                    user_color: params.user_color.unwrap_or_else(|| "#38bdf8".into()),
                    target_kind: params.target_kind,
                    target_id: params.target_id,
                    locked_at: now,
                };
                collab.locks.retain(|l| !(l.target_kind == params.target_kind && l.target_id == params.target_id));
                collab.locks.push(lock);
                if !collab.active_users.contains(&params.user_name) {
                    collab.active_users.push(params.user_name.clone());
                }
                let change_id = self.document.project.next_id;
                self.document.project.next_id = change_id.saturating_add(1);
                collab.change_log.push(TimelineChangeEntry {
                    id: change_id,
                    user_id: params.user_id,
                    user_name: params.user_name,
                    timestamp: now,
                    action: "lock_acquired".into(),
                    summary: format!("Acquired lock on {:?} #{}", params.target_kind, params.target_id),
                });
                Ok(json!({"acquired": true}))
            }
            "collab.release_lock" => {
                let params: ReleaseLockParams = parse_params(request.params)?;
                self.checkpoint();
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let collab = &mut self.document.project.collaboration;
                let initial_len = collab.locks.len();
                collab.locks.retain(|l| !(l.target_kind == params.target_kind && l.target_id == params.target_id && l.user_id == params.user_id));
                let released = collab.locks.len() < initial_len;
                if released {
                    let change_id = self.document.project.next_id;
                    self.document.project.next_id = change_id.saturating_add(1);
                    collab.change_log.push(TimelineChangeEntry {
                        id: change_id,
                        user_id: params.user_id,
                        user_name: "Collaborator".into(),
                        timestamp: now,
                        action: "lock_released".into(),
                        summary: format!("Released lock on {:?} #{}", params.target_kind, params.target_id),
                    });
                }
                Ok(json!({"released": released}))
            }
            "collab.record_change" => {
                let params: RecordChangeParams = parse_params(request.params)?;
                self.checkpoint();
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let entry_id = self.document.project.next_id;
                self.document.project.next_id = entry_id.saturating_add(1);
                self.document.project.collaboration.change_log.push(TimelineChangeEntry {
                    id: entry_id,
                    user_id: params.user_id,
                    user_name: params.user_name,
                    timestamp: now,
                    action: params.action,
                    summary: params.summary,
                });
                Ok(json!({"recorded": true, "entry_id": entry_id}))
            }

            "speech.transcribe_to_subtitles" => {
                let params: TranscribeToSubtitlesParams = parse_params(request.params)?;
                if params.segments.is_empty() {
                    return Err(DispatchError::InvalidParameters("No speech segments provided".into()));
                }
                self.checkpoint();
                let new_subs = crate::speech::speech_segments_to_subtitles(&params.segments);
                let count = new_subs.len();
                if params.replace_existing.unwrap_or(true) {
                    self.document.project.sequence.subtitles = new_subs;
                } else {
                    self.document.project.sequence.subtitles.extend(new_subs);
                    self.document.project.sequence.subtitles.sort_by_key(|s| s.start_ticks);
                }
                Ok(json!({"transcribed": true, "subtitles_count": count}))
            }
            "speech.ripple_delete_silence" => {
                let params: RippleDeleteSilenceParams = parse_params(request.params)?;
                let opts = params.options.unwrap_or_default();
                let total_dur = self.document.project.sequence.duration();
                let silences = crate::speech::detect_silence_from_peaks(&params.peaks, total_dur, &opts);
                let ranges: Vec<(Tick, Tick)> = silences.into_iter().map(|s| (s.start_ticks, s.end_ticks)).collect();
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(&mut self.document.project.sequence, &ranges)
                    .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            }
            "speech.ripple_delete_fillers" => {
                let params: RippleDeleteFillersParams = parse_params(request.params)?;
                let custom_slice: Option<Vec<&str>> = params.custom_fillers.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());
                let fillers = crate::speech::extract_filler_intervals(&params.segments, custom_slice.as_deref());
                let ranges: Vec<(Tick, Tick)> = fillers.into_iter().map(|s| (s.start_ticks, s.end_ticks)).collect();
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(&mut self.document.project.sequence, &ranges)
                    .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            }
            "timeline.ripple_delete_ranges" => {
                let params: RippleDeleteRangesParams = parse_params(request.params)?;
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(&mut self.document.project.sequence, &params.ranges)
                    .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"deleted": true})))
            }
            "interop.import_edl" => {
                let params: ImportEdlParams = parse_params(request.params)?;
                let name = params.project_name.unwrap_or_else(|| "Imported EDL".to_string());
                let imported = crate::interop::import_cmx3600_edl(&params.edl, &name)
                    .map_err(DispatchError::Operation)?;
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            }
            "interop.import_fcpxml" => {
                let params: ImportXmlParams = parse_params(request.params)?;
                let imported = crate::interop::import_fcpxml(&params.xml)
                    .map_err(DispatchError::Operation)?;
                let name = imported.project.name.clone();
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            }
            "interop.import_premiere_xml" => {
                let params: ImportXmlParams = parse_params(request.params)?;
                let imported = crate::interop::import_premiere_xml(&params.xml)
                    .map_err(DispatchError::Operation)?;
                let name = imported.project.name.clone();
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            }

            _ => Err(DispatchError::UnknownCommand(request.id)),
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

fn parse_params<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, DispatchError> {
    serde_json::from_value(value).map_err(|e| DispatchError::InvalidParameters(e.to_string()))
}

fn find_track_index(document: &ProjectDocument, track_id: u64) -> Result<usize, DispatchError> {
    document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| {
            DispatchError::InvalidParameters(format!("Timeline track {track_id} does not exist"))
        })
}

fn find_clip(
    document: &ProjectDocument,
    track_index: usize,
    clip_index: usize,
) -> Result<&Clip, DispatchError> {
    document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or_else(|| DispatchError::InvalidParameters("Timeline clip does not exist".into()))
}

fn valid_keyframe_value(property: &KeyframeProperty, value: f64) -> bool {
    match property {
        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
            (-100_000.0..=100_000.0).contains(&value)
        }
        KeyframeProperty::Scale => (0.01..=100.0).contains(&value),
        KeyframeProperty::Rotation => (-36_000.0..=36_000.0).contains(&value),
        KeyframeProperty::Opacity => (0.0..=1.0).contains(&value),
    }
}

fn keyframe_value_at(keyframes: &[Keyframe], property: &KeyframeProperty, time: i64) -> Option<f64> {
    let mut keys: Vec<_> = keyframes.iter().filter(|key| &key.property == property).collect();
    keys.sort_by_key(|key| key.time);
    let first = *keys.first()?;
    if time <= first.time.0 { return Some(first.value); }
    let last = *keys.last()?;
    if time >= last.time.0 { return Some(last.value); }
    let next_index = keys.iter().position(|key| key.time.0 >= time)?;
    let previous = keys[next_index - 1];
    let next = keys[next_index];
    let fraction = (time - previous.time.0) as f64 / (next.time.0 - previous.time.0) as f64;
    Some(previous.value + (next.value - previous.value) * fraction)
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
