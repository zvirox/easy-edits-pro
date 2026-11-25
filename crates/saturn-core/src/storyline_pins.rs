// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Magnetic Storyline Pins (Packet 5.1).
//!
//! A secondary clip (B-roll, title, SFX) can be pinned to a parent clip with a
//! fixed tick offset. Ripple edits that move the parent move every descendant
//! by the same delta. Links live on the project document so they survive
//! save/reopen and do not force a schema version bump (`#[serde(default)]`).

use crate::{Tick, project::ProjectDocument, project::Sequence};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorylinePin {
    pub child_clip_id: u64,
    pub parent_clip_id: u64,
    pub offset_ticks: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PinReport {
    pub child_clip_id: u64,
    pub parent_clip_id: u64,
    pub offset_ticks: i64,
    pub replaced_pins: usize,
}

fn all_clip_ids(document: &ProjectDocument) -> Result<Vec<u64>, String> {
    let mut ids: Vec<u64> = document
        .project
        .sequence
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter().map(|clip| clip.id))
        .collect();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1] || pair[0] == 0) {
        return Err("clips need stable, unique ids before pinning".into());
    }
    Ok(ids)
}

/// Pin the child clip to the parent clip at their current relative offset.
pub fn pin_clip(
    document: &mut ProjectDocument,
    child_track_id: u64,
    child_clip_index: usize,
    parent_track_id: u64,
    parent_clip_index: usize,
) -> Result<PinReport, String> {
    let ids = all_clip_ids(document)?;
    let sequence = &document.project.sequence;
    let child_track_index = sequence
        .tracks
        .iter()
        .position(|track| track.id == child_track_id)
        .ok_or("child track not found")?;
    let parent_track_index = sequence
        .tracks
        .iter()
        .position(|track| track.id == parent_track_id)
        .ok_or("parent track not found")?;
    let child = sequence
        .tracks
        .get(child_track_index)
        .and_then(|track| track.clips.get(child_clip_index))
        .ok_or("child clip not found")?;
    let parent = sequence
        .tracks
        .get(parent_track_index)
        .and_then(|track| track.clips.get(parent_clip_index))
        .ok_or("parent clip not found")?;
    if child_track_index == parent_track_index {
        return Err("a storyline pin must cross tracks".into());
    }
    let child_id = child.id;
    let parent_id = parent.id;
    if child_id == parent_id {
        return Err("a clip cannot be pinned to itself".into());
    }
    let offset = child
        .timeline_start
        .0
        .checked_sub(parent.timeline_start.0)
        .ok_or("pin offset arithmetic overflow")?;
    if offset < 0 || offset > parent.duration.0 {
        return Err("child clip must start inside the parent clip's span".into());
    }

    // Pin links form a tree: walk up from the prospective parent and refuse a
    // link that would make the child its own ancestor.
    let mut current = parent_id;
    let mut steps = 0usize;
    while steps <= ids.len() {
        steps += 1;
        if current == child_id {
            return Err("storyline pins cannot form a cycle".into());
        }
        let Some(next) = document
            .storyline_pins
            .iter()
            .find(|pin| pin.child_clip_id == current)
            .map(|pin| pin.parent_clip_id)
        else {
            break;
        };
        current = next;
    }
    if steps > ids.len() {
        return Err("storyline pins cannot form a cycle".into());
    }

    let existing = document
        .storyline_pins
        .iter_mut()
        .find(|pin| pin.child_clip_id == child_id);
    let replaced = if let Some(pin) = existing {
        pin.parent_clip_id = parent_id;
        pin.offset_ticks = offset;
        1
    } else {
        document.storyline_pins.push(StorylinePin {
            child_clip_id: child_id,
            parent_clip_id: parent_id,
            offset_ticks: offset,
        });
        0
    };

    Ok(PinReport {
        child_clip_id: child_id,
        parent_clip_id: parent_id,
        offset_ticks: offset,
        replaced_pins: replaced,
    })
}

/// Move every descendant of the given clips by the same delta. Returns the
/// number of child clips moved (not counting the originating parents).
pub fn propagate_pins(sequence: &mut Sequence, pins: &[StorylinePin], moves: &[(u64, i64)]) -> usize {
    let mut pending: Vec<(u64, i64)> = moves.to_vec();
    let mut moved = 0usize;
    let budget = sequence
        .tracks
        .iter()
        .map(|track| track.clips.len())
        .sum::<usize>()
        .saturating_mul(4)
        .max(1);

    while let Some((parent_id, delta)) = pending.pop() {
        if delta == 0 {
            continue;
        }
        for track in sequence.tracks.iter_mut() {
            if track.locked {
                continue;
            }
            for clip in track.clips.iter_mut() {
                if pins.iter().any(|pin| pin.child_clip_id == clip.id && pin.parent_clip_id == parent_id) {
                    let old_start = clip.timeline_start.0;
                    clip.timeline_start = Tick(old_start.saturating_add(delta));
                    for transition in track.transitions.iter_mut() {
                        if transition.cut_at.0 >= old_start
                            && transition.cut_at.0 <= old_start.saturating_add(clip.duration.0)
                        {
                            transition.cut_at.0 = transition.cut_at.0.saturating_add(delta);
                        }
                    }
                    pending.push((clip.id, delta));
                    moved += 1;
                }
            }
        }
        if moved > budget {
            break;
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: u64, media_id: u64, start: i64, duration: i64) -> crate::Clip {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "media_id": media_id,
            "timeline_start": start,
            "source_in": 0,
            "duration": duration,
        }))
        .expect("clip fixture")
    }

    fn two_track_document() -> ProjectDocument {
        let mut document = ProjectDocument::new("Pins");
        document.project.sequence.tracks[0].clips.push(clip(1, 1, 100, 200));
        if document.project.sequence.tracks.len() < 2 {
            document.project.sequence.tracks.push(
                serde_json::from_value(serde_json::json!({
                    "id": 2, "kind": "video", "name": "V2", "clips": [], "transitions": []
                }))
                .expect("track fixture"),
            );
        }
        document.project.sequence.tracks[1].clips.push(clip(2, 2, 140, 60));
        document
    }

    #[test]
    fn pins_a_child_at_its_relative_offset() {
        let mut document = two_track_document();
        let report = pin_clip(&mut document, 2, 0, 1, 0).expect("pin");
        assert_eq!(report.offset_ticks, 40);
        assert_eq!(
            document.storyline_pins,
            vec![StorylinePin {
                child_clip_id: 2,
                parent_clip_id: 1,
                offset_ticks: 40,
            }]
        );
    }

    #[test]
    fn ripple_moves_pinned_descendants() {
        let mut document = two_track_document();
        pin_clip(&mut document, 2, 0, 1, 0).expect("pin");
        let moved = propagate_pins(
            &mut document.project.sequence,
            &document.storyline_pins,
            &[(1, -25)],
        );
        assert_eq!(moved, 1);
        assert_eq!(
            document.project.sequence.tracks[1].clips[0].timeline_start,
            Tick(115)
        );
    }

    #[test]
    fn refuses_a_self_pin() {
        let mut document = two_track_document();
        assert!(pin_clip(&mut document, 1, 0, 1, 0).is_err());
    }
}
