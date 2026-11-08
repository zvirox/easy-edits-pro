// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use saturn_core::Tick;

fn frame(x: usize) -> SampledFrame {
    let mut rgb = vec![20; 160 * 90 * 3];
    for y in 30..50 {
        for xx in x..x + 15 {
            rgb[(y * 160 + xx) * 3..(y * 160 + xx) * 3 + 3].fill(230);
        }
    }
    SampledFrame {
        tick: Tick(0),
        width: 160,
        height: 90,
        rgb,
    }
}

#[test]
fn focal_coordinates_move_with_subject_and_reject_flat_frames() {
    let left = focal_position(&frame(20)).unwrap().unwrap();
    let right = focal_position(&frame(110)).unwrap().unwrap();
    assert!(right[0] - left[0] > 0.5);
    assert!((right[1] - left[1]).abs() < 0.01);
    let mut flat = frame(0);
    flat.rgb.fill(200);
    assert!(focal_position(&flat).unwrap().is_none());
    flat.rgb.pop();
    assert!(focal_position(&flat).is_err());
}

#[test]
fn motion_vectors_preserve_direction_and_static_is_absent() {
    let a = frame(30);
    let b = frame(34);
    let c = frame(26);
    let forward = motion_vector(&a, &b).unwrap().unwrap();
    let backward = motion_vector(&a, &c).unwrap().unwrap();
    assert!(forward[0] > 0.0 && backward[0] < 0.0);
    assert!(distance(forward, backward) > distance(forward, forward));
    assert!(motion_vector(&a, &a).unwrap().is_none());
}

#[test]
fn attacks_find_known_onsets_match_gain_independently_and_reject_silence() {
    let mut pcm = vec![0.0; 16_000];
    for (start, gain) in [(1600, 0.5), (8000, 0.25)] {
        for n in 0..640 {
            pcm[start + n] = gain * (1.0 - n as f32 / 640.0);
        }
    }
    let hits = attacks(&pcm, 16_000).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].sample, 1600);
    assert_eq!(hits[1].sample, 8000);
    assert!(attack_distance(&hits[0], &hits[1]) < 1e-8);
    assert!(attacks(&vec![0.0; 16000], 16000).unwrap().is_empty());
    assert!(attacks(&[f32::NAN; 1600], 16000).is_err());
}

#[test]
fn attack_at_last_full_envelope_boundary_is_retained() {
    let mut pcm = vec![0.0; 1600];
    for n in 0..640 {
        pcm[960 + n] = 0.5 * (1.0 - n as f32 / 640.0);
    }
    let hits = attacks(&pcm, 16000).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].sample, 960);
    assert!(attacks(&pcm[..1500], 16000).unwrap().is_empty());
}
