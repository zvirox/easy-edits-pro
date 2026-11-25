// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Object removal settings (Packet 6.1). Pixel-level clean-plate reconstruction
//! lives in `saturn-render::inpaint`; the core owns validated parameters only.

use crate::{Tick, project::ProjectDocument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipInpaint {
    pub clip_id: u64,
    /// Normalized removal region: left, top, width, height in 0..1.
    pub region: [f64; 4],
    /// How many neighbouring frames the caller may use as clean-plate donors.
    pub search_frames: u32,
    pub start_ticks: Tick,
    pub duration_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct InpaintReport {
    pub clip_id: u64,
    pub region: [f64; 4],
    pub search_frames: u32,
}

pub fn configure_inpaint(
    document: &mut ProjectDocument,
    track_id: u64,
    clip_index: usize,
    region: [f64; 4],
    search_frames: u32,
) -> Result<InpaintReport, String> {
    let [left, top, width, height] = region;
    if !region.iter().all(|value| value.is_finite())
        || !(0.0..1.0).contains(&left)
        || !(0.0..1.0).contains(&top)
        || !(0.0..=1.0).contains(&width)
        || !(0.0..=1.0).contains(&height)
        || width <= 0.0
        || height <= 0.0
        || left + width > 1.0
        || top + height > 1.0
    {
        return Err("removal region must be a finite, non-empty normalized rectangle inside the frame".into());
    }
    if !(1..=8).contains(&search_frames) {
        return Err("clean-plate search must use between 1 and 8 neighbouring frames".into());
    }
    let track_index = document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or("inpaint track not found")?;
    let clip = document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or("inpaint clip not found")?;
    if clip.id == 0 {
        return Err("clip needs a stable id before object removal".into());
    }
    let record = ClipInpaint {
        clip_id: clip.id,
        region,
        search_frames,
        start_ticks: clip.timeline_start,
        duration_ticks: clip.duration,
    };
    let existing = document.vfx_inpaints.iter_mut().find(|item| item.clip_id == clip.id);
    match existing {
        Some(slot) => *slot = record,
        None => document.vfx_inpaints.push(record),
    }
    Ok(InpaintReport {
        clip_id: clip.id,
        region,
        search_frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_edit::audio_clip;

    fn fixture() -> ProjectDocument {
        let mut document = ProjectDocument::new("Inpaint");
        let mut clip = audio_clip(1, Tick(0), Tick(100));
        clip.id = 1;
        document.project.sequence.tracks[0].clips.push(clip);
        document
    }

    #[test]
    fn stores_a_valid_region_and_rejects_bad_ones() {
        let mut document = fixture();
        let report = configure_inpaint(&mut document, 1, 0, [0.2, 0.3, 0.4, 0.5], 4)
            .expect("configure");
        assert_eq!(report.clip_id, 1);
        assert_eq!(document.vfx_inpaints.len(), 1);
        assert!(configure_inpaint(&mut document, 1, 0, [0.8, 0.1, 0.4, 0.4], 4).is_err());
        assert!(configure_inpaint(&mut document, 1, 0, [0.1, 0.1, 0.2, 0.2], 0).is_err());
    }
}
