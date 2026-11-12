// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Review round timeline snapshots and diff computation (TS4).
//!
//! Captures compact sequence snapshots (without recursively cloning `task_sheet`)
//! when opening or closing review rounds and isolates changed tick ranges across
//! clips, markers, caption cues, and graphics.

use crate::project::{Project, TrackKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Compact snapshot of a timeline clip captured at a review round boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundClipSnapshot {
    pub clip_id: u64,
    pub track_id: u64,
    pub media_id: u64,
    pub timeline_start: i64,
    pub source_in: i64,
    pub duration: i64,
    /// Clip playback speed scaled by 1000 so snapshots stay `Eq`-comparable.
    pub speed_milli: i64,
    pub enabled: bool,
    pub audio_enabled: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

impl RoundClipSnapshot {
    pub fn end_tick(&self) -> i64 {
        self.timeline_start.saturating_add(self.duration)
    }

    fn key(&self, fallback_index: usize) -> (u64, usize) {
        if self.clip_id != 0 {
            (self.clip_id, 0)
        } else {
            (self.track_id, fallback_index + 1)
        }
    }
}

/// Compact snapshot of a timeline track captured at a review round boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundTrackSnapshot {
    pub track_id: u64,
    pub kind: String,
    pub name: String,
    pub muted: bool,
    pub solo: bool,
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<RoundClipSnapshot>,
}

/// Compact snapshot of a marker captured at a review round boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundMarkerSnapshot {
    pub id: u64,
    pub name: String,
    pub start_ticks: i64,
    pub duration_ticks: i64,
}

/// Compact snapshot of a caption cue captured at a review round boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundCueSnapshot {
    pub track_id: u64,
    pub id: u64,
    pub start_ticks: i64,
    pub end_ticks: i64,
    pub text: String,
}

/// Compact snapshot of a graphic overlay captured at a review round boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundGraphicSnapshot {
    pub id: u64,
    pub name: String,
    pub start_ticks: i64,
    pub duration_ticks: i64,
    pub visible: bool,
}

/// Compact timeline snapshot stored per review round (`v1`, `v2`, …).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundTimelineSnapshot {
    pub head_revision: u64,
    pub duration_ticks: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<RoundTrackSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<RoundMarkerSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<RoundCueSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graphics: Vec<RoundGraphicSnapshot>,
}

/// Classification of a clip change between two review rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipDiffKind {
    Added,
    Removed,
    Modified,
}

/// Detailed clip delta inside a [`RoundDiffReport`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipDiffEntry {
    pub kind: ClipDiffKind,
    pub clip_id: u64,
    pub track_id: u64,
    pub start_tick: i64,
    pub end_tick: i64,
    pub description: String,
}

/// Individual time-bounded change isolated between two review rounds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedTickRange {
    pub start_tick: i64,
    pub end_tick: i64,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_id: Option<u64>,
    pub summary: String,
}

/// Structured diff comparing the current review round against a prior snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundDiffReport {
    pub from_round: u32,
    pub to_round: u32,
    pub from_revision: u64,
    pub to_revision: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_ranges: Vec<ChangedTickRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_changes: Vec<ClipDiffEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged_tick_ranges: Vec<[i64; 2]>,
    pub summary: String,
}

/// Capture a compact timeline snapshot from the current active sequence.
pub fn capture_timeline_snapshot(project: &Project, head_revision: u64) -> RoundTimelineSnapshot {
    let seq = &project.sequence;
    let tracks = seq
        .tracks
        .iter()
        .map(|track| RoundTrackSnapshot {
            track_id: track.id,
            kind: match track.kind {
                TrackKind::Video => "video".into(),
                TrackKind::Audio => "audio".into(),
            },
            name: track.name.clone(),
            muted: track.muted,
            solo: track.solo,
            locked: track.locked,
            clips: track
                .clips
                .iter()
                .map(|c| RoundClipSnapshot {
                    clip_id: c.id,
                    track_id: track.id,
                    media_id: c.media_id,
                    timeline_start: c.timeline_start.0,
                    source_in: c.source_in.0,
                    duration: c.duration.0,
                    speed_milli: (c.speed * 1000.0).round() as i64,
                    enabled: c.enabled,
                    audio_enabled: c.audio_enabled,
                    label: c.label.clone(),
                })
                .collect(),
        })
        .collect();

    let markers = seq
        .markers
        .iter()
        .map(|m| RoundMarkerSnapshot {
            id: m.id,
            name: m.name.clone(),
            start_ticks: m.start_ticks.0,
            duration_ticks: m.duration_ticks.0,
        })
        .collect();

    let mut cues = Vec::new();
    for ct in &seq.caption_tracks {
        for cue in &ct.cues {
            cues.push(RoundCueSnapshot {
                track_id: ct.id,
                id: cue.id,
                start_ticks: cue.start_ticks.0,
                end_ticks: cue.end_ticks.0,
                text: cue.text.clone(),
            });
        }
    }

    let graphics = seq
        .graphics
        .iter()
        .map(|g| RoundGraphicSnapshot {
            id: g.id,
            name: g.name.clone(),
            start_ticks: g.start_ticks.0,
            duration_ticks: g.duration_ticks.0,
            visible: g.visible,
        })
        .collect();

    RoundTimelineSnapshot {
        head_revision,
        duration_ticks: seq.duration().0,
        tracks,
        markers,
        cues,
        graphics,
    }
}

fn flatten_clips(snap: &RoundTimelineSnapshot) -> BTreeMap<(u64, usize), &RoundClipSnapshot> {
    let mut map = BTreeMap::new();
    for track in &snap.tracks {
        for (idx, clip) in track.clips.iter().enumerate() {
            map.insert(clip.key(idx), clip);
        }
    }
    map
}

fn intervals_overlap_or_touch(a_start: i64, a_end: i64, b_start: i64, b_end: i64) -> bool {
    a_start <= b_end && b_start <= a_end
}

fn merge_tick_intervals(ranges: &[ChangedTickRange]) -> Vec<[i64; 2]> {
    if ranges.is_empty() {
        return Vec::new();
    }
    let mut pairs: Vec<[i64; 2]> = ranges
        .iter()
        .map(|r| {
            let s = r.start_tick.min(r.end_tick);
            let e = r.start_tick.max(r.end_tick);
            [s, e]
        })
        .collect();
    pairs.sort_by_key(|p| (p[0], p[1]));
    let mut merged: Vec<[i64; 2]> = Vec::new();
    for [start, end] in pairs {
        if let Some(last) = merged.last_mut() {
            if start <= last[1] {
                if end > last[1] {
                    last[1] = end;
                }
                continue;
            }
        }
        merged.push([start, end]);
    }
    merged
}

/// Compute an isolated, accurate diff report between two review round snapshots.
pub fn compute_round_diff(
    from_round: u32,
    from: &RoundTimelineSnapshot,
    to_round: u32,
    to: &RoundTimelineSnapshot,
) -> RoundDiffReport {
    let mut changed_ranges = Vec::new();
    let mut clip_changes = Vec::new();

    diff_clips(from, to, &mut changed_ranges, &mut clip_changes);
    diff_markers(&from.markers, &to.markers, &mut changed_ranges);
    diff_cues(&from.cues, &to.cues, &mut changed_ranges);
    diff_graphics(&from.graphics, &to.graphics, &mut changed_ranges);

    changed_ranges.sort_by_key(|r| (r.start_tick, r.end_tick));
    let merged_tick_ranges = merge_tick_intervals(&changed_ranges);

    let summary = if changed_ranges.is_empty() {
        format!("No timeline changes between v{from_round} and v{to_round}")
    } else {
        format!(
            "{} change(s) across {} merged range(s) between v{from_round} and v{to_round}",
            changed_ranges.len(),
            merged_tick_ranges.len()
        )
    };

    RoundDiffReport {
        from_round,
        to_round,
        from_revision: from.head_revision,
        to_revision: to.head_revision,
        changed_ranges,
        clip_changes,
        merged_tick_ranges,
        summary,
    }
}

fn diff_clips(
    from: &RoundTimelineSnapshot,
    to: &RoundTimelineSnapshot,
    changed_ranges: &mut Vec<ChangedTickRange>,
    clip_changes: &mut Vec<ClipDiffEntry>,
) {
    let before = flatten_clips(from);
    let after = flatten_clips(to);
    let mut keys: BTreeSet<(u64, usize)> = before.keys().copied().collect();
    keys.extend(after.keys().copied());

    for key in keys {
        match (before.get(&key), after.get(&key)) {
            (None, Some(c)) => {
                let end = c.end_tick();
                let desc = format!("Clip {} added on track {}", c.clip_id, c.track_id);
                changed_ranges.push(ChangedTickRange {
                    start_tick: c.timeline_start,
                    end_tick: end,
                    kind: "clip_added".into(),
                    entity_id: Some(c.clip_id),
                    track_id: Some(c.track_id),
                    summary: desc.clone(),
                });
                clip_changes.push(ClipDiffEntry {
                    kind: ClipDiffKind::Added,
                    clip_id: c.clip_id,
                    track_id: c.track_id,
                    start_tick: c.timeline_start,
                    end_tick: end,
                    description: desc,
                });
            }
            (Some(c), None) => {
                let end = c.end_tick();
                let desc = format!("Clip {} removed from track {}", c.clip_id, c.track_id);
                changed_ranges.push(ChangedTickRange {
                    start_tick: c.timeline_start,
                    end_tick: end,
                    kind: "clip_removed".into(),
                    entity_id: Some(c.clip_id),
                    track_id: Some(c.track_id),
                    summary: desc.clone(),
                });
                clip_changes.push(ClipDiffEntry {
                    kind: ClipDiffKind::Removed,
                    clip_id: c.clip_id,
                    track_id: c.track_id,
                    start_tick: c.timeline_start,
                    end_tick: end,
                    description: desc,
                });
            }
            (Some(old_c), Some(new_c)) if old_c != new_c => {
                let old_end = old_c.end_tick();
                let new_end = new_c.end_tick();
                let desc = format!("Clip {} modified on track {}", new_c.clip_id, new_c.track_id);
                if intervals_overlap_or_touch(
                    old_c.timeline_start,
                    old_end,
                    new_c.timeline_start,
                    new_end,
                ) {
                    let s = old_c.timeline_start.min(new_c.timeline_start);
                    let e = old_end.max(new_end);
                    changed_ranges.push(ChangedTickRange {
                        start_tick: s,
                        end_tick: e,
                        kind: "clip_modified".into(),
                        entity_id: Some(new_c.clip_id),
                        track_id: Some(new_c.track_id),
                        summary: desc.clone(),
                    });
                    clip_changes.push(ClipDiffEntry {
                        kind: ClipDiffKind::Modified,
                        clip_id: new_c.clip_id,
                        track_id: new_c.track_id,
                        start_tick: s,
                        end_tick: e,
                        description: desc,
                    });
                } else {
                    // Disjoint move: isolate old and new footprints separately so untouched middle stays clean.
                    changed_ranges.push(ChangedTickRange {
                        start_tick: old_c.timeline_start,
                        end_tick: old_end,
                        kind: "clip_moved_from".into(),
                        entity_id: Some(new_c.clip_id),
                        track_id: Some(old_c.track_id),
                        summary: format!("{desc} (vacated region)"),
                    });
                    changed_ranges.push(ChangedTickRange {
                        start_tick: new_c.timeline_start,
                        end_tick: new_end,
                        kind: "clip_moved_to".into(),
                        entity_id: Some(new_c.clip_id),
                        track_id: Some(new_c.track_id),
                        summary: format!("{desc} (new region)"),
                    });
                    clip_changes.push(ClipDiffEntry {
                        kind: ClipDiffKind::Modified,
                        clip_id: new_c.clip_id,
                        track_id: new_c.track_id,
                        start_tick: new_c.timeline_start,
                        end_tick: new_end,
                        description: desc,
                    });
                }
            }
            _ => {}
        }
    }
}

fn diff_markers(
    before: &[RoundMarkerSnapshot],
    after: &[RoundMarkerSnapshot],
    changed_ranges: &mut Vec<ChangedTickRange>,
) {
    let old_map: BTreeMap<u64, &RoundMarkerSnapshot> =
        before.iter().map(|m| (m.id, m)).collect();
    let new_map: BTreeMap<u64, &RoundMarkerSnapshot> = after.iter().map(|m| (m.id, m)).collect();
    let mut ids: BTreeSet<u64> = old_map.keys().copied().collect();
    ids.extend(new_map.keys().copied());

    for id in ids {
        match (old_map.get(&id), new_map.get(&id)) {
            (None, Some(m)) => changed_ranges.push(ChangedTickRange {
                start_tick: m.start_ticks,
                end_tick: m.start_ticks.saturating_add(m.duration_ticks.max(1)),
                kind: "marker_added".into(),
                entity_id: Some(m.id),
                track_id: None,
                summary: format!("Marker `{}` added", m.name),
            }),
            (Some(m), None) => changed_ranges.push(ChangedTickRange {
                start_tick: m.start_ticks,
                end_tick: m.start_ticks.saturating_add(m.duration_ticks.max(1)),
                kind: "marker_removed".into(),
                entity_id: Some(m.id),
                track_id: None,
                summary: format!("Marker `{}` removed", m.name),
            }),
            (Some(o), Some(n)) if o != n => {
                let s = o.start_ticks.min(n.start_ticks);
                let e = o
                    .start_ticks
                    .saturating_add(o.duration_ticks.max(1))
                    .max(n.start_ticks.saturating_add(n.duration_ticks.max(1)));
                changed_ranges.push(ChangedTickRange {
                    start_tick: s,
                    end_tick: e,
                    kind: "marker_modified".into(),
                    entity_id: Some(n.id),
                    track_id: None,
                    summary: format!("Marker `{}` updated", n.name),
                });
            }
            _ => {}
        }
    }
}

fn diff_cues(
    before: &[RoundCueSnapshot],
    after: &[RoundCueSnapshot],
    changed_ranges: &mut Vec<ChangedTickRange>,
) {
    let old_map: BTreeMap<(u64, u64), &RoundCueSnapshot> =
        before.iter().map(|c| ((c.track_id, c.id), c)).collect();
    let new_map: BTreeMap<(u64, u64), &RoundCueSnapshot> =
        after.iter().map(|c| ((c.track_id, c.id), c)).collect();
    let mut keys: BTreeSet<(u64, u64)> = old_map.keys().copied().collect();
    keys.extend(new_map.keys().copied());

    for key in keys {
        match (old_map.get(&key), new_map.get(&key)) {
            (None, Some(c)) => changed_ranges.push(ChangedTickRange {
                start_tick: c.start_ticks,
                end_tick: c.end_ticks.max(c.start_ticks.saturating_add(1)),
                kind: "caption_added".into(),
                entity_id: Some(c.id),
                track_id: Some(c.track_id),
                summary: format!("Caption cue {} added", c.id),
            }),
            (Some(c), None) => changed_ranges.push(ChangedTickRange {
                start_tick: c.start_ticks,
                end_tick: c.end_ticks.max(c.start_ticks.saturating_add(1)),
                kind: "caption_removed".into(),
                entity_id: Some(c.id),
                track_id: Some(c.track_id),
                summary: format!("Caption cue {} removed", c.id),
            }),
            (Some(o), Some(n)) if o != n => {
                let s = o.start_ticks.min(n.start_ticks);
                let e = o
                    .end_ticks
                    .max(n.end_ticks)
                    .max(s.saturating_add(1));
                changed_ranges.push(ChangedTickRange {
                    start_tick: s,
                    end_tick: e,
                    kind: "caption_modified".into(),
                    entity_id: Some(n.id),
                    track_id: Some(n.track_id),
                    summary: format!("Caption cue {} updated", n.id),
                });
            }
            _ => {}
        }
    }
}

fn diff_graphics(
    before: &[RoundGraphicSnapshot],
    after: &[RoundGraphicSnapshot],
    changed_ranges: &mut Vec<ChangedTickRange>,
) {
    let old_map: BTreeMap<u64, &RoundGraphicSnapshot> =
        before.iter().map(|g| (g.id, g)).collect();
    let new_map: BTreeMap<u64, &RoundGraphicSnapshot> = after.iter().map(|g| (g.id, g)).collect();
    let mut ids: BTreeSet<u64> = old_map.keys().copied().collect();
    ids.extend(new_map.keys().copied());

    for id in ids {
        match (old_map.get(&id), new_map.get(&id)) {
            (None, Some(g)) => changed_ranges.push(ChangedTickRange {
                start_tick: g.start_ticks,
                end_tick: g.start_ticks.saturating_add(g.duration_ticks.max(1)),
                kind: "graphic_added".into(),
                entity_id: Some(g.id),
                track_id: None,
                summary: format!("Graphic `{}` added", g.name),
            }),
            (Some(g), None) => changed_ranges.push(ChangedTickRange {
                start_tick: g.start_ticks,
                end_tick: g.start_ticks.saturating_add(g.duration_ticks.max(1)),
                kind: "graphic_removed".into(),
                entity_id: Some(g.id),
                track_id: None,
                summary: format!("Graphic `{}` removed", g.name),
            }),
            (Some(o), Some(n)) if o != n => {
                let s = o.start_ticks.min(n.start_ticks);
                let e = o
                    .start_ticks
                    .saturating_add(o.duration_ticks.max(1))
                    .max(n.start_ticks.saturating_add(n.duration_ticks.max(1)));
                changed_ranges.push(ChangedTickRange {
                    start_tick: s,
                    end_tick: e,
                    kind: "graphic_modified".into(),
                    entity_id: Some(n.id),
                    track_id: None,
                    summary: format!("Graphic `{}` updated", n.name),
                });
            }
            _ => {}
        }
    }
}
