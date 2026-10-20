//! Track Module

use crate::time::{TICKS_PER_SECOND, Ticks};
use crate::{ClipId, ClipRef};
use chrono;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Track kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
    Submix,
}

impl TrackKind {
    pub fn is_audio(&self) -> bool {
        matches!(self, TrackKind::Audio)
    }

    pub fn is_video(&self) -> bool {
        matches!(self, TrackKind::Video)
    }

    pub fn default_name(&self, index: usize) -> String {
        match self {
            TrackKind::Video => format!("V{}", index + 1),
            TrackKind::Audio => format!("A{}", index + 1),
            TrackKind::Subtitle => format!("S{}", index + 1),
            TrackKind::Submix => format!("Submix {}", index + 1),
        }
    }
}

/// Blend modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::Normal
    }
}

/// Track settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSettings {
    pub name: String,
    pub kind: TrackKind,
    pub enabled: bool,
    pub locked: bool,
    pub solo: bool,
    pub muted: bool,
    pub height: f32,
    pub color: Option<String>,
    pub volume: f32,
    pub pan: f32,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

impl Default for TrackSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: TrackKind::Video,
            enabled: true,
            locked: false,
            solo: false,
            muted: false,
            height: 60.0,
            color: None,
            volume: 1.0,
            pan: 0.0,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
        }
    }
}

/// Track state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackState {
    pub clips: Vec<ClipId>,
    pub selection: Vec<ClipId>,
    pub playhead_visible: bool,
    pub zoom_level: f32,
}

/// Unique identifier for a track
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub uuid::Uuid);

impl TrackId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TrackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Track container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub settings: TrackSettings,
    pub state: TrackState,
    pub clips: Vec<ClipRef>,
}

impl Track {
    pub fn new(kind: TrackKind, index: usize) -> Self {
        let mut settings = TrackSettings::default();
        settings.kind = kind;
        settings.name = kind.default_name(index);

        Self {
            id: TrackId::new(),
            settings,
            state: TrackState::default(),
            clips: Vec::new(),
        }
    }

    pub fn with_id(id: TrackId, kind: TrackKind, index: usize) -> Self {
        let mut settings = TrackSettings::default();
        settings.kind = kind;
        settings.name = kind.default_name(index);

        Self {
            id,
            settings,
            state: TrackState::default(),
            clips: Vec::new(),
        }
    }

    pub fn add_clip(&mut self, clip_ref: ClipRef) {
        self.clips.push(clip_ref);
    }

    pub fn remove_clip(&mut self, clip_id: ClipId) -> Option<ClipRef> {
        if let Some(pos) = self.clips.iter().position(|c| c.clip_id == clip_id) {
            self.state.clips.retain(|&id| id != clip_id);
            Some(self.clips.remove(pos))
        } else {
            None
        }
    }

    pub fn get_clip(&self, clip_id: ClipId) -> Option<&ClipRef> {
        self.clips.iter().find(|c| c.clip_id == clip_id)
    }

    pub fn get_clip_mut(&mut self, clip_id: ClipId) -> Option<&mut ClipRef> {
        self.clips.iter_mut().find(|c| c.clip_id == clip_id)
    }

    pub fn clips_in_range(&self, start: Ticks, end: Ticks) -> Vec<&ClipRef> {
        self.clips
            .iter()
            .filter(|r| r.timing.timeline_start < end && r.timeline_end() > start)
            .collect()
    }

    pub fn total_duration(&self) -> Ticks {
        self.clips
            .iter()
            .map(|r| r.timeline_end())
            .max()
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }
}
