//! Clip Module

use crate::time::{TICKS_PER_SECOND, Ticks};
use crate::{BlendMode, FrameRate, SequenceId, TrackId};
use chrono;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Clip source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipSource {
    Media {
        media_id: uuid::Uuid,
        file_path: String,
    },
    Generated {
        generator: String,
        parameters: serde_json::Value,
    },
    NestedSequence {
        sequence_id: SequenceId,
    },
    Placeholder {
        width: u32,
        height: u32,
        color: String,
    },
}

impl ClipSource {
    pub fn media_id(&self) -> Option<uuid::Uuid> {
        match self {
            ClipSource::Media { media_id, .. } => Some(*media_id),
            _ => None,
        }
    }

    pub fn is_nested_sequence(&self) -> bool {
        matches!(self, ClipSource::NestedSequence { .. })
    }
}

/// Clip timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipTiming {
    pub timeline_start: Ticks,
    pub timeline_duration: Ticks,
    pub source_in: Ticks,
    pub source_out: Ticks,
    pub speed: f64,
    pub reverse: bool,
    pub frame_blending: bool,
}

impl Default for ClipTiming {
    fn default() -> Self {
        Self {
            timeline_start: 0,
            timeline_duration: TICKS_PER_SECOND * 5,
            source_in: 0,
            source_out: TICKS_PER_SECOND * 5,
            speed: 1.0,
            reverse: false,
            frame_blending: true,
        }
    }
}

impl ClipTiming {
    pub fn timeline_end(&self) -> Ticks {
        self.timeline_start + self.timeline_duration
    }

    pub fn source_duration(&self) -> Ticks {
        self.source_out.saturating_sub(self.source_in)
    }
}

/// Clip effects
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipEffects {
    pub video_effects: Vec<VideoEffect>,
    pub audio_effects: Vec<AudioEffect>,
    pub color_grade: Option<ColorGrade>,
    pub transform: Transform,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

/// Video effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEffect {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
    pub order: usize,
}

/// Audio effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEffect {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
    pub order: usize,
}

/// Color grade
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorGrade {
    pub lift: [f32; 3],
    pub gamma: [f32; 3],
    pub gain: [f32; 3],
    pub offset: [f32; 3],
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
}

/// Transform
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Transform {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub anchor: [f32; 2],
    pub flip_x: bool,
    pub flip_y: bool,
}

/// Clip reference on a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipRef {
    pub clip_id: ClipId,
    pub track_id: TrackId,
    pub timing: ClipTiming,
    pub effects: ClipEffects,
    pub source: ClipSource,
    pub metadata: ClipMetadata,
}

impl ClipRef {
    pub fn new(clip_id: ClipId, timeline_start: Ticks, timeline_duration: Ticks) -> Self {
        Self {
            clip_id,
            track_id: TrackId::new(),
            timing: ClipTiming {
                timeline_start,
                timeline_duration,
                ..Default::default()
            },
            effects: ClipEffects::default(),
            source: ClipSource::Placeholder {
                width: 1920,
                height: 1080,
                color: "#000000".to_string(),
            },
            metadata: ClipMetadata::default(),
        }
    }

    pub fn with_source(mut self, source: ClipSource) -> Self {
        self.source = source;
        self
    }

    pub fn timeline_end(&self) -> Ticks {
        self.timing.timeline_end()
    }

    pub fn set_track(&mut self, track_id: TrackId) {
        self.track_id = track_id;
    }
}

/// Clip metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub rating: u8,
    pub color_label: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub custom: serde_json::Value,
}

/// Full clip object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub source: ClipSource,
    pub duration: Ticks,
    pub frame_rate: FrameRate,
    pub resolution: (u32, u32),
    pub audio_channels: u32,
    pub metadata: ClipMetadata,
    pub effects: ClipEffects,
    pub proxy_path: Option<String>,
}

impl Clip {
    pub fn new(source: ClipSource) -> Self {
        Self {
            id: ClipId::new(),
            source,
            duration: 0,
            frame_rate: FrameRate::default(),
            resolution: (1920, 1080),
            audio_channels: 2,
            metadata: ClipMetadata::default(),
            effects: ClipEffects::default(),
            proxy_path: None,
        }
    }

    pub fn create_ref(&self, timeline_start: Ticks, timeline_duration: Ticks) -> ClipRef {
        ClipRef::new(self.id, timeline_start, timeline_duration).with_source(self.source.clone())
    }
}

/// Unique identifier for a clip
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipId(pub uuid::Uuid);

impl ClipId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ClipId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
