//! Sequence Module

use crate::time::{Ticks, TICKS_PER_SECOND, TimelineError};
use crate::{FrameRate, Timecode, TimelineTime, TrackId, ClipId, ClipRef, Track};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use chrono;

/// Sequence settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceSettings {
    pub name: String,
    pub description: String,
    pub frame_rate: FrameRate,
    pub resolution: (u32, u32),
    pub sample_rate: u32,
    pub audio_channels: u32,
    pub timecode_start: Timecode,
    pub drop_frame: bool,
    pub color_space: crate::ColorSpace,
    pub working_color_space: crate::ColorSpace,
}

impl Default for SequenceSettings {
    fn default() -> Self {
        Self {
            name: "New Sequence".to_string(),
            description: String::new(),
            frame_rate: FrameRate::Fps30,
            resolution: (1920, 1080),
            sample_rate: 48000,
            audio_channels: 2,
            timecode_start: Timecode::new(0, 0, 0, 0, false),
            drop_frame: false,
            color_space: crate::ColorSpace::Rec709,
            working_color_space: crate::ColorSpace::Rec709,
        }
    }
}

/// Marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    pub id: uuid::Uuid,
    pub position: Ticks,
    pub name: String,
    pub color: String,
    pub comment: String,
    pub marker_type: MarkerType,
    pub duration: Option<Ticks>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerType {
    Chapter,
    WebLink,
    FlashCue,
    Comment,
    SyncPoint,
}

/// Sequence state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SequenceState {
    pub tracks: Vec<TrackId>,
    pub selection: Vec<ClipId>,
    pub playhead: Ticks,
    pub zoom_level: f32,
    pub work_area_start: Option<Ticks>,
    pub work_area_end: Option<Ticks>,
    pub markers: Vec<Marker>,
}

/// Sequence reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceRef {
    pub sequence_id: SequenceId,
    pub timeline_start: Ticks,
    pub timeline_duration: Ticks,
    pub source_in: Ticks,
    pub source_out: Ticks,
    pub speed: f64,
    pub reverse: bool,
}

impl SequenceRef {
    pub fn new(sequence_id: SequenceId, timeline_start: Ticks, timeline_duration: Ticks) -> Self {
        Self {
            sequence_id,
            timeline_start,
            timeline_duration,
            source_in: 0,
            source_out: timeline_duration,
            speed: 1.0,
            reverse: false,
        }
    }

    pub fn timeline_end(&self) -> Ticks {
        self.timeline_start + self.timeline_duration
    }
}

/// Unique identifier for a sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SequenceId(pub uuid::Uuid);

impl SequenceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for SequenceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sequence metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SequenceMetadata {
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: String,
    pub tags: Vec<String>,
    pub color_label: Option<String>,
    pub custom: serde_json::Value,
}

/// Main Sequence struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    pub id: SequenceId,
    pub settings: SequenceSettings,
    pub state: SequenceState,
    pub tracks: HashMap<TrackId, Track>,
    pub nested_sequences: Vec<SequenceRef>,
    pub metadata: SequenceMetadata,
}

impl Sequence {
    pub fn new(settings: SequenceSettings) -> Self {
        let id = SequenceId::new();
        Self {
            id,
            settings,
            state: SequenceState::default(),
            tracks: HashMap::new(),
            nested_sequences: Vec::new(),
            metadata: SequenceMetadata::default(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.state.tracks.push(track.id);
        self.tracks.insert(track.id, track);
    }

    pub fn remove_track(&mut self, track_id: TrackId) {
        self.state.tracks.retain(|&id| id != track_id);
        self.tracks.remove(&track_id);
    }

    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.get(&track_id)
    }

    pub fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(&track_id)
    }

    pub fn move_clip(&mut self, clip_id: ClipId, to_track: Option<TrackId>, new_start: Option<Ticks>) -> Result<(), TimelineError> {
        // Find and remove clip from current track
        let mut clip_ref = None;
        let mut from_track_id = None;

        for (tid, track) in &mut self.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.clip_id == clip_id) {
                clip_ref = Some(track.clips.remove(pos));
                from_track_id = Some(*tid);
                break;
            }
        }

        let mut clip_ref = clip_ref.ok_or(TimelineError::ClipNotFound)?;

        // Add to new track if specified
        if let Some(target_track) = to_track.or(from_track_id) {
            clip_ref.track_id = target_track;
            if let Some(new_start) = new_start {
                clip_ref.timing.timeline_start = new_start;
            }
            if let Some(track) = self.tracks.get_mut(&target_track) {
                track.clips.push(clip_ref);
            }
        }

        Ok(())
    }

    pub fn add_nested_sequence(&mut self, sequence_ref: SequenceRef) -> Result<(), TimelineError> {
        self.nested_sequences.push(sequence_ref);
        Ok(())
    }

    pub fn remove_nested_sequence(&mut self, sequence_id: SequenceId) {
        self.nested_sequences.retain(|s| s.sequence_id != sequence_id);
    }
}