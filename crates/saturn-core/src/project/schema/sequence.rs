use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    Crossfade,
    DipToBlack,
    DipToWhite,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transition {
    pub id: u64,
    pub kind: TransitionKind,
    pub cut_at: Tick,
    pub duration: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SubtitleItem {
    pub id: u64,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub text: String,
    /// Speaker label for the cue (Phase 5 captions: essential-sound style tags).
    #[serde(default)]
    pub speaker: Option<String>,
    /// Reference to a `CaptionStyle` on the owning track; `None` = track default.
    #[serde(default)]
    pub style_id: Option<u64>,
}

/// A reusable caption look (Phase 5 captions: style presets). Colours are data
/// carried in the project file, not CSS tokens.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptionStyle {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_caption_font_family")]
    pub font_family: String,
    #[serde(default = "default_caption_font_size")]
    pub font_size: f32,
    #[serde(default = "default_caption_fill")]
    pub fill_color: String,
    #[serde(default)]
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub background_color: Option<String>,
}

fn default_caption_font_family() -> String {
    "Inter".into()
}

fn default_caption_font_size() -> f32 {
    42.0
}

fn default_caption_fill() -> String {
    "#ffffff".into()
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Default".into(),
            font_family: default_caption_font_family(),
            font_size: default_caption_font_size(),
            fill_color: default_caption_fill(),
            stroke_color: None,
            stroke_width: 0.0,
            background_color: None,
        }
    }
}

/// One caption/subtitle track on a sequence (Phase 5). Cues are sorted by
/// `start_ticks`; styles are per-track presets the cues may reference.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptionTrack {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_caption_language")]
    pub language: String,
    #[serde(default)]
    pub cues: Vec<SubtitleItem>,
    #[serde(default)]
    pub styles: Vec<CaptionStyle>,
}

fn default_caption_language() -> String {
    "en".into()
}

/// Kind of a timeline graphic (Phase 5). `Adjustment` carries no ink of its
/// own; it hosts effects over the layers beneath it once the compositor
/// consumes graphics.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphicKind {
    #[default]
    Text,
    Rectangle,
    Ellipse,
    Polygon,
    Adjustment,
}

/// Position, size, and affine transform in project pixels. The origin is the
/// top-left of the project frame; `anchor` is the normalized pivot (0,0 =
/// top-left, 0.5,0.5 = center) used for rotation and scaling.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicTransform {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default = "default_graphic_size")]
    pub width: f64,
    #[serde(default = "default_graphic_size")]
    pub height: f64,
    #[serde(default = "default_graphic_scale")]
    pub scale: f64,
    /// Rotation in degrees, clockwise.
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "default_graphic_anchor")]
    pub anchor_x: f64,
    #[serde(default = "default_graphic_anchor")]
    pub anchor_y: f64,
    #[serde(default = "default_graphic_opacity")]
    pub opacity: f64,
}

fn default_graphic_size() -> f64 {
    400.0
}

fn default_graphic_scale() -> f64 {
    1.0
}

fn default_graphic_anchor() -> f64 {
    0.5
}

fn default_graphic_opacity() -> f64 {
    1.0
}

impl Default for GraphicTransform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: default_graphic_size(),
            height: default_graphic_size(),
            scale: default_graphic_scale(),
            rotation: 0.0,
            anchor_x: default_graphic_anchor(),
            anchor_y: default_graphic_anchor(),
            opacity: default_graphic_opacity(),
        }
    }
}

/// Solid or gradient ink. Gradient stops deliberately deferred to the colour
/// slice so the shape stays forward-compatible (an empty `gradient` list
/// means solid).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicFill {
    #[serde(default = "default_graphic_fill_color")]
    pub color: String,
    #[serde(default = "default_graphic_fill_opacity")]
    pub opacity: f64,
}

fn default_graphic_fill_color() -> String {
    "#ffffff".into()
}

fn default_graphic_fill_opacity() -> f64 {
    1.0
}

impl Default for GraphicFill {
    fn default() -> Self {
        Self {
            color: default_graphic_fill_color(),
            opacity: default_graphic_fill_opacity(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicStroke {
    #[serde(default = "default_graphic_fill_color")]
    pub color: String,
    /// Stroke weight in project pixels; 0 disables the stroke.
    #[serde(default)]
    pub width: f64,
}

impl Default for GraphicStroke {
    fn default() -> Self {
        Self {
            color: default_graphic_fill_color(),
            width: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicShadow {
    #[serde(default = "default_graphic_shadow_color")]
    pub color: String,
    #[serde(default)]
    pub blur: f64,
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
}

fn default_graphic_shadow_color() -> String {
    "#000000".into()
}

impl Default for GraphicShadow {
    fn default() -> Self {
        Self {
            color: default_graphic_shadow_color(),
            blur: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Text payload for [`GraphicKind::Text`]. Geometry lives in
/// [`GraphicTransform`]; everything glyph-related lives here.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicText {
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_title_font_family")]
    pub font_family: String,
    #[serde(default = "default_graphic_font_size")]
    pub font_size: f64,
    /// Letter spacing in project pixels.
    #[serde(default)]
    pub tracking: f64,
    /// Line height multiplier (1.0 = single).
    #[serde(default = "default_graphic_leading")]
    pub leading: f64,
    #[serde(default)]
    pub align: String,
    /// Vertical writing mode (tategaki); honed with CJK fixtures in a
    /// follow-up slice.
    #[serde(default)]
    pub vertical: bool,
}

fn default_title_font_family() -> String {
    "Inter".into()
}

fn default_graphic_font_size() -> f64 {
    42.0
}

fn default_graphic_leading() -> f64 {
    1.2
}

impl Default for GraphicText {
    fn default() -> Self {
        Self {
            content: String::new(),
            font_family: default_title_font_family(),
            font_size: default_graphic_font_size(),
            tracking: 0.0,
            leading: default_graphic_leading(),
            align: "left".into(),
            vertical: false,
        }
    }
}

/// A saved graphic look project-wide (Phase 5 templates). The body keeps the
/// full `Graphic` minus identity: applying assigns a fresh id and start.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphicTemplate {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub body: Graphic,
}

/// One editable graphic on the timeline (Phase 5: Essential Graphics). Items
/// float over video tracks by layer order and render through the wgpu
/// compositor once graphics support lands there.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Graphic {
    pub id: u64,
    #[serde(default = "default_graphic_name")]
    pub name: String,
    #[serde(default)]
    pub kind: GraphicKind,
    pub start_ticks: Tick,
    #[serde(default = "default_graphic_duration")]
    pub duration_ticks: Tick,
    /// Stack order over video; higher renders on top.
    #[serde(default)]
    pub layer: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub transform: GraphicTransform,
    #[serde(default)]
    pub fill: Option<GraphicFill>,
    #[serde(default)]
    pub stroke: Option<GraphicStroke>,
    #[serde(default)]
    pub shadow: Option<GraphicShadow>,
    #[serde(default)]
    pub text: Option<GraphicText>,
    /// Polygon points relative to the transform box, normalized 0..1.
    #[serde(default)]
    pub points: Option<Vec<[f64; 2]>>,
}

fn default_graphic_name() -> String {
    "Graphic".into()
}

fn default_graphic_duration() -> Tick {
    Tick(5 * crate::time::TICKS_PER_SECOND)
}

impl Default for Graphic {
    fn default() -> Self {
        Self {
            id: 0,
            name: default_graphic_name(),
            kind: GraphicKind::default(),
            start_ticks: Tick(0),
            duration_ticks: default_graphic_duration(),
            layer: 0,
            visible: true,
            locked: false,
            transform: GraphicTransform::default(),
            fill: Some(GraphicFill::default()),
            stroke: None,
            shadow: None,
            text: Some(GraphicText::default()),
            points: None,
        }
    }
}

/// Semantic kind of a timeline marker. Range markers use `duration_ticks > 0`
/// regardless of kind; point markers use `duration_ticks == 0`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarkerKind {
    #[default]
    Comment,
    Chapter,
    FlashCue,
    WebLink,
    Segmentation,
}

impl MarkerKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Comment => "Comment",
            Self::Chapter => "Chapter",
            Self::FlashCue => "Flash Cue",
            Self::WebLink => "Web Link",
            Self::Segmentation => "Segmentation",
        }
    }

    /// Stable lowercase id used by the CSV format, the CLI, and the UI.
    pub fn id(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Chapter => "chapter",
            Self::FlashCue => "flash_cue",
            Self::WebLink => "web_link",
            Self::Segmentation => "segmentation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "" | "comment" | "marker" => Some(Self::Comment),
            "chapter" => Some(Self::Chapter),
            "flash_cue" | "flash" | "flash-cue" => Some(Self::FlashCue),
            "web_link" | "web-link" | "weblink" | "url" => Some(Self::WebLink),
            "segmentation" | "segment" => Some(Self::Segmentation),
            _ => None,
        }
    }
}

/// Sequence-level marker. Point markers (`duration_ticks == 0`) mark an instant;
/// range markers span `[start_ticks, start_ticks + duration_ticks)`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Marker {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub kind: MarkerKind,
    pub start_ticks: Tick,
    #[serde(default)]
    pub duration_ticks: Tick,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub note: String,
}

impl Marker {
    pub fn end_ticks(&self) -> Tick {
        Tick(self.start_ticks.0.saturating_add(self.duration_ticks.0))
    }

    pub fn is_range(&self) -> bool {
        self.duration_ticks.0 > 0
    }

    /// True when `tick` falls inside a range marker (end exclusive).
    pub fn contains(&self, tick: Tick) -> bool {
        self.is_range() && tick.0 >= self.start_ticks.0 && tick.0 < self.end_ticks().0
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiCamSyncMode {
    #[default]
    Timecode,
    AudioWaveform,
    Marker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamAngle {
    pub id: u64,
    pub name: String,
    pub media_id: u64,
    #[serde(default)]
    pub sync_offset_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamClip {
    pub id: u64,
    pub name: String,
    pub angles: Vec<MultiCamAngle>,
    pub active_angle_id: u64,
    #[serde(default)]
    pub sync_mode: MultiCamSyncMode,
}
