// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Deterministic kinetic typography layout (Packet 6.2).
//!
//! Word timing drives a damped spring; face boxes choose a placement lane.
//! This module does no decoding or inference — face boxes and word timestamps
//! are supplied by the caller — and makes no claims about learned tracking.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedWord {
    pub text_start_tick: i64,
    pub text_end_tick: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticLayoutOptions {
    pub ticks_per_second: i64,
    pub stiffness: f32,
    pub damping: f32,
    pub lanes: u32,
    pub margin: f32,
}

impl Default for KineticLayoutOptions {
    fn default() -> Self {
        Self {
            ticks_per_second: 254_016_000_000,
            stiffness: 180.0,
            damping: 18.0,
            lanes: 3,
            margin: 0.03,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedWord {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub opacity: f32,
}

/// Damped spring response at `elapsed_seconds` after a word trigger.
/// Returns displacement (0..1) and velocity-normalised scale (1..1.2).
pub fn spring_response(elapsed: f32, stiffness: f32, damping: f32) -> (f32, f32) {
    if elapsed < 0.0 {
        return (0.0, 1.0);
    }
    let step = 1.0 / 240.0;
    let mut position = 0.0f32;
    let mut velocity = 0.0f32;
    let steps = ((elapsed / step).ceil() as usize).min(2400);
    for _ in 0..steps {
        let acceleration = stiffness * (1.0 - position) - damping * velocity;
        velocity += acceleration * step;
        position += velocity * step;
    }
    let scale = 1.0 + (velocity.abs().min(4.0) * 0.05);
    (position.clamp(0.0, 1.0), scale)
}

fn overlaps_face(lane_y: f32, faces: &[FaceBox], margin: f32) -> bool {
    faces.iter().any(|face| {
        lane_y + margin > face.y && lane_y - margin < face.y + face.height
    })
}

fn choose_lane(index: usize, faces: &[FaceBox], options: &KineticLayoutOptions) -> f32 {
    let lanes = options.lanes.max(1);
    let top = 0.12f32;
    let bottom = 0.88f32;
    let span = bottom - top;
    for offset in 0..lanes {
        let lane = (index as u32 + offset) % lanes;
        let y = top + span * lane as f32 / lanes as f32;
        if !overlaps_face(y, faces, options.margin) {
            return y;
        }
    }
    top + span * (index as u32 % lanes) as f32 / lanes as f32
}

/// Lay out the words that are visible at `playhead_tick`.
pub fn layout_kinetic_words(
    words: &[TimedWord],
    faces: &[FaceBox],
    playhead_tick: i64,
    options: &KineticLayoutOptions,
) -> Vec<PositionedWord> {
    let ticks_per_second = options.ticks_per_second.max(1) as f32;
    words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| {
            if playhead_tick < word.text_start_tick || playhead_tick > word.text_end_tick {
                return None;
            }
            let elapsed = (playhead_tick - word.text_start_tick) as f32 / ticks_per_second;
            let (arrival, scale) = spring_response(elapsed, options.stiffness, options.damping);
            Some(PositionedWord {
                index,
                x: 0.08 + arrival * 0.06,
                y: choose_lane(index, faces, options),
                scale,
                opacity: arrival,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_and_starts_at_rest() {
        let (start, scale) = spring_response(0.0, 180.0, 18.0);
        assert_eq!(start, 0.0);
        assert!(scale >= 1.0);
        let (settled, _) = spring_response(2.0, 180.0, 18.0);
        assert!((settled - 1.0).abs() < 0.01);
    }

    #[test]
    fn face_boxes_push_words_to_another_lane() {
        let words = [TimedWord { text_start_tick: 0, text_end_tick: 100 }];
        let faces = [FaceBox { x: 0.2, y: 0.10, width: 0.2, height: 0.25 }];
        let options = KineticLayoutOptions {
            ticks_per_second: 100,
            lanes: 3,
            ..Default::default()
        };
        let positioned = layout_kinetic_words(&words, &faces, 100, &options);
        assert_eq!(positioned.len(), 1);
        assert!(positioned[0].y > 0.4, "word should avoid the face lane");
    }
}
