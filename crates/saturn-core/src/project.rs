use crate::time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const PROJECT_FORMAT: &str = "project-saturn.project";
pub const PROJECT_SCHEMA_VERSION: u32 = 6;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub format: String,
    pub schema_version: u32,
    pub generator: String,
    pub project: Project,
    #[serde(default)]
    pub view: ProjectView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub settings: ProjectSettings,
    pub media: Vec<MediaAsset>,
    pub sequence: Sequence,
    pub next_id: u64,
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
    pub title: Option<TitleProperties>,
    #[serde(default)]
    pub waveform: Option<Vec<u8>>,
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    pub name: String,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub mark_in: Option<Tick>,
    #[serde(default)]
    pub mark_out: Option<Tick>,
}

impl Sequence {
    pub fn duration(&self) -> Tick {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| Tick(c.timeline_start.0.saturating_add(c.duration.0)))
            .max()
            .unwrap_or(Tick(0))
    }

    pub fn active_clips_at(&self, tick: Tick) -> Vec<(&Track, &Clip)> {
        let mut results = Vec::new();
        for track in &self.tracks {
            for clip in &track.clips {
                let start = clip.timeline_start.0;
                let end = start.saturating_add(clip.duration.0);
                if tick.0 >= start && tick.0 < end {
                    results.push((track, clip));
                }
            }
        }
        results
    }

    pub fn transition_at(&self, tick: Tick) -> Option<(&Track, &Transition)> {
        for track in &self.tracks {
            for transition in &track.transitions {
                let half_dur = transition.duration.0 / 2;
                let start = transition.cut_at.0.saturating_sub(half_dur);
                let end = transition.cut_at.0.saturating_add(half_dur);
                if tick.0 >= start && tick.0 <= end {
                    return Some((track, transition));
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub pan: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl Track {
    pub fn is_audible(&self, any_solo: bool) -> bool {
        !self.muted && (!any_solo || self.solo)
    }

    pub fn clip_at(&self, tick: Tick) -> Option<&Clip> {
        self.clips.iter().find(|clip| {
            let start = clip.timeline_start.0;
            let end = start.saturating_add(clip.duration.0);
            tick.0 >= start && tick.0 < end
        })
    }

    pub fn next_clip_after(&self, tick: Tick) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.timeline_start.0 > tick.0)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

fn default_speed() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub media_id: u64,
    pub timeline_start: Tick,
    pub source_in: Tick,
    pub duration: Tick,
    #[serde(default = "default_speed")]
    pub speed: f64,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub color: ColorAdjustments,
    #[serde(default)]
    pub effects: Vec<EffectInstance>,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ColorCorrection,
    GaussianBlur,
    Crop,
    Vignette,
    ChromaKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorCorrectionParams {
    #[serde(default = "default_exposure")]
    pub exposure: f64,
    #[serde(default = "default_contrast")]
    pub contrast: f64,
    #[serde(default = "default_saturation")]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
}

fn default_exposure() -> f64 { 0.0 }
fn default_contrast() -> f64 { 1.0 }
fn default_saturation() -> f64 { 1.0 }

impl Default for ColorCorrectionParams {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GaussianBlurParams {
    #[serde(default = "default_blur_sigma")]
    pub sigma: f64,
}

fn default_blur_sigma() -> f64 { 2.0 }

impl Default for GaussianBlurParams {
    fn default() -> Self {
        Self { sigma: 2.0 }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CropParams {
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub bottom: f64,
    #[serde(default)]
    pub left: f64,
    #[serde(default)]
    pub right: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VignetteParams {
    #[serde(default = "default_vignette_intensity")]
    pub intensity: f64,
    #[serde(default = "default_vignette_radius")]
    pub radius: f64,
    #[serde(default = "default_vignette_softness")]
    pub softness: f64,
}

fn default_vignette_intensity() -> f64 { 0.5 }
fn default_vignette_radius() -> f64 { 0.8 }
fn default_vignette_softness() -> f64 { 0.5 }

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            radius: 0.8,
            softness: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChromaKeyParams {
    #[serde(default = "default_chroma_color")]
    pub color: String,
    #[serde(default = "default_chroma_tolerance")]
    pub tolerance: f64,
    #[serde(default = "default_chroma_softness")]
    pub softness: f64,
    #[serde(default = "default_chroma_spill")]
    pub spill_suppression: f64,
}

fn default_chroma_color() -> String { "#00ff00".into() }
fn default_chroma_tolerance() -> f64 { 25.0 }
fn default_chroma_softness() -> f64 { 10.0 }
fn default_chroma_spill() -> f64 { 0.5 }

impl Default for ChromaKeyParams {
    fn default() -> Self {
        Self {
            color: default_chroma_color(),
            tolerance: default_chroma_tolerance(),
            softness: default_chroma_softness(),
            spill_suppression: default_chroma_spill(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum EffectPayload {
    ColorCorrection(ColorCorrectionParams),
    GaussianBlur(GaussianBlurParams),
    Crop(CropParams),
    Vignette(VignetteParams),
    ChromaKey(ChromaKeyParams),
}

fn default_effect_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_effect_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub payload: EffectPayload,
}

impl EffectInstance {
    pub fn kind(&self) -> EffectKind {
        match &self.payload {
            EffectPayload::ColorCorrection(_) => EffectKind::ColorCorrection,
            EffectPayload::GaussianBlur(_) => EffectKind::GaussianBlur,
            EffectPayload::Crop(_) => EffectKind::Crop,
            EffectPayload::Vignette(_) => EffectKind::Vignette,
            EffectPayload::ChromaKey(_) => EffectKind::ChromaKey,
        }
    }

    pub fn new_color_correction(id: u64, params: ColorCorrectionParams) -> Self {
        Self {
            id,
            name: "Color Correction".into(),
            enabled: true,
            payload: EffectPayload::ColorCorrection(params),
        }
    }

    pub fn new_gaussian_blur(id: u64, params: GaussianBlurParams) -> Self {
        Self {
            id,
            name: "Gaussian Blur".into(),
            enabled: true,
            payload: EffectPayload::GaussianBlur(params),
        }
    }

    pub fn new_crop(id: u64, params: CropParams) -> Self {
        Self {
            id,
            name: "Crop".into(),
            enabled: true,
            payload: EffectPayload::Crop(params),
        }
    }

    pub fn new_vignette(id: u64, params: VignetteParams) -> Self {
        Self {
            id,
            name: "Vignette".into(),
            enabled: true,
            payload: EffectPayload::Vignette(params),
        }
    }

    pub fn new_chroma_key(id: u64, params: ChromaKeyParams) -> Self {
        Self {
            id,
            name: "Chroma Key".into(),
            enabled: true,
            payload: EffectPayload::ChromaKey(params),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorAdjustments {
    pub exposure: f64,
    pub contrast: f64,
    pub saturation: f64,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KeyframeProperty {
    PositionX,
    PositionY,
    Scale,
    Rotation,
    Opacity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keyframe {
    pub property: KeyframeProperty,
    /// Time relative to the clip's source in point.
    pub time: Tick,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectView {
    #[serde(default)]
    pub active_sequence: Option<String>,
    #[serde(default)]
    pub timeline_zoom: f64,
    #[serde(default)]
    pub timeline_scroll: f64,
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self::new("Untitled project")
    }
}

impl ProjectDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format: PROJECT_FORMAT.to_owned(),
            schema_version: PROJECT_SCHEMA_VERSION,
            generator: format!("Project Saturn {}", env!("CARGO_PKG_VERSION")),
            project: Project {
                name: name.into(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::FPS_30,
                    audio_sample_rate: 48_000,
                },
                media: Vec::new(),
                sequence: Sequence {
                    name: "Sequence 1".into(),
                    mark_in: None,
                    mark_out: None,
                    tracks: vec![
                        Track {
                            id: 1,
                            kind: TrackKind::Video,
                            name: "V1".into(),
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                        Track {
                            id: 2,
                            kind: TrackKind::Audio,
                            name: "A1".into(),
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                    ],
                },
                next_id: 3,
            },
            view: ProjectView {
                active_sequence: Some("Sequence 1".into()),
                timeline_zoom: 1.0,
                timeline_scroll: 0.0,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != PROJECT_FORMAT {
            return Err(format!("Unsupported project format `{}`", self.format));
        }
        if self.project.name.trim().is_empty() {
            return Err("Project name cannot be empty".into());
        }
        if self.project.settings.width == 0 || self.project.settings.height == 0 {
            return Err("Project dimensions must be greater than zero".into());
        }
        if self.project.settings.audio_sample_rate == 0
            || self.project.settings.frame_rate.frame_ticks().is_none()
        {
            return Err("Project uses an unsupported frame or audio rate".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.project.media {
            if !ids.insert(item.id) {
                return Err(format!("Duplicate media id {}", item.id));
            }
        }
        for track in &self.project.sequence.tracks {
            if !track.gain_db.is_finite() || !(-60.0..=12.0).contains(&track.gain_db) {
                return Err(format!("Track {} has invalid gain", track.name));
            }
            for clip in &track.clips {
                if clip.duration.0 <= 0
                    || !clip.color.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&clip.color.exposure)
                    || !clip.color.contrast.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.contrast)
                    || !clip.color.saturation.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.saturation)
                {
                    return Err("Project contains invalid clip properties".into());
                }
                for effect in &clip.effects {
                    match &effect.payload {
                        EffectPayload::ColorCorrection(p) => {
                            if !p.exposure.is_finite()
                                || !(-5.0..=5.0).contains(&p.exposure)
                                || !p.contrast.is_finite()
                                || !(0.0..=2.0).contains(&p.contrast)
                                || !p.saturation.is_finite()
                                || !(0.0..=2.0).contains(&p.saturation)
                                || !p.hue.is_finite()
                                || !(-1.0..=1.0).contains(&p.hue)
                            {
                                return Err("Project contains invalid color correction effect values".into());
                            }
                        }
                        EffectPayload::GaussianBlur(p) => {
                            if !p.sigma.is_finite() || !(0.0..=50.0).contains(&p.sigma) {
                                return Err("Project contains invalid gaussian blur effect values".into());
                            }
                        }
                        EffectPayload::Crop(p) => {
                            if !p.top.is_finite() || !(0.0..=100.0).contains(&p.top)
                                || !p.bottom.is_finite() || !(0.0..=100.0).contains(&p.bottom)
                                || !p.left.is_finite() || !(0.0..=100.0).contains(&p.left)
                                || !p.right.is_finite() || !(0.0..=100.0).contains(&p.right)
                            {
                                return Err("Project contains invalid crop effect values".into());
                            }
                        }
                        EffectPayload::Vignette(p) => {
                            if !p.intensity.is_finite() || !(0.0..=1.0).contains(&p.intensity)
                                || !p.radius.is_finite() || !(0.0..=2.0).contains(&p.radius)
                                || !p.softness.is_finite() || !(0.0..=1.0).contains(&p.softness)
                            {
                                return Err("Project contains invalid vignette effect values".into());
                            }
                        }
                        EffectPayload::ChromaKey(p) => {
                            if p.color.trim().is_empty()
                                || !p.tolerance.is_finite() || !(0.0..=100.0).contains(&p.tolerance)
                                || !p.softness.is_finite() || !(0.0..=100.0).contains(&p.softness)
                                || !p.spill_suppression.is_finite() || !(0.0..=1.0).contains(&p.spill_suppression)
                            {
                                return Err("Project contains invalid chroma key effect values".into());
                            }
                        }
                    }
                }
                for keyframe in &clip.keyframes {
                    let value_in_range = match &keyframe.property {
                        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
                            (-100_000.0..=100_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Scale => (0.01..=100.0).contains(&keyframe.value),
                        KeyframeProperty::Rotation => {
                            (-36_000.0..=36_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Opacity => (0.0..=1.0).contains(&keyframe.value),
                    };
                    if keyframe.time.0 < 0
                        || keyframe.time.0 > clip.duration.0
                        || !keyframe.value.is_finite()
                        || !value_in_range
                    {
                        return Err("Project contains an invalid keyframe".into());
                    }
                }
            }
            for transition in &track.transitions {
                if transition.duration.0 <= 0 || transition.cut_at.0 < 0 {
                    return Err("Project contains an invalid transition".into());
                }
            }
        }
        Ok(())
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("Could not create project folder: {e}"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| "Project path has no file name".to_string())?;
        let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            suffix
        ));
        let serialized = serde_json::to_vec_pretty(self)
            .map_err(|e| format!("Could not serialize project: {e}"))?;

        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|e| format!("Could not create temporary project file: {e}"))?;
            file.write_all(&serialized)
                .map_err(|e| format!("Could not write project file: {e}"))?;
            file.write_all(b"\n")
                .map_err(|e| format!("Could not finish project file: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("Could not flush project file: {e}"))?;
            drop(file);
            fs::rename(&temporary_path, path)
                .map_err(|e| format!("Could not replace project file: {e}"))?;
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Could not open project: {e}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Could not read project: {e}"))?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Project file is not valid JSON: {e}"))?;
        migrate_project(&mut value)?;
        let document: Self = serde_json::from_value(value)
            .map_err(|e| format!("Project file has an invalid structure: {e}"))?;
        document.validate()?;
        Ok(document)
    }
}

fn migrate_project(value: &mut serde_json::Value) -> Result<(), String> {
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "Project file has no schema_version".to_string())? as u32;
    if value.get("format").and_then(serde_json::Value::as_str) != Some(PROJECT_FORMAT) {
        return Err("This is not a Project Saturn project file".into());
    }
    match version {
        PROJECT_SCHEMA_VERSION => Ok(()),
        1 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            object.insert("schema_version".into(), 2.into());
            object.entry("view").or_insert_with(|| {
                serde_json::json!({
                    "active_sequence": "Sequence 1",
                    "timeline_zoom": 1.0,
                    "timeline_scroll": 0.0
                })
            });
            migrate_project(value)
        }
        2 | 3 | 4 | 5 => {
            value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?
                .insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        unsupported => Err(format!(
            "Project schema version {unsupported} is not supported"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_project_migrates_to_current_schema() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("Legacy")).unwrap();
        legacy["schema_version"] = 1.into();
        legacy.as_object_mut().unwrap().remove("view");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.view.active_sequence.as_deref(), Some("Sequence 1"));
    }

    #[test]
    fn v2_project_migrates_with_default_edit_properties() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V2 project")).unwrap();
        legacy["schema_version"] = 2.into();
        for track in legacy["project"]["sequence"]["tracks"]
            .as_array_mut()
            .unwrap()
        {
            track.as_object_mut().unwrap().remove("gain_db");
        }
        legacy["project"]["sequence"]["tracks"][0]["clips"] = serde_json::json!([
            {"media_id": 3, "timeline_start": 0, "source_in": 0, "duration": 100}
        ]);
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.project.sequence.tracks[0].gain_db, 0.0);
        let clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(clip.color.exposure, 0.0);
        assert_eq!(clip.color.contrast, 1.0);
        assert!(clip.keyframes.is_empty());
    }

    #[test]
    fn project_saves_atomically_and_keeps_external_media_paths() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("cut.saturn");
        let mut doc = ProjectDocument::new("Cut");
        doc.project.media.push(MediaAsset {
            id: 4,
            name: "shot.mov".into(),
            path: PathBuf::from("/media/shot.mov"),
            kind: MediaKind::Video,
            duration: Some(Tick(42)),
            width: Some(1920),
            height: Some(1080),
            title: None,
            waveform: None,
        });
        doc.save_atomic(&path).unwrap();
        let loaded = ProjectDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.media[0].path,
            PathBuf::from("/media/shot.mov")
        );
        assert_eq!(loaded.project.media[0].duration, Some(Tick(42)));
    }

    #[test]
    fn test_sequence_playback_queries() {
        let mut doc = ProjectDocument::new("Playback test");
        let video_track = &mut doc.project.sequence.tracks[0];
        video_track.clips.push(Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(100),
            duration: Tick(500),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
        });
        video_track.clips.push(Clip {
            media_id: 2,
            timeline_start: Tick(500),
            source_in: Tick(0),
            duration: Tick(400),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
        });
        video_track.transitions.push(Transition {
            id: 1,
            kind: TransitionKind::Crossfade,
            cut_at: Tick(500),
            duration: Tick(100),
        });

        let seq = &doc.project.sequence;
        assert_eq!(seq.duration(), Tick(900));

        let active_mid_first = seq.active_clips_at(Tick(250));
        assert_eq!(active_mid_first.len(), 1);
        assert_eq!(active_mid_first[0].1.media_id, 1);

        let active_at_cut = seq.active_clips_at(Tick(500));
        assert_eq!(active_at_cut.len(), 1);
        assert_eq!(active_at_cut[0].1.media_id, 2);

        let trans = seq.transition_at(Tick(500));
        assert!(trans.is_some());
        assert_eq!(trans.unwrap().1.kind, TransitionKind::Crossfade);

        let trans_boundary = seq.transition_at(Tick(460));
        assert!(trans_boundary.is_some());

        let trans_outside = seq.transition_at(Tick(400));
        assert!(trans_outside.is_none());

        assert_eq!(seq.tracks[0].next_clip_after(Tick(200)).unwrap().media_id, 2);
    }

    #[test]
    fn test_effect_stack_serialization_and_migration() {
        let mut doc = ProjectDocument::new("Effect test");
        let video_track = &mut doc.project.sequence.tracks[0];
        let mut clip = Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(0),
            duration: Tick(300),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
        };

        clip.effects.push(EffectInstance::new_color_correction(
            10,
            ColorCorrectionParams {
                exposure: 0.5,
                contrast: 1.1,
                saturation: 1.2,
                hue: 0.05,
            },
        ));
        clip.effects.push(EffectInstance::new_gaussian_blur(
            11,
            GaussianBlurParams { sigma: 4.5 },
        ));
        clip.effects.push(EffectInstance::new_crop(
            12,
            CropParams {
                top: 5.0,
                bottom: 10.0,
                left: 2.0,
                right: 2.0,
            },
        ));
        clip.effects.push(EffectInstance::new_vignette(
            13,
            VignetteParams {
                intensity: 0.7,
                radius: 0.9,
                softness: 0.6,
            },
        ));
        clip.effects.push(EffectInstance::new_chroma_key(
            14,
            ChromaKeyParams {
                color: "#00ff00".into(),
                tolerance: 35.0,
                softness: 12.0,
                spill_suppression: 0.75,
            },
        ));

        video_track.clips.push(clip);
        assert!(doc.validate().is_ok());

        // Serialize and deserialize roundtrip
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let loaded: ProjectDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        let loaded_clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(loaded_clip.effects.len(), 5);
        assert_eq!(loaded_clip.effects[0].kind(), EffectKind::ColorCorrection);
        assert_eq!(loaded_clip.effects[1].kind(), EffectKind::GaussianBlur);
        assert_eq!(loaded_clip.effects[2].kind(), EffectKind::Crop);
        assert_eq!(loaded_clip.effects[3].kind(), EffectKind::Vignette);
        assert_eq!(loaded_clip.effects[4].kind(), EffectKind::ChromaKey);
    }

    #[test]
    fn test_track_audio_properties_and_audibility() {
        let mut t1 = Track {
            id: 1,
            kind: TrackKind::Audio,
            name: "A1".into(),
            gain_db: -3.0,
            pan: -0.5,
            muted: false,
            solo: false,
            locked: false,
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        let mut t2 = Track {
            id: 2,
            kind: TrackKind::Audio,
            name: "A2".into(),
            gain_db: 0.0,
            pan: 0.5,
            muted: false,
            solo: false,
            locked: false,
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        // Neither soloed, neither muted -> both audible
        assert!(t1.is_audible(false));
        assert!(t2.is_audible(false));

        // Mute t1
        t1.muted = true;
        assert!(!t1.is_audible(false));
        assert!(t2.is_audible(false));
        t1.muted = false;

        // Solo t2 -> any_solo is true, t1 not soloed -> t1 inaudible, t2 audible
        t2.solo = true;
        assert!(!t1.is_audible(true));
        assert!(t2.is_audible(true));

        // Both soloed -> both audible
        t1.solo = true;
        assert!(t1.is_audible(true));
        assert!(t2.is_audible(true));
    }
}
