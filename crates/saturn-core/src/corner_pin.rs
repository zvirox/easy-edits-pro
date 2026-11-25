// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Planar corner pins (Packet 6.1). Stores a four-corner homography target per
//! clip; pixel warping lives in `saturn-render::corner_pin`.

use crate::{project::ProjectDocument, Tick};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipCornerPin {
    pub clip_id: u64,
    /// Normalized destination corners in reading order: TL, TR, BR, BL.
    pub corners: [[f64; 2]; 4],
    pub motion_blur: bool,
    pub start_ticks: Tick,
    pub duration_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CornerPinReport {
    pub clip_id: u64,
    pub motion_blur: bool,
    pub corners: [[f64; 2]; 4],
}

pub fn pin_corners(
    document: &mut ProjectDocument,
    track_id: u64,
    clip_index: usize,
    corners: [[f64; 2]; 4],
    motion_blur: bool,
) -> Result<CornerPinReport, String> {
    for corner in corners {
        if !corner[0].is_finite() || !corner[1].is_finite()
            || corner[0] < -0.5 || corner[0] > 1.5 || corner[1] < -0.5 || corner[1] > 1.5
        {
            return Err("each corner must be finite and within [-0.5, 1.5]".into());
        }
    }
    // Shoelace area: a degenerate or inverted quad cannot define a homography.
    let mut area = 0.0;
    for i in 0..4 {
        let (x1, y1) = (corners[i][0], corners[i][1]);
        let (x2, y2) = (corners[(i + 1) % 4][0], corners[(i + 1) % 4][1]);
        area += x1 * y2 - x2 * y1;
    }
    if area.abs() < 0.01 {
        return Err("corner pin quad is degenerate; corners must enclose an area".into());
    }

    let track_index = document.project.sequence.tracks.iter()
        .position(|track| track.id == track_id)
        .ok_or("corner pin track not found")?;
    let clip = document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or("corner pin clip not found")?;
    if clip.id == 0 {
        return Err("clip needs a stable id before pinning corners".into());
    }
    let record = ClipCornerPin {
        clip_id: clip.id,
        corners,
        motion_blur,
        start_ticks: clip.timeline_start,
        duration_ticks: clip.duration,
    };
    let existing = document.vfx_corner_pins.iter_mut().find(|pin| pin.clip_id == clip.id);
    match existing {
        Some(slot) => *slot = record.clone(),
        None => document.vfx_corner_pins.push(record),
    }
    Ok(CornerPinReport {
        clip_id: clip.id,
        motion_blur,
        corners,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_edit::audio_clip;

    fn fixture() -> ProjectDocument {
        let mut document = ProjectDocument::new("Corner pin");
        let mut clip = audio_clip(1, Tick(0), Tick(100));
        clip.id = 1;
        document.project.sequence.tracks[0].clips.push(clip);
        document
    }

    #[test]
    fn pins_a_quad_and_replaces_an_existing_pin() {
        let mut document = fixture();
        let quad = [[0.1, 0.1], [0.9, 0.2], [0.8, 0.9], [0.2, 0.8]];
        let report = pin_corners(&mut document, 1, 0, quad, true).expect("pin");
        assert_eq!(report.clip_id, 1);
        assert_eq!(document.vfx_corner_pins.len(), 1);
        let other = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        pin_corners(&mut document, 1, 0, other, false).expect("replace");
        assert_eq!(document.vfx_corner_pins.len(), 1);
        assert!(!document.vfx_corner_pins[0].motion_blur);
    }

    #[test]
    fn refuses_a_degenerate_quad() {
        let mut document = fixture();
        let line = [[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 0.0]];
        assert!(pin_corners(&mut document, 1, 0, line, false).is_err());
    }
}
