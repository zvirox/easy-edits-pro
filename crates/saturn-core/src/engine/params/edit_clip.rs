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
