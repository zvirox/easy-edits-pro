//! `saturnaudioauto`: a GStreamer audio filter that plays track automation
//! curves (Phase 6 slice 1).
//!
//! The element is registered at render start and attached per audio clip in
//! [`crate::apply_clip_effects`] when the owning track has active automation
//! (or a static pan that the render path must now honour). Per buffer it takes
//! the buffer PTS as timeline time, evaluates the track's volume (dB) and pan
//! breakpoint curves for every sample, and scales the interleaved samples in
//! place. Stereo streams use constant-power pan; mono and >2-channel streams
//! get volume only.
//!
//! Automation reaches the element through a module-level handoff
//! ([`stage_automation`]) keyed by the track's `track-id` element property,
//! because `ges::Effect` instantiates the wrapped element from a bin
//! description with no typed property path. GES may instantiate the effect
//! several times (probe + live instances), so the staged map is peeked, never
//! consumed, per instance.

use gstreamer as gst;
use gstreamer_base as gst_base;

use gst::glib;
use gst::prelude::*;
use saturn_core::project::{AutomationInterpolation, AutomationPoint};
use saturn_core::time::{Tick, TICKS_PER_SECOND};

pub const ELEMENT_NAME: &str = "saturnaudioauto";

/// Per-track automation payload staged for element instances.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AudioAutomationPayload {
    pub track_id: u64,
    /// Fader gain used where the volume curve has no coverage.
    pub base_gain_db: f64,
    /// Fader pan used where the pan curve has no coverage.
    pub base_pan: f64,
    /// Muted / soloed-away tracks render silence.
    pub mute: bool,
    pub interpolation: AutomationInterpolation,
    #[serde(default)]
    pub volume_points: Vec<AutomationPoint>,
    #[serde(default)]
    pub pan_points: Vec<AutomationPoint>,
}

mod imp {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use gstreamer as gst;
    use gstreamer_base as gst_base;

    use gst_base::subclass::prelude::*;

    use super::*;

    /// Track-keyed payloads staged by `build_timeline` for this render.
    /// Peeked (not consumed) because GES instantiates effect elements several
    /// times per clip and every instance needs its track's payload.
    static STAGED_PAYLOADS: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();

    fn staged_payloads() -> &'static Mutex<HashMap<u64, String>> {
        STAGED_PAYLOADS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Stages (replaces) the per-track automation payloads for this render.
    pub(super) fn stage_payloads(
        payloads: &[AudioAutomationPayload],
    ) -> Result<(), serde_json::Error> {
        let mut staged = staged_payloads().lock().expect("automation staging lock");
        staged.clear();
        for payload in payloads {
            staged.insert(payload.track_id, serde_json::to_string(payload)?);
        }
        Ok(())
    }

    fn payload_for(track_id: u64) -> Option<AudioAutomationPayload> {
        let staged = staged_payloads().lock().expect("automation staging lock");
        let json = staged.get(&track_id)?;
        match serde_json::from_str::<AudioAutomationPayload>(json) {
            Ok(payload) => Some(payload),
            Err(error) => {
                eprintln!("saturnaudioauto: invalid staged payload: {error}");
                None
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum SampleFormat {
        F32,
        S16,
        S32,
    }

    /// Moving-pointer sampler over a sorted breakpoint curve.
    #[derive(Debug)]
    struct CurveSampler {
        points: Vec<(i64, f64)>,
        interpolation: AutomationInterpolation,
        fallback: f64,
        segment: usize,
    }

    impl CurveSampler {
        fn new(
            points: &[AutomationPoint],
            interpolation: AutomationInterpolation,
            fallback: f64,
        ) -> Self {
            Self {
                points: points.iter().map(|p| (p.time.0, p.value)).collect(),
                interpolation,
                fallback,
                segment: 0,
            }
        }

        fn sample(&mut self, time: i64) -> f64 {
            let points = &self.points;
            if points.is_empty() {
                return self.fallback;
            }
            if time <= points[0].0 {
                return points[0].1;
            }
            let last = points.len() - 1;
            if time >= points[last].0 {
                return points[last].1;
            }
            // Reset the cursor if time jumps backwards (rare; defensive).
            if points[self.segment].0 > time {
                self.segment = 0;
            }
            while self.segment + 1 < points.len() && points[self.segment + 1].0 <= time {
                self.segment += 1;
            }
            let (a_time, a_value) = points[self.segment];
            let next = self.segment + 1;
            let (b_time, b_value) = if next < points.len() {
                points[next]
            } else {
                (a_time, a_value)
            };
            let span = (b_time - a_time).max(1);
            let raw = (time - a_time) as f64 / span as f64;
            match self.interpolation {
                AutomationInterpolation::Constant => a_value,
                AutomationInterpolation::Smooth => {
                    let t = 0.5 * (1.0 - (raw * std::f64::consts::PI).cos());
                    a_value + (b_value - a_value) * t
                }
                AutomationInterpolation::Linear => a_value + (b_value - a_value) * raw,
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct SaturnAudioAuto {
        track_id: std::sync::atomic::AtomicU64,
        payload: Mutex<Option<AudioAutomationPayload>>,
        format: Mutex<Option<(SampleFormat, u32, u32)>>, // (format, channels, rate)
    }

    impl SaturnAudioAuto {
        fn new() -> Self {
            Self::default()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SaturnAudioAuto {
        const NAME: &'static str = "SaturnAudioAuto";
        type Type = super::SaturnAudioAuto;
        type ParentType = gst_base::BaseTransform;

        fn with_class(_klass: &Self::Class) -> Self {
            Self::new()
        }
    }

    impl ObjectImpl for SaturnAudioAuto {
        fn constructed(&self) {
            self.parent_constructed();
            let track_id = self.track_id.load(std::sync::atomic::Ordering::Relaxed);
            *self.payload.lock().expect("audio automation payload lock") = payload_for(track_id);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![glib::ParamSpecUInt64::builder("track-id")
                    .nick("Track id")
                    .blurb("Timeline track whose staged automation payload applies")
                    .build()]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "track-id" => self
                    .track_id
                    .store(value.get().unwrap_or(0), std::sync::atomic::Ordering::Relaxed),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "track-id" => self
                    .track_id
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl GstObjectImpl for SaturnAudioAuto {}

    impl ElementImpl for SaturnAudioAuto {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "Saturn audio automation",
                    "Filter/Effect/Audio",
                    "Applies track volume/pan automation curves to interleaved audio",
                    "Project Saturn",
                )
            }))
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
            TEMPLATES.get_or_init(|| {
                let caps = gst::Caps::builder("audio/x-raw")
                    .field("format", gst::List::new(["F32LE", "S16LE", "S32LE"]))
                    .field("layout", "interleaved")
                    .field("rate", gst::IntRange::<i32>::new(1, i32::MAX))
                    .field("channels", gst::IntRange::<i32>::new(1, i32::MAX))
                    .build();
                let src = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    &caps,
                )
                .expect("saturnaudioauto src template");
                let sink = gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &caps,
                )
                .expect("saturnaudioauto sink template");
                vec![src, sink]
            })
        }
    }

    impl BaseTransformImpl for SaturnAudioAuto {
        const MODE: gst_base::subclass::BaseTransformMode =
            gst_base::subclass::BaseTransformMode::AlwaysInPlace;
        const PASSTHROUGH_ON_SAME_CAPS: bool = false;
        const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

        fn set_caps(
            &self,
            incaps: &gst::Caps,
            _outcaps: &gst::Caps,
        ) -> Result<(), gst::LoggableError> {
            let structure = incaps.structure(0).ok_or_else(|| {
                gst::loggable_error!(gst::CAT_RUST, "saturnaudioauto: caps without a structure")
            })?;
            let format = match structure.get::<&str>("format").unwrap_or("") {
                "F32LE" => SampleFormat::F32,
                "S16LE" => SampleFormat::S16,
                "S32LE" => SampleFormat::S32,
                other => {
                    return Err(gst::loggable_error!(
                        gst::CAT_RUST,
                        "saturnaudioauto: unsupported format {other}"
                    ));
                }
            };
            let channels: i32 = structure.get("channels").unwrap_or(2);
            let rate: i32 = structure.get("rate").unwrap_or(48_000);
            *self.format.lock().expect("audio info lock") =
                Some((format, channels.max(1) as u32, rate.max(1) as u32));
            Ok(())
        }

        fn transform_ip(
            &self,
            buf: &mut gst::BufferRef,
        ) -> Result<gst::FlowSuccess, gst::FlowError> {
            let mut payload = self.payload.lock().expect("audio automation payload lock");
            // GObject properties are assigned after constructed(), so resolve
            // the staged payload lazily on the first buffer instead.
            if payload.is_none() {
                let track_id = self.track_id.load(std::sync::atomic::Ordering::Relaxed);
                *payload = payload_for(track_id);
            }
            let Some(payload) = payload.as_mut() else {
                return Ok(gst::FlowSuccess::Ok);
            };
            let Some((format, channels, rate)) = self.format.lock().expect("audio info lock").clone()
            else {
                return Ok(gst::FlowSuccess::Ok);
            };

            let sample_width = match format {
                SampleFormat::F32 | SampleFormat::S32 => 4usize,
                SampleFormat::S16 => 2usize,
            };
            let frame_size = sample_width * channels as usize;
            let total_frames = buf.size() / frame_size;
            if total_frames == 0 {
                return Ok(gst::FlowSuccess::Ok);
            }

            // Fast path: muted tracks render digital silence.
            if payload.mute {
                let mut data = buf.map_writable().map_err(|_| gst::FlowError::Error)?;
                data.as_mut_slice().fill(0);
                return Ok(gst::FlowSuccess::Ok);
            }

            // Fast path: constant unity gain and no pan needs no work.
            let pan_curve_active =
                !payload.pan_points.is_empty() || payload.base_pan.abs() > 1e-9;
            if payload.volume_points.is_empty()
                && payload.base_gain_db.abs() < 1e-9
                && !pan_curve_active
            {
                return Ok(gst::FlowSuccess::Ok);
            }

            let stereo = channels == 2;
            let mut volume_sampler = CurveSampler::new(
                &payload.volume_points,
                payload.interpolation,
                payload.base_gain_db,
            );
            let mut pan_sampler = CurveSampler::new(
                &payload.pan_points,
                payload.interpolation,
                payload.base_pan,
            );

            let pts_ns = buf.pts().map(|pts| pts.nseconds()).unwrap_or(0);
            let start_tick = pts_to_ticks(pts_ns);
            let ticks_per_sample = TICKS_PER_SECOND as i64 / rate as i64;

            let mut data = buf.map_writable().map_err(|_| gst::FlowError::Error)?;
            let bytes = data.as_mut_slice();

            match format {
                SampleFormat::F32 => {
                    let samples = as_mut_slice::<f32>(bytes)?;
                    let frame_stride = channels as usize;
                    for frame in 0..total_frames {
                        let time = start_tick.0 + frame as i64 * ticks_per_sample;
                        let amplitude = 10_f64.powf(volume_sampler.sample(time) / 20.0) as f32;
                        if stereo {
                            let pan = pan_sampler.sample(time) as f32;
                            let (left, right) = constant_power(pan);
                            let base = frame * frame_stride;
                            samples[base] *= amplitude * left;
                            samples[base + 1] *= amplitude * right;
                        } else {
                            let base = frame * frame_stride;
                            for sample in &mut samples[base..base + frame_stride] {
                                *sample *= amplitude;
                            }
                        }
                    }
                }
                SampleFormat::S16 => {
                    let samples = as_mut_slice::<i16>(bytes)?;
                    let frame_stride = channels as usize;
                    for frame in 0..total_frames {
                        let time = start_tick.0 + frame as i64 * ticks_per_sample;
                        let ampl = 10_f64.powf(volume_sampler.sample(time) / 20.0) as f32;
                        if stereo {
                            let pan = pan_sampler.sample(time) as f32;
                            let (left, right) = constant_power(pan);
                            let base = frame * frame_stride;
                            samples[base] = scale_i16(samples[base], ampl * left);
                            samples[base + 1] = scale_i16(samples[base + 1], ampl * right);
                        } else {
                            let base = frame * frame_stride;
                            for sample in &mut samples[base..base + frame_stride] {
                                *sample = scale_i16(*sample, ampl);
                            }
                        }
                    }
                }
                SampleFormat::S32 => {
                    let samples = as_mut_slice::<i32>(bytes)?;
                    let frame_stride = channels as usize;
                    for frame in 0..total_frames {
                        let time = start_tick.0 + frame as i64 * ticks_per_sample;
                        let ampl = 10_f64.powf(volume_sampler.sample(time) / 20.0) as f32;
                        if stereo {
                            let pan = pan_sampler.sample(time) as f32;
                            let (left, right) = constant_power(pan);
                            let base = frame * frame_stride;
                            samples[base] = scale_i32(samples[base], ampl * left);
                            samples[base + 1] = scale_i32(samples[base + 1], ampl * right);
                        } else {
                            let base = frame * frame_stride;
                            for sample in &mut samples[base..base + frame_stride] {
                                *sample = scale_i32(*sample, ampl);
                            }
                        }
                    }
                }
            }
            Ok(gst::FlowSuccess::Ok)
        }
    }

    /// Reinterprets a writable byte slice as a sample slice; length is trusted
    /// because `total_frames` was derived from the same byte count.
    fn as_mut_slice<T>(bytes: &mut [u8]) -> Result<&mut [T], gst::FlowError> {
        if bytes.len() % std::mem::size_of::<T>() != 0 {
            return Err(gst::FlowError::Error);
        }
        let count = bytes.len() / std::mem::size_of::<T>();
        Ok(unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr() as *mut T, count) })
    }

    fn scale_i16(sample: i16, gain: f32) -> i16 {
        let scaled = sample as f32 * gain;
        scaled.round().clamp(-32_768.0, 32_767.0) as i16
    }

    fn scale_i32(sample: i32, gain: f32) -> i32 {
        // f32 cannot represent i32 limits exactly; clamp into a safe range.
        let scaled = sample as f32 * gain;
        scaled.clamp(-2_147_483_000.0, 2_147_483_000.0) as i32
    }

    /// Constant-power pan: -1.0 hard left, 0.0 centre (-3 dB both sides),
    /// +1.0 hard right.
    fn constant_power(pan: f32) -> (f32, f32) {
        let clamped = pan.clamp(-1.0, 1.0);
        let angle = (clamped + 1.0) * std::f32::consts::FRAC_PI_4;
        (angle.cos(), angle.sin())
    }

    /// Timeline ticks from buffer PTS nanoseconds (GES timelines start at 0).
    fn pts_to_ticks(nanoseconds: u64) -> Tick {
        let ticks = (nanoseconds as i128)
            .saturating_mul(TICKS_PER_SECOND as i128)
            .saturating_div(1_000_000_000);
        Tick(ticks.clamp(0, i64::MAX as i128) as i64)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn point(time_ms: i64, value: f64) -> AutomationPoint {
            AutomationPoint {
                time: Tick::from_millis(time_ms),
                value,
            }
        }

        #[test]
        fn sampler_matches_core_evaluation() {
            let points = vec![point(0, 0.0), point(1_000, 12.0)];
            let mut sampler = CurveSampler::new(&points, AutomationInterpolation::Linear, -3.0);
            // Halfway through a 0..12 dB ramp is 6 dB.
            assert!((sampler.sample(TICKS_PER_SECOND / 2) - 6.0).abs() < 1e-6);
            // Before and after the curve, the edges hold.
            assert!((sampler.sample(-5) - 0.0).abs() < 1e-9);
            assert!((sampler.sample(2 * TICKS_PER_SECOND) - 12.0).abs() < 1e-9);
            // A fresh sampler without points falls back to the fader value.
            let mut fallback = CurveSampler::new(&[], AutomationInterpolation::Linear, -3.0);
            assert!((fallback.sample(42) - -3.0).abs() < 1e-9);
        }

        #[test]
        fn sampler_walks_segments_monotonically() {
            let points: Vec<AutomationPoint> = (0..100)
                .map(|step| point(step * 10, step as f64))
                .collect();
            let mut sampler = CurveSampler::new(&points, AutomationInterpolation::Linear, 0.0);
            for step in 0..100 {
                let t = Tick::from_millis(step * 10).0;
                assert!((sampler.sample(t) - step as f64).abs() < 1e-9, "t={t}");
            }
        }

        #[test]
        fn constant_power_pan_law() {
            let (l, r) = constant_power(0.0);
            assert!((l - 0.707_106_78).abs() < 1e-6);
            assert!((r - 0.707_106_78).abs() < 1e-6);
            let (l, r) = constant_power(-1.0);
            assert!((l - 1.0).abs() < 1e-6 && r.abs() < 1e-6);
            let (l, r) = constant_power(1.0);
            assert!(l.abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
        }
    }
}

glib::wrapper! {
    pub struct SaturnAudioAuto(ObjectSubclass<imp::SaturnAudioAuto>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

/// Stages (replaces) the per-track automation payloads element instances read.
pub fn stage_automation(payloads: &[AudioAutomationPayload]) -> Result<(), serde_json::Error> {
    imp::stage_payloads(payloads)
}

/// Registers the element with GStreamer; call once after `gst::init`.
/// Idempotent: re-registering after an earlier registration is fine.
pub fn register() -> Result<(), glib::BoolError> {
    if gst::ElementFactory::find(ELEMENT_NAME).is_some() {
        return Ok(());
    }
    gst::Element::register(
        None,
        ELEMENT_NAME,
        gst::Rank::NONE,
        SaturnAudioAuto::static_type(),
    )
}
