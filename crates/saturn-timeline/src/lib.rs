//! Enhanced Timeline Data Model
//!
//! Provides nested sequences, atomic edits, and advanced timeline features
//! inspired by the reference editor's 254B ticks/sec timeline model.

pub mod atomic;
pub mod clip;
pub mod edit;
pub mod sequence;
pub mod time;
pub mod track;

// Re-export everything from submodules
pub use atomic::{
    AtomicEditEngine, UndoRedoManager, create_atomic_engine, create_undo_redo_manager,
};
pub use clip::{
    AudioEffect, Clip, ClipEffects, ClipId, ClipMetadata, ClipRef, ClipSource, ClipTiming,
    ColorGrade, Transform, VideoEffect,
};
pub use edit::{Edit, EditId, EditOperation, EditTransaction};
pub use sequence::{
    Marker, MarkerType, Sequence, SequenceId, SequenceMetadata, SequenceRef, SequenceSettings,
    SequenceState,
};
pub use time::{
    ColorSpace, FrameRate, TICKS_PER_SECOND, Ticks, Timecode, TimelineError, TimelineTime,
};
pub use track::{BlendMode, Track, TrackId, TrackKind, TrackSettings, TrackState};
