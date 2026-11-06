// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! End-to-end SigLIP embedding + semantic index check (Packet 2.2).
//!
//! Gated like `whisper_jfk.rs`: downloads the pinned ONNX exports (~213 MB
//! total, SHA-256-verified) on first run, so constrained CI skips it unless
//! `SATURN_PERCEPTION_E2E=1` or the files are already cached in
//! `SATURN_MODELS_DIR`.
//!
//! What is verified against real inference:
//! 1. Embeddings are deterministic, finite, L2-normalized, and distinct for
//!    different frames.
//! 2. The image and text encoders share a projection space: a green-dominant
//!    frame is measurably closer to "an image of green grass" than to a
//!    dark-ocean phrase (SigLIP's zero-shot capability, exactly what
//!    `media_semantic_query` depends on).
//! 3. The SQLite index ranks stored frames by exact cosine against the query
//!    and a scan over hundreds of stored vectors stays far below the 50 ms
//!    query budget the plan sets for `media_semantic_query` (model inference
//!    time is *not* part of that budget — the query is a vector scan).

use saturn_perception::index_db::SemanticIndex;
use saturn_perception::{cosine_similarity, SiglipEmbedder, SIGLIP_EMBED_DIM};
use saturn_perception::shot_detect::SampledFrame;
use saturn_core::Tick;

fn e2e_enabled() -> bool {
    std::env::var("SATURN_PERCEPTION_E2E").is_ok_and(|v| v != "0")
        || std::env::var("SATURN_MODELS_DIR").is_ok()
}

/// Deterministic RGB frame: a vertical green gradient (like a grass field —
/// brighter at the horizon, darker in the foreground).
fn grass_frame(tick_millis: i64) -> SampledFrame {
    let (w, h) = (320_u32, 180_u32);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            // Green-dominant with texture from a cheap deterministic pattern.
            let ripple = ((x as f32 * 0.31 + y as f32 * 0.19).sin() * 18.0) as i32;
            let g = (150.0 * (1.0 - t * 0.4)) as i32 + ripple;
            rgb.push((40 + ripple / 3).clamp(0, 255) as u8);
            rgb.push(g.clamp(0, 255) as u8);
            rgb.push((30 + ripple / 4).clamp(0, 255) as u8);
        }
    }
    SampledFrame {
        tick: Tick::from_millis(tick_millis),
        width: w,
        height: h,
        rgb,
    }
}

/// A very dark blue frame (night ocean: near-black with a faint blue cast).
fn night_ocean_frame(tick_millis: i64) -> SampledFrame {
    let (w, h) = (320_u32, 180_u32);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let ripple = ((x as f32 * 0.17 + y as f32 * 0.41).sin() * 6.0) as i32;
            rgb.push((10 + ripple / 2).clamp(0, 255) as u8);
            rgb.push((14 + ripple / 2).clamp(0, 255) as u8);
            rgb.push((46 + ripple).clamp(0, 255) as u8);
        }
    }
    SampledFrame {
        tick: Tick::from_millis(tick_millis),
        width: w,
        height: h,
        rgb,
    }
}

#[test]
fn siglip_real_inference_cross_modal_and_index_query() {
    if !e2e_enabled() {
        eprintln!("skipping: set SATURN_PERCEPTION_E2E=1 to run the SigLIP end-to-end test");
        return;
    }

    let mut embedder = SiglipEmbedder::load().expect("SigLIP models load (downloaded on first run)");

    let grass = grass_frame(0);
    let ocean = night_ocean_frame(500);

    // 1. Determinism, normalization, distinctness.
    let grass_a = embedder.embed_frame(&grass).expect("grass embeds");
    let grass_b = embedder.embed_frame(&grass).expect("grass re-embeds");
    assert_eq!(grass_a.len(), SIGLIP_EMBED_DIM);
    assert_eq!(grass_a, grass_b, "embeddings must be deterministic");
    let norm: f32 = grass_a.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-3, "expected unit norm, got {norm}");
    let ocean_embed = embedder.embed_frame(&ocean).expect("ocean embeds");
    assert!(
        cosine_similarity(&grass_a, &ocean_embed) < 0.999,
        "distinct frames must not produce identical embeddings"
    );

    // 2. Cross-modal space: the green frame is closer to its phrase than to
    // the ocean phrase. This is the property media_semantic_query needs.
    let grass_text = embedder
        .embed_query("an image of green grass")
        .expect("grass query embeds");
    let ocean_text = embedder
        .embed_query("the ocean at night")
        .expect("ocean query embeds");
    let to_grass = cosine_similarity(&grass_a, &grass_text);
    let cross = cosine_similarity(&grass_a, &ocean_text);
    assert!(
        (-1.0..=1.0).contains(&to_grass) && (-1.0..=1.0).contains(&cross),
        "cosine out of range: {to_grass} / {cross}"
    );
    assert!(
        to_grass > cross,
        "green frame should match its phrase ({to_grass}) better than the ocean phrase \
         ({cross}) — if this fails the encoders likely do not share a projection space \
         or the pooling is wrong"
    );
    let reverse = cosine_similarity(&ocean_embed, &ocean_text);
    assert!(
        reverse > cross.max(0.0) - 0.05 || reverse > to_grass,
        "sanity: ocean frame relates to its own phrase ({reverse})"
    );

    // 3. Index ranking + query latency (< 50 ms budget for the scan).
    let project_dir = std::env::temp_dir().join(format!(
        "saturn-siglip-e2e-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&project_dir).unwrap();
    let project = project_dir.join("e2e.eep");
    let index = SemanticIndex::open_for_project(&project).expect("index opens");
    index.register_media("grass.mp4", &project_dir.join("grass.mp4"), "test-rev").unwrap();
    index.register_media("ocean.mp4", &project_dir.join("ocean.mp4"), "test-rev").unwrap();

    // Store enough vectors to make the scan non-trivial: the two real frames
    // plus deterministic soft-noise fillers.
    let mut lcg: u64 = 0x2545F4914F6CDD1D;
    for i in 0..500_u64 {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut vector = vec![0.0_f32; SIGLIP_EMBED_DIM];
        for slot in vector.iter_mut() {
            lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *slot = ((lcg >> 33) as f32 / (u32::MAX >> 1) as f32 - 1.0) * 0.03;
        }
        let norm_sq: f32 = vector.iter().map(|v| v * v).sum();
        for slot in &mut vector {
            *slot /= norm_sq.sqrt();
        }
        index
            .store_frame_vector("ocean.mp4", Tick::from_millis((1000 + i * 40) as i64), "test-rev", &vector)
            .unwrap();
    }
    index.store_frame_vector("grass.mp4", Tick::from_millis(0), "test-rev", &grass_a).unwrap();
    index.store_frame_vector("ocean.mp4", Tick::from_millis(500), "test-rev", &ocean_embed).unwrap();

    // Warm-up then time the scan.
    let _ = index.query(&grass_a, "test-rev", 5).unwrap();
    let mut elapsed_total = std::time::Duration::ZERO;
    for _ in 0..20 {
        let start = std::time::Instant::now();
        let hits = index.query(&grass_a, "test-rev", 5).unwrap();
        elapsed_total += start.elapsed();
        assert_eq!(hits[0].media_id, "grass.mp4");
        assert_eq!(hits[0].tick_ticks, Tick::from_millis(0).0);
        assert!(hits[0].similarity > 0.999);
        assert!(
            hits.iter().all(|h| h.media_id != "grass.mp4" || h.tick_ticks == Tick::from_millis(0).0),
            "only the stored grass vector may rank as exact match"
        );
    }
    let average = elapsed_total / 20;
    assert!(
        average < std::time::Duration::from_millis(50),
        "media_semantic_query budget is < 50 ms post-indexing; scan over 502 vectors \
         averaged {average:?}"
    );

    let _ = std::fs::remove_dir_all(&project_dir);
}
