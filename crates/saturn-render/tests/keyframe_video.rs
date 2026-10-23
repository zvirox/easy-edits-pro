//! Integration test: `saturnkfvid` bakes clip keyframes into real video (F4).
//!
//! Pushes raw BGRx video through the keyframe element with appsink capture:
//!
//! - an opacity fade from 0.0 at t=0 to 1.0 at t=2s must land the frame
//!   alpha at ~50% exactly halfway and full opacity after;
//! - a constant scale of 0.5 around the centre must leave the centre opaque
//!   and the frame corners transparent (the source no longer covers them).

use gstreamer as gst;
use gstreamer_app as gst_app;
use gst::prelude::*;

use saturn_core::project::{InterpolationKind, Keyframe, KeyframeProperty};
use saturn_core::time::Tick;
use saturn_render::keyframe_element;
use saturn_render::keyframes::VisualKeyframePayload;

fn kf(property: KeyframeProperty, time_ms: i64, value: f64) -> Keyframe {
    let mut keyframe = Keyframe::new(property, Tick::from_millis(time_ms), value);
    keyframe.interpolation = InterpolationKind::Linear;
    keyframe
}

/// Runs raw BGRx video through `saturnkfvid` and returns (pts_ns, bytes)
/// BGRA frames from the appsink. The staged payload map is process-global,
/// so pipeline runs serialize to keep cargo's parallel tests from clobbering
/// each other's payloads (same approach as the audio automation test).
fn run_pipeline(payload: &VisualKeyframePayload, num_buffers: u32) -> Vec<(u64, Vec<u8>)> {
    static STAGE_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = STAGE_GUARD.lock().expect("stage guard");

    gst::init().expect("gstreamer init");
    keyframe_element::register().expect("register saturnkfvid");
    keyframe_element::stage_keyframes(std::slice::from_ref(payload)).expect("stage payload");

    let pipeline = gst::parse::launch(&format!(
        "videotestsrc pattern=white num-buffers={num_buffers} \
         ! video/x-raw,format=BGRx,width=64,height=64,framerate=10/1 \
         ! saturnkfvid clip-key={} \
         ! video/x-raw,format=BGRA \
         ! appsink name=sink sync=false",
        payload.clip_key
    ))
    .expect("launch pipeline");
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("pipeline element");

    let appsink = pipeline
        .by_name("sink")
        .expect("appsink")
        .downcast::<gst_app::AppSink>()
        .expect("appsink type");

    pipeline
        .set_state(gst::State::Playing)
        .expect("play pipeline");

    let mut frames = Vec::new();
    while let Some(sample) = appsink.try_pull_sample(Some(gst::ClockTime::from_seconds(10))) {
        let pts = sample
            .buffer()
            .and_then(|buffer| buffer.pts())
            .map(|pts| pts.nseconds())
            .unwrap_or(0);
        let buffer = sample.buffer().expect("sample buffer");
        let map = buffer.map_readable().expect("map buffer");
        frames.push((pts, map.as_slice().to_vec()));
    }
    pipeline.set_state(gst::State::Null).expect("stop pipeline");
    frames
}

fn alpha_at(frame: &[u8], x: usize, y: usize) -> u8 {
    let stride = 64 * 4;
    frame[y * stride + x * 4 + 3]
}

#[test]
fn opacity_fade_bakes_into_frame_alpha() {
    let payload = VisualKeyframePayload {
        clip_key: 9101,
        opacity: vec![kf(KeyframeProperty::Opacity, 0, 0.0), kf(KeyframeProperty::Opacity, 2_000, 1.0)],
        ..Default::default()
    };
    let frames = run_pipeline(&payload, 40);
    // 40 buffers at 10 fps = 4 s of PTS-ordered frames.
    assert_eq!(frames.len(), 40, "appsink got every frame");
    assert_eq!(frames[0].0, 0, "first frame starts at PTS 0");

    // t=0: fully transparent.
    assert_eq!(alpha_at(&frames[0].1, 32, 32), 0, "start alpha");
    // t=1s: halfway through the linear ramp, half opacity.
    let mid = frames
        .iter()
        .find(|(pts, _)| *pts >= 1_000_000_000)
        .expect("mid frame");
    let mid_alpha = alpha_at(&mid.1, 32, 32);
    assert!(
        (125..=131).contains(&mid_alpha),
        "expected ~128 alpha halfway, got {mid_alpha}"
    );
    // t=3s: fully opaque.
    let tail_alpha = alpha_at(&frames[30].1, 32, 32);
    assert_eq!(tail_alpha, 255, "tail alpha");
}

#[test]
fn scale_down_leaves_corners_transparent() {
    let payload = VisualKeyframePayload {
        clip_key: 9102,
        scale: vec![kf(KeyframeProperty::Scale, 0, 0.5)],
        ..Default::default()
    };
    let frames = run_pipeline(&payload, 3);
    assert!(!frames.is_empty(), "appsink got frames");
    let frame = &frames[frames.len() - 1].1;

    // Centre of the frame is well inside the shrunken source: opaque.
    assert_eq!(alpha_at(frame, 32, 32), 255, "centre stays opaque");
    // The warped source no longer reaches the corners: transparent.
    assert_eq!(alpha_at(frame, 0, 0), 0, "top-left transparent");
    assert_eq!(alpha_at(frame, 63, 0), 0, "top-right transparent");
    assert_eq!(alpha_at(frame, 0, 63), 0, "bottom-left transparent");
    assert_eq!(alpha_at(frame, 63, 63), 0, "bottom-right transparent");
}

#[test]
fn identity_payload_maps_to_an_opaque_copy() {
    // An empty payload leaves the element in the identity path: every pixel
    // opaque, colour planes untouched (white in, white out).
    let payload = VisualKeyframePayload {
        clip_key: 9103,
        ..Default::default()
    };
    let frames = run_pipeline(&payload, 2);
    assert!(!frames.is_empty(), "appsink got frames");
    let frame = &frames[frames.len() - 1].1;
    assert_eq!(alpha_at(frame, 32, 32), 255, "identity stays opaque");
    assert_eq!(alpha_at(frame, 0, 0), 255, "identity covers corners");
    let stride = 64 * 4;
    let pixel = &frame[32 * stride + 32 * 4..32 * stride + 32 * 4 + 3];
    assert!(
        pixel.iter().all(|channel| *channel == 255),
        "white pattern stays white through the identity path: {pixel:?}"
    );
}

#[test]
fn translate_moves_the_sample_window_across_the_source() {
    // A pure rightward translation samples the source shifted left: with a
    // white pattern the frame stays white, but combined with a scale-down the
    // transparent border moves off-centre, proving translation applies.
    let payload = VisualKeyframePayload {
        clip_key: 9104,
        position_x: vec![kf(KeyframeProperty::PositionX, 0, 32.0)],
        scale: vec![kf(KeyframeProperty::Scale, 0, 0.25)],
        ..Default::default()
    };
    let frames = run_pipeline(&payload, 2);
    assert!(!frames.is_empty(), "appsink got frames");
    let frame = &frames[frames.len() - 1].1;

    // Source box: 16 px wide (0.25 scale), centred at 32 + 32 px right =
    // x in 56..72 (clipped to the 64 px frame). The right edge region is
    // inside the box; the left edge is far outside it.
    assert_eq!(alpha_at(frame, 60, 32), 255, "inside translated box");
    assert_eq!(alpha_at(frame, 0, 32), 0, "outside translated box");
}
