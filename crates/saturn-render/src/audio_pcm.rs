// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Raw PCM audio decoding for telemetry — the audio counterpart of the
//! frame extractor's decode pipeline (Packet 1.3).
//!
//! Single-shot GStreamer pipeline per decode:
//! `uridecodebin → audioconvert → audioresample → (F32LE caps) → appsink`.
//! Runs to Playing, drains buffers up to the requested frame budget, then
//! tears down. Only the region's worth of samples is ever pulled.

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use saturn_core::Tick;
use std::path::Path;
use std::time::{Duration, Instant};

/// Total decode wall-clock ceiling: long ranges fail with a typed error
/// instead of hanging the caller.
const DECODE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: usize,
    /// Interleaved f32 samples in −1.0..1.0 (`len == frames × channels`).
    pub samples: Vec<f32>,
}

fn position_ns(tick: Tick) -> Result<u64, String> {
    tick.as_nanoseconds()
        .ok_or_else(|| "Audio range start cannot be negative".to_owned())
}

/// Decode a media file's first audio track into interleaved f32 PCM.
///
/// `start_tick`/`end_tick` select the region (project ticks): decoding
/// starts with a `FLUSH|ACCURATE` seek to `start` and stops once `end` is
/// reached. Git-edge case: a requested region past EOF yields whatever the
/// file has (callers report coverage via the analyzed duration).
pub fn decode_audio_pcm(
    source: &Path,
    start_tick: Tick,
    end_tick: Option<Tick>,
) -> Result<DecodedAudio, String> {
    decode_audio_pcm_constrained(source, start_tick, end_tick, None, None)
}

/// Decode a media file's first audio track into **mono 16 kHz** f32 PCM —
/// the exact input format the Whisper engine expects (`saturn-perception`).
/// `audioconvert`/`audioresample` handle the mixdown and resampling; asking
/// the caps filter up front means no post-decode conversion pass.
pub fn decode_audio_pcm_mono_16k(
    source: &Path,
    start_tick: Tick,
    end_tick: Option<Tick>,
) -> Result<Vec<f32>, String> {
    let decoded = decode_audio_pcm_constrained(source, start_tick, end_tick, Some(16_000), Some(1))?;
    Ok(decoded.samples)
}

pub(crate) fn decode_audio_pcm_constrained(
    source: &Path,
    start_tick: Tick,
    end_tick: Option<Tick>,
    rate: Option<u32>,
    channels: Option<u32>,
) -> Result<DecodedAudio, String> {
    crate::init_media_runtime()?;
    if !source.is_file() {
        return Err(format!("Audio source is missing: {}", source.display()));
    }
    let uri = gst::glib::filename_to_uri(source, None)
        .map_err(|error| format!("Could not address the audio source: {error}"))?;

    let start_ns = position_ns(start_tick)?;
    let end_ns = end_tick.map(|t| position_ns(t)).transpose()?;
    if let Some(end) = end_ns {
        if end <= start_ns {
            return Err("Audio range end must come after its start".into());
        }
    }
    // Frame budget: unknown until caps arrive (rate/channels); recomputed
    // once known. `None` analyzes to the cap bounded by MAX_ANALYSIS_SECONDS
    // downstream in the telemetry analyzer.
    let span_ns: Option<u64> = end_ns.map(|end| end - start_ns);
    let mut budget_frames: Option<usize> = None;

    let pipeline = gst::Pipeline::new();
    let decodebin = gst::ElementFactory::make("uridecodebin")
        .property("uri", &uri)
        .build()
        .map_err(|error| format!("GStreamer uridecodebin is unavailable: {error}"))?;
    let convert = gst::ElementFactory::make("audioconvert")
        .build()
        .map_err(|error| format!("GStreamer audioconvert is unavailable: {error}"))?;
    let resample = gst::ElementFactory::make("audioresample")
        .build()
        .map_err(|error| format!("GStreamer audioresample is unavailable: {error}"))?;
    let appsink = AppSink::builder()
        .caps(&{
            let mut caps = gst::Caps::builder("audio/x-raw")
                .field("format", "F32LE")
                .field("layout", "interleaved");
            if let Some(rate) = rate {
                caps = caps.field("rate", rate as i32);
            }
            if let Some(channels) = channels {
                caps = caps.field("channels", channels as i32);
            }
            caps.build()
        })
        .max_buffers(64)
        .sync(false)
        .build();
    let appsink_element = appsink.upcast_ref::<gst::Element>().clone();
    pipeline
        .add_many([&decodebin, &convert, &resample, &appsink_element])
        .map_err(|error| format!("Could not build the audio pipeline: {error}"))?;
    gst::Element::link_many([&convert, &resample, &appsink_element])
        .map_err(|error| format!("Could not link the audio pipeline: {error}"))?;

    // Link audio pads only; video pads stay unlinked (this is an audio
    // decode; uridecodebin handles unlinked video branches for short reads).
    {
        let convert_sink = convert.static_pad("sink").expect("audioconvert has a sink pad");
        decodebin.connect_pad_added(move |_bin, pad| {
            if convert_sink.is_linked() {
                return;
            }
            let caps = pad
                .current_caps()
                .or_else(|| Some(pad.query_caps(None)));
            let is_audio = caps.is_some_and(|caps| {
                caps.iter().any(|s| s.name().starts_with("audio/"))
            });
            if is_audio {
                let _ = pad.link(&convert_sink);
            }
        });
    }

    // The extractor's proven flow: preroll to Paused, confirm, run to
    // Playing, confirm — and only then (if needed) FLUSH|ACCURATE-seek in
    // Playing, where push is guaranteed to resume. A FLUSH seek issued
    // while Paused on an appsink chain stalls: the prerolled buffer is
    // dropped and nothing refills until Playing (observed in Packet 1.2).
    pipeline
        .set_state(gst::State::Paused)
        .map_err(|error| format!("Could not preroll the audio pipeline: {error}"))?;
    let (_, state, _) = pipeline.state(gst::ClockTime::from_seconds(10));
    if state != gst::State::Paused {
        let _ = pipeline.set_state(gst::State::Null);
        return Err("Timed out prerolling the audio pipeline (no audio track?)".into());
    }
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| format!("Could not start the audio pipeline: {error}"))?;
    let (_, state, _) = pipeline.state(gst::ClockTime::from_seconds(10));
    if state != gst::State::Playing {
        let _ = pipeline.set_state(gst::State::Null);
        return Err("Timed out starting the audio pipeline".into());
    }
    if start_ns > 0 {
        // Drain the prerolled/stale samples so the region starts clean.
        while appsink.try_pull_sample(gst::ClockTime::ZERO).is_some() {}
        pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::ClockTime::from_nseconds(start_ns),
            )
            .map_err(|error| format!("Could not seek the audio source: {error}"))?;
    }

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_rate = 0_u32;
    let mut channels = 0_usize;
    let started = Instant::now();

    loop {
        if started.elapsed() > DECODE_TIMEOUT {
            let _ = pipeline.set_state(gst::State::Null);
            return Err("Timed out decoding the audio range".into());
        }
        if budget_frames.is_none() && span_ns.is_some() && sample_rate > 0 {
            budget_frames =
                Some(((span_ns.unwrap() as f64 / 1e9) * sample_rate as f64).ceil() as usize);
        }
        if let Some(budget) = budget_frames {
            if channels > 0 && samples.len() / channels >= budget {
                break;
            }
        }
        match appsink.try_pull_sample(gst::ClockTime::from_mseconds(100)) {
            Some(sample) => {
                if sample_rate == 0 {
                    let caps = sample.caps().ok_or_else(|| {
                        let _ = pipeline.set_state(gst::State::Null);
                        "Audio sample arrived without caps".to_owned()
                    })?;
                    let s = caps.structure(0).ok_or_else(|| {
                        let _ = pipeline.set_state(gst::State::Null);
                        "Audio caps have no structure".to_owned()
                    })?;
                    sample_rate = s.get::<i32>("rate").map_err(|error| {
                        let _ = pipeline.set_state(gst::State::Null);
                        format!("Audio caps lack a rate: {error}")
                    })? as u32;
                    channels = s.get::<i32>("channels").map_err(|error| {
                        let _ = pipeline.set_state(gst::State::Null);
                        format!("Audio caps lack channels: {error}")
                    })? as usize;
                }
                let Some(buffer) = sample.buffer() else { continue };
                let Ok(map) = buffer.map_readable() else { continue };
                let bytes = map.as_slice();
                let count = bytes.len() / 4;
                // Safety: F32LE caps guarantee 4-byte little-endian floats,
                // so the byte slice is valid f32 data of exactly this length.
                let chunk: &[f32] =
                    unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast(), count) };
                samples.extend_from_slice(chunk);
            }
            None => {
                // appsink never posts EOS to the pipeline bus (its EOS
                // handling is internal), so poll the sink's own EOS state.
                // A short file can decode to EOS fully before the first pull.
                if appsink.is_eos() {
                    break;
                }
                if channels == 0 && started.elapsed() > Duration::from_secs(5) {
                    // No caps after 5 s: no decodable audio track in range.
                    break;
                }
            }
        }
    }
    let _ = pipeline.set_state(gst::State::Null);

    if channels == 0 {
        return Err("The source has no decodable audio track".into());
    }
    if samples.is_empty() {
        return Err("The requested audio range decoded to zero samples".into());
    }
    let take = budget_frames
        .map(|budget| (budget * channels).min(samples.len()))
        .unwrap_or(samples.len());
    samples.truncate(take);
    Ok(DecodedAudio {
        sample_rate,
        channels,
        samples,
    })
}
