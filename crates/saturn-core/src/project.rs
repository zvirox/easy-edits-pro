use crate::time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROJECT_FORMAT: &str = "easy-edits-pro.project";
pub const LEGACY_PROJECT_FORMAT: &str = "project-saturn.project";
pub const PROJECT_SCHEMA_VERSION: u32 = 10;
#[path = "project/io.rs"]
mod io;
#[path = "project/migrate.rs"]
mod migrate;
pub(crate) use migrate::migrate_project;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub format: String,
    pub schema_version: u32,
    pub generator: String,
    pub project: Project,
    #[serde(default)]
    pub view: ProjectView,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaBin {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<u64>,
    #[serde(default = "default_bin_color")]
    pub color: String,
}

fn default_bin_color() -> String {
    "#38bdf8".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub settings: ProjectSettings,
    pub media: Vec<MediaAsset>,
    #[serde(default)]
    pub bins: Vec<MediaBin>,
    pub sequence: Sequence,
    #[serde(default)]
    pub sequences: Vec<Sequence>,
    pub next_id: u64,
    #[serde(default)]
    pub collaboration: CollaborationState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Title,
    CompoundSequence,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TitleProperties {
    pub text: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default = "default_title_style")]
    pub style: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_title_color")]
    pub color: String,
    #[serde(default)]
    pub bg_color: Option<String>,
}

fn default_title_style() -> String { "lower_third".into() }
fn default_font_family() -> String { "Inter".into() }
fn default_font_size() -> f64 { 42.0 }
fn default_title_color() -> String { "#ffffff".into() }

impl Default for TitleProperties {
    fn default() -> Self {
        Self {
            text: "Title".into(),
            subtitle: None,
            style: default_title_style(),
            font_family: default_font_family(),
            font_size: default_font_size(),
            color: default_title_color(),
            bg_color: Some("rgba(15, 23, 42, 0.75)".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: u64,
    pub name: String,
    /// Source media stays external; project files store its path, not its contents.
    pub path: PathBuf,
    pub kind: MediaKind,
    #[serde(default)]
    pub duration: Option<Tick>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub audio_channels: Option<u32>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub bin_id: Option<u64>,
    #[serde(default)]
    pub color_label: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_missing: bool,
    #[serde(default)]
    pub title: Option<TitleProperties>,
    #[serde(default)]
    pub waveform: Option<Vec<u8>>,
    #[serde(default)]
    pub nested_sequence: Option<Sequence>,
}

impl Default for MediaAsset {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            path: PathBuf::new(),
            kind: MediaKind::Unknown,
            duration: None,
            width: None,
            height: None,
            fps: None,
            audio_channels: None,
            codec: None,
            bin_id: None,
            color_label: None,
            tags: Vec::new(),
            is_missing: false,
            title: None,
            waveform: None,
            nested_sequence: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    Crossfade,
    DipToBlack,
    DipToWhite,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transition {
    pub id: u64,
    pub kind: TransitionKind,
    pub cut_at: Tick,
    pub duration: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleItem {
    pub id: u64,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiCamSyncMode {
    #[default]
    Timecode,
    AudioWaveform,
    Marker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamAngle {
    pub id: u64,
    pub name: String,
    pub media_id: u64,
    #[serde(default)]
    pub sync_offset_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamClip {
    pub id: u64,
    pub name: String,
    pub angles: Vec<MultiCamAngle>,
    pub active_angle_id: u64,
    #[serde(default)]
    pub sync_mode: MultiCamSyncMode,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusTarget {
    #[default]
    Master,
    Submix1,
    Submix2,
    AuxFx,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationMode {
    #[default]
    Off,
    Read,
    Touch,
    Latch,
    Write,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutomationPoint {
    pub time: Tick,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrackAutomation {
    #[serde(default)]
    pub mode: AutomationMode,
    #[serde(default)]
    pub volume_points: Vec<AutomationPoint>,
    #[serde(default)]
    pub pan_points: Vec<AutomationPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioPluginInstance {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_plugin_format")]
    pub plugin_format: String,
    #[serde(default = "default_plugin_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub params: std::collections::HashMap<String, f64>,
}

fn default_plugin_format() -> String { "builtin".into() }
fn default_plugin_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioBus {
    pub target: BusTarget,
    pub name: String,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub processing: TrackAudioProcessing,
}

pub fn default_audio_buses() -> Vec<AudioBus> {
    vec![
        AudioBus {
            target: BusTarget::Master,
            name: "Master Out".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::Submix1,
            name: "Submix 1 (Dialogue)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::Submix2,
            name: "Submix 2 (Music)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::AuxFx,
            name: "Aux FX (Reverb)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
    ]
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositingNodeKind {
    #[default]
    Merge,
    Transform,
    Keyer,
    Mask,
    Particles,
    MediaSource,
    OutputSink,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositingNode {
    pub id: u64,
    pub kind: CompositingNodeKind,
    pub title: String,
    pub pos_x: f64,
    pub pos_y: f64,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default = "default_node_enabled")]
    pub enabled: bool,
}

fn default_node_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeConnection {
    pub from_node: u64,
    pub from_socket: String,
    pub to_node: u64,
    pub to_socket: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RotoSplinePoint {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub handle_in: (f64, f64),
    #[serde(default)]
    pub handle_out: (f64, f64),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RotoSplineMask {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_mask_closed")]
    pub closed: bool,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub feather: f64,
    #[serde(default = "default_mask_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub points: Vec<RotoSplinePoint>,
}

fn default_mask_closed() -> bool { true }
fn default_mask_opacity() -> f64 { 1.0 }

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeGraph {
    #[serde(default)]
    pub nodes: Vec<CompositingNode>,
    #[serde(default)]
    pub connections: Vec<NodeConnection>,
    #[serde(default)]
    pub masks: Vec<RotoSplineMask>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeedRampPoint {
    pub time: Tick,
    pub speed_factor: f64,
    #[serde(default)]
    pub bezier_tangent: (f64, f64),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SpeedRamp {
    #[serde(default)]
    pub points: Vec<SpeedRampPoint>,
    #[serde(default = "default_optical_flow")]
    pub optical_flow_interpolation: bool,
    #[serde(default)]
    pub frame_blend: bool,
}

fn default_optical_flow() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MotionBlurParams {
    #[serde(default = "default_motion_blur_enabled")]
    pub enabled: bool,
    #[serde(default = "default_shutter_angle")]
    pub shutter_angle_deg: f64,
    #[serde(default = "default_motion_samples")]
    pub samples: u32,
}

fn default_motion_blur_enabled() -> bool { true }
fn default_shutter_angle() -> f64 { 180.0 }
fn default_motion_samples() -> u32 { 8 }

impl Default for MotionBlurParams {
    fn default() -> Self {
        Self {
            enabled: true,
            shutter_angle_deg: 180.0,
            samples: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockTargetKind {
    #[default]
    Bin,
    Sequence,
    Track,
    Clip,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserLock {
    pub user_id: String,
    pub user_name: String,
    #[serde(default = "default_user_color")]
    pub user_color: String,
    pub target_kind: LockTargetKind,
    pub target_id: u64,
    pub locked_at: u64,
}

fn default_user_color() -> String { "#38bdf8".into() }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineChangeEntry {
    pub id: u64,
    pub user_id: String,
    pub user_name: String,
    pub timestamp: u64,
    pub action: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollaborationState {
    #[serde(default = "default_session_id")]
    pub session_id: String,
    #[serde(default)]
    pub active_users: Vec<String>,
    #[serde(default)]
    pub locks: Vec<UserLock>,
    #[serde(default)]
    pub change_log: Vec<TimelineChangeEntry>,
}

fn default_session_id() -> String { "local-collab-session".into() }

impl Default for CollaborationState {
    fn default() -> Self {
        Self {
            session_id: default_session_id(),
            active_users: Vec::new(),
            locks: Vec::new(),
            change_log: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackGroup {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_track_group_color")]
    pub color: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
}

fn default_track_group_color() -> String {
    "#a855f7".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    #[serde(default = "default_sequence_id")]
    pub id: u64,
    pub name: String,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub track_groups: Vec<TrackGroup>,
    #[serde(default)]
    pub mark_in: Option<Tick>,
    #[serde(default)]
    pub mark_out: Option<Tick>,
    #[serde(default)]
    pub subtitles: Vec<SubtitleItem>,
    #[serde(default)]
    pub multicam_clips: Vec<MultiCamClip>,
    #[serde(default = "default_audio_buses")]
    pub buses: Vec<AudioBus>,
    #[serde(default)]
    pub node_graph: Option<NodeGraph>,
}

fn default_sequence_id() -> u64 {
    1
}

impl Sequence {
    pub fn duration(&self) -> Tick {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| Tick(c.timeline_start.0.saturating_add(c.duration.0)))
            .max()
            .unwrap_or(Tick(0))
    }

    pub fn active_clips_at(&self, tick: Tick) -> Vec<(&Track, &Clip)> {
        let mut results = Vec::new();
        for track in &self.tracks {
            for clip in &track.clips {
                let start = clip.timeline_start.0;
                let end = start.saturating_add(clip.duration.0);
                if tick.0 >= start && tick.0 < end {
                    results.push((track, clip));
                }
            }
        }
        results
    }

    pub fn transition_at(&self, tick: Tick) -> Option<(&Track, &Transition)> {
        for track in &self.tracks {
            for transition in &track.transitions {
                let half_dur = transition.duration.0 / 2;
                let start = transition.cut_at.0.saturating_sub(half_dur);
                let end = transition.cut_at.0.saturating_add(half_dur);
                if tick.0 >= start && tick.0 <= end {
                    return Some((track, transition));
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrackAudioProcessing {
    pub eq_enabled: bool,
    pub eq_low_gain: f64,    // -15.0 to +15.0 dB
    pub eq_mid_gain: f64,    // -15.0 to +15.0 dB
    pub eq_high_gain: f64,   // -15.0 to +15.0 dB
    pub comp_enabled: bool,
    pub comp_threshold: f64, // -60.0 to 0.0 dB
    pub comp_ratio: f64,     // 1.0 to 20.0
    pub comp_attack: f64,    // 0.001 to 0.1 s
    pub comp_release: f64,   // 0.05 to 1.0 s
    pub denoise_enabled: bool,
    pub denoise_amount: f64, // 0.0 to 1.0
}

impl Default for TrackAudioProcessing {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_low_gain: 0.0,
            eq_mid_gain: 0.0,
            eq_high_gain: 0.0,
            comp_enabled: false,
            comp_threshold: -24.0,
            comp_ratio: 4.0,
            comp_attack: 0.005,
            comp_release: 0.1,
            denoise_enabled: false,
            denoise_amount: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    #[default]
    Transform,
    Stabilize,
    Pin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackPoint {
    pub time: Tick,
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_track_confidence")]
    pub confidence: f64,
}

fn default_track_confidence() -> f64 { 1.0 }
fn default_smoothness() -> f64 { 0.5 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MotionTrackingData {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub mode: TrackingMode,
    #[serde(default)]
    pub points: Vec<TrackPoint>,
    #[serde(default)]
    pub pinned_clip_id: Option<u64>,
    #[serde(default = "default_smoothness")]
    pub smoothness: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub pan: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub audio_processing: TrackAudioProcessing,
    #[serde(default)]
    pub bus: BusTarget,
    #[serde(default)]
    pub automation: TrackAutomation,
    #[serde(default)]
    pub plugins: Vec<AudioPluginInstance>,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl Track {
    pub fn is_audible(&self, any_solo: bool) -> bool {
        !self.muted && (!any_solo || self.solo)
    }

    pub fn clip_at(&self, tick: Tick) -> Option<&Clip> {
        self.clips.iter().find(|clip| {
            let start = clip.timeline_start.0;
            let end = start.saturating_add(clip.duration.0);
            tick.0 >= start && tick.0 < end
        })
    }

    pub fn next_clip_after(&self, tick: Tick) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.timeline_start.0 > tick.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    #[default]
    Video,
    Audio,
}

fn default_speed() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub media_id: u64,
    pub timeline_start: Tick,
    pub source_in: Tick,
    pub duration: Tick,
    #[serde(default = "default_speed")]
    pub speed: f64,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub color: ColorAdjustments,
    #[serde(default)]
    pub effects: Vec<EffectInstance>,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
    #[serde(default)]
    pub multicam_id: Option<u64>,
    #[serde(default)]
    pub multicam_angle_id: Option<u64>,
    #[serde(default)]
    pub tracking: Option<MotionTrackingData>,
    #[serde(default)]
    pub node_graph: Option<NodeGraph>,
    #[serde(default)]
    pub speed_ramp: Option<SpeedRamp>,
    #[serde(default)]
    pub motion_blur: Option<MotionBlurParams>,
    #[serde(default)]
    pub compound_sequence_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ColorCorrection,
    GaussianBlur,
    Crop,
    Vignette,
    ChromaKey,
    Lut,
    CustomShader,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ColorWheelParam {
    #[serde(default)]
    pub red: f64,
    #[serde(default)]
    pub green: f64,
    #[serde(default)]
    pub blue: f64,
    #[serde(default)]
    pub luma: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ColorWheels {
    #[serde(default)]
    pub lift: ColorWheelParam,
    #[serde(default)]
    pub gamma: ColorWheelParam,
    #[serde(default)]
    pub gain: ColorWheelParam,
    #[serde(default)]
    pub offset: ColorWheelParam,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorCorrectionParams {
    #[serde(default = "default_exposure")]
    pub exposure: f64,
    #[serde(default = "default_contrast")]
    pub contrast: f64,
    #[serde(default = "default_saturation")]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
    #[serde(default)]
    pub wheels: ColorWheels,
}

fn default_exposure() -> f64 { 0.0 }
fn default_contrast() -> f64 { 1.0 }
fn default_saturation() -> f64 { 1.0 }

impl Default for ColorCorrectionParams {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
            wheels: ColorWheels::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GaussianBlurParams {
    #[serde(default = "default_blur_sigma")]
    pub sigma: f64,
}

fn default_blur_sigma() -> f64 { 2.0 }

impl Default for GaussianBlurParams {
    fn default() -> Self {
        Self { sigma: 2.0 }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CropParams {
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub bottom: f64,
    #[serde(default)]
    pub left: f64,
    #[serde(default)]
    pub right: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VignetteParams {
    #[serde(default = "default_vignette_intensity")]
    pub intensity: f64,
    #[serde(default = "default_vignette_radius")]
    pub radius: f64,
    #[serde(default = "default_vignette_softness")]
    pub softness: f64,
}

fn default_vignette_intensity() -> f64 { 0.5 }
fn default_vignette_radius() -> f64 { 0.8 }
fn default_vignette_softness() -> f64 { 0.5 }

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            radius: 0.8,
            softness: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChromaKeyParams {
    #[serde(default = "default_chroma_color")]
    pub color: String,
    #[serde(default = "default_chroma_tolerance")]
    pub tolerance: f64,
    #[serde(default = "default_chroma_softness")]
    pub softness: f64,
    #[serde(default = "default_chroma_spill")]
    pub spill_suppression: f64,
}

fn default_chroma_color() -> String { "#00ff00".into() }
fn default_chroma_tolerance() -> f64 { 25.0 }
fn default_chroma_softness() -> f64 { 10.0 }
fn default_chroma_spill() -> f64 { 0.5 }

impl Default for ChromaKeyParams {
    fn default() -> Self {
        Self {
            color: default_chroma_color(),
            tolerance: default_chroma_tolerance(),
            softness: default_chroma_softness(),
            spill_suppression: default_chroma_spill(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LutParams {
    #[serde(default = "default_lut_name")]
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_lut_intensity")]
    pub intensity: f64,
    #[serde(default)]
    pub preset_id: Option<String>,
}

fn default_lut_name() -> String { "Teal & Orange".into() }
fn default_lut_intensity() -> f64 { 1.0 }

impl Default for LutParams {
    fn default() -> Self {
        Self {
            name: default_lut_name(),
            path: None,
            intensity: default_lut_intensity(),
            preset_id: Some("teal_orange".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomShaderParams {
    pub name: String,
    pub shader_code: String,
    #[serde(default)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub uniforms: std::collections::HashMap<String, f64>,
}

impl Default for CustomShaderParams {
    fn default() -> Self {
        Self {
            name: "Custom Shader".into(),
            shader_code: r#"
// Saturn Custom Fragment Shader (GLSL)
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_time;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    gl_FragColor = color;
}
"#.trim().into(),
            preset_id: Some("passthrough".into()),
            uniforms: std::collections::HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum EffectPayload {
    ColorCorrection(ColorCorrectionParams),
    GaussianBlur(GaussianBlurParams),
    Crop(CropParams),
    Vignette(VignetteParams),
    ChromaKey(ChromaKeyParams),
    Lut(LutParams),
    CustomShader(CustomShaderParams),
}

fn default_effect_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_effect_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub payload: EffectPayload,
}

impl EffectInstance {
    pub fn kind(&self) -> EffectKind {
        match &self.payload {
            EffectPayload::ColorCorrection(_) => EffectKind::ColorCorrection,
            EffectPayload::GaussianBlur(_) => EffectKind::GaussianBlur,
            EffectPayload::Crop(_) => EffectKind::Crop,
            EffectPayload::Vignette(_) => EffectKind::Vignette,
            EffectPayload::ChromaKey(_) => EffectKind::ChromaKey,
            EffectPayload::Lut(_) => EffectKind::Lut,
            EffectPayload::CustomShader(_) => EffectKind::CustomShader,
        }
    }

    pub fn new_color_correction(id: u64, params: ColorCorrectionParams) -> Self {
        Self {
            id,
            name: "Color Correction".into(),
            enabled: true,
            payload: EffectPayload::ColorCorrection(params),
        }
    }

    pub fn new_gaussian_blur(id: u64, params: GaussianBlurParams) -> Self {
        Self {
            id,
            name: "Gaussian Blur".into(),
            enabled: true,
            payload: EffectPayload::GaussianBlur(params),
        }
    }

    pub fn new_crop(id: u64, params: CropParams) -> Self {
        Self {
            id,
            name: "Crop".into(),
            enabled: true,
            payload: EffectPayload::Crop(params),
        }
    }

    pub fn new_vignette(id: u64, params: VignetteParams) -> Self {
        Self {
            id,
            name: "Vignette".into(),
            enabled: true,
            payload: EffectPayload::Vignette(params),
        }
    }

    pub fn new_chroma_key(id: u64, params: ChromaKeyParams) -> Self {
        Self {
            id,
            name: "Chroma Key".into(),
            enabled: true,
            payload: EffectPayload::ChromaKey(params),
        }
    }

    pub fn new_lut(id: u64, params: LutParams) -> Self {
        Self {
            id,
            name: "3D LUT".into(),
            enabled: true,
            payload: EffectPayload::Lut(params),
        }
    }

    pub fn new_custom_shader(id: u64, params: CustomShaderParams) -> Self {
        Self {
            id,
            name: params.name.clone(),
            enabled: true,
            payload: EffectPayload::CustomShader(params),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CurvePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RgbCurves {
    #[serde(default = "default_curve_points")]
    pub master: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub red: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub green: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub blue: Vec<CurvePoint>,
}

pub fn default_curve_points() -> Vec<CurvePoint> {
    vec![
        CurvePoint { x: 0.0, y: 0.0 },
        CurvePoint { x: 1.0, y: 1.0 },
    ]
}

impl Default for RgbCurves {
    fn default() -> Self {
        Self {
            master: default_curve_points(),
            red: default_curve_points(),
            green: default_curve_points(),
            blue: default_curve_points(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HslQualifier {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_hue_center")]
    pub hue_center: f64,
    #[serde(default = "default_hue_width")]
    pub hue_width: f64,
    #[serde(default = "default_softness")]
    pub hue_softness: f64,
    #[serde(default = "default_sat_center")]
    pub sat_center: f64,
    #[serde(default = "default_sat_width")]
    pub sat_width: f64,
    #[serde(default = "default_softness")]
    pub sat_softness: f64,
    #[serde(default = "default_luma_center")]
    pub luma_center: f64,
    #[serde(default = "default_luma_width")]
    pub luma_width: f64,
    #[serde(default = "default_softness")]
    pub luma_softness: f64,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub blur_radius: f64,
    #[serde(default)]
    pub hue_shift: f64,
    #[serde(default = "default_one")]
    pub sat_boost: f64,
    #[serde(default)]
    pub exposure_shift: f64,
}

fn default_hue_center() -> f64 { 120.0 }
fn default_hue_width() -> f64 { 40.0 }
fn default_sat_center() -> f64 { 0.5 }
fn default_sat_width() -> f64 { 0.5 }
fn default_luma_center() -> f64 { 0.5 }
fn default_luma_width() -> f64 { 0.5 }
fn default_softness() -> f64 { 0.15 }
fn default_one() -> f64 { 1.0 }

impl Default for HslQualifier {
    fn default() -> Self {
        Self {
            enabled: false,
            hue_center: default_hue_center(),
            hue_width: default_hue_width(),
            hue_softness: default_softness(),
            sat_center: default_sat_center(),
            sat_width: default_sat_width(),
            sat_softness: default_softness(),
            luma_center: default_luma_center(),
            luma_width: default_luma_width(),
            luma_softness: default_softness(),
            invert: false,
            blur_radius: 0.0,
            hue_shift: 0.0,
            sat_boost: 1.0,
            exposure_shift: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ColorAdjustments {
    #[serde(default)]
    pub exposure: f64,
    #[serde(default = "default_contrast")]
    pub contrast: f64,
    #[serde(default = "default_saturation")]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
    #[serde(default)]
    pub wheels: ColorWheels,
    #[serde(default)]
    pub curves: Option<RgbCurves>,
    #[serde(default)]
    pub qualifier: Option<HslQualifier>,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
            wheels: ColorWheels::default(),
            curves: None,
            qualifier: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KeyframeProperty {
    PositionX,
    PositionY,
    Scale,
    Rotation,
    Opacity,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationKind {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bezier,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Keyframe {
    pub property: KeyframeProperty,
    /// Time relative to the clip's source in point.
    pub time: Tick,
    pub value: f64,
    #[serde(default)]
    pub interpolation: InterpolationKind,
    #[serde(default)]
    pub handle_in_x: Option<f64>,
    #[serde(default)]
    pub handle_in_y: Option<f64>,
    #[serde(default)]
    pub handle_out_x: Option<f64>,
    #[serde(default)]
    pub handle_out_y: Option<f64>,
}

impl Keyframe {
    pub fn new(property: KeyframeProperty, time: Tick, value: f64) -> Self {
        Self {
            property,
            time,
            value,
            interpolation: InterpolationKind::Linear,
            handle_in_x: None,
            handle_in_y: None,
            handle_out_x: None,
            handle_out_y: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectView {
    #[serde(default)]
    pub active_sequence: Option<String>,
    #[serde(default)]
    pub timeline_zoom: f64,
    #[serde(default)]
    pub timeline_scroll: f64,
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self::new("Untitled project")
    }
}

impl ProjectDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format: PROJECT_FORMAT.to_owned(),
            schema_version: PROJECT_SCHEMA_VERSION,
            generator: format!("Project Saturn {}", env!("CARGO_PKG_VERSION")),
            project: Project {
                name: name.into(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::FPS_30,
                    audio_sample_rate: 48_000,
                },
                media: Vec::new(),
                bins: Vec::new(),
                sequence: Sequence {
                    id: 1,
                    name: "Sequence 1".into(),
                    mark_in: None,
                    mark_out: None,
                    subtitles: Vec::new(),
                    multicam_clips: Vec::new(),
                    buses: default_audio_buses(),
                    node_graph: None,
                    track_groups: Vec::new(),
                    tracks: vec![
                        Track {
                            id: 1,
                            kind: TrackKind::Video,
                            name: "V1".into(),
                            group_id: None,
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            audio_processing: TrackAudioProcessing::default(),
                            bus: BusTarget::Master,
                            automation: TrackAutomation::default(),
                            plugins: Vec::new(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                        Track {
                            id: 2,
                            kind: TrackKind::Audio,
                            name: "A1".into(),
                            group_id: None,
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            audio_processing: TrackAudioProcessing::default(),
                            bus: BusTarget::Master,
                            automation: TrackAutomation::default(),
                            plugins: Vec::new(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                    ],
                },
                sequences: Vec::new(),
                next_id: 3,
                collaboration: CollaborationState::default(),
            },
            view: ProjectView {
                active_sequence: Some("Sequence 1".into()),
                timeline_zoom: 1.0,
                timeline_scroll: 0.0,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != PROJECT_FORMAT {
            return Err(format!("Unsupported project format `{}`", self.format));
        }
        if self.project.name.trim().is_empty() {
            return Err("Project name cannot be empty".into());
        }
        if self.project.settings.width == 0 || self.project.settings.height == 0 {
            return Err("Project dimensions must be greater than zero".into());
        }
        if self.project.settings.audio_sample_rate == 0
            || self.project.settings.frame_rate.frame_ticks().is_none()
        {
            return Err("Project uses an unsupported frame or audio rate".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.project.media {
            if !ids.insert(item.id) {
                return Err(format!("Duplicate media id {}", item.id));
            }
        }
        for track in &self.project.sequence.tracks {
            if !track.gain_db.is_finite() || !(-60.0..=12.0).contains(&track.gain_db) {
                return Err(format!("Track {} has invalid gain", track.name));
            }
            let ap = &track.audio_processing;
            if !ap.eq_low_gain.is_finite() || !(-24.0..=24.0).contains(&ap.eq_low_gain)
                || !ap.eq_mid_gain.is_finite() || !(-24.0..=24.0).contains(&ap.eq_mid_gain)
                || !ap.eq_high_gain.is_finite() || !(-24.0..=24.0).contains(&ap.eq_high_gain)
                || !ap.comp_threshold.is_finite() || !(-80.0..=0.0).contains(&ap.comp_threshold)
                || !ap.comp_ratio.is_finite() || !(1.0..=50.0).contains(&ap.comp_ratio)
            {
                return Err(format!("Track {} has invalid audio processing parameters", track.name));
            }
            for clip in &track.clips {
                if clip.duration.0 <= 0
                    || !clip.color.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&clip.color.exposure)
                    || !clip.color.contrast.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.contrast)
                    || !clip.color.saturation.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.saturation)
                    || !clip.color.hue.is_finite()
                    || !(-1.0..=1.0).contains(&clip.color.hue)
                {
                    return Err("Project contains invalid clip properties".into());
                }
                for effect in &clip.effects {
                    match &effect.payload {
                        EffectPayload::ColorCorrection(p) => {
                            if !p.exposure.is_finite()
                                || !(-5.0..=5.0).contains(&p.exposure)
                                || !p.contrast.is_finite()
                                || !(0.0..=2.0).contains(&p.contrast)
                                || !p.saturation.is_finite()
                                || !(0.0..=2.0).contains(&p.saturation)
                                || !p.hue.is_finite()
                                || !(-1.0..=1.0).contains(&p.hue)
                            {
                                return Err("Project contains invalid color correction effect values".into());
                            }
                        }
                        EffectPayload::GaussianBlur(p) => {
                            if !p.sigma.is_finite() || !(0.0..=50.0).contains(&p.sigma) {
                                return Err("Project contains invalid gaussian blur effect values".into());
                            }
                        }
                        EffectPayload::Crop(p) => {
                            if !p.top.is_finite() || !(0.0..=100.0).contains(&p.top)
                                || !p.bottom.is_finite() || !(0.0..=100.0).contains(&p.bottom)
                                || !p.left.is_finite() || !(0.0..=100.0).contains(&p.left)
                                || !p.right.is_finite() || !(0.0..=100.0).contains(&p.right)
                            {
                                return Err("Project contains invalid crop effect values".into());
                            }
                        }
                        EffectPayload::Vignette(p) => {
                            if !p.intensity.is_finite() || !(0.0..=1.0).contains(&p.intensity)
                                || !p.radius.is_finite() || !(0.0..=2.0).contains(&p.radius)
                                || !p.softness.is_finite() || !(0.0..=1.0).contains(&p.softness)
                            {
                                return Err("Project contains invalid vignette effect values".into());
                            }
                        }
                        EffectPayload::ChromaKey(p) => {
                            if p.color.trim().is_empty()
                                || !p.tolerance.is_finite() || !(0.0..=100.0).contains(&p.tolerance)
                                || !p.softness.is_finite() || !(0.0..=100.0).contains(&p.softness)
                                || !p.spill_suppression.is_finite() || !(0.0..=1.0).contains(&p.spill_suppression)
                            {
                                return Err("Project contains invalid chroma key effect values".into());
                            }
                        }
                        EffectPayload::Lut(p) => {
                            if !p.intensity.is_finite() || !(0.0..=1.0).contains(&p.intensity) {
                                return Err("Project contains invalid LUT effect intensity".into());
                            }
                        }
                        EffectPayload::CustomShader(p) => {
                            if p.shader_code.trim().is_empty() {
                                return Err("Project contains empty custom shader code".into());
                            }
                        }
                    }
                }
                for keyframe in &clip.keyframes {
                    let value_in_range = match &keyframe.property {
                        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
                            (-100_000.0..=100_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Scale => (0.01..=100.0).contains(&keyframe.value),
                        KeyframeProperty::Rotation => {
                            (-36_000.0..=36_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Opacity => (0.0..=1.0).contains(&keyframe.value),
                    };
                    if keyframe.time.0 < 0
                        || keyframe.time.0 > clip.duration.0
                        || !keyframe.value.is_finite()
                        || !value_in_range
                    {
                        return Err("Project contains an invalid keyframe".into());
                    }
                }
            }
            for transition in &track.transitions {
                if transition.duration.0 <= 0 || transition.cut_at.0 < 0 {
                    return Err("Project contains an invalid transition".into());
                }
            }
        }
        for sub in &self.project.sequence.subtitles {
            if sub.start_ticks.0 < 0 || sub.end_ticks.0 <= sub.start_ticks.0 || sub.text.trim().is_empty() {
                return Err("Project contains an invalid subtitle item".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v9_project_migrates_to_v10_schema() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V9 project")).unwrap();
        legacy["schema_version"] = 9.into();
        legacy["project"].as_object_mut().unwrap().remove("sequences");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert!(loaded.project.sequences.is_empty());
        assert_eq!(loaded.project.sequence.id, 1);
    }

    #[test]
    fn v1_project_migrates_to_current_schema() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("Legacy")).unwrap();
        legacy["schema_version"] = 1.into();
        legacy.as_object_mut().unwrap().remove("view");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.view.active_sequence.as_deref(), Some("Sequence 1"));
    }

    #[test]
    fn v8_project_migrates_to_v9_schema() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V8 project")).unwrap();
        legacy["schema_version"] = 8.into();
        legacy["project"].as_object_mut().unwrap().remove("collaboration");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.project.collaboration.session_id, "local-collab-session");
        assert!(loaded.project.sequence.node_graph.is_none());
    }

    #[test]
    fn v2_project_migrates_with_default_edit_properties() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V2 project")).unwrap();
        legacy["schema_version"] = 2.into();
        for track in legacy["project"]["sequence"]["tracks"]
            .as_array_mut()
            .unwrap()
        {
            track.as_object_mut().unwrap().remove("gain_db");
        }
        legacy["project"]["sequence"]["tracks"][0]["clips"] = serde_json::json!([
            {"media_id": 3, "timeline_start": 0, "source_in": 0, "duration": 100}
        ]);
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.project.sequence.tracks[0].gain_db, 0.0);
        let clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(clip.color.exposure, 0.0);
        assert_eq!(clip.color.contrast, 1.0);
        assert!(clip.keyframes.is_empty());
    }

    #[test]
    fn project_saves_atomically_and_keeps_external_media_paths() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("cut.saturn");
        let mut doc = ProjectDocument::new("Cut");
        doc.project.media.push(MediaAsset {
            id: 4,
            name: "shot.mov".into(),
            path: PathBuf::from("/media/shot.mov"),
            kind: MediaKind::Video,
            duration: Some(Tick(42)),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        });
        doc.save_atomic(&path).unwrap();
        let loaded = ProjectDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.media[0].path,
            PathBuf::from("/media/shot.mov")
        );
        assert_eq!(loaded.project.media[0].duration, Some(Tick(42)));
    }

    #[test]
    fn test_sequence_playback_queries() {
        let mut doc = ProjectDocument::new("Playback test");
        let video_track = &mut doc.project.sequence.tracks[0];
        video_track.clips.push(Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(100),
            duration: Tick(500),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
        });
        video_track.clips.push(Clip {
            media_id: 2,
            timeline_start: Tick(500),
            source_in: Tick(0),
            duration: Tick(400),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
        });
        video_track.transitions.push(Transition {
            id: 1,
            kind: TransitionKind::Crossfade,
            cut_at: Tick(500),
            duration: Tick(100),
        });

        let seq = &doc.project.sequence;
        assert_eq!(seq.duration(), Tick(900));

        let active_mid_first = seq.active_clips_at(Tick(250));
        assert_eq!(active_mid_first.len(), 1);
        assert_eq!(active_mid_first[0].1.media_id, 1);

        let active_at_cut = seq.active_clips_at(Tick(500));
        assert_eq!(active_at_cut.len(), 1);
        assert_eq!(active_at_cut[0].1.media_id, 2);

        let trans = seq.transition_at(Tick(500));
        assert!(trans.is_some());
        assert_eq!(trans.unwrap().1.kind, TransitionKind::Crossfade);

        let trans_boundary = seq.transition_at(Tick(460));
        assert!(trans_boundary.is_some());

        let trans_outside = seq.transition_at(Tick(400));
        assert!(trans_outside.is_none());

        assert_eq!(seq.tracks[0].next_clip_after(Tick(200)).unwrap().media_id, 2);
    }

    #[test]
    fn test_effect_stack_serialization_and_migration() {
        let mut doc = ProjectDocument::new("Effect test");
        let video_track = &mut doc.project.sequence.tracks[0];
        let mut clip = Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(0),
            duration: Tick(300),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
        };

        clip.effects.push(EffectInstance::new_color_correction(
            10,
            ColorCorrectionParams {
                exposure: 0.5,
                contrast: 1.1,
                saturation: 1.2,
                hue: 0.05,
                wheels: ColorWheels::default(),
            },
        ));
        clip.effects.push(EffectInstance::new_gaussian_blur(
            11,
            GaussianBlurParams { sigma: 4.5 },
        ));
        clip.effects.push(EffectInstance::new_crop(
            12,
            CropParams {
                top: 5.0,
                bottom: 10.0,
                left: 2.0,
                right: 2.0,
            },
        ));
        clip.effects.push(EffectInstance::new_vignette(
            13,
            VignetteParams {
                intensity: 0.7,
                radius: 0.9,
                softness: 0.6,
            },
        ));
        clip.effects.push(EffectInstance::new_chroma_key(
            14,
            ChromaKeyParams {
                color: "#00ff00".into(),
                tolerance: 35.0,
                softness: 12.0,
                spill_suppression: 0.75,
            },
        ));

        video_track.clips.push(clip);
        assert!(doc.validate().is_ok());

        // Serialize and deserialize roundtrip
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let loaded: ProjectDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        let loaded_clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(loaded_clip.effects.len(), 5);
        assert_eq!(loaded_clip.effects[0].kind(), EffectKind::ColorCorrection);
        assert_eq!(loaded_clip.effects[1].kind(), EffectKind::GaussianBlur);
        assert_eq!(loaded_clip.effects[2].kind(), EffectKind::Crop);
        assert_eq!(loaded_clip.effects[3].kind(), EffectKind::Vignette);
        assert_eq!(loaded_clip.effects[4].kind(), EffectKind::ChromaKey);
    }

    #[test]
    fn test_track_audio_properties_and_audibility() {
        let mut t1 = Track {
            id: 1,
            group_id: None,
            kind: TrackKind::Audio,
            name: "A1".into(),
            gain_db: -3.0,
            pan: -0.5,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: TrackAudioProcessing::default(),
            bus: BusTarget::Master,
            automation: TrackAutomation::default(),
            plugins: Vec::new(),
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        let mut t2 = Track {
            id: 2,
            group_id: None,
            kind: TrackKind::Audio,
            name: "A2".into(),
            gain_db: 0.0,
            pan: 0.5,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: TrackAudioProcessing::default(),
            bus: BusTarget::Master,
            automation: TrackAutomation::default(),
            plugins: Vec::new(),
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        // Neither soloed, neither muted -> both audible
        assert!(t1.is_audible(false));
        assert!(t2.is_audible(false));

        // Mute t1
        t1.muted = true;
        assert!(!t1.is_audible(false));
        assert!(t2.is_audible(false));
        t1.muted = false;

        // Solo t2 -> any_solo is true, t1 not soloed -> t1 inaudible, t2 audible
        t2.solo = true;
        assert!(!t1.is_audible(true));
        assert!(t2.is_audible(true));

        // Both soloed -> both audible
        t1.solo = true;
        assert!(t1.is_audible(true));
        assert!(t2.is_audible(true));
    }
}
