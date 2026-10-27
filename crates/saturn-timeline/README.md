# saturn-timeline

Professional NLE timeline library for Easy Edits Pro, inspired by the reference editor's architecture.

## Features

- **254,016,000,000 ticks/sec** — Exact rational timebase divisible by all frame rates
- **Nested sequences** — Full composition support with SequenceRef
- **Atomic edits** — Transactional undo/redo with EditOperation, Edit, EditTransaction
- **Track kinds** — Video, Audio, Subtitle, Submix with blend modes
- **Clip timing** — Variable speed, reverse, frame blending, trim handles
- **Effects stack** — Video/audio effects, color grade (Lift/Gamma/Gain/Offset), transforms
- **Markers** — Chapter, WebLink, FlashCue, Comment, SyncPoint

## Modules

| Module | Purpose |
|--------|---------|
| `time` | FrameRate, Timecode, TimelineTime, Ticks, TICKS_PER_SECOND, TimelineError, ColorSpace |
| `sequence` | Sequence, SequenceRef, SequenceSettings, SequenceState, SequenceMetadata, Marker |
| `track` | Track, TrackId, TrackKind, TrackSettings, TrackState, BlendMode |
| `clip` | Clip, ClipId, ClipRef, ClipSource, ClipTiming, ClipEffects, VideoEffect, AudioEffect, ColorGrade, Transform, ClipMetadata |
| `edit` | EditOperation, Edit, EditTransaction, EditId |
| `atomic` | AtomicEditEngine, UndoRedoManager |

## Timebase

The timeline uses a fixed integer tick rate of **254,016,000,000 ticks/second** (reference-editor resolution). This value is evenly divisible by:

- Video frame rates: 23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60 fps
- Audio sample rates: 44.1kHz, 48kHz, 96kHz, 192kHz

This eliminates accumulated frame drift over long sequences.

## Usage

```rust
use saturn_timeline::{
    Sequence, SequenceSettings, Track, TrackKind, TrackId,
    Clip, ClipSource, ClipId, ClipRef,
    FrameRate, Timecode, Ticks, TICKS_PER_SECOND,
    EditOperation, Edit, AtomicEditEngine, UndoRedoManager,
};
use std::sync::Arc;
use parking_lot::RwLock;

// Create a sequence
let settings = SequenceSettings {
    name: "My Project".to_string(),
    frame_rate: FrameRate::Fps30,
    resolution: (1920, 1080),
    ..Default::default()
};
let mut sequence = Sequence::new(settings);

// Add tracks
let video_track = Track::new(TrackKind::Video, 0);
let audio_track = Track::new(TrackKind::Audio, 0);
sequence.add_track(video_track);
sequence.add_track(audio_track);

// Create a clip
let clip = Clip::new(ClipSource::Placeholder { width: 1920, height: 1080, color: "#000000".to_string() });

// Add clip to timeline via atomic edit engine
let engine = Arc::new(RwLock::new(sequence));
let mut undo_manager = UndoRedoManager::new(engine.clone());

let clip_ref = clip.create_ref(0, TICKS_PER_SECOND * 10); // 10 seconds
undo_manager.apply_operation(EditOperation::AddClip {
    track_id: video_track.id,
    clip_ref,
})?;
```

## Running Tests

```bash
cargo test -p saturn-timeline
```

## Architecture

Each type is defined **exactly once** in its module file. The `lib.rs` only re-exports:

```rust
// lib.rs - ONLY re-exports
pub use time::{TimelineTime, Timecode, FrameRate, Ticks, TICKS_PER_SECOND, TimelineError, ColorSpace};
pub use sequence::{Sequence, SequenceId, SequenceRef, SequenceSettings, SequenceState, SequenceMetadata, Marker, MarkerType};
pub use track::{Track, TrackId, TrackKind, TrackSettings, TrackState, BlendMode};
pub use clip::{Clip, ClipId, ClipRef, ClipSource, ClipTiming, ClipEffects, VideoEffect, AudioEffect, ColorGrade, Transform, ClipMetadata};
pub use edit::{EditOperation, Edit, EditTransaction, EditId};
pub use atomic::{AtomicEditEngine, UndoRedoManager, create_atomic_engine, create_undo_redo_manager};
```

No duplicate definitions, no `IndexMap` (uses `Vec` for serde compatibility).