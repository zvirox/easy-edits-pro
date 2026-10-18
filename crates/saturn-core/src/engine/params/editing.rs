use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackAudioProcessingParams {
    pub(in crate::engine) track_id: u64,
    #[serde(default)]
    pub(in crate::engine) eq_enabled: bool,
    #[serde(default)]
    pub(in crate::engine) eq_low_gain: f64,
    #[serde(default)]
    pub(in crate::engine) eq_mid_gain: f64,
    #[serde(default)]
    pub(in crate::engine) eq_high_gain: f64,
    #[serde(default)]
    pub(in crate::engine) comp_enabled: bool,
    #[serde(default = "default_threshold")]
    pub(in crate::engine) comp_threshold: f64,
    #[serde(default = "default_ratio")]
    pub(in crate::engine) comp_ratio: f64,
    #[serde(default = "default_attack")]
    pub(in crate::engine) comp_attack: f64,
    #[serde(default = "default_release")]
    pub(in crate::engine) comp_release: f64,
    #[serde(default)]
    pub(in crate::engine) denoise_enabled: bool,
    #[serde(default = "default_denoise_amount")]
    pub(in crate::engine) denoise_amount: f64,
}

pub(in crate::engine) fn default_threshold() -> f64 {
    -24.0
}
pub(in crate::engine) fn default_ratio() -> f64 {
    4.0
}
pub(in crate::engine) fn default_attack() -> f64 {
    0.005
}
pub(in crate::engine) fn default_release() -> f64 {
    0.1
}
pub(in crate::engine) fn default_denoise_amount() -> f64 {
    0.5
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackGainParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) gain_db: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackPanParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) pan: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackMutedParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) muted: bool,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackSoloParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) solo: bool,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackLockedParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) locked: bool,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetKeyframeParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) property: KeyframeProperty,
    pub(in crate::engine) time: Tick,
    pub(in crate::engine) value: f64,
    #[serde(default)]
    pub(in crate::engine) interpolation: Option<InterpolationKind>,
    #[serde(default)]
    pub(in crate::engine) handle_in_x: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) handle_in_y: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) handle_out_x: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) handle_out_y: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddSubtitleParams {
    pub(in crate::engine) start_ticks: Tick,
    pub(in crate::engine) end_ticks: Tick,
    pub(in crate::engine) text: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveSubtitleParams {
    pub(in crate::engine) id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateSubtitleParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) start_ticks: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) end_ticks: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) text: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveKeyframeParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) property: KeyframeProperty,
    pub(in crate::engine) time: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddEffectParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) kind: EffectKind,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveEffectParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) effect_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateEffectParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) effect_id: u64,
    pub(in crate::engine) payload: EffectPayload,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ToggleEffectParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) effect_id: u64,
    #[serde(default)]
    pub(in crate::engine) enabled: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ReorderEffectsParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) from_index: usize,
    pub(in crate::engine) to_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetColorWheelsParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) wheels: ColorWheels,
}

#[derive(Deserialize)]
pub(in crate::engine) struct CreateMulticamParams {
    pub(in crate::engine) name: String,
    pub(in crate::engine) angles: Vec<MultiCamAngle>,
    #[serde(default)]
    pub(in crate::engine) sync_mode: MultiCamSyncMode,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SwitchMulticamAngleParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) angle_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SyncMulticamParams {
    pub(in crate::engine) multicam_id: u64,
    pub(in crate::engine) sync_mode: MultiCamSyncMode,
    #[serde(default)]
    pub(in crate::engine) offsets: Option<std::collections::HashMap<u64, i64>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetMotionTrackingParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) tracking: MotionTrackingData,
}

#[derive(Deserialize)]
pub(in crate::engine) struct PinToTrackParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) pinned_clip_id: Option<u64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackBusRouteParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) bus: BusTarget,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackAutomationParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) automation: TrackAutomation,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddAudioPluginParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) plugin: AudioPluginInstance,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ToggleAudioPluginParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) plugin_id: u64,
    pub(in crate::engine) enabled: bool,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetBusGainParams {
    pub(in crate::engine) target: BusTarget,
    pub(in crate::engine) gain_db: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddNodeParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) kind: CompositingNodeKind,
    #[serde(default = "default_node_title")]
    pub(in crate::engine) title: String,
    #[serde(default)]
    pub(in crate::engine) pos_x: f64,
    #[serde(default)]
    pub(in crate::engine) pos_y: f64,
    #[serde(default)]
    pub(in crate::engine) params: Value,
}

pub(in crate::engine) fn default_node_title() -> String {
    "New Node".into()
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveNodeParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) node_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ConnectNodesParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) from_node: u64,
    pub(in crate::engine) from_socket: String,
    pub(in crate::engine) to_node: u64,
    pub(in crate::engine) to_socket: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct DisconnectNodesParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) from_node: u64,
    pub(in crate::engine) from_socket: String,
    pub(in crate::engine) to_node: u64,
    pub(in crate::engine) to_socket: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateNodeParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) node_id: u64,
    pub(in crate::engine) pos_x: Option<f64>,
    pub(in crate::engine) pos_y: Option<f64>,
    pub(in crate::engine) enabled: Option<bool>,
    pub(in crate::engine) params: Option<Value>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddMaskParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) clip_index: Option<usize>,
    pub(in crate::engine) mask: RotoSplineMask,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetSpeedRampParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) speed_ramp: SpeedRamp,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetMotionBlurParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) motion_blur: MotionBlurParams,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AcquireLockParams {
    pub(in crate::engine) user_id: String,
    pub(in crate::engine) user_name: String,
    pub(in crate::engine) user_color: Option<String>,
    pub(in crate::engine) target_kind: LockTargetKind,
    pub(in crate::engine) target_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ReleaseLockParams {
    pub(in crate::engine) user_id: String,
    pub(in crate::engine) target_kind: LockTargetKind,
    pub(in crate::engine) target_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RecordChangeParams {
    pub(in crate::engine) user_id: String,
    pub(in crate::engine) user_name: String,
    pub(in crate::engine) action: String,
    pub(in crate::engine) summary: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct TranscribeToSubtitlesParams {
    pub(in crate::engine) segments: Vec<crate::speech::SpeechSegment>,
    pub(in crate::engine) replace_existing: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleDeleteSilenceParams {
    pub(in crate::engine) peaks: Vec<u8>,
    pub(in crate::engine) options: Option<crate::speech::SilenceDetectOptions>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleDeleteFillersParams {
    pub(in crate::engine) segments: Vec<crate::speech::SpeechSegment>,
    pub(in crate::engine) custom_fillers: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleDeleteRangesParams {
    pub(in crate::engine) ranges: Vec<(Tick, Tick)>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ImportEdlParams {
    pub(in crate::engine) edl: String,
    pub(in crate::engine) project_name: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ImportXmlParams {
    pub(in crate::engine) xml: String,
}

#[derive(Clone, Copy, Deserialize)]
pub(in crate::engine) struct SelectedClipRef {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct CreateCompoundClipParams {
    #[serde(default = "default_compound_name")]
    pub(in crate::engine) name: String,
    pub(in crate::engine) target_track_id: Option<u64>,
    pub(in crate::engine) clips: Vec<SelectedClipRef>,
}

pub(in crate::engine) fn default_compound_name() -> String {
    "Compound Clip".into()
}

#[derive(Deserialize)]
pub(in crate::engine) struct DecomposeCompoundClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SwitchSequenceParams {
    pub(in crate::engine) sequence_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct CreateBinParams {
    pub(in crate::engine) name: String,
    pub(in crate::engine) parent_id: Option<u64>,
    pub(in crate::engine) color: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RenameBinParams {
    pub(in crate::engine) bin_id: u64,
    pub(in crate::engine) name: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct DeleteBinParams {
    pub(in crate::engine) bin_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MoveToBinParams {
    pub(in crate::engine) media_id: u64,
    pub(in crate::engine) bin_id: Option<u64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateMediaPropertiesParams {
    pub(in crate::engine) media_id: u64,
    pub(in crate::engine) name: Option<String>,
    pub(in crate::engine) color_label: Option<String>,
    pub(in crate::engine) tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddTrackParams {
    #[serde(default)]
    pub(in crate::engine) kind: TrackKind,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveTrackParams {
    pub(in crate::engine) track_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleDeleteGapParams {
    pub(in crate::engine) track_id: Option<u64>,
    pub(in crate::engine) at_tick: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct CreateTrackGroupParams {
    pub(in crate::engine) name: String,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AssignTrackGroupParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) group_id: Option<u64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ToggleTrackGroupCollapsedParams {
    pub(in crate::engine) group_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct DeleteTrackGroupParams {
    pub(in crate::engine) group_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetTrackGroupPropertiesParams {
    pub(in crate::engine) group_id: u64,
    pub(in crate::engine) name: Option<String>,
    pub(in crate::engine) color: Option<String>,
    pub(in crate::engine) muted: Option<bool>,
    pub(in crate::engine) solo: Option<bool>,
    pub(in crate::engine) locked: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateRgbCurvesParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) curves: crate::project::RgbCurves,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateHslQualifierParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) qualifier: crate::project::HslQualifier,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddCustomShaderParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) shader_code: Option<String>,
    #[serde(default)]
    pub(in crate::engine) preset_id: Option<String>,
    #[serde(default)]
    pub(in crate::engine) uniforms: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateCustomShaderParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) effect_id: u64,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) shader_code: Option<String>,
    #[serde(default)]
    pub(in crate::engine) preset_id: Option<String>,
    #[serde(default)]
    pub(in crate::engine) uniforms: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ImportOtioParams {
    pub(in crate::engine) content: String,
    #[serde(default)]
    pub(in crate::engine) project_name: Option<String>,
}
