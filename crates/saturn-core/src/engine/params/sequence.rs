//! Parameters for the sequence-family commands added with the Tier-1
//! sequence batch (settings, split-everywhere, nested reveal).

use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct SequenceSettingsParams {
    #[serde(default)]
    pub(in crate::engine) width: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) height: Option<u32>,
    #[serde(default)]
    pub(in crate::engine) frame_rate: Option<crate::time::FrameRate>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct SequenceAddEditAllTracksParams {
    pub(in crate::engine) at: crate::time::Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RevealNestedSequenceParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
}
