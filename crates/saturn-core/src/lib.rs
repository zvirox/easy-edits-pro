// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
pub mod catalogue;
pub mod actor;
pub mod brief;
pub mod audio_edit;
pub mod asymmetric_trim;
pub mod match_cut;
pub mod beat_grid;
pub mod branch;
pub mod color;
pub mod command;
pub mod engine;
pub mod graphics_layout;
pub mod interop;
pub mod job;
pub mod journal;
pub mod local_control;
pub mod lock;
pub mod media_fingerprint;
pub mod markers;
pub mod project;
pub mod script_sync;
pub mod speech;
pub mod timeline_edits;
pub mod subtitles;
pub mod time;
pub mod verification;
pub mod viewport_state;

pub use actor::{ActorContext, ActorId, SessionViewState};
pub use brief::{BriefSource, ClipAgentNote, ConfidenceLevel, HumanCorrection, ProjectBrief};
pub use command::{
    Category, CommandRequest, CommandSpec, CommandStatus, MENU_BAR, UndoGroup,
    catalogue_json, catalogue_markdown, command_catalogue, find_command, menu_commands,
    search_commands, validate_catalogue,
};
pub use engine::{DispatchError, EditorEngine};
pub use journal::{ChangeJournal, ChangeRecord};
pub use lock::{LockOwner, LockRecord, LockRegistry, LockScope};
pub use interop::{
    export_cmx3600_edl, export_fcpxml, export_otio, export_xmeml_xml, import_cmx3600_edl,
    import_fcpxml, import_otio, import_xmeml_xml, ticks_to_timecode, timecode_to_ticks,
};
pub use job::{
    CancellationToken, JobContext, JobId, JobInfo, JobKind, JobListener, JobManager, JobPriority,
    JobState,
};
pub use project::{
    AudioBus, AutomationInterpolation, AutomationLane, AutomationMode,
    AutomationPoint, BusTarget, CaptionTrack, ChromaKeyParams, Clip, ColorAdjustments, ColorCorrectionParams,
    ColorWheelParam, ColorWheels, CropParams, EffectInstance, EffectKind, EffectPayload,
    GaussianBlurParams, InterpolationKind, Keyframe, KeyframeProperty, LutParams, Marker,
    MarkerKind, MediaAsset, MediaKind, MotionTrackingData, MultiCamAngle, MultiCamClip,
    MultiCamSyncMode, ProjectDocument, ProjectSettings, Sequence, SubtitleItem, TitleProperties,
    Track, TrackAudioProcessing, TrackAutomation, TrackKind, TrackPoint, TrackingMode, Transition,
    TransitionKind, VignetteParams,
};
pub use script_sync::{
    SCRIPT_LINE_MATCH_FLOOR, ScriptLineMatch, ScriptSyncReport, ScriptTake, TakeAlignment,
    align_script_line_to_words, parse_script_lines, sync_script,
};
pub use speech::{
    RippleDeleteReport, SilenceDetectOptions, SilenceInterval, SpeechSegment, SpeechWord,
    TranscriptionOptions, detect_filler_words, detect_silence_from_peaks, extract_filler_intervals,
    is_filler_word, ripple_delete_time_ranges, speech_segments_to_subtitles,
};
pub use time::{FrameRate, TICKS_PER_SECOND, Tick};
pub use verification::{
    AnalyzedRange, CheckEvidence, CheckStatus, CheckVerdict, TickRange, VerifyNotes,
    VerifyParams, VerifyPreset, VerifyReport, VerifySummary, run_verification,
};
