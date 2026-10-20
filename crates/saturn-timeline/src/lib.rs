//! Enhanced Timeline Data Model
//!
//! Provides nested sequences, atomic edits, and advanced timeline features
//! inspired by FilmCraft's 254B ticks/sec timeline model.

pub mod time;
pub mod sequence;
pub mod track;
pub mod clip;
pub mod edit;
pub mod atomic;

// Re-export everything from submodules
pub use time::{
    TimelineTime, Timecode, FrameRate, Ticks, TICKS_PER_SECOND,
    TimelineError, ColorSpace,
};
pub use sequence::{
    Sequence, SequenceId, SequenceRef, SequenceSettings, SequenceState,
    SequenceMetadata, Marker, MarkerType,
};
pub use track::{Track, TrackId, TrackKind, TrackSettings, TrackState, BlendMode};
pub use clip::{
    Clip, ClipId, ClipRef, ClipSource, ClipTiming, ClipEffects,
    VideoEffect, AudioEffect, ColorGrade, Transform, ClipMetadata,
};
pub use edit::{
    EditOperation, Edit, EditTransaction, EditId,
};
pub use atomic::{AtomicEditEngine, UndoRedoManager, create_atomic_engine, create_undo_redo_manager};

use saturn_commands::{CommandId, CommandSpec, CommandCategory, CommandPriority, ExecutionMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::sync::Arc;
use parking_lot::RwLock;
use indexmap::IndexMap;
use smallvec::SmallVec;
use chrono;