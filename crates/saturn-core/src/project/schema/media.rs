use super::*;

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
