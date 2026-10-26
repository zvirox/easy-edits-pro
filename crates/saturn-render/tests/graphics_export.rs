//! Integration test: track-level graphics are composited into GES renders
//! through the `saturngraphicsblend` element (Phase 5 slice 5).
//!
//! Uses a title clip (no media file required) plus a rectangle graphic and
//! asserts a real export runs to EOS and produces a non-trivial file. Pixel
//! correctness is covered by the unit tests in `graphics_overlay`.

use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use saturn_core::project::{Clip, Track};
use saturn_core::project::{
    Graphic, GraphicFill, GraphicKind, GraphicTransform, MediaAsset, MediaKind, ProjectDocument,
    TitleProperties, TrackKind,
};
use saturn_core::time::{TICKS_PER_SECOND, Tick};
use saturn_render::{RenderSettings, render_project_range_with_settings};

/// Moves the graphic to the bottom-right so it cannot collide with the
/// GES title's default centred text when checking pixels.
fn frame_scan_test_document() -> ProjectDocument {
    let mut document = document_with_title_and_graphic();
    let graphic = document.project.sequence.graphics.last_mut().unwrap();
    graphic.transform.x = 190.0;
    graphic.transform.y = 150.0;
    document
}

/// Decodes the first frame of `output` as RGBA and asserts the graphic's
/// red rectangle is present in the bottom-right region.
fn assert_red_rectangle_in_export(output: &std::path::Path) {
    use gst::prelude::*;

    gst::init().expect("gstreamer init for decode check");
    let pipeline = gst::parse::launch(&format!(
        "filesrc location=\"{}\" ! decodebin ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink",
        output.display()
    ))
    .expect("decode pipeline");
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("pipeline element");
    let sink = pipeline
        .by_name("sink")
        .expect("appsink")
        .downcast::<gst_app::AppSink>()
        .expect("appsink type");
    sink.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .build(),
    ));
    pipeline
        .set_state(gst::State::Playing)
        .expect("pipeline to playing");

    let bus = pipeline.bus().expect("bus");
    let mut frame = None;
    for _ in 0..600 {
        if let Ok(sample) = sink.pull_sample() {
            let buffer = sample.buffer().expect("sample buffer");
            let caps = sample.caps().expect("sample caps");
            let info = gst_video::VideoInfo::from_caps(caps).expect("video info");
            let map = buffer.map_readable().expect("buffer map");
            let stride = info.stride()[0] as usize;
            let width = info.width() as usize;
            let height = info.height() as usize;
            let data = map.as_slice();
            // Probe the middle of the graphic's rectangle (190,150)+60x40.
            let probe_x = 190 + 60;
            let probe_y = 150 + 40;
            if probe_y < height && probe_x < width {
                let index = probe_y * stride + probe_x * 4;
                if index + 3 < data.len() {
                    frame = Some([data[index], data[index + 1], data[index + 2]]);
                }
            }
            break;
        }
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(50)) else {
            continue;
        };
        if matches!(message.view(), gst::MessageView::Error(_)) {
            break;
        }
    }
    let _ = pipeline.set_state(gst::State::Null);
    let pixel = frame.expect("decoded a frame with the graphic region");
    assert!(
        pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60,
        "expected red graphic ink at (250,190), got RGB {pixel:?}"
    );
}

fn document_with_title_and_graphic() -> ProjectDocument {
    let mut document = ProjectDocument::new("Graphics export test");
    document.project.media.push(MediaAsset {
        id: 1,
        name: "Title card".into(),
        kind: MediaKind::Title,
        title: Some(TitleProperties {
            text: "Hello Saturn".into(),
            ..Default::default()
        }),
        ..Default::default()
    });
    document.project.settings.width = 320;
    document.project.settings.height = 240;

    document.project.sequence.tracks.push(Track {
        id: 1,
        kind: TrackKind::Video,
        name: "V1".into(),
        group_id: None,
        gain_db: 0.0,
        pan: 0.0,
        muted: false,
        solo: false,
        locked: false,
        audio_processing: Default::default(),
        bus: Default::default(),
        automation: Default::default(),
        clips: vec![Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(0),
            duration: Tick(TICKS_PER_SECOND * 2),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: vec![],
            keyframes: vec![],
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
            label: String::new(),
        }],
        transitions: vec![],
    });
    let mut graphic = Graphic {
        id: 2,
        kind: GraphicKind::Rectangle,
        ..Default::default()
    };
    graphic.transform = GraphicTransform {
        x: 10.0,
        y: 10.0,
        width: 120.0,
        height: 80.0,
        ..Default::default()
    };
    graphic.fill = Some(GraphicFill {
        color: "#ff0000".into(),
        opacity: 1.0,
    });
    document.project.sequence.graphics.push(graphic);
    document
}

#[test]
fn graphics_are_blended_into_ges_export() {
    saturn_render::detect_hardware_encoders(); // registers the element too
    // Bottom-right placement, away from the GES title's centred text.
    let document = frame_scan_test_document();
    let output = std::env::temp_dir().join(format!(
        "saturn-graphics-export-{}-{}.mp4",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let settings = RenderSettings {
        width: 320,
        height: 240,
        frame_rate_numerator: 15,
        frame_rate_denominator: 1,
        use_hardware_encoding: false,
    };
    let result =
        render_project_range_with_settings(&document, &output, None, settings, |_progress| {});
    match result {
        Ok(()) => {
            let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
            assert!(size > 1024, "export produced only {size} bytes");
            assert_red_rectangle_in_export(&output);
            let _ = std::fs::remove_file(&output);
        }
        Err(error) => {
            let _ = std::fs::remove_file(&output);
            panic!("graphics export failed: {error}");
        }
    }
}
