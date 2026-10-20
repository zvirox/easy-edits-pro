//! Edit Operations Module

use crate::time::{TICKS_PER_SECOND, Ticks};
use crate::{
    AudioEffect, BlendMode, ClipEffects, ClipId, ClipMetadata, ClipRef, ClipSource, ClipTiming,
    ColorGrade, FrameRate, Marker, MarkerType, SequenceId, SequenceRef, Track, TrackId, TrackKind,
    Transform, VideoEffect,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Edit operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    AddClip {
        track_id: TrackId,
        clip_ref: ClipRef,
    },
    RemoveClip {
        track_id: TrackId,
        clip_id: ClipId,
        clip_ref: ClipRef,
    },
    MoveClip {
        clip_id: ClipId,
        from_track: TrackId,
        to_track: TrackId,
        old_start: Ticks,
        new_start: Ticks,
    },
    TrimClip {
        clip_id: ClipId,
        old_timing: ClipTiming,
        new_timing: ClipTiming,
    },
    ChangeSpeed {
        clip_id: ClipId,
        old_speed: f64,
        new_speed: f64,
    },
    AddEffect {
        clip_id: ClipId,
        effect: VideoEffect,
    },
    RemoveEffect {
        clip_id: ClipId,
        effect_id: uuid::Uuid,
        effect: VideoEffect,
    },
    ModifyEffect {
        clip_id: ClipId,
        effect_id: uuid::Uuid,
        old_params: serde_json::Value,
        new_params: serde_json::Value,
    },
    TransformClip {
        clip_id: ClipId,
        old_transform: Transform,
        new_transform: Transform,
    },
    ChangeOpacity {
        clip_id: ClipId,
        old_opacity: f32,
        new_opacity: f32,
    },
    ChangeBlendMode {
        clip_id: ClipId,
        old_mode: BlendMode,
        new_mode: BlendMode,
    },
    AddTrack {
        track_id: TrackId,
        kind: TrackKind,
        index: usize,
    },
    RemoveTrack {
        track_id: TrackId,
        track: Track,
    },
    ReorderTracks {
        old_order: Vec<TrackId>,
        new_order: Vec<TrackId>,
    },
    AddMarker {
        marker: Marker,
    },
    RemoveMarker {
        marker_id: uuid::Uuid,
        marker: Marker,
    },
    MoveMarker {
        marker_id: uuid::Uuid,
        old_position: Ticks,
        new_position: Ticks,
    },
    SetWorkArea {
        old_start: Option<Ticks>,
        old_end: Option<Ticks>,
        new_start: Option<Ticks>,
        new_end: Option<Ticks>,
    },
    AddNestedSequence {
        sequence_ref: SequenceRef,
    },
    RemoveNestedSequence {
        sequence_id: SequenceId,
        sequence_ref: SequenceRef,
    },
    Batch {
        operations: Vec<EditOperation>,
        description: String,
    },
}

impl EditOperation {
    pub fn description(&self) -> String {
        match self {
            EditOperation::AddClip { .. } => "Add Clip".to_string(),
            EditOperation::RemoveClip { .. } => "Remove Clip".to_string(),
            EditOperation::MoveClip { .. } => "Move Clip".to_string(),
            EditOperation::TrimClip { .. } => "Trim Clip".to_string(),
            EditOperation::ChangeSpeed { .. } => "Change Speed".to_string(),
            EditOperation::AddEffect { .. } => "Add Effect".to_string(),
            EditOperation::RemoveEffect { .. } => "Remove Effect".to_string(),
            EditOperation::ModifyEffect { .. } => "Modify Effect".to_string(),
            EditOperation::TransformClip { .. } => "Transform Clip".to_string(),
            EditOperation::ChangeOpacity { .. } => "Change Opacity".to_string(),
            EditOperation::ChangeBlendMode { .. } => "Change Blend Mode".to_string(),
            EditOperation::AddTrack { .. } => "Add Track".to_string(),
            EditOperation::RemoveTrack { .. } => "Remove Track".to_string(),
            EditOperation::ReorderTracks { .. } => "Reorder Tracks".to_string(),
            EditOperation::AddMarker { .. } => "Add Marker".to_string(),
            EditOperation::RemoveMarker { .. } => "Remove Marker".to_string(),
            EditOperation::MoveMarker { .. } => "Move Marker".to_string(),
            EditOperation::SetWorkArea { .. } => "Set Work Area".to_string(),
            EditOperation::AddNestedSequence { .. } => "Add Nested Sequence".to_string(),
            EditOperation::RemoveNestedSequence { .. } => "Remove Nested Sequence".to_string(),
            EditOperation::Batch { description, .. } => description.clone(),
        }
    }

    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            EditOperation::AddTrack { .. }
                | EditOperation::RemoveTrack { .. }
                | EditOperation::ReorderTracks { .. }
                | EditOperation::AddNestedSequence { .. }
                | EditOperation::RemoveNestedSequence { .. }
        )
    }

    pub fn affected_clips(&self) -> Vec<ClipId> {
        match self {
            EditOperation::AddClip { clip_ref, .. } => vec![clip_ref.clip_id],
            EditOperation::RemoveClip { clip_id, .. } => vec![*clip_id],
            EditOperation::MoveClip { clip_id, .. } => vec![*clip_id],
            EditOperation::TrimClip { clip_id, .. } => vec![*clip_id],
            EditOperation::ChangeSpeed { clip_id, .. } => vec![*clip_id],
            EditOperation::AddEffect { clip_id, .. } => vec![*clip_id],
            EditOperation::RemoveEffect { clip_id, .. } => vec![*clip_id],
            EditOperation::ModifyEffect { clip_id, .. } => vec![*clip_id],
            EditOperation::TransformClip { clip_id, .. } => vec![*clip_id],
            EditOperation::ChangeOpacity { clip_id, .. } => vec![*clip_id],
            EditOperation::ChangeBlendMode { clip_id, .. } => vec![*clip_id],
            EditOperation::Batch { operations, .. } => operations
                .iter()
                .flat_map(|op| op.affected_clips())
                .collect(),
            _ => vec![],
        }
    }

    pub fn affected_tracks(&self) -> Vec<TrackId> {
        match self {
            EditOperation::AddClip { track_id, .. } => vec![*track_id],
            EditOperation::RemoveClip { track_id, .. } => vec![*track_id],
            EditOperation::MoveClip {
                from_track,
                to_track,
                ..
            } => vec![*from_track, *to_track],
            EditOperation::AddTrack { track_id, .. } => vec![*track_id],
            EditOperation::RemoveTrack { track_id, .. } => vec![*track_id],
            EditOperation::ReorderTracks { new_order, .. } => new_order.clone(),
            EditOperation::Batch { operations, .. } => operations
                .iter()
                .flat_map(|op| op.affected_tracks())
                .collect(),
            _ => vec![],
        }
    }
}

/// Unique identifier for an edit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditId(pub uuid::Uuid);

impl EditId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EditId {
    fn default() -> Self {
        Self::new()
    }
}

/// Single atomic edit with undo/redo support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    pub id: EditId,
    pub operation: EditOperation,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub group_id: Option<EditId>,
}

impl Edit {
    pub fn new(operation: EditOperation) -> Self {
        let description = operation.description();
        Self {
            id: EditId::new(),
            operation,
            timestamp: chrono::Utc::now(),
            description,
            group_id: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_group(mut self, group_id: EditId) -> Self {
        self.group_id = Some(group_id);
        self
    }
}

/// Edit transaction for grouping multiple operations atomically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTransaction {
    pub id: EditId,
    pub edits: Vec<Edit>,
    pub description: String,
    pub committed: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl EditTransaction {
    pub fn new(description: String) -> Self {
        Self {
            id: EditId::new(),
            edits: Vec::new(),
            description,
            committed: false,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn add_edit(&mut self, edit: Edit) {
        let edit = edit.with_group(self.id);
        self.edits.push(edit);
    }

    pub fn add_operation(&mut self, operation: EditOperation) {
        self.add_edit(Edit::new(operation));
    }

    pub fn commit(&mut self) {
        self.committed = true;
    }

    pub fn rollback(&mut self) {
        self.edits.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn edit_count(&self) -> usize {
        self.edits.len()
    }
}
