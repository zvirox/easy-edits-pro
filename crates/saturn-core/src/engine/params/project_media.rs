use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct NewProjectParams {
    #[serde(default = "default_project_name")]
    pub(in crate::engine) name: String,
    #[serde(default = "default_sequence_name")]
    pub(in crate::engine) sequence_name: String,
    #[serde(default = "default_project_width")]
    pub(in crate::engine) width: u32,
    #[serde(default = "default_project_height")]
    pub(in crate::engine) height: u32,
    #[serde(default = "default_project_frame_rate")]
    pub(in crate::engine) frame_rate: FrameRate,
}
pub(in crate::engine) fn default_project_name() -> String {
    "Untitled project".into()
}
pub(in crate::engine) fn default_sequence_name() -> String {
    "Sequence 1".into()
}
pub(in crate::engine) fn default_project_width() -> u32 {
    1920
}
pub(in crate::engine) fn default_project_height() -> u32 {
    1080
}
pub(in crate::engine) fn default_project_frame_rate() -> FrameRate {
    FrameRate::FPS_30
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddMediaParams {
    pub(in crate::engine) name: String,
    pub(in crate::engine) path: PathBuf,
    #[serde(default)]
    pub(in crate::engine) kind: crate::project::MediaKind,
    #[serde(default)]
    pub(in crate::engine) duration: Option<crate::time::Tick>,
    #[serde(default)]
    pub(in crate::engine) width: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) height: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) fps: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) audio_channels: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) codec: Option<String>,
    #[serde(default)]
    pub(in crate::engine) bin_id: Option<u64>,
    #[serde(default)]
    pub(in crate::engine) color_label: Option<String>,
    #[serde(default)]
    pub(in crate::engine) tags: Vec<String>,
    #[serde(default)]
    pub(in crate::engine) title: Option<TitleProperties>,
    #[serde(default)]
    pub(in crate::engine) waveform: Option<Vec<u8>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateMediaParams {
    pub(in crate::engine) path: PathBuf,
    #[serde(default)]
    pub(in crate::engine) kind: crate::project::MediaKind,
    #[serde(default)]
    pub(in crate::engine) duration: Option<crate::time::Tick>,
    #[serde(default)]
    pub(in crate::engine) width: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) height: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) waveform: Option<Vec<u8>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddTransitionParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) cut_at: Tick,
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) kind: Option<TransitionKind>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveTransitionParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) transition_id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddTitleParams {
    pub(in crate::engine) text: String,
    #[serde(default)]
    pub(in crate::engine) subtitle: Option<String>,
    #[serde(default)]
    pub(in crate::engine) style: Option<String>,
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) track_id: Option<u64>,
    #[serde(default)]
    pub(in crate::engine) timeline_start: Option<Tick>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateTitleParams {
    pub(in crate::engine) media_id: u64,
    #[serde(default)]
    pub(in crate::engine) text: Option<String>,
    #[serde(default)]
    pub(in crate::engine) subtitle: Option<String>,
    #[serde(default)]
    pub(in crate::engine) style: Option<String>,
    #[serde(default)]
    pub(in crate::engine) font_size: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
    #[serde(default)]
    pub(in crate::engine) bg_color: Option<String>,
}


#[derive(Deserialize)]
pub(in crate::engine) struct RelinkMediaParams {
    pub(in crate::engine) media_id: u64,
    pub(in crate::engine) path: PathBuf,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FilePathParams {
    pub(in crate::engine) path: PathBuf,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddClipParams {
    pub(in crate::engine) media_id: u64,
    #[serde(default)]
    pub(in crate::engine) track_id: Option<u64>,
    #[serde(default)]
    pub(in crate::engine) timeline_start: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) source_in: Tick,
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ClipReferenceParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RemoveClipsParams {
    pub(in crate::engine) clips: Vec<ClipReferenceParams>,
    #[serde(default)]
    pub(in crate::engine) ripple: bool,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MoveClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) target_track_id: Option<u64>,
    pub(in crate::engine) timeline_start: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SplitClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) at: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SlipClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) delta: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RollEditParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) left_clip_index: usize,
    pub(in crate::engine) delta: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SlideClipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) delta: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RateStretchParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) new_duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) speed: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct LinkClipsParams {
    pub(in crate::engine) clips: Vec<ClipReferenceParams>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UnlinkClipsParams {
    pub(in crate::engine) clips: Vec<ClipReferenceParams>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MoveClipBatchItem {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    #[allow(dead_code)]
    pub(in crate::engine) target_track_id: Option<u64>,
    pub(in crate::engine) timeline_start: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MoveClipsBatchParams {
    pub(in crate::engine) moves: Vec<MoveClipBatchItem>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetMarkParams {
    pub(in crate::engine) at: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SetColorParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) exposure: f64,
    pub(in crate::engine) contrast: f64,
    pub(in crate::engine) saturation: f64,
    #[serde(default)]
    pub(in crate::engine) hue: f64,
}

