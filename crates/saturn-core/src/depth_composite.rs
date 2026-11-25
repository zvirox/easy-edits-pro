// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Depth compositor configuration (Packet 6.1). Inference and pixel work stay
//! in `saturn-perception` and `saturn-render`; the core owns only the settings.

use crate::{Tick, project::ProjectDocument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DepthCompositeMode {
    ZThreshold,
    DofBokeh,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipDepthComposite {
    pub clip_id: u64,
    pub mode: DepthCompositeMode,
    pub threshold: f32,
    pub blur_radius: u32,
    pub start_ticks: Tick,
    pub duration_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DepthCompositeReport {
    pub clip_id: u64,
    pub mode: DepthCompositeMode,
    pub threshold: f32,
    pub blur_radius: u32,
}

pub fn configure_depth_composite(
    document: &mut ProjectDocument,
    track_id: u64,
    clip_index: usize,
    mode: DepthCompositeMode,
    threshold: f32,
    blur_radius: u32,
) -> Result<DepthCompositeReport, String> {
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err("depth threshold must be finite and between 0 and 1".into());
    }
    if blur_radius > 16 {
        return Err("depth blur radius must be at most 16 pixels".into());
    }
    let track_index = document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or("depth composite track not found")?;
    let clip = document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or("depth composite clip not found")?;
    if clip.id == 0 {
        return Err("clip needs a stable id before depth compositing".into());
    }
    let record = ClipDepthComposite {
        clip_id: clip.id,
        mode,
        threshold,
        blur_radius,
        start_ticks: clip.timeline_start,
        duration_ticks: clip.duration,
    };
    let existing = document
        .vfx_depth_composites
        .iter_mut()
        .find(|item| item.clip_id == clip.id);
    match existing {
        Some(slot) => *slot = record,
        None => document.vfx_depth_composites.push(record),
    }
    Ok(DepthCompositeReport {
        clip_id: clip.id,
        mode,
        threshold,
        blur_radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_edit::audio_clip;

    fn fixture() -> ProjectDocument {
        let mut document = ProjectDocument::new("Depth composite");
        let mut clip = audio_clip(1, Tick(0), Tick(100));
        clip.id = 1;
        document.project.sequence.tracks[0].clips.push(clip);
        document
    }

    #[test]
    fn configures_and_replaces_a_depth_composite() {
        let mut document = fixture();
        let report = configure_depth_composite(
            &mut document, 1, 0, DepthCompositeMode::ZThreshold, 0.4, 0,
        )
        .expect("configure");
        assert_eq!(report.clip_id, 1);
        assert_eq!(document.vfx_depth_composites.len(), 1);
        configure_depth_composite(
            &mut document, 1, 0, DepthCompositeMode::DofBokeh, 0.7, 8,
        )
        .expect("replace");
        assert_eq!(document.vfx_depth_composites.len(), 1);
        assert_eq!(document.vfx_depth_composites[0].mode, DepthCompositeMode::DofBokeh);
    }

    #[test]
    fn rejects_out_of_range_settings() {
        let mut document = fixture();
        assert!(configure_depth_composite(
            &mut document, 1, 0, DepthCompositeMode::ZThreshold, 1.5, 0,
        )
        .is_err());
        assert!(configure_depth_composite(
            &mut document, 1, 0, DepthCompositeMode::DofBokeh, 0.5, 32,
        )
        .is_err());
    }
}
