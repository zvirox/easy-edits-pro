// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Unified playhead, selection and export-queue state owned by the engine
//! (AI-native plan Packet 0.1).
//!
//! These are the surfaces that previously lived only in the React state
//! ("handled by the interface"): playhead position, clip/gap selection, bin
//! selection, targeting and the export queue. They move behind
//! `EditorEngine` so MCP, the CLI and the desktop UI all read and mutate the
//! exact same deterministic state. Undo is *not* recorded: this is view &
//! queue state, not project content.

use crate::project::Sequence;
use crate::time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Workspaces the UI can be parked in; must mirror the `workspace.*` ids
/// in the catalogue (window.rs).
pub const WORKSPACES: &[&str] = &[
    "media", "edit", "fusion", "color", "audio", "effects", "keyframes", "deliver",
];

/// Upper bound on queued export entries; malformed agent macros must not be
/// able to grow unbounded state.
pub const MAX_EXPORT_QUEUE_ITEMS: usize = 500;

/// A selected timeline clip reference (`track_id`, position in the track).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedClipRef {
    pub track_id: u64,
    pub clip_index: usize,
}

/// A selected empty timeline interval on one track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedGap {
    pub track_id: u64,
    pub start_tick: Tick,
    pub end_tick: Tick,
}

/// Lifecycle of one queued export entry (state machine, not execution:
/// the actual GStreamer render stays in the shell's JobManager).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportItemState {
    #[default]
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    /// Render stopped by request; retry moves it back to `queued`.
    Cancelled,
}

/// One entry in the engine-owned export queue. `output` and `error` carry
/// the terminal-state payloads next to the flat `state` discriminant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportQueueItem {
    pub id: u64,
    pub created_at_unix_ms: u64,
    pub output_path: String,
    pub settings: Value,
    #[serde(default)]
    pub state: ExportItemState,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ExportQueueItem {
    pub(crate) fn new(id: u64, output_path: String, settings: Value) -> Self {
        Self {
            id,
            created_at_unix_ms: now_epoch_ms(),
            output_path,
            settings,
            state: ExportItemState::Queued,
            output: None,
            error: None,
        }
    }
}

/// Wall-clock epoch ms for queue entry creation stamps.
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

/// Absorbs the former ui_only playhead/selection/queue commands into
/// deterministic engine state (spec §4.1). Targeted video/audio tracks stay
/// owned by the engine's existing `TargetingState` (single source of truth);
/// the snapshot merges them in.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedViewportState {
    pub playhead_tick: Tick,
    pub ai_ghost_playhead_tick: Option<Tick>,
    pub selected_clips: Vec<SelectedClipRef>,
    pub selected_gap: Option<SelectedGap>,
    pub selected_media_ids: Vec<u64>,
    pub active_workspace: String,
    pub active_branch: String,
    pub selection_follows_playhead: bool,
    pub export_queue_paused: bool,
    pub export_queue: Vec<ExportQueueItem>,
    pub next_export_item_id: u64,
}

impl UnifiedViewportState {
    pub fn new() -> Self {
        Self {
            active_workspace: "edit".into(),
            active_branch: "main".into(),
            next_export_item_id: 1,
            ..Default::default()
        }
    }

    pub fn validate_workspace(workspace: &str) -> Result<String, String> {
        let workspace = workspace.trim().to_lowercase();
        if workspace.is_empty() {
            return Err("Active workspace cannot be empty".into());
        }
        if WORKSPACES.contains(&workspace.as_str()) {
            Ok(workspace)
        } else {
            Err(format!(
                "Unknown workspace `{workspace}`, expected one of: {}",
                WORKSPACES.join(", ")
            ))
        }
    }

    /// Clamp to a merged, sorted edit-point timeline; out-of-range playhead
    /// positions saturate to `[0, duration]`.
    pub fn set_playhead(&mut self, tick: Tick, duration: Tick) {
        self.playhead_tick = if tick.0 < 0 {
            Tick(0)
        } else if tick.0 > duration.0 {
            duration
        } else {
            tick
        };
    }

    /// JSON snapshot returned to MCP, the CLI and the desktop UI. Targeted
    /// tracks and workspace-aware extras merge in from the engine side.
    pub fn snapshot(
        &self,
        targeted_video_tracks: &[u64],
        targeted_audio_tracks: &[u64],
    ) -> Value {
        json!({
            "playhead_tick": self.playhead_tick.0,
            "ai_ghost_playhead_tick": self.ai_ghost_playhead_tick.map(|t| t.0),
            "selected_clips": self.selected_clips.iter().map(|clip| json!({
                "track_id": clip.track_id,
                "clip_index": clip.clip_index,
            })).collect::<Vec<_>>(),
            "selected_gap": self.selected_gap.map(|gap| json!({
                "track_id": gap.track_id,
                "start_tick": gap.start_tick.0,
                "end_tick": gap.end_tick.0,
            })),
            "selected_media_ids": self.selected_media_ids,
            "active_workspace": self.active_workspace,
            "active_branch": self.active_branch,
            "targeted_video_tracks": targeted_video_tracks,
            "targeted_audio_tracks": targeted_audio_tracks,
            "selection_follows_playhead": self.selection_follows_playhead,
            "export_queue_paused": self.export_queue_paused,
            "export_queue": self.export_queue,
        })
    }
}

/// Sorted unique cut points (clip starts and ends) across every track.
pub fn edit_points(sequence: &Sequence) -> Vec<i64> {
    let mut points: Vec<i64> = sequence
        .tracks
        .iter()
        .flat_map(|track| {
            track.clips.iter().flat_map(|clip| {
                [clip.timeline_start.0, clip.timeline_start.saturating_add(clip.duration.0).0]
            })
        })
        .collect();
    points.sort_unstable();
    points.dedup();
    points
}

/// Duration of the active sequence as a saturated tick ceiling.
pub fn sequence_duration(sequence: &Sequence) -> Tick {
    Tick(
        sequence
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .map(|clip| clip.timeline_start.saturating_add(clip.duration.0).0)
            .max()
            .unwrap_or(0),
    )
}

/// The clip under the playhead on the topmost unlocked video track, skipping
/// disabled clips (parity with the UI's `findClipAtPlayhead`).
pub fn find_clip_at_playhead(sequence: &Sequence, playhead: Tick) -> Option<SelectedClipRef> {
    for track in &sequence.tracks {
        if track.kind != crate::project::TrackKind::Video {
            continue;
        }
        for (clip_index, clip) in track.clips.iter().enumerate() {
            if !clip.enabled {
                continue;
            }
            let end = clip.timeline_start.saturating_add(clip.duration.0).0;
            if playhead.0 >= clip.timeline_start.0 && playhead.0 < end {
                return Some(SelectedClipRef {
                    track_id: track.id,
                    clip_index,
                });
            }
        }
    }
    None
}

/// Frame length in ticks for the project frame rate.
pub fn frame_ticks(frame_rate: &FrameRate) -> i64 {
    frame_rate.frame_ticks().unwrap_or(10_584_000_000).max(1)
}
