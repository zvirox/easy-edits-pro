// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Offline master bounce and measured loudness normalization.
use crate::loudness::{EbuR128Meter, LoudnessStandard};
use saturn_audio_dsp::{ducking::limit_lookahead, true_peak::measure_peak};
use serde_json::{Value, json};

pub fn normalize(
    samples: &[f32],
    rate: u32,
    channels: usize,
    standard: LoudnessStandard,
    ceiling_db: f64,
) -> Result<(Vec<f32>, Value), String> {
    if !(8000..=192000).contains(&rate)
        || !(1..=2).contains(&channels)
        || samples.len() % channels != 0
        || samples.len() < rate as usize * channels * 2 / 5
        || samples.len() > rate as usize * channels * 120
        || samples.iter().any(|v| !v.is_finite())
        || !ceiling_db.is_finite()
        || !(-12.0..=-0.1).contains(&ceiling_db)
    {
        return Err("Normalization requires finite mono/stereo PCM, 0.4..120 seconds, and a -12..-0.1 dBTP ceiling".into());
    }
    let measure = |pcm: &[f32]| {
        let mut meter = EbuR128Meter::new(rate, channels, standard);
        meter.process_interleaved(pcm);
        meter.integrated_lufs_exact()
    };
    let before = measure(samples);
    if before <= -69.9 {
        return Err("Cannot normalize silence or audio below the loudness gate".into());
    }
    let target = standard.target_lufs();
    let ceiling = 10_f64.powf((ceiling_db - 0.01) / 20.0);
    let mut gain_db = target - before;
    for pass in 1..=8 {
        if !gain_db.is_finite() || gain_db > 60.0 {
            break;
        }
        let gain = 10_f64.powf(gain_db / 20.0);
        let mut output: Vec<_> = samples.iter().map(|v| (*v as f64 * gain) as f32).collect();
        limit_lookahead(&mut output, channels, rate, ceiling * 0.9)?;
        let peak = measure_peak(&output, channels)?;
        if peak > ceiling {
            let trim = (ceiling / peak) as f32;
            for sample in &mut output {
                *sample *= trim;
            }
        }
        let after = measure(&output);
        let final_peak = measure_peak(&output, channels)?;
        if (after - target).abs() <= 0.05 && final_peak <= 10_f64.powf(ceiling_db / 20.0) {
            return Ok((
                output,
                json!({"beforeLufs": before, "integratedLufs": after,
                "truePeakDbtp": 20.0 * final_peak.log10(), "targetLufs": target, "passes": pass,
                "limiter": "5 ms stereo-linked lookahead; 100 ms release; measured 4x FIR true-peak correction"}),
            ));
        }
        gain_db += target - after;
    }
    Err("Could not meet both the loudness target and true-peak ceiling within eight passes".into())
}

/// Render the same GES mix used by export, without a lossy intermediate codec.
pub(crate) fn render_mix(
    document: &saturn_core::ProjectDocument,
    path: &std::path::Path,
) -> Result<(), String> {
    use gstreamer as gst;
    use gstreamer_editing_services as ges;
    use gstreamer_pbutils as pb;
    use pb::encoding_profile::EncodingProfileBuilder;
    document.validate()?;
    let _render_guard = crate::ges_timeline::RENDER_GATE
        .lock()
        .map_err(|_| "Render lock poisoned")?;
    crate::GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|e| e.to_string())?;
            ges::init().map_err(|e| e.to_string())?;
            crate::graphics_element::register().map_err(|e| e.to_string())?;
            crate::automation_element::register().map_err(|e| e.to_string())?;
            crate::keyframe_element::register().map_err(|e| e.to_string())?;
            crate::plugin_element::register().map_err(|e| e.to_string())
        })
        .clone()?;
    // Hardware discovery may have initialized GST_INIT before a render.
    crate::plugin_element::register().map_err(|e| e.to_string())?;
    let timeline = crate::ges_timeline::build_timeline(document)?;
    let raw = gst::Caps::builder("audio/x-raw")
        .field("format", "F32LE")
        .field("rate", 48000_i32)
        .field("channels", 2_i32)
        .build();
    let audio = pb::EncodingAudioProfile::builder(&raw).build();
    let container =
        pb::EncodingContainerProfile::builder(&gst::Caps::builder("audio/x-wav").build())
            .name("PCM master")
            .add_profile(audio)
            .build();
    let uri = gst::glib::filename_to_uri(path, None).map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    crate::ges_timeline::render_timeline(&timeline, &uri, &container, &mut |_| {}, &|| {
        start.elapsed() > std::time::Duration::from_secs(180)
    })
}
