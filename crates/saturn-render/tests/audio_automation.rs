//! Integration test: `saturnaudioauto` applies track automation envelopes to
//! real audio (Phase 6 slice 1).
//!
//! Pushes a 440 Hz sine through the automation element with a 0 dB to -20 dB
//! linear volume curve and asserts measured RMS windows follow the curve
//! (unity at the head, -10 dB halfway, -20 dB at the tail). Mute renders
//! digital silence.

use gstreamer as gst;
use gstreamer_app as gst_app;
use gst::prelude::*;

use saturn_core::project::AutomationPoint;
use saturn_core::time::{Tick, TICKS_PER_SECOND};
use saturn_render::automation_element::{self, AudioAutomationPayload};

fn ramp_payload(track_id: u64) -> AudioAutomationPayload {
    AudioAutomationPayload {
        track_id,
        base_gain_db: 0.0,
        base_pan: 0.0,
        mute: false,
        // Constant plateaus make every window's expected level exact.
        interpolation: saturn_core::project::AutomationInterpolation::Constant,
        volume_points: vec![
            AutomationPoint {
                time: Tick(0),
                value: 0.0,
            },
            AutomationPoint {
                time: Tick(TICKS_PER_SECOND / 2),
                value: -6.0,
            },
            AutomationPoint {
                time: Tick(TICKS_PER_SECOND + TICKS_PER_SECOND / 2),
                value: -20.0,
            },
        ],
        pan_points: vec![],
    }
}

fn mute_payload(track_id: u64) -> AudioAutomationPayload {
    AudioAutomationPayload {
        mute: true,
        ..ramp_payload(track_id)
    }
}

fn block_rms(samples: &[f32]) -> f64 {
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len().max(1) as f64).sqrt()
}

fn decode_block_f32(buffer: &gst::BufferRef) -> Vec<f32> {
    let map = buffer.map_readable().expect("map buffer");
    let bytes = map.as_slice();
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Runs `payloads` through a sine pipeline and returns (pts_ns, rms) blocks.
/// The staged payload map is process-global, so pipeline runs are serialized
/// to keep cargo's parallel tests from clobbering each other's payloads.
fn run_pipeline(payloads: &[AudioAutomationPayload], track_id: u64) -> Vec<(u64, f64)> {
    static STAGE_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = STAGE_GUARD.lock().expect("stage guard");

    gst::init().expect("gstreamer init");
    automation_element::register().expect("register saturnaudioauto");
    automation_element::stage_automation(payloads).expect("stage payload");

    let pipeline = gst::parse::launch(&format!(
        "audiotestsrc wave=sine freq=440 num-buffers=96 \
         ! audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2 \
         ! saturnaudioauto track-id={track_id} \
         ! appsink name=sink sync=false"
    ))
    .expect("automation pipeline");
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("pipeline element");
    let sink = pipeline
        .by_name("sink")
        .expect("appsink")
        .downcast::<gst_app::AppSink>()
        .expect("appsink type");
    pipeline
        .set_state(gst::State::Playing)
        .expect("pipeline to playing");

    let mut blocks = Vec::new();
    while let Some(sample) = sink.try_pull_sample(Some(gst::ClockTime::from_mseconds(500))) {
        let pts = sample
            .buffer()
            .and_then(|buffer| buffer.pts())
            .map(|p| p.nseconds())
            .unwrap_or(0);
        let buffer = sample.buffer().expect("sample buffer");
        let rms = block_rms(&decode_block_f32(buffer));
        blocks.push((pts, rms));
    }
    let _ = pipeline.set_state(gst::State::Null);
    assert!(!blocks.is_empty(), "expected pulled audio blocks");
    blocks
}

/// RMS over blocks whose PTS falls in `[from_ns, to_ns)`.
fn span_rms(blocks: &[(u64, f64)], from_ns: u128, to_ns: u128) -> f64 {
    let window: Vec<f64> = blocks
        .iter()
        .filter(|(pts, _)| (*pts as u128) >= from_ns && (*pts as u128) < to_ns)
        .map(|(_, rms)| *rms)
        .collect();
    assert!(
        window.len() >= 10,
        "span [{from_ns},{to_ns}) too sparse: {} blocks",
        window.len()
    );
    let sum: f64 = window.iter().map(|rms| rms * rms).sum();
    (sum / window.len() as f64).sqrt()
}

#[test]
fn automation_element_follows_volume_curve() {
    let blocks = run_pipeline(&[ramp_payload(1)], 1);

    // Constant-interpolation plateaus: 0..0.5 s at 0 dB, 0.5..1.5 s at -6 dB,
    // 1.5..2.0 s at -20 dB. Sample well inside each plateau.
    let unity = span_rms(&blocks, 0, 400_000_000);
    let minus6 = span_rms(&blocks, 600_000_000, 1_400_000_000);
    let minus20 = span_rms(&blocks, 1_600_000_000, 2_000_000_000);

    assert!(unity > 0.2, "sine at unity automation should be loud");
    let mid_ratio = minus6 / unity;
    let tail_ratio = minus20 / unity;
    assert!(
        (mid_ratio - 10_f64.powf(-6.0 / 20.0)).abs() < 0.01,
        "-6 dB plateau ratio {mid_ratio} should match ~0.501 amplitude"
    );
    assert!(
        (tail_ratio - 10_f64.powf(-20.0 / 20.0)).abs() < 0.01,
        "-20 dB plateau ratio {tail_ratio} should match 0.1 amplitude"
    );
}

#[test]
fn automation_element_mute_renders_silence() {
    let blocks = run_pipeline(&[mute_payload(2)], 2);
    for (pts, rms) in &blocks {
        assert!(
            *rms < 1e-6,
            "muted track must be silent, pts={pts} rms={rms}"
        );
    }
}

#[test]
fn automation_element_without_payload_is_passthrough() {
    // A different track id: no staged payload, samples must pass unchanged.
    let blocks = run_pipeline(&[ramp_payload(1)], 99);
    let head = span_rms(&blocks, 0, 400_000_000);
    assert!(head > 0.2, "passthrough should be loud, rms={head}");
    let tail = span_rms(&blocks, 1_600_000_000, 2_000_000_000);
    assert!(
        (tail - head).abs() < 0.01,
        "no payload means no automation: {tail} vs {head}"
    );
}
