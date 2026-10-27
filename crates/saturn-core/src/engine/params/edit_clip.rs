//! Parameters for the edit-family clip commands (labels, attributes,
//! duplicate, consolidate, ripple-delete-head).

use super::*;

#[derive(Clone, Copy, Deserialize)]
pub(in crate::engine) struct EditClipTargetParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}

#[derive(Deserialize)]
pub(in crate::engine) struct EditLabelTargetsParams {
    pub(in crate::engine) clips: Vec<SelectedClipRef>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleDeleteHeadParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    /// How many frames of head to remove; defaults to one frame.
    #[serde(default = "default_ripple_head_frames")]
    pub(in crate::engine) frames: i64,
}

pub(in crate::engine) fn default_ripple_head_frames() -> i64 {
    1
}

#[derive(Clone, Copy, Deserialize)]
pub(in crate::engine) struct SetClipVolumeParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) volume_db: f64,
}

/// FC Enable: `enabled` omitted means toggle the current state.
#[derive(Clone, Copy, Deserialize)]
pub(in crate::engine) struct SetClipEnabledParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) enabled: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MakeSubclipParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::engine) enum FrameFitMode {
    Fill,
    Fit,
    Scale100,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FrameFitParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) mode: FrameFitMode,
}

#[derive(Clone, Copy, Deserialize)]
pub(in crate::engine) struct ReplaceMediaParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) media_id: u64,
}
