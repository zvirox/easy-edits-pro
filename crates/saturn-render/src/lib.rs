use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_editing_services as ges;
use gstreamer_editing_services::prelude::*;
use gstreamer_pbutils as gst_pbutils;
use saturn_core::project::{
    Clip, EffectPayload, MediaKind, ProjectDocument, TrackAudioProcessing, TrackKind,
    TransitionKind,
};
use saturn_core::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub mod automation_element;
pub mod compositor;
pub mod curves;
pub mod graphics_element;
pub mod graphics_overlay;pub mod hsl;
pub mod loudness;
pub mod lut;
pub mod matting;
pub mod multicam;
pub mod shader;
pub mod speedramp;
pub mod tracking;
pub mod wgpu_surface;
pub mod wheels;

pub use compositor::{
    BlendMode, Particle, ParticleEmitter, merge_alpha_over_rgba, rasterize_roto_mask,
    simulate_particle_field,
};
pub use curves::{apply_rgb_curves, apply_rgb_curves_buffer, evaluate_spline, generate_curve_lut};
pub use automation_element::{AudioAutomationPayload, SaturnAudioAuto, ELEMENT_NAME as AUDIO_AUTOMATION_ELEMENT};
pub use graphics_element::{ELEMENT_NAME, SaturnGraphicsBlend, set_pending_graphics_json};
pub use graphics_overlay::{GraphicLayer, render_graphics_overlay};
pub use hsl::{
    apply_hsl_qualifier_buffer, calculate_matte_factor, generate_hsl_matte, hsl_to_rgb, rgb_to_hsl,
};
pub use loudness::{EbuR128Meter, LoudnessReport, LoudnessStandard, analyze_pcm_loudness};
pub use lut::{Lut3D, Lut3DInfo};
pub use matting::{
    BackgroundMode, MattingModel, MattingOptions, MattingTemporalState, run_background_matting,
};
pub use multicam::sync_waveforms_by_correlation;
pub use shader::{SHADER_PRESETS, ShaderPreset, apply_shader_preset_buffer, get_preset_by_id};
pub use speedramp::{
    apply_motion_blur_synthesis, blend_bidirectional_optical_flow, interpolate_speed_ramp,
};
pub use tracking::{compute_stabilization_offset, evaluate_tracking_at, smooth_trajectory};
pub use wgpu_surface::{
    DisplayColorSpace, GpuPreviewPipeline, PresentationMetrics, ScalingFilter, SurfaceBuffer,
    calculate_letterbox_viewport,
};
pub use wheels::{apply_color_wheels_rgb, apply_color_wheels_rgba};

static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareEncoderInfo {
    pub has_hardware_encoder: bool,
    pub preferred_encoder: String,
    pub encoder_type: String, // "vaapi" | "nvenc" | "qsv" | "v4l2" | "software"
    pub description: String,
    pub available_encoders: Vec<String>,
}

pub fn detect_hardware_encoders() -> HardwareEncoderInfo {
    let _ = GST_INIT.get_or_init(|| {
        gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        graphics_element::register()
            .map_err(|error| format!("Could not register the graphics element: {error}"))?;
        automation_element::register()
            .map_err(|error| format!("Could not register the audio automation element: {error}"))?;
        ges::init()
            .map_err(|error| format!("Could not initialize GStreamer Editing Services: {error}"))
    });

    let candidates = [
        (
            "vah264enc",
            "vaapi",
            "VA-API Hardware H.264 (Intel / AMD GPU)",
        ),
        (
            "nvh264enc",
            "nvenc",
            "NVIDIA NVENC Hardware H.264 (NVIDIA GPU)",
        ),
        (
            "qsvh264enc",
            "qsv",
            "Intel Quick Sync Video H.264 (Intel QSV)",
        ),
        ("v4l2h264enc", "v4l2", "V4L2 Hardware Codec H.264"),
    ];

    let mut available = Vec::new();
    let mut selected = None;

    for (name, enc_type, desc) in &candidates {
        if gst::ElementFactory::find(name).is_some() {
            available.push((*name).to_string());
            if selected.is_none() {
                selected = Some((
                    (*name).to_string(),
                    (*enc_type).to_string(),
                    (*desc).to_string(),
                ));
            }
        }
    }

    if let Some((name, enc_type, desc)) = selected {
        HardwareEncoderInfo {
            has_hardware_encoder: true,
            preferred_encoder: name,
            encoder_type: enc_type,
            description: desc,
            available_encoders: available,
        }
    } else {
        let sw_name = if gst::ElementFactory::find("x264enc").is_some() {
            "x264enc"
        } else if gst::ElementFactory::find("openh264enc").is_some() {
            "openh264enc"
        } else {
            "software"
        };
        available.push(sw_name.to_string());
        HardwareEncoderInfo {
            has_hardware_encoder: false,
            preferred_encoder: sw_name.to_string(),
            encoder_type: "software".to_string(),
            description: "High-Quality CPU Software Encoding (x264 / OpenH264)".to_string(),
            available_encoders: available,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderProgress {
    pub fraction_percent: u8,
    pub phase: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderRange {
    pub start: Tick,
    pub end: Tick,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub use_hardware_encoding: bool,
}

impl RenderSettings {
    pub fn from_project(document: &ProjectDocument) -> Self {
        Self {
            width: document.project.settings.width,
            height: document.project.settings.height,
            frame_rate_numerator: document.project.settings.frame_rate.numerator,
            frame_rate_denominator: document.project.settings.frame_rate.denominator,
            use_hardware_encoding: true,
        }
    }
}

struct EncoderRankOverrides(Vec<(gst::ElementFactory, gst::Rank)>);

impl Drop for EncoderRankOverrides {
    fn drop(&mut self) {
        for (factory, rank) in &self.0 {
            factory.set_rank(*rank);
        }
    }
}

fn prefer_hardware_h264_encoder() -> Option<(String, EncoderRankOverrides)> {
    let candidates = ["vah264enc", "qsvh264enc", "nvh264enc", "v4l2h264enc"];
    let (name, selected) = candidates
        .iter()
        .find_map(|name| gst::ElementFactory::find(name).map(|factory| (*name, factory)))?;
    let mut overrides = Vec::new();
    overrides.push((selected.clone(), selected.rank()));
    selected.set_rank(gst::Rank::PRIMARY + 1);
    Some((name.to_owned(), EncoderRankOverrides(overrides)))
}

fn prefer_software_h264_encoder() -> EncoderRankOverrides {
    let mut overrides = Vec::new();
    for name in ["vah264enc", "qsvh264enc", "nvh264enc", "v4l2h264enc"] {
        if let Some(factory) = gst::ElementFactory::find(name) {
            overrides.push((factory.clone(), factory.rank()));
            factory.set_rank(gst::Rank::NONE);
        }
    }
    let software =
        gst::ElementFactory::find("x264enc").or_else(|| gst::ElementFactory::find("openh264enc"));
    if let Some(factory) = software {
        overrides.push((factory.clone(), factory.rank()));
        factory.set_rank(gst::Rank::PRIMARY + 1);
    }
    EncoderRankOverrides(overrides)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Mp4,
    WebM,
}

impl OutputFormat {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("mp4") => Ok(Self::Mp4),
            Some("webm") => Ok(Self::WebM),
            _ => Err("Choose an .mp4 or .webm output file".into()),
        }
    }
}

/// Renders an Easy Edits Pro sequence to MP4 or WebM with GStreamer Editing Services.
/// GES objects are created, used, and dropped on the calling thread because its API is not Send.
pub fn render_project(
    document: &ProjectDocument,
    output_path: &Path,
    on_progress: impl FnMut(RenderProgress),
) -> Result<(), String> {
    render_project_range(document, output_path, None, on_progress)
}

pub fn render_project_range(
    document: &ProjectDocument,
    output_path: &Path,
    range: Option<RenderRange>,
    on_progress: impl FnMut(RenderProgress),
) -> Result<(), String> {
    render_project_range_with_settings(
        document,
        output_path,
        range,
        RenderSettings::from_project(document),
        on_progress,
    )
}

pub fn render_project_range_with_settings(
    document: &ProjectDocument,
    output_path: &Path,
    range: Option<RenderRange>,
    settings: RenderSettings,
    on_progress: impl FnMut(RenderProgress),
) -> Result<(), String> {
    render_project_range_with_control(document, output_path, range, settings, on_progress, || {
        false
    })
}

/// Like `render_project_range_with_settings`, but also lets the owner stop a
/// render while GStreamer is processing it. The callback is checked between
/// bus polls so cancellation latency stays bounded without interrupting GES
/// from another thread.
pub fn render_project_range_with_control(
    document: &ProjectDocument,
    output_path: &Path,
    range: Option<RenderRange>,
    settings: RenderSettings,
    mut on_progress: impl FnMut(RenderProgress),
    is_cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    if is_cancelled() {
        return Err("Render cancelled".into());
    }
    document.validate().map_err(|error| error.to_string())?;
    let format = OutputFormat::from_path(output_path)?;
    if document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("Add at least one clip to the timeline before rendering".into());
    }
    let frame_rate = FrameRate::new(
        settings.frame_rate_numerator,
        settings.frame_rate_denominator,
    )
    .filter(|rate| rate.numerator <= 240 * rate.denominator)
    .ok_or_else(|| "Export frame rate must be between 1 and 240 fps".to_string())?;
    if !(16..=16_384).contains(&settings.width) || !(16..=16_384).contains(&settings.height) {
        return Err("Export dimensions must be between 16 and 16,384 pixels".into());
    }
    let mut export_document = document.clone();
    export_document.project.settings.width = settings.width;
    export_document.project.settings.height = settings.height;
    export_document.project.settings.frame_rate = frame_rate;
    let ranged_document = match range {
        Some(range) => Some(slice_document(&export_document, range)?),
        None => None,
    };
    let render_document = ranged_document.as_ref().unwrap_or(&export_document);
    if render_document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("The In/Out range does not contain any clips to render".into());
    }
    let output_path = absolute_output_path(output_path)?;

        GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
            ges::init().map_err(|error| {
                format!("Could not initialize GStreamer Editing Services: {error}")
            })?;
            graphics_element::register()
                .map_err(|error| format!("Could not register the graphics element: {error}"))?;
            automation_element::register().map_err(|error| {
                format!("Could not register the audio automation element: {error}")
            })
        })
        .clone()?;

    // GStreamer normally gives several optional hardware encoders rank NONE.
    // Raise an installed H.264 hardware encoder above the software encoders so
    // encodebin can select it; if none is installed, its normal x264/OpenH264
    // ranking keeps MP4 export on the software path.
    let encoder_rank_overrides = if format == OutputFormat::Mp4 && settings.use_hardware_encoding {
        prefer_hardware_h264_encoder().map(|(name, overrides)| {
            eprintln!("Using {name} as the preferred H.264 export encoder");
            overrides
        })
    } else {
        None
    };

    on_progress(RenderProgress {
        fraction_percent: 0,
        phase: "Preparing timeline",
    });
    let timeline = build_timeline(render_document)?;
    let profile = encoding_profile(format)?;
    let output_uri = gst::glib::filename_to_uri(&output_path, None)
        .map_err(|error| format!("Output path is not a valid file URI: {error}"))?;

    let mut result = render_timeline(
        &timeline,
        &output_uri,
        &profile,
        &mut on_progress,
        &is_cancelled,
    );
    if result.is_err() && encoder_rank_overrides.is_some() && !is_cancelled() {
        let hardware_error = result.unwrap_err();
        eprintln!(
            "Hardware H.264 render failed ({hardware_error}); retrying with software encoding"
        );
        drop(encoder_rank_overrides);
        let _software_encoder_rank_overrides = prefer_software_h264_encoder();
        let _ = std::fs::remove_file(&output_path);
        on_progress(RenderProgress {
            fraction_percent: 0,
            phase: "Retrying with software encoder",
        });
        let software_timeline = build_timeline(render_document)?;
        result = render_timeline(
            &software_timeline,
            &output_uri,
            &profile,
            &mut on_progress,
            &is_cancelled,
        );
    }
    if is_cancelled() {
        let _ = std::fs::remove_file(&output_path);
        return Err("Render cancelled".into());
    }
    if let Err(error) = result {
        // A failed render must not leave a partial output behind either: a
        // half-written MP4 is worse than no file at all.
        let _ = std::fs::remove_file(&output_path);
        return Err(error);
    }
    on_progress(RenderProgress {
        fraction_percent: 100,
        phase: "Complete",
    });
    Ok(())
}

fn render_timeline(
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

fn slice_document(
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

fn build_timeline(document: &ProjectDocument) -> Result<ges::Timeline, String> {
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

    for track in &document.project.sequence.tracks {
        let is_audible = track.is_audible(any_solo);
        let effective_gain_db = if is_audible { track.gain_db } else { -120.0 };
        let layer = timeline.append_layer();
        for clip in &track.clips {
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

            apply_clip_effects(
                &ges_clip,
                clip,
                effective_gain_db,
                &track.audio_processing,
                track_automation_payload,
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

fn apply_clip_effects(
    clip: &ges::UriClip,
    model: &Clip,
    gain_db: f64,
    audio_proc: &TrackAudioProcessing,
    automation_payload: Option<&automation_element::AudioAutomationPayload>,
    has_video: bool,
    project_width: u32,
    project_height: u32,
) -> Result<(), String> {
    if has_video {
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
    if audio_proc.eq_enabled {
        let eq_str = format!(
            "equalizer-3bands band1={:.1} band2={:.1} band3={:.1}",
            audio_proc.eq_low_gain.clamp(-24.0, 12.0),
            audio_proc.eq_mid_gain.clamp(-24.0, 12.0),
            audio_proc.eq_high_gain.clamp(-24.0, 12.0)
        );
        if let Ok(effect) = ges::Effect::new(&eq_str) {
            let _ = clip.add_top_effect(&effect, -1);
        }
    }
    if audio_proc.denoise_enabled {
        if let Ok(effect) = ges::Effect::new("audiocheblimit mode=1 cutoff=80.0") {
            let _ = clip.add_top_effect(&effect, -1);
        }
    }
    Ok(())
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

fn absolute_output_path(path: &Path) -> Result<PathBuf, String> {
    let filename = path
        .file_name()
        .ok_or_else(|| "Choose a valid output filename".to_string())?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create output folder: {error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("Could not access output folder: {error}"))?;
    Ok(parent.join(filename))
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

fn encoding_profile(format: OutputFormat) -> Result<gst_pbutils::EncodingContainerProfile, String> {
    match format {
        OutputFormat::Mp4 => {
            let container_caps = gst::Caps::builder("video/quicktime")
                .field("variant", "iso")
                .build();
            let video_caps = gst::Caps::builder("video/x-h264").build();
            let audio_caps = gst::Caps::builder("audio/mpeg")
                .field("mpegversion", 4)
                .build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("H.264 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("AAC audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edits Pro MP4")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
        OutputFormat::WebM => {
            let container_caps = gst::Caps::builder("video/webm").build();
            let video_caps = gst::Caps::builder("video/x-vp8").build();
            let audio_caps = gst::Caps::builder("audio/x-vorbis").build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("VP8 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("Vorbis audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edits Pro WebM")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_is_selected_from_extension() {
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.MP4")),
            Ok(OutputFormat::Mp4)
        );
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.webm")),
            Ok(OutputFormat::WebM)
        );
        assert!(OutputFormat::from_path(Path::new("cut.mov")).is_err());
    }

    #[test]
    fn exposure_conversion_has_neutral_and_bounded_values() {
        assert_eq!(exposure_to_brightness(0.0), 0.0);
        assert_eq!(exposure_to_brightness(-5.0), -1.0);
        assert_eq!(exposure_to_brightness(5.0), 1.0);
    }

    #[test]
    fn hex_color_parsing_and_fallback() {
        assert_eq!(parse_hex_color("#00ff00"), (0, 255, 0));
        assert_eq!(parse_hex_color("#ff0000"), (255, 0, 0));
        assert_eq!(parse_hex_color("0000ff"), (0, 0, 255));
        assert_eq!(parse_hex_color("invalid"), (0, 255, 0));
    }

    #[test]
    fn render_cancellation_is_checked_before_pipeline_setup() {
        let document = saturn_core::EditorEngine::default().document().clone();
        // A pre-existing file at the output path belongs to the caller: a
        // cancel that happens before the pipeline starts must not delete it.
        let decoy = std::env::temp_dir().join("eep-cancel-before-start.mp4");
        std::fs::write(&decoy, b"not a render").unwrap();
        let result = render_project_range_with_control(
            &document,
            &decoy,
            None,
            RenderSettings::from_project(&document),
            |_| {},
            || true,
        );
        assert_eq!(result.unwrap_err(), "Render cancelled");
        assert!(decoy.is_file(), "pre-existing output must be untouched");
        let _ = std::fs::remove_file(&decoy);
    }

    /// Laptop-native evidence for planning.md M1.3: cancelling an in-flight
    /// render removes the partial output. Runs the real GES/GStreamer pipeline
    /// and cancels it two seconds in, so it is `#[ignore]`d for CI (runners
    /// have no GES runtime plugins). Execute with:
    ///   cargo test -p saturn-render --lib -- --ignored render_cancel_midway
    #[test]
    #[ignore = "laptop-native: drives the real GES pipeline; CI runners lack the GES runtime plugins"]
    fn render_cancel_midway_removes_partial_output() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/saturn-tauri/tests/fixtures/editor-smoke.webm");
        assert!(fixture.is_file(), "maintained fixture missing: {fixture:?}");

        // Real fixture length via ffprobe, so the appended clips use true
        // media duration (same policy as tests/native/laptop_workflow.py).
        let probe = std::process::Command::new("ffprobe")
            .args(["-v", "quiet", "-print_format", "json", "-show_format"])
            .arg(&fixture)
            .output()
            .expect("ffprobe must be on PATH for laptop-native tests");
        let probe: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        let seconds: f64 = probe["format"]["duration"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let duration_ticks = (seconds * saturn_core::TICKS_PER_SECOND as f64) as u64;

        let mut engine = saturn_core::EditorEngine::default();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.new",
                serde_json::json!({"name": "Cancel evidence"}),
            ))
            .unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                serde_json::json!({
                    "name": fixture.file_name().unwrap().to_string_lossy(),
                    "path": fixture.to_string_lossy(),
                    "kind": "video",
                    "duration": duration_ticks
                }),
            ))
            .unwrap();
        let media_id = engine.document().project.media[0].id;
        let video_track_id = engine
            .document()
            .project
            .sequence
            .tracks
            .iter()
            .find(|track| matches!(track.kind, saturn_core::TrackKind::Video))
            .unwrap()
            .id;
        // A dozen back-to-back full-length clips: far longer than any machine
        // renders inside the two-second cancel window.
        for _ in 0..12 {
            engine
                .execute(saturn_core::CommandRequest::new(
                    "timeline.add_clip",
                    serde_json::json!({"media_id": media_id, "track_id": video_track_id}),
                ))
                .unwrap();
        }
        let document = engine.document().clone();

        let folder = tempfile::tempdir().unwrap();
        let output = folder.path().join("cancelled-evidence.mp4");
        let started = std::time::Instant::now();
        let result = render_project_range_with_control(
            &document,
            &output,
            None,
            RenderSettings::from_project(&document),
            |_| {},
            || started.elapsed() > std::time::Duration::from_secs(2),
        );

        match result {
            Err(error) => {
                assert_eq!(error, "Render cancelled");
                assert!(
                    !output.exists(),
                    "cancelled render left a partial output at {output:?}"
                );
            }
            Ok(()) => panic!(
                "a ~12-clip software-H264 render finished inside the 2s cancel window; \
                 lengthen the timeline before trusting this test"
            ),
            Err(_) => unreachable!(),
        }
    }
}
