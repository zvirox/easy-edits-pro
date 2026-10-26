//! Parameters for the timeline targeting family.

use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct SetTargetingParams {
    /// 1-based video track ordinals (V1 = first video track).
    #[serde(default)]
    pub(in crate::engine) video: Option<Vec<u64>>,
    /// 1-based audio track ordinals (A1 = first audio track).
    #[serde(default)]
    pub(in crate::engine) audio: Option<Vec<u64>>,
}
