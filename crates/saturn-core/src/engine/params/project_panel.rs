//! Parameters for the project-panel family (views, sort, columns,
//! view presets, notes, source marks).

use super::*;
use serde::Serialize;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::engine) struct PanelColumnSpec {
    pub(in crate::engine) id: String,
    #[serde(default)]
    pub(in crate::engine) width: Option<f64>,
    #[serde(default)]
    pub(in crate::engine) visible: Option<bool>,
}

#[derive(Clone, Deserialize)]
pub(in crate::engine) struct PanelSortSpec {
    pub(in crate::engine) key: String,
    #[serde(default = "default_sort_dir")]
    pub(in crate::engine) dir: String,
}

pub(in crate::engine) fn default_sort_dir() -> String {
    "asc".into()
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectViewSetParams {
    #[serde(default)]
    pub(in crate::engine) view_mode: Option<String>,
    #[serde(default)]
    pub(in crate::engine) sort: Option<PanelSortSpec>,
    #[serde(default)]
    pub(in crate::engine) columns: Option<Vec<PanelColumnSpec>>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectSortParams {
    pub(in crate::engine) key: String,
    #[serde(default)]
    pub(in crate::engine) dir: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct PanelColumnResizeParams {
    pub(in crate::engine) id: String,
    pub(in crate::engine) width: f64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectNotesParams {
    #[serde(default)]
    pub(in crate::engine) text: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectMarksParams {
    pub(in crate::engine) media_id: u64,
    #[serde(default)]
    pub(in crate::engine) source_in: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) source_out: Option<Tick>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectPresetSaveParams {
    /// Defaults to the currently active preset (Save); required for Save As.
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectPresetRestoreParams {
    pub(in crate::engine) name: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct ProjectPresetRenameParams {
    pub(in crate::engine) from: String,
    pub(in crate::engine) to: String,
}
