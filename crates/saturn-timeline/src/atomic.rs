//! Atomic Operations Module

use crate::time::{Ticks, TimelineError};
use crate::{Sequence, ClipId, TrackId, ClipRef, ClipTiming, EditOperation, Edit, EditTransaction, EditId, VideoEffect, Track};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono;

/// Atomic edit engine for applying and reverting edits
pub struct AtomicEditEngine {
    sequence: Arc<parking_lot::RwLock<crate::sequence::Sequence>>,
    history: Vec<Edit>,
    redo_stack: Vec<Edit>,
    max_history: usize,
    current_transaction: Option<EditTransaction>,
}

impl AtomicEditEngine {
    pub fn new(sequence: Arc<parking_lot::RwLock<crate::sequence::Sequence>>) -> Self {
        Self {
            sequence,
            history: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 1000,
            current_transaction: None,
        }
    }

    pub fn apply_edit(&mut self, edit: Edit) -> Result<(), crate::TimelineError> {
        self.apply_operation(&edit.operation)?;

        self.history.push(edit);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.redo_stack.clear();

        Ok(())
    }

    fn apply_operation(&self, operation: &EditOperation) -> Result<(), crate::TimelineError> {
        let mut seq = self.sequence.write();

        match operation {
            crate::edit::EditOperation::AddClip { track_id, clip_ref } => {
                let mut clip = clip_ref.clone();
                clip.track_id = *track_id;
                seq.get_track_mut(*track_id).ok_or(crate::TimelineError::TrackNotFound)?.add_clip(clip);
            }
            crate::edit::EditOperation::RemoveClip { track_id, clip_id, .. } => {
                seq.get_track_mut(*track_id).ok_or(crate::TimelineError::TrackNotFound)?.remove_clip(*clip_id);
            }
            crate::edit::EditOperation::MoveClip { clip_id, to_track, new_start, .. } => {
                seq.move_clip(*clip_id, Some(*to_track), Some(*new_start))?;
            }
            crate::edit::EditOperation::TrimClip { clip_id, new_timing, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.timing = new_timing.clone();
                        break;
                    }
                }
            }
            crate::edit::EditOperation::ChangeSpeed { clip_id, new_speed, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.timing.speed = *new_speed;
                        break;
                    }
                }
            }
            crate::edit::EditOperation::AddEffect { clip_id, effect } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.video_effects.push(effect.clone());
                        break;
                    }
                }
            }
            crate::edit::EditOperation::RemoveEffect { clip_id, effect_id, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.video_effects.retain(|e| e.id != *effect_id);
                        break;
                    }
                }
            }
            crate::edit::EditOperation::ModifyEffect { clip_id, effect_id, new_params, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        if let Some(effect) = clip.effects.video_effects.iter_mut().find(|e| e.id == *effect_id) {
                            effect.parameters = new_params.clone();
                            break;
                        }
                    }
                }
            }
            crate::edit::EditOperation::TransformClip { clip_id, new_transform, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.transform = new_transform.clone();
                        break;
                    }
                }
            }
            crate::edit::EditOperation::ChangeOpacity { clip_id, new_opacity, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.opacity = *new_opacity;
                        break;
                    }
                }
            }
            crate::edit::EditOperation::ChangeBlendMode { clip_id, new_mode, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.blend_mode = *new_mode;
                        break;
                    }
                }
            }
            crate::edit::EditOperation::AddTrack { track_id, kind, index } => {
                let track = crate::track::Track::with_id(*track_id, *kind, *index);
                seq.state.tracks.push(*track_id);
                seq.tracks.insert(*track_id, track);
            }
            crate::edit::EditOperation::RemoveTrack { track_id, .. } => {
                seq.remove_track(*track_id);
            }
            crate::edit::EditOperation::ReorderTracks { new_order, .. } => {
                seq.state.tracks = new_order.clone();
            }
            crate::edit::EditOperation::AddMarker { marker } => {
                seq.state.markers.push(marker.clone());
            }
            crate::edit::EditOperation::RemoveMarker { marker_id, .. } => {
                seq.state.markers.retain(|m| m.id != *marker_id);
            }
            crate::edit::EditOperation::MoveMarker { marker_id, new_position, .. } => {
                for marker in &mut seq.state.markers {
                    if marker.id == *marker_id {
                        marker.position = *new_position;
                        break;
                    }
                }
            }
            crate::edit::EditOperation::SetWorkArea { new_start, new_end, .. } => {
                seq.state.work_area_start = *new_start;
                seq.state.work_area_end = *new_end;
            }
            crate::edit::EditOperation::AddNestedSequence { sequence_ref } => {
                seq.add_nested_sequence(sequence_ref.clone())?;
            }
            crate::edit::EditOperation::RemoveNestedSequence { sequence_id, .. } => {
                seq.remove_nested_sequence(*sequence_id);
            }
            crate::edit::EditOperation::Batch { operations, .. } => {
                for op in operations {
                    self.apply_operation(op)?;
                }
            }
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<Edit, crate::TimelineError> {
        let edit = self.history.pop()
            .ok_or(crate::TimelineError::InvalidOperation("Nothing to undo".to_string()))?;

        self.apply_inverse(&edit.operation)?;

        self.redo_stack.push(edit.clone());
        Ok(edit)
    }

    pub fn redo(&mut self) -> Result<Edit, crate::TimelineError> {
        let edit = self.redo_stack.pop()
            .ok_or(crate::TimelineError::InvalidOperation("Nothing to redo".to_string()))?;

        self.apply_operation(&edit.operation)?;
        self.history.push(edit.clone());
        Ok(edit)
    }

    fn apply_inverse(&self, operation: &EditOperation) -> Result<(), crate::TimelineError> {
        let mut seq = self.sequence.write();

        match operation {
            crate::edit::EditOperation::AddClip { track_id, .. } => {
                // Placeholder - would need to store clip_id for proper inverse
            }
            crate::edit::EditOperation::RemoveClip { track_id, clip_ref, .. } => {
                seq.get_track_mut(*track_id).ok_or(crate::TimelineError::TrackNotFound)?.add_clip(clip_ref.clone());
            }
            crate::edit::EditOperation::MoveClip { clip_id, from_track, old_start, .. } => {
                let mut found = false;
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.track_id = *from_track;
                        clip.timing.timeline_start = *old_start;
                        found = true;
                        break;
                    }
                }
                if !found {
                    for track in seq.tracks.values_mut() {
                        if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                            clip.track_id = *from_track;
                            clip.timing.timeline_start = *old_start;
                            break;
                        }
                    }
                }
            }
            crate::edit::EditOperation::TrimClip { clip_id, old_timing, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.timing = old_timing.clone();
                        break;
                    }
                }
            }
            crate::edit::EditOperation::ChangeSpeed { clip_id, old_speed, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.timing.speed = *old_speed;
                        break;
                    }
                }
            }
            crate::edit::EditOperation::AddEffect { clip_id, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.video_effects.pop();
                        break;
                    }
                }
            }
            crate::edit::EditOperation::RemoveEffect { clip_id, effect, .. } => {
                for track in seq.tracks.values_mut() {
                    if let Some(clip) = track.clips.iter_mut().find(|c| c.clip_id == *clip_id) {
                        clip.effects.video_effects.push(effect.clone());
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn history(&self) -> &[Edit] {
        &self.history
    }

    pub fn start_transaction(&mut self, description: String) -> EditTransaction {
        let transaction = EditTransaction::new(description);
        self.current_transaction = Some(transaction);
        self.current_transaction.as_ref().unwrap().clone()
    }

    pub fn commit_transaction(&mut self) -> Result<(), crate::TimelineError> {
        if let Some(transaction) = self.current_transaction.take() {
            for edit in transaction.edits {
                self.apply_edit(edit)?;
            }
            Ok(())
        } else {
            Err(crate::TimelineError::InvalidOperation("No active transaction".to_string()))
        }
    }

    pub fn cancel_transaction(&mut self) {
        self.current_transaction = None;
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

/// Undo/Redo manager with transaction support
pub struct UndoRedoManager {
    engine: AtomicEditEngine,
}

impl UndoRedoManager {
    pub fn new(sequence: Arc<parking_lot::RwLock<crate::sequence::Sequence>>) -> Self {
        Self {
            engine: AtomicEditEngine::new(sequence),
        }
    }

    pub fn apply_edit(&mut self, edit: Edit) -> Result<(), crate::TimelineError> {
        self.engine.apply_edit(edit)
    }

    pub fn apply_operation(&mut self, operation: EditOperation) -> Result<(), crate::TimelineError> {
        self.engine.apply_edit(Edit::new(operation))
    }

    pub fn undo(&mut self) -> Result<Edit, crate::TimelineError> {
        self.engine.undo()
    }

    pub fn redo(&mut self) -> Result<Edit, crate::TimelineError> {
        self.engine.redo()
    }

    pub fn can_undo(&self) -> bool {
        self.engine.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.engine.can_redo()
    }

    pub fn history(&self) -> &[Edit] {
        self.engine.history()
    }

    pub fn begin_transaction(&mut self, description: String) -> EditTransaction {
        self.engine.start_transaction(description)
    }

    pub fn commit_transaction(&mut self) -> Result<(), crate::TimelineError> {
        self.engine.commit_transaction()
    }

    pub fn cancel_transaction(&mut self) {
        self.engine.cancel_transaction()
    }
}

/// Factory functions for external use
pub fn create_atomic_engine(sequence: Arc<parking_lot::RwLock<crate::sequence::Sequence>>) -> AtomicEditEngine {
    AtomicEditEngine::new(sequence)
}

pub fn create_undo_redo_manager(sequence: Arc<parking_lot::RwLock<crate::sequence::Sequence>>) -> UndoRedoManager {
    UndoRedoManager::new(sequence)
}