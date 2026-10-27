//! Timeline range-edit operations (lift semantics for marked ranges).
//!
//! Extracted from `speech.rs` to keep the speech-specific caption
//! synchronisation separate from the generic clip range cutting.

use crate::project::{Clip, Sequence};
use crate::time::Tick;
use serde::{Deserialize, Serialize};

/// Summary report returned after a multi-range lift operation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiftRangeReport {
    pub lifted_ranges_count: usize,
    pub tracks_affected: usize,
    pub clips_modified: usize,
}

/// Normalizes, sorts, and merges overlapping or adjacent time ranges.
pub fn merge_time_ranges(ranges: &[(Tick, Tick)]) -> Vec<(Tick, Tick)> {
    if ranges.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<(i64, i64)> = ranges
        .iter()
        .filter_map(|(s, e)| {
            let start = s.0.min(e.0);
            let end = s.0.max(e.0);
            if end > start {
                Some((start, end))
            } else {
                None
            }
        })
        .collect();

    sorted.sort_by_key(|r| r.0);

    let mut merged: Vec<(i64, i64)> = Vec::with_capacity(sorted.len());
    for (start, end) in sorted {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        } else {
            merged.push((start, end));
        }
    }

    merged
        .into_iter()
        .map(|(s, e)| (Tick(s), Tick(e)))
        .collect()
}

/// Cuts a track's clips against merged deleted ranges, leaving surviving
/// fragments at their original positions (gaps are preserved). Returns the
/// surviving fragments, whether anything changed, and how many fragments
/// intersected a range.

pub(crate) fn cut_track_clips_against_ranges(
    clips: Vec<Clip>,
    merged: &[(Tick, Tick)],
) -> (Vec<Clip>, bool, usize) {
    let mut modified = 0usize;
    let mut had_changes = false;
    let mut out: Vec<Clip> = Vec::new();

    for clip in clips {
        let mut clip_fragments = vec![clip];

        for &(range_start, range_end) in merged {
            let mut next_fragments = Vec::new();
            for frag in clip_fragments {
                let frag_start = frag.timeline_start.0;
                let frag_dur = frag.duration.0;
                let frag_end = frag_start + frag_dur;

                let r_start = range_start.0;
                let r_end = range_end.0;

                // Case 1: Frag is completely outside range
                if frag_end <= r_start || frag_start >= r_end {
                    next_fragments.push(frag);
                    continue;
                }

                had_changes = true;
                modified += 1;

                // Case 2: Frag completely inside range -> eliminated entirely
                if frag_start >= r_start && frag_end <= r_end {
                    continue;
                }

                // Case 3: Left piece survives (starts before range)
                if frag_start < r_start {
                    let left_dur = r_start - frag_start;
                    let mut left = frag.clone();
                    left.duration = Tick(left_dur);
                    next_fragments.push(left);
                }

                // Case 4: Right piece survives (ends after range)
                if frag_end > r_end {
                    let right_dur = frag_end - r_end;
                    let right_src_offset = r_end - frag_start;
                    let mut right = frag.clone();
                    right.timeline_start = Tick(r_end);
                    right.source_in = Tick(frag.source_in.0 + right_src_offset);
                    right.duration = Tick(right_dur);
                    next_fragments.push(right);
                }
            }
            clip_fragments = next_fragments;
        }

        out.extend(clip_fragments);
    }

    (out, had_changes, modified)
}

/// Performs a multi-range lift across all unlocked tracks in a sequence.
///
/// Cuts clips intersecting any lifted interval and removes the content inside
/// the ranges, but unlike [`ripple_delete_time_ranges`] the gaps are preserved:
/// later clips keep their positions and caption cues stay untouched.
pub fn lift_time_ranges(
    sequence: &mut Sequence,
    ranges: &[(Tick, Tick)],
) -> Result<LiftRangeReport, String> {
    let merged = merge_time_ranges(ranges);
    if merged.is_empty() {
        return Ok(LiftRangeReport {
            lifted_ranges_count: 0,
            tracks_affected: 0,
            clips_modified: 0,
        });
    }

    let mut tracks_affected = 0;
    let mut clips_modified = 0;

    for track in &mut sequence.tracks {
        if track.locked {
            continue;
        }

        let (mut new_clips, track_had_changes, modified) =
            cut_track_clips_against_ranges(std::mem::take(&mut track.clips), &merged);
        clips_modified += modified;

        if track_had_changes {
            tracks_affected += 1;
        }

        new_clips.sort_by_key(|c| c.timeline_start.0);
        track.clips = new_clips;
    }

    Ok(LiftRangeReport {
        lifted_ranges_count: merged.len(),
        tracks_affected,
        clips_modified,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{CaptionTrack, SubtitleItem, Track, TrackKind};

    #[test]
    fn test_multi_range_lift_preserves_gaps() {
        // The same 0..10s single-clip fixture as the ripple-delete test.
        let clip = Clip {
            media_id: 1,
            timeline_start: Tick::from_millis(0),
            source_in: Tick::from_millis(0),
            duration: Tick::from_millis(10_000),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
                label: String::new(),
                volume_db: 0.0,
        };

        let mut seq = Sequence {
            id: 1,
            name: "Main".to_string(),
            track_groups: Vec::new(),
            tracks: vec![Track {
                id: 1,
                group_id: None,
                kind: TrackKind::Video,
                name: "V1".to_string(),
                gain_db: 0.0,
                pan: 0.0,
                muted: false,
                solo: false,
                locked: false,
                audio_processing: Default::default(),
                bus: Default::default(),
                automation: Default::default(),
                clips: vec![clip],
                transitions: Vec::new(),
            }],
            mark_in: None,
            mark_out: None,
            graphics: Vec::new(),
            caption_tracks: vec![CaptionTrack {
                id: 1,
                name: "Captions".into(),
                language: "en".into(),
                visible: true,
                cues: vec![SubtitleItem {
                    id: 1,
                    start_ticks: Tick::from_millis(6000),
                    end_ticks: Tick::from_millis(6500),
                    text: "Kept in place".to_string(),
                    speaker: None,
                    style_id: None,
                }],
                styles: Vec::new(),
            }],
            markers: Vec::new(),
            multicam_clips: Vec::new(),
            buses: Vec::new(),
            node_graph: None,
        };

        let ranges = vec![
            (Tick::from_millis(3000), Tick::from_millis(5000)),
            (Tick::from_millis(7000), Tick::from_millis(8000)),
        ];

        let report = lift_time_ranges(&mut seq, &ranges).expect("lift succeeds");
        assert_eq!(report.lifted_ranges_count, 2);
        assert_eq!(report.tracks_affected, 1);
        assert!(report.clips_modified >= 1);

        // Fragments keep their timeline positions: gaps stay open.
        // 1: 0..3000 (source 0), 2: 5000..7000 (source 5000),
        // 3: 8000..10000 (source 8000).
        let clips = &seq.tracks[0].clips;
        assert_eq!(clips.len(), 3);

        assert_eq!(clips[0].timeline_start.as_millis(), 0);
        assert_eq!(clips[0].duration.as_millis(), 3000);
        assert_eq!(clips[0].source_in.as_millis(), 0);

        assert_eq!(clips[1].timeline_start.as_millis(), 5000);
        assert_eq!(clips[1].duration.as_millis(), 2000);
        assert_eq!(clips[1].source_in.as_millis(), 5000);

        assert_eq!(clips[2].timeline_start.as_millis(), 8000);
        assert_eq!(clips[2].duration.as_millis(), 2000);
        assert_eq!(clips[2].source_in.as_millis(), 8000);

        // Sequence duration is unchanged (gaps preserved).
        assert_eq!(seq.duration().as_millis(), 10_000);

        // Caption cues are untouched by a lift.
        let cues = &seq.caption_tracks[0].cues;
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_ticks.as_millis(), 6000);
        assert_eq!(cues[0].end_ticks.as_millis(), 6500);

        // An empty lift is a no-op that reports zero activity.
        let before = seq.tracks[0].clips.len();
        let empty = lift_time_ranges(&mut seq, &[]).expect("empty lift succeeds");
        assert_eq!(empty.lifted_ranges_count, 0);
        assert_eq!(empty.tracks_affected, 0);
        assert_eq!(seq.tracks[0].clips.len(), before);
    }

}
