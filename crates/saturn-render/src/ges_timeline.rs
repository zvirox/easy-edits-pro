// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! GES timeline construction for renders and frame exports.
//!
//! Split out of `lib.rs` (Packet 1.1) to keep every source file under the
//! 800-line gate: document slicing to a render range, the GES
//! timeline/clip/effect graph builders, and the render bus loop.
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_editing_services as ges;
use gstreamer_editing_services::prelude::*;
use gstreamer_pbutils as gst_pbutils;
use saturn_core::project::{
    Clip, EffectPayload, MediaKind, ProjectDocument, TrackAudioProcessing, TrackKind,
    TransitionKind,
};
use saturn_core::Tick;

use crate::automation_element;
use crate::graphics_element::{self, ELEMENT_NAME};
use crate::keyframe_element;
use crate::keyframes;
use crate::plugin_element;
use crate::{RenderProgress, RenderRange};

// GES can instantiate effects again during preroll. Keep staged automation,
// graphics and plugin payloads stable until the pipeline is torn down.
pub(crate) static RENDER_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn render_timeline(
    timeline: &ges::Timeline,
    output_uri: &str,
    profile: &gst_pbutils::EncodingContainerProfile,
    on_progress: &mut impl FnMut(RenderProgress),
    is_cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let pipeline = ges::Pipeline::new();
    pipeline
        .set_timeline(timeline)
        .map_err(|error| format!("Could not connect the render timeline: {error}"))?;
    pipeline
        .set_render_settings(output_uri, profile)
        .map_err(|error| format!("Could not configure the render output: {error}"))?;
    pipeline
        .set_mode(ges::PipelineFlags::RENDER)
        .map_err(|error| format!("Could not set rendering mode: {error}"))?;

    let duration = timeline.duration().nseconds().max(1);
    let bus = pipeline
        .bus()
        .ok_or_else(|| "Render pipeline has no message bus".to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| format!("Could not start rendering: {error}"))?;

    let mut last_percent = 0_u8;
    let result = loop {
        if is_cancelled() {
            break Err("Render cancelled".into());
        }
        if let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
            match message.view() {
                gst::MessageView::Eos(..) => break Ok(()),
                gst::MessageView::Error(error) => {
                    break Err(format!(
                        "Render failed: {}{}",
                        error.error(),
                        error
                            .debug()
                            .map(|debug| format!(" ({debug})"))
                            .unwrap_or_default()
                    ));
                }
                _ => {}
            }
        }
        let position = pipeline
            .query_position::<gst::ClockTime>()
            .map(|position| position.nseconds())
            .unwrap_or(0);
        let fraction_percent = ((position.saturating_mul(100) / duration).min(99)) as u8;
        if fraction_percent > last_percent {
            last_percent = fraction_percent;
            on_progress(RenderProgress {
                fraction_percent,
                phase: "Rendering",
            });
        }
    };

    let _ = pipeline.set_state(gst::State::Null);
    result?;
    Ok(())
}


pub(crate) fn slice_document(
    document: &ProjectDocument,
    range: RenderRange,
) -> Result<ProjectDocument, String> {
    if range.start.0 < 0 || range.end.0 <= range.start.0 {
        return Err("Export range must have an Out point after its In point".into());
    }
    let mut sliced = document.clone();
    for track in &mut sliced.project.sequence.tracks {
        let mut clips = Vec::new();
        for original in &track.clips {
            let original_start = original.timeline_start.0;
            let original_end = original_start.saturating_add(original.duration.0);
            let intersect_start = original_start.max(range.start.0);
            let intersect_end = original_end.min(range.end.0);
            if intersect_start >= intersect_end {
                continue;
            }
            let trim = intersect_start - original_start;
            let mut clip = original.clone();
            clip.timeline_start = Tick(intersect_start - range.start.0);
            clip.source_in.0 = clip.source_in.0.saturating_add(trim);
            clip.duration = Tick(intersect_end - intersect_start);
            clip.keyframes
                .retain(|key| key.time.0 >= trim && key.time.0 <= trim + clip.duration.0);
            for key in &mut clip.keyframes {
                key.time.0 -= trim;
            }
            clips.push(clip);
        }
        track.clips = clips;
    }
    sliced.project.sequence.mark_in = None;
    sliced.project.sequence.mark_out = None;
    Ok(sliced)
}

fn transition_clip_timing(track: &saturn_core::Track, clip: &Clip) -> (Tick, Tick, Tick) {
    let mut start = clip.timeline_start.0;
    let mut source_in = clip.source_in.0;
    let mut end = clip.timeline_start.0.saturating_add(clip.duration.0);
    for transition in &track.transitions {
        let overlap = (transition.duration.0 / 2).max(0);
        if end == transition.cut_at.0 {
            end = end.saturating_add(overlap);
        }
        if start == transition.cut_at.0 {
            let pre_roll = overlap.min(source_in.max(0));
            start = start.saturating_sub(pre_roll);
            source_in = source_in.saturating_sub(pre_roll);
        }
    }
    (
        Tick(start),
        Tick(source_in),
        Tick(end.saturating_sub(start)),
    )
}

pub(crate) fn build_timeline(document: &ProjectDocument) -> Result<ges::Timeline, String> {
    let timeline = ges::Timeline::new_audio_video();
    let Some(video_track) = timeline
        .tracks()
        .into_iter()
        .find(|track| track.track_type().contains(ges::TrackType::VIDEO))
    else {
        return Err("Could not create the GStreamer video track".into());
    };
    let Some(audio_track) = timeline
        .tracks()
        .into_iter()
        .find(|track| track.track_type().contains(ges::TrackType::AUDIO))
    else {
        return Err("Could not create the GStreamer audio track".into());
    };

    let video_caps = gst::Caps::builder("video/x-raw")
        .field("width", document.project.settings.width as i32)
        .field("height", document.project.settings.height as i32)
        // Project dimensions describe square pixels. Keep PAR explicit across
        // gaps and clips: adding it at a cut renegotiates encodebin's videorate
        // on GStreamer 1.24, which drops its pending pre-cut frame.
        .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
        .field(
            "framerate",
            gst::Fraction::new(
                document.project.settings.frame_rate.numerator as i32,
                document.project.settings.frame_rate.denominator as i32,
            ),
        )
        .build();
    video_track.update_restriction_caps(&video_caps);
    let audio_caps = gst::Caps::builder("audio/x-raw")
        .field("rate", document.project.settings.audio_sample_rate as i32)
        .build();
    audio_track.update_restriction_caps(&audio_caps);

    // Phase 5 slice 5: compose timeline graphics into export frames through
    // the saturngraphicsblend track effect (saturn-text rasterization).
    if !document.project.sequence.graphics.is_empty() {
        graphics_element::stage_graphics(&document.project.sequence.graphics)
            .map_err(|error| format!("Could not serialize graphics overlay: {error}"))?;
        let effect = ges::Effect::new(ELEMENT_NAME)
            .map_err(|error| format!("Could not create the graphics overlay effect: {error}"))?;
        video_track
            .add_element_full(&effect)
            .map_err(|error| format!("Could not attach the graphics overlay: {error}"))?;
    }

    let any_solo = document.project.sequence.tracks.iter().any(|t| t.solo);

    // Phase 6 slice 1: stage per-track automation for the saturnaudioauto
    // element. A track gets a payload when its automation lane is active, or
    // when it has a static pan (previously ignored by the render path).
    let audio_payloads: Vec<automation_element::AudioAutomationPayload> = document
        .project
        .sequence
        .tracks
        .iter()
        .filter(|track| matches!(track.kind, TrackKind::Audio))
        .filter(|track| track.automation.plays_curve() || track.pan.abs() > 1e-9)
        .map(|track| automation_element::AudioAutomationPayload {
            track_id: track.id,
            base_gain_db: track.gain_db,
            base_pan: track.pan,
            mute: !track.is_audible(any_solo),
            interpolation: track.automation.interpolation,
            volume_points: track.automation.volume_points.clone(),
            pan_points: track.automation.pan_points.clone(),
        })
        .collect();
    if !audio_payloads.is_empty() {
        automation_element::stage_automation(&audio_payloads)
            .map_err(|error| format!("Could not serialize track automation: {error}"))?;
    }

    // Packet 4.1: stage per-track CLAP plugin chains for the saturnaudioplug
    // element. A track gets a payload when its persisted chain has slots; the
    // element runs the same sandboxed host chain the export render uses.
    let plugin_payloads: Vec<plugin_element::PluginChainPayload> = document
        .project
        .sequence
        .tracks
        .iter()
        .filter(|track| matches!(track.kind, TrackKind::Audio))
        .filter(|track| !track.dsp_plugin_chain.is_empty())
        .map(|track| plugin_element::PluginChainPayload {
            track_id: track.id,
            slots: track.dsp_plugin_chain.clone(),
        })
        .collect();
    if !plugin_payloads.is_empty() {
        plugin_element::stage_plugin_chains(&plugin_payloads)
            .map_err(|error| format!("Could not serialize track plugin chains: {error}"))?;
    }

    // F4: stage per-clip visual keyframes for the saturnkfvid element. A clip
    // gets a payload when its model has position/scale/rotation/opacity
    // keyframes; curve times shift to timeline-absolute ticks so the element
    // can sample them straight from the buffer PTS.
    let keyframe_payloads: Vec<keyframes::VisualKeyframePayload> = document
        .project
        .sequence
        .tracks
        .iter()
        .filter(|track| matches!(track.kind, TrackKind::Video))
        .flat_map(|track| {
            track.clips.iter().enumerate().filter_map(
                move |(clip_index, clip)| {
                    keyframes::build_visual_payload(
                        keyframe_clip_key(track.id, clip_index),
                        clip.timeline_start,
                        &clip.keyframes,
                    )
                },
            )
        })
        .collect();
    if !keyframe_payloads.is_empty() {
        keyframe_element::stage_keyframes(&keyframe_payloads)
            .map_err(|error| format!("Could not serialize clip keyframes: {error}"))?;
    }

    for track in &document.project.sequence.tracks {
        let is_audible = track.is_audible(any_solo);
        if track.kind == TrackKind::Audio && !is_audible {
            continue;
        }
        let effective_gain_db = if is_audible { track.gain_db } else { -120.0 };
        let layer = timeline.append_layer();
        for (clip_index, clip) in track.clips.iter().enumerate() {
            let is_audible = is_audible && clip.audio_enabled;
            if track.kind == TrackKind::Audio && !is_audible {
                continue;
            }
            if !clip.enabled {
                // Reference-editor Enable: disabled clips stay on the timeline but render
                // nothing (no picture, no sound).
                continue;
            }
            let (render_start, render_source_in, render_duration) =
                transition_clip_timing(track, clip);
            let media = document
                .project
                .media
                .iter()
                .find(|media| media.id == clip.media_id)
                .ok_or_else(|| {
                    format!("Timeline clip references missing media {}", clip.media_id)
                })?;
            if matches!(media.kind, MediaKind::Title) {
                if let Some(title_clip) = ges::TitleClip::new() {
                    let _ = title_clip.set_start(clock_time(render_start)?);
                    let _ = title_clip.set_duration(clock_time(render_duration)?);
                    if let Some(props) = &media.title {
                        let _ = title_clip.set_child_property("text", &props.text);
                    }
                    let _ = layer.add_clip(&title_clip);
                }
                continue;
            }
            if media.kind == MediaKind::Audio && !is_audible {
                continue;
            }
            if !media.path.is_file() {
                return Err(format!("Source media is missing: {}", media.path.display()));
            }
            if matches!(track.kind, TrackKind::Audio) && !matches!(media.kind, MediaKind::Audio) {
                return Err(format!(
                    "{} is not audio media but is placed on an audio track",
                    media.name
                ));
            }
            let uri = gst::glib::filename_to_uri(&media.path, None).map_err(|error| {
                format!(
                    "Could not read media path {}: {error}",
                    media.path.display()
                )
            })?;
            let ges_clip = ges::UriClip::new(&uri)
                .map_err(|error| format!("Could not decode {}: {error}", media.name))?;
            if !is_audible {
                ges_clip.set_supported_formats(ges::TrackType::VIDEO);
            }
            ges_clip
                .set_start(clock_time(render_start)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the start time for {}", media.name))?;
            ges_clip
                .set_inpoint(clock_time(render_source_in)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the source in-point for {}", media.name))?;
            ges_clip
                .set_duration(clock_time(render_duration)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the duration for {}", media.name))?;

            let track_automation_payload = audio_payloads
                .iter()
                .find(|payload| payload.track_id == track.id);
            let keyframe_clip_key = if matches!(track.kind, TrackKind::Video) {
                let key = keyframe_clip_key(track.id, clip_index);
                keyframe_payloads
                    .iter()
                    .any(|payload| payload.clip_key == key)
                    .then_some(key)
            } else {
                None
            };

            let effective_clip_gain_db =
                if is_audible { effective_gain_db + clip.volume_db } else { 0.0 };
            let silent_processing = TrackAudioProcessing::default();

            apply_clip_effects(
                &ges_clip,
                clip,
                effective_clip_gain_db,
                if is_audible { &track.audio_processing } else { &silent_processing },
                track_automation_payload.filter(|_| is_audible),
                plugin_payloads
                    .iter()
                    .find(|payload| payload.track_id == track.id && is_audible),
                keyframe_clip_key,
                matches!(media.kind, MediaKind::Video | MediaKind::Image),
                document.project.settings.width,
                document.project.settings.height,
            )?;
            layer.add_clip(&ges_clip).map_err(|error| {
                format!(
                    "Could not add {} to the render timeline: {error}",
                    media.name
                )
            })?;
        }
        for trans in &track.transitions {
            let nick = match trans.kind {
                TransitionKind::Crossfade => "crossfade",
                TransitionKind::DipToBlack | TransitionKind::DipToWhite => "fade",
                TransitionKind::WipeLeft | TransitionKind::WipeRight => "wipe",
                TransitionKind::SlideLeft | TransitionKind::SlideRight => "slide",
            };
            if let Some(ges_trans) = ges::TransitionClip::for_nick(nick) {
                // Each model track is rendered as its own GES layer. A GES
                // transition defaults to audio+video; on a video-only source
                // its audio operation has too few inputs and invalidates the
                // whole composition, so constrain it to the model track kind.
                ges_trans.set_supported_formats(match track.kind {
                    TrackKind::Video => ges::TrackType::VIDEO,
                    TrackKind::Audio => ges::TrackType::AUDIO,
                });
                let half_dur = trans.duration.0 / 2;
                let start_tick = Tick(trans.cut_at.0.saturating_sub(half_dur).max(0));
                if let Ok(st) = clock_time(start_tick) {
                    let _ = ges_trans.set_start(st);
                }
                if let Ok(dur) = clock_time(trans.duration) {
                    let _ = ges_trans.set_duration(dur);
                }
                let _ = layer.add_clip(&ges_trans);
            }
        }
    }

    if !timeline.commit_sync() {
        return Err("GStreamer could not finalize the render timeline".into());
    }
    Ok(timeline)
}

/// Stable per-clip-instance key for staged keyframe payloads: the track id
/// in the high 32 bits, the clip index in the low 32.
fn keyframe_clip_key(track_id: u64, clip_index: usize) -> u64 {
    (track_id << 32) | (clip_index as u64 & 0xFFFF_FFFF)
}

fn apply_clip_effects(
    clip: &ges::UriClip,
    model: &Clip,
    gain_db: f64,
    audio_proc: &TrackAudioProcessing,
    automation_payload: Option<&automation_element::AudioAutomationPayload>,
    plugin_payload: Option<&plugin_element::PluginChainPayload>,
    keyframe_clip_key: Option<u64>,
    has_video: bool,
    project_width: u32,
    project_height: u32,
) -> Result<(), String> {
    if has_video {
        // F4: clip keyframes ride saturnkfvid, which warps each frame around
        // the centre (translate/scale/rotate) and multiplies in keyframed
        // opacity through a straight-alpha output.
        if let Some(clip_key) = keyframe_clip_key {
            let effect_str = format!("{} clip-key={}", keyframe_element::ELEMENT_NAME, clip_key);
            let effect = ges::Effect::new(&effect_str).map_err(|error| {
                format!("Could not create the keyframe transform effect: {error}")
            })?;
            clip.add_top_effect(&effect, -1).map_err(|error| {
                format!("Could not attach the keyframe transform effect: {error}")
            })?;
        }
        if model.effects.is_empty() {
            if model.color.exposure != 0.0
                || model.color.contrast != 1.0
                || model.color.saturation != 1.0
                || model.color.hue != 0.0
            {
                let brightness = exposure_to_brightness(model.color.exposure);
                let effect = ges::Effect::new(&format!(
                    "videobalance brightness={brightness:.3} contrast={:.3} saturation={:.3} hue={:.3}",
                    model.color.contrast, model.color.saturation, model.color.hue
                ))
                .map_err(|error| format!("Could not create clip color effect: {error}"))?;
                clip.add_top_effect(&effect, -1)
                    .map_err(|error| format!("Could not apply clip color effect: {error}"))?;
            }
        } else {
            for effect_inst in model.effects.iter().filter(|e| e.enabled) {
                match &effect_inst.payload {
                    EffectPayload::ColorCorrection(params) => {
                        let brightness = exposure_to_brightness(params.exposure);
                        let effect_str = format!(
                            "videobalance brightness={brightness:.3} contrast={:.3} saturation={:.3} hue={:.3}",
                            params.contrast, params.saturation, params.hue
                        );
                        let effect = ges::Effect::new(&effect_str).map_err(|error| {
                            format!("Could not create color correction effect: {error}")
                        })?;
                        clip.add_top_effect(&effect, -1).map_err(|error| {
                            format!("Could not apply color correction effect: {error}")
                        })?;
                    }
                    EffectPayload::GaussianBlur(params) => {
                        if params.sigma > 0.0 {
                            let effect_str = format!("gaussianblur sigma={:.2}", params.sigma);
                            let effect = ges::Effect::new(&effect_str).map_err(|error| {
                                format!("Could not create gaussian blur effect: {error}")
                            })?;
                            clip.add_top_effect(&effect, -1).map_err(|error| {
                                format!("Could not apply gaussian blur effect: {error}")
                            })?;
                        }
                    }
                    EffectPayload::Crop(params) => {
                        let top_px = ((params.top / 100.0) * project_height as f64).round() as u32;
                        let bottom_px =
                            ((params.bottom / 100.0) * project_height as f64).round() as u32;
                        let left_px = ((params.left / 100.0) * project_width as f64).round() as u32;
                        let right_px =
                            ((params.right / 100.0) * project_width as f64).round() as u32;
                        if top_px > 0 || bottom_px > 0 || left_px > 0 || right_px > 0 {
                            let effect_str = format!(
                                "videocrop top={top_px} bottom={bottom_px} left={left_px} right={right_px}"
                            );
                            let effect = ges::Effect::new(&effect_str).map_err(|error| {
                                format!("Could not create crop effect: {error}")
                            })?;
                            clip.add_top_effect(&effect, -1)
                                .map_err(|error| format!("Could not apply crop effect: {error}"))?;
                        }
                    }
                    EffectPayload::ChromaKey(params) => {
                        let (r, g, b) = parse_hex_color(&params.color);
                        let angle = (params.tolerance / 100.0 * 90.0).clamp(0.0, 90.0);
                        let effect_str = format!(
                            "alpha method=custom target-r={r} target-g={g} target-b={b} angle={angle:.1}"
                        );
                        let effect = ges::Effect::new(&effect_str).map_err(|error| {
                            format!("Could not create chroma key effect: {error}")
                        })?;
                        clip.add_top_effect(&effect, -1).map_err(|error| {
                            format!("Could not apply chroma key effect: {error}")
                        })?;
                    }
                    EffectPayload::Vignette(_) => {
                        // Vignette is rendered visually on program monitor; no standalone GStreamer core element
                    }
                    EffectPayload::Lut(params) => {
                        // Apply LUT color grading in GStreamer timeline export
                        if params.intensity > 0.0 {
                            let preset = params.preset_id.as_deref().unwrap_or("teal_orange");
                            let (contrast, saturation, hue) = match preset {
                                "teal_orange" => (1.15, 1.20, -0.05),
                                "vintage" => (0.95, 0.85, 0.05),
                                "film_noir" => (1.35, 0.0, 0.0),
                                "rec709" => (1.10, 1.10, 0.0),
                                _ => (1.05, 1.05, 0.0),
                            };
                            let blend = params.intensity.clamp(0.0, 1.0);
                            let final_contrast = 1.0 + blend * (contrast - 1.0);
                            let final_sat = 1.0 + blend * (saturation - 1.0);
                            let final_hue = blend * hue;
                            let effect_str = format!(
                                "videobalance contrast={:.3} saturation={:.3} hue={:.3}",
                                final_contrast, final_sat, final_hue
                            );
                            if let Ok(effect) = ges::Effect::new(&effect_str) {
                                let _ = clip.add_top_effect(&effect, -1);
                            }
                        }
                    }
                    EffectPayload::CustomShader(_) => {
                        // Custom fragment shaders are executed in the wgpu presentation and offline shader compositor
                    }
                }
            }
        }
    }
    if let Some(payload) = automation_payload {
        // Automation (or static pan) rides saturnaudioauto: per-sample curve
        // evaluation replaces the static clip volume effect entirely.
        let effect_str = format!("{} track-id={}", automation_element::ELEMENT_NAME, payload.track_id);
        match ges::Effect::new(&effect_str) {
            Ok(effect) => clip
                .add_top_effect(&effect, -1)
                .map_err(|error| format!("Could not apply track automation effect: {error}"))?,
            Err(error) => return Err(format!("Could not create track automation effect: {error}")),
        }
    } else if gain_db != 0.0 {
        let amplitude = 10_f64.powf(gain_db / 20.0);
        let effect = ges::Effect::new(&format!("volume volume={amplitude}"))
            .map_err(|error| format!("Could not create track gain effect: {error}"))?;
        clip.add_top_effect(&effect, -1)
            .map_err(|error| format!("Could not apply track gain effect: {error}"))?;
    }
    if let Some(payload) = plugin_payload {
        // Packet 4.1: the track's CLAP chain rides saturnaudioplug, running
        // the sandboxed worker the parity gate compares to the export render.
        let effect_str = format!("{} track-id={}", plugin_element::ELEMENT_NAME, payload.track_id);
        match ges::Effect::new(&effect_str) {
            Ok(effect) => clip
                .add_top_effect(&effect, -1)
                .map_err(|error| format!("Could not apply track plugin effect: {error}"))?,
            Err(error) => return Err(format!("Could not create track plugin effect: {error}")),
        }
    }
    for effect_str in eq_effect_strings(audio_proc) {
        let effect = ges::Effect::new(&effect_str)
            .map_err(|error| format!("Could not create the track EQ effect ({effect_str}): {error}"))?;
        clip.add_top_effect(&effect, -1)
            .map_err(|error| format!("Could not apply the track EQ effect: {error}"))?;
    }
    if audio_proc.denoise_enabled {
        if let Ok(effect) = ges::Effect::new("audiocheblimit mode=1 cutoff=80.0") {
            let _ = clip.add_top_effect(&effect, -1);
        }
    }
    Ok(())
}

/// GStreamer effect strings backing the 4-band track EQ. `equalizer-3bands`
/// carries the low shelf / mid bell / high shelf (its properties are exactly
/// `band0`/`band1`/`band2`); the high-mid bell rides
/// `equalizer-10bands band6` (nearest fixed band to 2.5 kHz) and is skipped
/// when the band rests at 0 dB.
fn eq_effect_strings(proc: &saturn_core::TrackAudioProcessing) -> Vec<String> {
    if !proc.eq_enabled {
        return Vec::new();
    }
    let mut strings = vec![format!(
        "equalizer-3bands band0={:.1} band1={:.1} band2={:.1}",
        proc.eq_low_gain.clamp(-24.0, 12.0),
        proc.eq_mid_gain.clamp(-24.0, 12.0),
        proc.eq_high_gain.clamp(-24.0, 12.0),
    )];
    if proc.eq_highmid_gain.abs() > 1e-9 {
        strings.push(format!(
            "equalizer-10bands band6={:.1}",
            proc.eq_highmid_gain.clamp(-24.0, 12.0),
        ));
    }
    strings
}


fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (0, 255, 0)
    }
}


fn exposure_to_brightness(stops: f64) -> f64 {
    (2_f64.powf(stops.abs()) - 1.0).copysign(stops) / (2_f64.powf(5.0) - 1.0)
}

fn clock_time(tick: Tick) -> Result<gst::ClockTime, String> {
    let nanoseconds = tick
        .as_nanoseconds()
        .ok_or_else(|| "Timeline time cannot be negative".to_string())?;
    Ok(gst::ClockTime::from_nseconds(nanoseconds))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_effect_strings_cover_all_four_bands() {
        let mut proc = saturn_core::TrackAudioProcessing::default();
        assert!(eq_effect_strings(&proc).is_empty(), "bypassed EQ emits nothing");

        proc.eq_enabled = true;
        proc.eq_low_gain = 3.5;
        proc.eq_mid_gain = -2.0;
        proc.eq_high_gain = 4.0;
        assert_eq!(
            eq_effect_strings(&proc),
            vec!["equalizer-3bands band0=3.5 band1=-2.0 band2=4.0".to_string()],
            "shelf bands stay on the 3-band element at zero high-mid"
        );

        proc.eq_highmid_gain = 1.5;
        assert_eq!(
            eq_effect_strings(&proc),
            vec![
                "equalizer-3bands band0=3.5 band1=-2.0 band2=4.0".to_string(),
                "equalizer-10bands band6=1.5".to_string(),
            ],
            "a non-zero high-mid bell chains the 10-band bell"
        );
    }
}
