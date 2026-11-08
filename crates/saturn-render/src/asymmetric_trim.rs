// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Prepare a stereo constant-power split-edit bounce outside the editor lock.
use crate::audio_edit_files::PreparedAudioEdit;
use crate::audio_pcm::decode_audio_pcm_constrained;
use saturn_core::asymmetric_trim::{AsymmetricTrimParams, plan};
use saturn_core::audio_edit::snapshot_digest;
use saturn_core::{Clip, ProjectDocument, TICKS_PER_SECOND, Tick};
use serde_json::{Value, json};
use std::path::Path;

const RATE: u32 = 48_000;

fn frames(tick: Tick) -> usize {
    (tick.0 as i128 * RATE as i128 / TICKS_PER_SECOND as i128) as usize
}

fn decode(doc: &ProjectDocument, clip: &Clip, count: usize) -> Result<Vec<f32>, String> {
    let source = doc
        .project
        .media
        .iter()
        .find(|m| m.id == clip.media_id)
        .ok_or("Split-edit source disappeared")?;
    let length = (count as i128 * TICKS_PER_SECOND as i128 / RATE as i128) as i64;
    let end = clip
        .source_in
        .checked_add(length)
        .ok_or("Split-edit source overflow")?;
    if Some(end) > source.duration {
        return Err("Insufficient source handles at 48 kHz; reduce the fade or cut offset".into());
    }
    let mut pcm =
        decode_audio_pcm_constrained(&source.path, clip.source_in, Some(end), Some(RATE), Some(2))?
            .samples;
    if pcm.len() < count * 2 {
        return Err(
            "Source audio ends before the split edit; relink the source or shorten the edit".into(),
        );
    }
    pcm.truncate(count * 2);
    let gain = 10_f32.powf(clip.volume_db as f32 / 20.0);
    for value in &mut pcm {
        *value *= gain;
    }
    Ok(pcm)
}

pub fn prepare(
    doc: &ProjectDocument,
    project: Option<&Path>,
    params: &Value,
) -> Result<PreparedAudioEdit, String> {
    doc.validate()?;
    let p: AsymmetricTrimParams =
        serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let plan = plan(doc, &p)?;
    let count = frames(plan.duration);
    let fade_start = frames(
        plan.fade_start
            .checked_sub(plan.start.0)
            .ok_or("Fade start overflow")?,
    );
    let fade_end = frames(
        plan.fade_end
            .checked_sub(plan.start.0)
            .ok_or("Fade end overflow")?,
    );
    if count == 0
        || fade_start == 0
        || fade_end >= count
        || (p.fade_ticks.0 > 0 && fade_end - fade_start < 2)
    {
        return Err("Keep at least one audio sample on each side and use a fade of at least two samples (or zero)".into());
    }
    let mut edit = PreparedAudioEdit::new(project, "timeline.asymmetric_trim")?;
    let left = decode(doc, &plan.left_audio, fade_end)?;
    // Keep the incoming source and its placement on the same sample grid. Without
    // this adjustment a fractional-tick fade can request a sample beyond a source
    // whose original out-point is exactly at EOF.
    let quantized_start =
        Tick((fade_start as i128 * TICKS_PER_SECOND as i128 / RATE as i128) as i64);
    let offset = quantized_start.0 - (plan.fade_start.0 - plan.start.0);
    let mut right_clip = plan.right_audio.clone();
    right_clip.source_in = right_clip
        .source_in
        .checked_add(offset)
        .filter(|t| t.0 >= 0)
        .ok_or("Insufficient incoming source handle at 48 kHz; reduce the sound offset or fade")?;
    let right = decode(doc, &right_clip, count - fade_start)?;
    let mixed = mix(&left, &right, count, fade_start, fade_end);
    let audio = edit.write(&mixed, 2, plan.duration)?;
    let mut command = serde_json::to_value(&p).map_err(|e| e.to_string())?;
    command["expected_digest"] = json!(snapshot_digest(doc)?);
    command["audio"] = json!(audio);
    edit.command.params = command;
    edit.telemetry = json!({"videoCut": plan.video_cut, "audioCut": plan.audio_cut,
        "fadeTicks": p.fade_ticks, "sampleRate": RATE, "channels": 2,
        "method": "cosine/sine constant-power crossfade centered on the sound cut",
        "mode": "editable audio bounce; original picture clips have audio disabled",
        "timing": "audio boundaries quantized to 48 kHz; picture timing stays in ticks"});
    Ok(edit)
}

fn mix(left: &[f32], right: &[f32], count: usize, start: usize, end: usize) -> Vec<f32> {
    let mut mixed = vec![0.0; count * 2];
    for frame in 0..count {
        let (a, b) = if frame < start {
            (1.0, 0.0)
        } else if frame >= end {
            (0.0, 1.0)
        } else {
            let theta =
                (frame - start) as f64 / (end - start - 1) as f64 * std::f64::consts::FRAC_PI_2;
            (theta.cos() as f32, theta.sin() as f32)
        };
        for channel in 0..2 {
            if frame < end {
                mixed[frame * 2 + channel] += left[frame * 2 + channel] * a;
            }
            if frame >= start {
                mixed[frame * 2 + channel] += right[(frame - start) * 2 + channel] * b;
            }
        }
    }
    mixed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orthogonal_channels_keep_power_and_endpoints() {
        let left = [1.0, 0.0].repeat(90);
        let right = [0.0, 1.0].repeat(90);
        let pcm = mix(&left, &right, 100, 10, 90);
        for pair in pcm.chunks_exact(2) {
            assert!((pair[0].powi(2) + pair[1].powi(2) - 1.0).abs() < 2e-7);
        }
        assert_eq!(&pcm[20..22], &[1.0, 0.0]);
        assert!(pcm[178].abs() < 1e-7);
        assert_eq!(pcm[179], 1.0);
        let hard = mix(&[1.0; 20], &[2.0; 20], 20, 10, 10);
        assert_eq!(&hard[..20], &[1.0; 20]);
        assert_eq!(&hard[20..], &[2.0; 20]);
    }
}
