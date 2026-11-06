// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Native timeline rendering for Easy Edits Pro (GStreamer Editing Services).
//!
//! This module owns the public render entry points (`render_project*`,
//! `render_frame`). The encoder-selection mechanics live in [`encoders`], the
//! GES timeline graph construction in [`ges_timeline`], and frame/sheet
//! extraction in [`frame_extractor`].
use gstreamer as gst;
use gstreamer_editing_services as ges;
use saturn_core::time::TICKS_PER_SECOND;
use saturn_core::project::ProjectDocument;
use saturn_core::{FrameRate, Tick};
use std::path::Path;

pub mod automation_element;
pub mod audio_pcm;
pub mod audio_telemetry;
pub mod compositor;
pub mod curves;
pub mod encoders;
pub mod frame_extractor;
pub mod ges_timeline;
pub mod graphics_element;
pub mod keyframe_element;
pub mod keyframes;
pub mod graphics_overlay;
pub mod preview_tools;
pub mod scopes_gpu;
pub mod hsl;
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
pub use encoders::{detect_hardware_encoders, HardwareEncoderInfo, OutputFormat};pub use graphics_element::{ELEMENT_NAME, SaturnGraphicsBlend, set_pending_graphics_json};
pub use graphics_overlay::{GraphicLayer, render_graphics_overlay};
pub use hsl::{
    apply_hsl_qualifier_buffer, calculate_matte_factor, generate_hsl_matte, hsl_to_rgb, rgb_to_hsl,
};
pub use scopes_gpu::{analyze_rgb_scopes, ScopeTelemetryReport};
pub use audio_telemetry::{AudioTelemetryReport, BAND_CENTER_HZ, analyze_pcm_audio_telemetry};
pub use audio_pcm::{decode_audio_pcm, decode_audio_pcm_mono_16k};
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

use encoders::{
    absolute_output_path, encoding_profile, prefer_hardware_h264_encoder,
    prefer_software_audio_encoder, prefer_software_h264_encoder,
};
use ges_timeline::{build_timeline, render_timeline, slice_document};
pub(crate) use encoders::GST_INIT;

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
            })?;
            keyframe_element::register().map_err(|error| {
                format!("Could not register the keyframe video element: {error}")
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
    // Held for both the hardware and software attempts: the audio encoder
    // must stay above encodebin's MARGINAL lookup for the whole render.
    let _audio_encoder_rank_overrides = prefer_software_audio_encoder(format);
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


/// Renders the timeline frame at `at` to a PNG still image.
///
/// The document is sliced to the single frame containing `at` (one frame in
/// duration) so GES only has to preroll a frame's worth of media. The frame is
/// pulled from an appsink video preview sink and encoded as an 8-bit RGB PNG.
/// GES objects are created, used, and dropped on the calling thread because
/// its API is not Send.
pub fn render_frame(
    document: &ProjectDocument,
    at: Tick,
    output_path: &Path,
) -> Result<(), String> {
    document.validate().map_err(|error| error.to_string())?;
    if output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some("png")
    {
        return Err("Choose a .png output file for the frame export".into());
    }
    if document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("Add at least one clip to the timeline before exporting a frame".into());
    }
    let frame_rate = &document.project.settings.frame_rate;
    let numerator = i64::from(frame_rate.numerator).max(1);
    let denominator = i64::from(frame_rate.denominator).max(1);
    let frame_duration_ticks = (TICKS_PER_SECOND * denominator / numerator).max(1);
    let render_document = slice_document(
        document,
        RenderRange {
            start: at,
            end: Tick(at.0 + frame_duration_ticks),
        },
    )?;
    if render_document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("The playhead is not over any clips to export".into());
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
            })?;
            keyframe_element::register().map_err(|error| {
                format!("Could not register the keyframe video element: {error}")
            })
        })
        .clone()?;

    // Reuse the matrix-proven render path instead of a bespoke preview
    // pipeline: a GES preview pull with a custom appsink deadlocks waiting
    // for the composition's initializing seek, so a frame export would
    // never finish. Rendering a two-frame snippet and decoding its first
    // frame is a few seconds for a still — and provably correct.
    let snippet_frames = 2;
    let snippet_path = crate::frame_extractor::previews_dir()?
        .join(format!("frame-export-{}.mp4", at.0));
    let result = render_project_range_with_settings(
        document,
        &snippet_path,
        Some(RenderRange {
            start: at,
            end: Tick(at.0 + frame_duration_ticks * snippet_frames),
        }),
        RenderSettings {
            use_hardware_encoding: false,
            ..RenderSettings::from_project(document)
        },
        |_| {},
    );
    if let Err(error) = result {
        let _ = std::fs::remove_file(&snippet_path);
        return Err(format!("Could not render the frame snippet: {error}"));
    }
    // First frame of the snippet is exactly the requested tick.
    let (width, height, rows) = match crate::frame_extractor::decode_frame_rgb(
        &snippet_path,
        Tick(0),
    ) {
        Ok((width, height, rgb)) => (width, height, rgb),
        Err(error) => {
            let _ = std::fs::remove_file(&snippet_path);
            return Err(format!("Could not decode the rendered frame: {error}"));
        }
    };
    let _ = std::fs::remove_file(&snippet_path);

    let file = std::fs::File::create(&output_path)
        .map_err(|error| format!("Could not create {output_path:?}: {error}"))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&rows))
        .map_err(|error| format!("Could not encode the PNG frame: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;


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


    #[test]
    fn render_frame_rejects_bad_output_paths_and_empty_timelines() {
        let document = saturn_core::EditorEngine::default().document().clone();

        // Only PNG stills are supported.
        let mp4_path = std::env::temp_dir().join("eep-frame-not-png.mp4");
        let error = render_frame(&document, saturn_core::Tick(0), &mp4_path)
            .err()
            .expect("an .mp4 output must be rejected");
        assert!(error.contains(".png"), "unexpected error: {error}");

        // An empty timeline has nothing to grab.
        let png_path = std::env::temp_dir().join("eep-frame-empty.png");
        let error = render_frame(&document, saturn_core::Tick(0), &png_path)
            .err()
            .expect("an empty timeline must be rejected");
        assert!(
            error.contains("at least one clip"),
            "unexpected error: {error}"
        );
        assert!(!png_path.exists(), "no output may be written on rejection");
    }


    #[test]
    fn render_frame_playhead_off_timeline_is_rejected_before_gst() {
        // Playhead past every clip: the sliced document has no clips.
        let mut engine = saturn_core::EditorEngine::default();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.new",
                serde_json::json!({"name": "Frame"}),
            ))
            .unwrap();
        let media = engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                serde_json::json!({
                    "name":"a.mov", "path":"/assets/a.mov", "kind":"video",
                    "duration": 254_016_000_000i64
                }),
            ))
            .unwrap();
        let media_id = media["media_id"].as_u64().unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "timeline.insert_clip",
                serde_json::json!({
                    "media_id": media_id, "track_id": 1, "at": 0,
                    "source_in": 0, "duration": 254_016_000_000i64
                }),
            ))
            .unwrap();
        let document = engine.document().clone();
        // One second past the clip end.
        let png_path = std::env::temp_dir().join("eep-frame-off-timeline.png");
        let error = render_frame(&document, saturn_core::Tick(254_016_000_000i64), &png_path)
            .err()
            .expect("a playhead past the timeline must be rejected");
        assert!(
            error.contains("not over any clips"),
            "unexpected error: {error}"
        );
        assert!(!png_path.exists());
    }


    /// Laptop-native evidence for planning.md M1.3: cancelling an in-flight
    /// render removes the partial output. Runs the real GES/GStreamer pipeline

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
            }
        }
}
