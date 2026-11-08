// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 4.1 parity evidence: the render path (`saturnaudioplug`) runs the
//! same sandboxed CLAP chain the engine commands persist.
//!
//! Three layers of evidence:
//!
//! 1. Element-level: the fixture gain plugin doubles a sine exactly (no
//!    codecs between stages).
//! 2. Full export render vs the source: the persisted 2× chain is audible
//!    through the AAC deliverable.
//! 3. Packet 4.1 gate: the engine preview render path and the full export
//!    render match within −90 dBFS — both run the identical plugin element,
//!    so any divergence is a real parity bug.

use gstreamer as gst;
use gstreamer_app as gst_app;
use gst::prelude::*;

use saturn_core::project::DspPluginSlot;
use saturn_render::plugin_element::{self, PluginChainPayload};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Every test here swaps `SATURN_PLUGIN_ROOT` (process-global) and shares
/// the staged payload map, so all pipeline runs serialize through one gate.
fn env_gate() -> MutexGuard<'static, ()> {
    static GATE: OnceLock<Mutex<()>> = OnceLock::new();
    GATE.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn workspace_target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug")
}

/// Locates the built fixture library, mirroring chain_smoke's search.
fn fixture_path() -> PathBuf {
    const LIB: &str = "libsaturn_plugin_fixture.so";
    let manifest_dir = workspace_target_dir();
    for candidate in [
        manifest_dir.join(LIB),
        manifest_dir.join("deps").join(LIB),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    let exe = std::env::current_exe().expect("current_exe");
    for ancestor in exe.ancestors() {
        for candidate in [
            ancestor.join("target/debug").join(LIB),
            ancestor.join("debug").join(LIB),
            ancestor.join(LIB),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!("fixture {LIB} not found; build it: cargo build -p saturn-plugin-fixture");
}

fn gain_slot(value: f64) -> DspPluginSlot {
    DspPluginSlot {
        format: saturn_core::project::DspPluginFormat::Clap,
        path: fixture_path().display().to_string(),
        plugin_id: "org.eep.fixture.gain".to_string(),
        enabled: true,
        params: vec![saturn_core::project::DspPluginSlotParam {
            param_id: 0,
            value,
        }],
    }
}

fn set_root(root: &std::path::Path) {
    // SAFETY: every env swap in this suite is serialized through env_gate().
    unsafe {
        std::env::set_var("SATURN_PLUGIN_ROOT", root.display().to_string());
    }
}

fn clear_root() {
    // SAFETY: see set_root().
    unsafe {
        std::env::remove_var("SATURN_PLUGIN_ROOT");
    }
}

fn block_rms(samples: &[f32]) -> f64 {
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len().max(1) as f64).sqrt()
}

fn decode_block_f32(buffer: &gst::BufferRef) -> Vec<f32> {
    let map = buffer.map_readable().expect("map buffer");
    map.as_slice()
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Pulls every interleaved F32 sample that `pipeline` produces.
fn pull_all_samples(pipeline: &gst::Pipeline, sink: &gst_app::AppSink) -> Vec<f32> {
    let mut samples = Vec::new();
    while let Some(sample) = sink.try_pull_sample(Some(gst::ClockTime::from_mseconds(2_000))) {
        if let Some(buffer) = sample.buffer() {
            samples.extend(decode_block_f32(buffer));
        }
    }
    let _ = pipeline.set_state(gst::State::Null);
    samples
}

// --- 1. Element-level exactness ---------------------------------------------

#[test]
fn plugin_element_doubles_full_scale_and_passthroughs_without_a_chain() {
    let _gate = env_gate();
    set_root(&fixture_path().parent().unwrap().to_path_buf());

    let chain = PluginChainPayload {
        track_id: 1,
        slots: vec![gain_slot(1.0)],
    };
    gst::init().expect("gstreamer init");
    plugin_element::register().expect("register saturnaudioplug");
    plugin_element::stage_plugin_chains(&[chain]).expect("stage chain");

    // Unity chain run: a full-scale sine in, doubled full-scale out.
    let pipeline = gst::parse::launch(
        "audiotestsrc wave=sine freq=1000 num-buffers=48 \
         ! audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2 \
         ! saturnaudioplug track-id=1 \
         ! appsink name=sink sync=false",
    )
    .expect("plugin pipeline")
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
    let doubled = pull_all_samples(&pipeline, &sink);
    assert!(!doubled.is_empty(), "expected pulled audio samples");
    let doubled_rms = block_rms(&doubled);

    // Control run: same pipeline, different track id (nothing staged for it).
    stage_no_chain();
    let passthrough = run_control_pipeline(99);
    let passthrough_rms = block_rms(&passthrough);
    assert!(
        passthrough_rms > 0.2,
        "passthrough should be loud, rms={passthrough_rms}"
    );

    let ratio = doubled_rms / passthrough_rms;
    assert!(
        (ratio - 2.0).abs() / 2.0 < 0.01,
        "fixture gain(1.0) must double the blocker exactly, ratio={ratio}"
    );

    clear_root();
}

/// Removes the staged map so the control pipeline sees no chain at all.
fn stage_no_chain() {
    plugin_element::stage_plugin_chains(&[]).expect("clear staged chains");
}

fn run_control_pipeline(track_id: u64) -> Vec<f32> {
    let pipeline = gst::parse::launch(&format!(
        "audiotestsrc wave=sine freq=1000 num-buffers=48 \
         ! audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2 \
         ! saturnaudioplug track-id={track_id} \
         ! appsink name=sink sync=false"
    ))
    .expect("control pipeline")
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
    pull_all_samples(&pipeline, &sink)
}

// --- 2/3. Render-path parity (engine preview vs full export) -----------------

fn can_render_and_decode() -> bool {
    let _ = gstreamer::init();
    gstreamer::ElementFactory::find("uridecodebin").is_some()
        && gstreamer::ElementFactory::find("x264enc").is_some()
        && (gstreamer::ElementFactory::find("avenc_aac").is_some()
            || gstreamer::ElementFactory::find("fdkaacenc").is_some())
}

/// 16-bit PCM stereo WAV at 48 kHz: 1 kHz sine at −8 dBFS, `frames` frames.
fn write_reference_wav(frames: usize) -> PathBuf {
    let amplitude = 0.4_f64;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f64 / 48_000.0;
        let v = amplitude * (2.0 * std::f64::consts::PI * 1_000.0 * t).sin();
        let q = (v * 32_767.0).round() as i16;
        pcm.extend_from_slice(&q.to_le_bytes());
        pcm.extend_from_slice(&q.to_le_bytes());
    }
    let mut bytes = Vec::with_capacity(44 + pcm.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&((36 + pcm.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&2_u16.to_le_bytes()); // stereo
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&(48_000_u32 * 4).to_le_bytes()); // byte rate
    bytes.extend_from_slice(&4_u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16_u16.to_le_bytes()); // bits
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&pcm);

    let path = std::env::temp_dir().join(format!(
        "eep-dsp-parity-ref-{}-{}.wav",
        std::process::id(),
        frames
    ));
    std::fs::write(&path, bytes).expect("write the reference WAV");
    path
}

/// Builds the engine document: media + audio track + clip + the 2× fixture
/// chain persisted through the real `dsp.load_plugin` command.
fn engine_document_with_chain(
    wav: &std::path::Path,
) -> (saturn_core::EditorEngine, u64) {
    use saturn_core::CommandRequest;

    let mut engine = saturn_core::EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", serde_json::json!({"name": "DSP"})))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            serde_json::json!({
                "name": wav.file_name().unwrap().to_string_lossy(),
                "path": wav.to_string_lossy(),
                "kind": "audio"
            }),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "timeline.add_track",
            serde_json::json!({"kind": "audio"}),
        ))
        .unwrap();
    let media_id = engine.document().project.media[0].id;
    let audio_track_id = engine
        .document()
        .project
        .sequence
        .tracks
        .iter()
        .find(|track| matches!(track.kind, saturn_core::TrackKind::Audio))
        .unwrap()
        .id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            serde_json::json!({"media_id": media_id, "track_id": audio_track_id}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            serde_json::json!({
                "track_id": audio_track_id,
                "format": "clap",
                "path": fixture_path().display().to_string(),
                "plugin_id": "org.eep.fixture.gain",
                "params": [{"param_id": 0, "value": 1.0}],
            }),
        ))
        .unwrap();
    (engine, audio_track_id)
}

/// Decodes a file's first audio track to interleaved F32 at project settings.
fn decode_to_f32(path: &std::path::Path) -> Vec<f32> {
    let uri = gst::glib::filename_to_uri(path, None).expect("file uri");
    let pipeline = gst::parse::launch(&format!(
        "uridecodebin uri={uri} ! audioconvert ! audioresample \
         ! audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2 \
         ! appsink name=sink sync=false"
    ))
    .expect("decode pipeline")
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
    pull_all_samples(&pipeline, &sink)
}

/// Mean-square residual of two equally sized signals, in dBFS (full scale = 1.0).
fn residual_dbfs(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len().min(b.len());
    assert!(n > 0, "cannot compare empty signals");
    let sum: f64 = a[..n]
        .iter()
        .zip(b[..n].iter())
        .map(|(&x, &y)| {
            let d = (x - y) as f64;
            d * d
        })
        .sum();
    10.0 * (sum / n as f64).log10()
}

#[test]
fn preview_and_export_renders_parity_within_90_dbfs() {
    let test_name = "preview_and_export_renders_parity_within_90_dbfs";
    let _gate = env_gate();
    set_root(&fixture_path().parent().unwrap().to_path_buf());
    if !can_render_and_decode() {
        eprintln!("skipping: no GStreamer decode/encode plugins; run laptop-native for evidence");
        clear_root();
        return;
    }

    let wav = write_reference_wav(48_000); // 1 s
    let (engine, _track_id) = engine_document_with_chain(&wav);
    let document = engine.document().clone();
    let scratch = std::env::temp_dir();
    let export_path = scratch.join(format!("eep-dsp-parity-export-{test_name}.mp4"));
    let preview_path = scratch.join(format!("eep-dsp-parity-preview-{test_name}.mp4"));
    let _ = std::fs::remove_file(&export_path);
    let _ = std::fs::remove_file(&preview_path);

    // Full export render.
    saturn_render::render_project(&document, &export_path, |_| {})
        .unwrap_or_else(|error| panic!("export render failed: {error}"));
    // Engine preview render path (plan §4.4: same element, preview settings).
    let settings = saturn_render::RenderSettings::from_project(&document);
    let tps = saturn_core::time::TICKS_PER_SECOND;
    saturn_render::render_project_range_with_settings(
        &document,
        &preview_path,
        Some(saturn_render::RenderRange {
            start: saturn_core::Tick(0),
            end: saturn_core::Tick(tps),
        }),
        settings,
        |_| {},
    )
    .unwrap_or_else(|error| panic!("preview render failed: {error}"));

    let export_pcm = decode_to_f32(&export_path);
    let preview_pcm = decode_to_f32(&preview_path);
    let source_pcm = decode_to_f32(&wav);
    std::fs::remove_file(&wav).ok();
    std::fs::remove_file(&export_path).ok();
    std::fs::remove_file(&preview_path).ok();
    clear_root();

    // Packet 4.1 gate: both paths run the identical element; the residual
    // between them must sit below −90 dBFS.
    let residual = residual_dbfs(&preview_pcm, &export_pcm);
    assert!(
        residual < -90.0,
        "preview vs export residual {residual} dBFS exceeds the −90 dBFS parity gate"
    );

    // Sanity: the persisted 2× chain is really audible in the deliverable
    // (guards against a silently skipped plugin element passing the gate).
    let gain_ratio = block_rms(&export_pcm) / block_rms(&source_pcm).max(1e-9);
    assert!(
        (gain_ratio - 2.0).abs() < 0.2,
        "export audio should carry the fixture 2× gain, ratio={gain_ratio}"
    );
}
