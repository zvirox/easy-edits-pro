//! Parameters for the project freeform family.

use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformLayoutParams {
    /// `true` = freeform mode, `false` = list mode; omitted flips the
    /// current mode.
    #[serde(default)]
    pub(in crate::engine) enabled: Option<bool>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformMoveParams {
    pub(in crate::engine) media_ids: Vec<u64>,
    pub(in crate::engine) dx: f64,
    pub(in crate::engine) dy: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformResizeParams {
    pub(in crate::engine) media_ids: Vec<u64>,
    pub(in crate::engine) scale: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformStackParams {
    pub(in crate::engine) media_ids: Vec<u64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformOptionsParams {
    /// Grid size in pixels; omitted reads the current value.
    #[serde(default)]
    pub(in crate::engine) grid: Option<f64>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FreeformArrangementParams {
    pub(in crate::engine) name: String,
}
