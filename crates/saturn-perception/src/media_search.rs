// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Caller-side glue for semantic media search and cut review (Packet 2.2).
//!
//! Frame/audio decoding stays with the callers (the desktop app and the MCP
//! headless backend own their render pipelines); this module only owns the
//! inference-side loops so both callers share one sampling and embedding
//! policy. Every result is measured — nothing here fabricates scores or
//! clips it did not actually embed.

use crate::embeddings::SiglipEmbedder;
use crate::index_db::{SearchHit, SemanticIndex};
use crate::shot_detect::SampledFrame;
use crate::siglip_model::SIGLIP_REVISION;
use saturn_core::Tick;
use std::path::Path;

/// What the indexing loop needs from an embedder — a trait so tests can mock
/// model inference out of the loop's accounting logic.
pub trait FrameEmbedder {
    fn embed_frame(&mut self, frame: &SampledFrame) -> Result<Vec<f32>, String>;
}

impl FrameEmbedder for SiglipEmbedder {
    fn embed_frame(&mut self, frame: &SampledFrame) -> Result<Vec<f32>, String> {
        SiglipEmbedder::embed_frame(self, frame)
    }
}

/// The embedding revision stamped on vectors written by this build. Query
/// scan results are filtered to it, so a model upgrade invalidates old
/// vectors instead of silently mixing incomparable spaces.
pub fn semantic_index_revision() -> &'static str {
    SIGLIP_REVISION
}

/// Post-index text query: embed `query` with the text encoder and cosine-scan
/// the index. Inference time is happens before the scan; the returned hits
/// carry exactly the stored cosine scores.
pub fn query_media_index(
    embedder: &mut SiglipEmbedder,
    index: &SemanticIndex,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let vector = embedder.embed_query(query)?;
    index.query(&vector, semantic_index_revision(), limit)
}

/// Evenly spaced sample ticks covering `0..=duration` (inclusive of both
/// ends when they fit), at most `max_frames` of them.
pub fn media_sample_ticks(duration: Tick, step: Tick, max_frames: usize) -> Vec<Tick> {
    if duration.0 <= 0 || step.0 <= 0 || max_frames == 0 {
        return Vec::new();
    }
    let mut ticks = Vec::new();
    let mut next = 0_i64;
    while next <= duration.0 && ticks.len() < max_frames {
        ticks.push(Tick(next));
        next += step.0;
    }
    // Always include the media end so the tail is searchable; collapse a
    // duplicate when the last step already landed on it.
    if duration.0 > 0 && ticks.last().is_some_and(|last| last.0 < duration.0) {
        if ticks.len() == max_frames {
            ticks.pop();
        }
        ticks.push(duration);
    }
    ticks
}

/// One media's indexing result: how many frames were embedded and which
/// tick positions failed to decode (listed, not silently dropped).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaIndexOutcome {
    pub media_id: String,
    pub frames_embedded: usize,
    /// `(tick, reason)` for decode/embed failures inside this media.
    pub failures: Vec<(i64, String)>,
}

/// Decode, embed and store `ticks` of one media into `index`, replacing any
/// vectors from an earlier revision, and registering `media_id` against
/// `source_path`. The `decode` closure (owned by the caller, typically
/// `saturn_render::frame_extractor::decode_frame_rgb` wrapped) returns RGB8
/// frames; failures are recorded and skipped, not fatal.
pub fn embed_media_frames(
    embedder: &mut dyn FrameEmbedder,
    index: &SemanticIndex,
    media_id: &str,
    source_path: &Path,
    ticks: &[Tick],
    decode: &mut dyn FnMut(Tick) -> Result<(u32, u32, Vec<u8>), String>,
) -> Result<MediaIndexOutcome, String> {
    index.register_media(media_id, source_path, semantic_index_revision())?;
    let mut outcome = MediaIndexOutcome {
        media_id: media_id.to_owned(),
        frames_embedded: 0,
        failures: Vec::new(),
    };
    for tick in ticks {
        let (width, height, rgb) = match decode(*tick) {
            Ok(frame) => frame,
            Err(reason) => {
                outcome.failures.push((tick.0, reason));
                continue;
            }
        };
        let frame = SampledFrame {
            tick: *tick,
            width,
            height,
            rgb,
        };
        match embedder.embed_frame(&frame) {
            Ok(vector) => {
                index.store_frame_vector(
                    media_id,
                    *tick,
                    semantic_index_revision(),
                    &vector,
                )?;
                outcome.frames_embedded += 1;
            }
            Err(reason) => outcome.failures.push((tick.0, reason)),
        }
    }
    Ok(outcome)
}

/// Frames and audio snippets needed to evaluate one hard cut, decoded by the
/// caller-supplied closures. Motion direction is measured *within* each shot
/// (see `CutContext`), which is why both neighbor frames are requested.
#[derive(Clone, Debug)]
pub struct CutFrames {
    pub outgoing_last: SampledFrame,
    pub incoming_first: SampledFrame,
    /// `None` when the neighbor decode failed — flow is then honestly
    /// reported unmeasurable rather than guessed.
    pub outgoing_prev: Option<SampledFrame>,
    pub incoming_next: Option<SampledFrame>,
}

/// Decode the four frames around `cut_tick`. `neighbor_step` is the gap
/// between the boundary frame and its same-shot neighbor (roughly one frame
/// duration; the caller knows the media's fps). The outgoing side reads at
/// `cut_tick − 1` (the last decode position strictly before the cut) so the
/// bound frame belongs to the outgoing shot.
pub fn decode_cut_frames(
    cut_tick: Tick,
    neighbor_step: Tick,
    decode: &mut dyn FnMut(Tick) -> Result<(u32, u32, Vec<u8>), String>,
) -> Result<CutFrames, String> {
    let mut decode_frame = |tick: Tick| -> Option<SampledFrame> {
        let (width, height, rgb) = decode(tick).ok()?;
        Some(SampledFrame {
            tick,
            width,
            height,
            rgb,
        })
    };    let boundary_step = Tick(1);
    let outgoing_tick = Tick(cut_tick.0 - boundary_step.0);
    if outgoing_tick.0 < 0 {
        return Err("cut_tick must be strictly inside the media (past the first frame)".into());
    }
    let outgoing_last = decode_frame(outgoing_tick)
        .ok_or_else(|| format!("could not decode the outgoing frame at tick {}", outgoing_tick.0))?;
    let incoming_first = decode_frame(cut_tick)
        .ok_or_else(|| format!("could not decode the incoming frame at tick {}", cut_tick.0))?;
    let outgoing_prev =
        decode_frame(Tick(outgoing_tick.0 - neighbor_step.0)).filter(|_| outgoing_tick.0 - neighbor_step.0 >= 0);
    let incoming_next = decode_frame(Tick(cut_tick.0 + neighbor_step.0));
    Ok(CutFrames {
        outgoing_last,
        incoming_first,
        outgoing_prev,
        incoming_next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock embedder: deterministic unit vector by frame tick parity — lets
    /// the store/skip/accounting loop run in unit tests without model files
    /// (the real model path is covered by the gated e2e).
    struct MockEmbedder;

    impl FrameEmbedder for MockEmbedder {
        fn embed_frame(&mut self, frame: &SampledFrame) -> Result<Vec<f32>, String> {
            let mut vector = vec![0.0_f32; 4];
            vector[if frame.tick.0 % 2 == 0 { 0 } else { 1 }] = 1.0;
            Ok(vector)
        }
    }

    #[test]
    fn sample_ticks_cover_range_and_cap() {
        // 10 s at 2 s steps → 6 ticks (0..=10 inclusive).
        let ticks = media_sample_ticks(Tick::from_millis(10_000), Tick::from_millis(2_000), 400);
        assert_eq!(ticks.len(), 6);
        assert_eq!(ticks[0], Tick(0));
        assert_eq!(ticks[5], Tick::from_millis(10_000));
        // Cap: 10 s in 1 s steps but max 4 → 4 ticks with the end present.
        let ticks = media_sample_ticks(Tick::from_millis(10_000), Tick::from_millis(1_000), 4);
        assert_eq!(ticks.len(), 4);
        assert_eq!(*ticks.last().unwrap(), Tick::from_millis(10_000));
        assert_eq!(
            media_sample_ticks(Tick(0), Tick::from_millis(2_000), 10),
            Vec::new(),
            "zero-duration media has no samples"
        );
    }

    #[test]
    fn embed_loop_stores_scores_and_records_failures() {
        let dir = std::env::temp_dir().join(format!(
            "saturn-pipeline-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("p.eep");
        let index = SemanticIndex::open_for_project(&project).unwrap();
        let mut embedder = MockEmbedder;
        let mut solid = |tick: Tick| -> Result<(u32, u32, Vec<u8>), String> {
            if tick.0 == 40 {
                return Err("decode unavailable".into());
            }
            Ok((4, 4, vec![9_u8; 4 * 4 * 3]))
        };
        let embedder_ref: &mut dyn FrameEmbedder = &mut embedder;
        let outcome = embed_media_frames(
            embedder_ref,
            &index,
            "m1",
            Path::new("/x.mp4"),
            &[Tick(0), Tick(40), Tick(80)],
            &mut solid,
        )
        .unwrap();
        assert_eq!(outcome.frames_embedded, 2);
        assert_eq!(
            outcome.failures,
            vec![(40, "decode unavailable".to_owned())]
        );
        // Registered + both successful vectors retrievable via cosine scan.
        let hits = index.query(&[0.0, 0.0, 0.0, 1.0], "test", 10).unwrap();
        assert!(hits.is_empty(), "revision stamp differs from store");
        let hits = index
            .query(&[1.0, 0.0, 0.0, 0.0], semantic_index_revision(), 10)
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].tick_ticks, 0, "even tick matches unit vector 0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cut_decode_reports_boundary_and_optional_neighbors() {
        let mut decode = |tick: Tick| -> Result<(u32, u32, Vec<u8>), String> {
            if tick.0 < 0 || tick.0 > 100 {
                return Err(format!("tick {} out of range", tick.0));
            }
            Ok((4, 4, vec![tick.0 as u8; 4 * 4 * 3]))
        };
        let frames = decode_cut_frames(Tick(50), Tick(40), &mut decode).unwrap();
        assert_eq!(frames.outgoing_last.tick.0, 49);        assert_eq!(frames.incoming_first.tick.0, 50);
        assert_eq!(
            frames.outgoing_prev.as_ref().unwrap().tick.0,
            9,
            "same-shot neighbor of the outgoing side"
        );
        assert_eq!(frames.incoming_next.as_ref().unwrap().tick.0, 90);
        // A cut that is the first frame has no outgoing side — honest error.
        assert!(decode_cut_frames(Tick(0), Tick(40), &mut decode).is_err());
    }
}
