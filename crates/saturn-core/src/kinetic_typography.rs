// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Kinetic typography settings (Packet 6.2). Word timing and face boxes come
//! from the caller; layout maths lives in `saturn-text::kinetic`.

use crate::{Tick, project::ProjectDocument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KineticWord {
    pub text: String,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct KineticFaceBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct KineticStyle {
    pub font_size: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub lanes: u32,
}

impl Default for KineticStyle {
    fn default() -> Self {
        Self {
            font_size: 48.0,
            stiffness: 180.0,
            damping: 18.0,
            lanes: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipKineticTypography {
    pub clip_id: u64,
    pub words: Vec<KineticWord>,
    pub faces: Vec<KineticFaceBox>,
    pub style: KineticStyle,
    pub start_ticks: Tick,
    pub duration_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct KineticReport {
    pub clip_id: u64,
    pub word_count: usize,
    pub face_count: usize,
}

pub fn configure_kinetic_typography(
    document: &mut ProjectDocument,
    track_id: u64,
    clip_index: usize,
    words: Vec<KineticWord>,
    faces: Vec<KineticFaceBox>,
    style: KineticStyle,
) -> Result<KineticReport, String> {
    if words.is_empty() {
        return Err("kinetic typography needs at least one timed word".into());
    }
    if words.windows(2).any(|pair| {
        pair[0].start_ticks.0 >= pair[0].end_ticks.0
            || pair[0].end_ticks.0 > pair[1].start_ticks.0
    }) || words.last().is_some_and(|word| word.start_ticks.0 >= word.end_ticks.0)
    {
        return Err("word timings must be non-empty and ordered".into());
    }
    if !style.font_size.is_finite() || !(8.0..=256.0).contains(&style.font_size) {
        return Err("kinetic font size must be between 8 and 256".into());
    }
    if !style.stiffness.is_finite() || !(1.0..=1000.0).contains(&style.stiffness)
        || !style.damping.is_finite() || !(0.1..=100.0).contains(&style.damping)
    {
        return Err("spring stiffness and damping are out of range".into());
    }
    if !(1..=5).contains(&style.lanes) {
        return Err("kinetic layout must use between 1 and 5 lanes".into());
    }
    if faces.iter().any(|face| {
        !face.x.is_finite() || !face.y.is_finite()
            || face.x < 0.0 || face.y < 0.0
            || face.x + face.width > 1.0 || face.y + face.height > 1.0
    }) {
        return Err("face boxes must be finite normalized rectangles inside the frame".into());
    }
    let track_index = document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or("kinetic typography track not found")?;
    let clip = document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or("kinetic typography clip not found")?;
    if clip.id == 0 {
        return Err("clip needs a stable id before kinetic typography".into());
    }
    let word_count = words.len();
    let face_count = faces.len();
    let record = ClipKineticTypography {
        clip_id: clip.id,
        words,
        faces,
        style,
        start_ticks: clip.timeline_start,
        duration_ticks: clip.duration,
    };
    let existing = document
        .vfx_kinetic_typography
        .iter_mut()
        .find(|item| item.clip_id == clip.id);
    match existing {
        Some(slot) => *slot = record,
        None => document.vfx_kinetic_typography.push(record),
    }
    Ok(KineticReport {
        clip_id: clip.id,
        word_count,
        face_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_edit::audio_clip;

    fn fixture() -> ProjectDocument {
        let mut document = ProjectDocument::new("Kinetic");
        let mut clip = audio_clip(1, Tick(0), Tick(100));
        clip.id = 1;
        document.project.sequence.tracks[0].clips.push(clip);
        document
    }

    fn words() -> Vec<KineticWord> {
        vec![
            KineticWord { text: "Hello".into(), start_ticks: Tick(0), end_ticks: Tick(10) },
            KineticWord { text: "world".into(), start_ticks: Tick(10), end_ticks: Tick(20) },
        ]
    }

    #[test]
    fn stores_words_and_rejects_bad_timings() {
        let mut document = fixture();
        let report = configure_kinetic_typography(
            &mut document, 1, 0, words(), vec![], KineticStyle::default(),
        )
        .expect("configure");
        assert_eq!(report.word_count, 2);
        assert_eq!(document.vfx_kinetic_typography.len(), 1);
        let bad = vec![KineticWord {
            text: "late".into(), start_ticks: Tick(30), end_ticks: Tick(10),
        }];
        assert!(configure_kinetic_typography(
            &mut document, 1, 0, bad, vec![], KineticStyle::default(),
        )
        .is_err());
    }
}
