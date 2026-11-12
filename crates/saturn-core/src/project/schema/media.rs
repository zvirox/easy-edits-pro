// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub format: String,
    pub schema_version: u32,
    pub generator: String,
    pub project: Project,
    #[serde(default)]
    pub view: ProjectView,
    /// Copy-on-write timeline branches (schema v15). The live edit state is
    /// not a branch record; each record holds an independent snapshot.
    #[serde(default)]
    pub branches: Vec<crate::branch::TimelineBranch>,
    /// Durable actor-owned clip and region locks (schema v17, AF2).
    #[serde(default)]
    pub locks: crate::lock::LockRegistry,
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
    #[serde(default)]
    pub notes: String,
    /// Human-owned project brief (AF4).
    #[serde(default)]
    pub brief: crate::brief::ProjectBrief,
    /// Task sheet work order (TS1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_sheet: Option<crate::tasksheet::TaskSheet>,
    pub settings: ProjectSettings,
    /// Scene-referred colour-management configuration (Packet 3.1).
    #[serde(default)]
    pub color_management: ColorManagement,
    pub media: Vec<MediaAsset>,
    #[serde(default)]
    pub bins: Vec<MediaBin>,
    pub sequence: Sequence,
    #[serde(default)]
    pub sequences: Vec<Sequence>,
    #[serde(default)]
    pub graphic_templates: Vec<crate::project::GraphicTemplate>,
    pub next_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
    /// Playback decodes proxy copies where an asset carries one.
    #[serde(default)]
    pub use_proxies: bool,
}

/// Scene-referred colour-management configuration (`ocio.set_pipeline`,
/// Packet 3.1). Working space and display transform apply project-wide; the
/// default IDT seeds newly imported assets without an explicit one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColorManagement {
    pub working_space: crate::color::WorkingSpace,
    pub display_odt: crate::color::DisplayOdt,
    #[serde(default)]
    pub default_idt: Option<crate::color::CameraIdt>,
}

impl Default for ColorManagement {
    fn default() -> Self {
        Self {
            working_space: crate::color::WorkingSpace::default(),
            display_odt: crate::color::DisplayOdt::default(),
            default_idt: None,
        }
    }
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

fn default_title_style() -> String {
    "lower_third".into()
}
fn default_font_family() -> String {
    "Inter".into()
}
fn default_font_size() -> f64 {
    42.0
}
fn default_title_color() -> String {
    "#ffffff".into()
}

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
    /// Free-form production notes. Metadata is descriptive only: it never
    /// affects playback or rendering.
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub take: String,
    #[serde(default)]
    pub is_missing: bool,
    /// Source-monitor marks set through `project.set_marks`; the default
    /// range offered by the panel's send-to-timeline actions.
    #[serde(default)]
    pub source_mark_in: Option<Tick>,
    #[serde(default)]
    pub source_mark_out: Option<Tick>,
    /// Lower-resolution viewing copy; playback decodes this when the
    /// project's `use_proxies` flag is on.
    #[serde(default)]
    pub proxy_path: Option<PathBuf>,
    /// Colour metadata the shell records for the asset (e.g. "rec709").
    #[serde(default)]
    pub colorspace: Option<String>,
    #[serde(default)]
    pub gamma: Option<String>,
    /// Camera Input Device Transform for this asset (`ocio.set_media_idt`,
    /// Packet 3.1); falls back to the project's `default_idt`.
    #[serde(default)]
    pub idt: Option<crate::color::CameraIdt>,
    /// Disk stamp recorded at import and by `media.refresh_changed`.
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub modified_ms: Option<u64>,
    #[serde(default)]
    pub title: Option<TitleProperties>,
    #[serde(default)]
    pub waveform: Option<Vec<u8>>,
    /// BLAKE3 fingerprint of the referenced file (head slice, tail slice,
    /// size; schema v15). A relink-suggestion key over moved files, **not** a
    /// content hash: nothing may relink silently on a fingerprint match, the
    /// suggestion always requires explicit confirmation through
    /// `project.relink_media`.
    #[serde(default)]
    pub fingerprint: Option<crate::media_fingerprint::MediaFingerprint>,
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
            comment: String::new(),
            scene: String::new(),
            take: String::new(),
            is_missing: false,
            source_mark_in: None,
            source_mark_out: None,
            proxy_path: None,
            colorspace: None,
            gamma: None,
            idt: None,
            file_size: None,
            modified_ms: None,
            title: None,
            waveform: None,
            fingerprint: None,
            nested_sequence: None,
        }
    }
}
