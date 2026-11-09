// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode-free payloads for offline audio edits. Drivers measure/render outside
//! the engine lock; the command checks the snapshot digest before committing.
use crate::{ProjectDocument, Tick};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn snapshot_digest(document: &ProjectDocument) -> Result<String, String> {
    serde_json::to_vec(&document.project)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedAudio {
    pub path: PathBuf,
    pub duration: Tick,
    pub channels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomToneInsert {
    pub start: Tick,
    pub audio: RenderedAudio,
}

/// Minimal neutral audio clip, with all further processing left on its track.
pub fn audio_clip(media_id: u64, start: Tick, duration: Tick) -> crate::Clip {
    crate::Clip {
        id: 0,
        media_id,
        timeline_start: start,
        source_in: Tick(0),
        duration,
        speed: 1.0,
        group_id: None,
        color: Default::default(),
        effects: vec![],
        keyframes: vec![],
        multicam_id: None,
        multicam_angle_id: None,
        tracking: None,
        node_graph: None,
        color_nodes: vec![],
        speed_ramp: None,
        motion_blur: None,
        compound_sequence_id: None,
        label: String::new(),
        volume_db: 0.0,
        audio_enabled: true,
        enabled: true,
        agent_note: None,
    }
}
