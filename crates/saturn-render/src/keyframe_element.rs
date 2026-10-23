//! `saturnkfvid`: a GStreamer video filter that bakes clip keyframes into the
//! export render loop (F4).
//!
//! The element is registered at render start and attached as a top effect on
//! video clips whose model has visual keyframes (position, scale, rotation,
//! opacity). Per buffer it takes the buffer PTS as timeline time (the same
//! convention the `saturnaudioauto` element proved), evaluates the clip's
//! keyframe curves, and applies an affine warp around the frame centre:
//!
//! `out = center + rotate(deg) * scale * (in - center) + translate(px)`
//!
//! which is exactly the preview's CSS transform
//! `translate(px) scale(s) rotate(deg)` with `transform-origin: center`.
//! Output is straight-alpha BGRA: pixels outside the warped source bounds are
//! transparent, and the sampled alpha is multiplied by the keyframed opacity
//! so GES's layer compositor blends the clip against the tracks below.
//!
//! Payloads reach the element through a module-level handoff
//! ([`stage_keyframes`]) keyed by the element's `clip-key` property, because
//! `ges::Effect` instantiates the wrapped element from a bin description with
//! no typed property path. GES may instantiate the effect several times
//! (probe + live instances), so the staged map is peeked, never consumed, per
//! instance.

use gstreamer as gst;
use gstreamer_base as gst_base;

use gst::glib;
use gst::prelude::*;
use crate::keyframes::VisualKeyframePayload;

pub const ELEMENT_NAME: &str = "saturnkfvid";

mod imp {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use gstreamer as gst;
    use gstreamer_base as gst_base;
    use gstreamer_base::subclass::prelude::*;

    use gst::glib;
    use gst::prelude::*;
    use saturn_core::project::KeyframeProperty;
    use saturn_core::time::{Tick, TICKS_PER_SECOND};

    use super::VisualKeyframePayload;

    /// Clip-keyed payloads staged by `build_timeline` for this render.
    /// Peeked (not consumed) because GES instantiates effect elements several
    /// times per clip and every instance needs its clip's payload.
    static STAGED_PAYLOADS: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();

    fn staged_payloads() -> &'static Mutex<HashMap<u64, String>> {
        STAGED_PAYLOADS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Replaces the staged per-clip payloads for this render.
    pub(super) fn stage_payloads(
        payloads: &[VisualKeyframePayload],
    ) -> Result<(), serde_json::Error> {
        let mut staged = staged_payloads().lock().expect("keyframe staging lock");
        staged.clear();
        for payload in payloads {
            staged.insert(payload.clip_key, serde_json::to_string(payload)?);
        }
        Ok(())
    }

    fn payload_for(clip_key: u64) -> Option<VisualKeyframePayload> {
        let staged = staged_payloads().lock().expect("keyframe staging lock");
        let json = staged.get(&clip_key)?;
        match serde_json::from_str::<VisualKeyframePayload>(json) {
            Ok(payload) => Some(payload),
            Err(error) => {
                eprintln!("saturnkfvid: invalid staged payload: {error}");
                None
            }
        }
    }

    /// Frame geometry captured at caps: both sides are 4 bytes per pixel, but
    /// row strides may be padded.
    #[derive(Debug, Clone, Copy)]
    struct Geometry {
        width: usize,
        height: usize,
        in_stride: usize,
        out_stride: usize,
    }

    #[derive(Debug, Default)]
    pub struct SaturnKfVid {
        clip_key: std::sync::atomic::AtomicU64,
        payload: Mutex<Option<VisualKeyframePayload>>,
        geometry: Mutex<Option<Geometry>>,
    }

    impl SaturnKfVid {
        fn new() -> Self {
            Self::default()
        }

        fn copy_identity(&self, inbuf: &gst::BufferRef, outbuf: &mut gst::BufferRef) -> Result<(), gst::FlowError> {
            let Some(geometry) = *self.geometry.lock().expect("keyframe geometry lock") else {
                return Err(gst::FlowError::NotNegotiated);
            };
            let input = inbuf.map_readable().map_err(|_| gst::FlowError::Error)?;
            let mut output = outbuf.map_writable().map_err(|_| gst::FlowError::Error)?;
            let input = input.as_slice();
            let output = output.as_mut_slice();
            for row in 0..geometry.height {
                let in_start = row * geometry.in_stride;
                let out_start = row * geometry.out_stride;
                output[out_start..out_start + geometry.width * 4]
                    .copy_from_slice(&input[in_start..in_start + geometry.width * 4]);
                // Decoded BGRx carries an undefined fourth byte; the element
                // promises real alpha, so make the identity pass opaque.
                for pixel in output[out_start..out_start + geometry.width * 4].chunks_exact_mut(4) {
                    pixel[3] = u8::MAX;
                }
            }
            Ok(())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SaturnKfVid {
        const NAME: &'static str = "SaturnKfVid";
        type Type = super::SaturnKfVid;
        type ParentType = gst_base::BaseTransform;

        fn with_class(_klass: &Self::Class) -> Self {
            Self::new()
        }
    }

    impl ObjectImpl for SaturnKfVid {
        fn constructed(&self) {
            self.parent_constructed();
            let clip_key = self.clip_key.load(std::sync::atomic::Ordering::Relaxed);
            *self.payload.lock().expect("keyframe payload lock") = payload_for(clip_key);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![glib::ParamSpecUInt64::builder("clip-key")
                    .nick("Clip key")
                    .blurb("Staged visual keyframe payload applying to this clip instance")
                    .build()]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "clip-key" => self
                    .clip_key
                    .store(value.get().unwrap_or(0), std::sync::atomic::Ordering::Relaxed),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "clip-key" => self
                    .clip_key
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl GstObjectImpl for SaturnKfVid {}

    impl ElementImpl for SaturnKfVid {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "Saturn keyframe video transform",
                    "Filter/Effect/Video",
                    "Bakes clip position/scale/rotation/opacity keyframes into video frames",
                    "Project Saturn",
                )
            }))
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
            TEMPLATES.get_or_init(|| {
                let geometry = |format: gst::List| {
                    gst::Caps::builder("video/x-raw")
                        .field("format", format)
                        .field("width", gst::IntRange::<i32>::new(1, i32::MAX))
                        .field("height", gst::IntRange::<i32>::new(1, i32::MAX))
                        .field(
                            "framerate",
                            gst::FractionRange::new(
                                gst::Fraction::new(0, 1),
                                gst::Fraction::new(i32::MAX, 1),
                            ),
                        )
                        .build()
                };
                let sink_caps = geometry(gst::List::new(["BGRx", "BGRA"]));
                let src_caps = geometry(gst::List::new(["BGRA"]));
                let src = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    &src_caps,
                )
                .expect("saturnkfvid src template");
                let sink = gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &sink_caps,
                )
                .expect("saturnkfvid sink template");
                vec![src, sink]
            })
        }
    }

    impl BaseTransformImpl for SaturnKfVid {
        const MODE: gst_base::subclass::BaseTransformMode =
            gst_base::subclass::BaseTransformMode::NeverInPlace;
        const PASSTHROUGH_ON_SAME_CAPS: bool = false;
        const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

        fn transform_caps(
            &self,
            direction: gst::PadDirection,
            caps: &gst::Caps,
            filter: Option<&gst::Caps>,
        ) -> Option<gst::Caps> {
            // Both directions keep every incoming field (width, height,
            // framerate) and swap the format for the formats the other pad
            // template allows. Over-permissive is fine: set_caps re-checks.
            let other_formats = match direction {
                gst::PadDirection::Sink => gst::List::new(["BGRA"]),
                gst::PadDirection::Src => gst::List::new(["BGRx", "BGRA"]),
                _ => return None,
            };
            let mut result = gst::Caps::new_empty();
            for structure in caps.iter() {
                let mut swapped = structure.to_owned();
                if swapped.has_field("format") {
                    swapped.set_value("format", other_formats.to_send_value());
                }
                result.make_mut().append_structure(swapped);
            }
            if let Some(filter) = filter {
                result = result.intersect(filter);
            }
            Some(result)
        }
        fn set_caps(
            &self,
            incaps: &gst::Caps,
            outcaps: &gst::Caps,
        ) -> Result<(), gst::LoggableError> {
            let to_info = |caps: &gst::Caps| -> Result<gstreamer_video::VideoInfo, gst::LoggableError> {
                gstreamer_video::VideoInfo::from_caps(caps).map_err(|error| {
                    gst::loggable_error!(gst::CAT_RUST, "saturnkfvid: bad caps: {error}")
                })
            };
            let in_info = to_info(incaps)?;
            let out_info = to_info(outcaps)?;
            let geometry = Geometry {
                width: in_info.width() as usize,
                height: in_info.height() as usize,
                in_stride: in_info.stride()[0] as usize,
                out_stride: out_info.stride()[0] as usize,
            };
            if out_info.width() != in_info.width() || out_info.height() != in_info.height() {
                return Err(gst::loggable_error!(
                    gst::CAT_RUST,
                    "saturnkfvid: output size must match input size"
                ));
            }
            *self.geometry.lock().expect("keyframe geometry lock") = Some(geometry);
            Ok(())
        }

        fn transform_size(
            &self,
            _direction: gst::PadDirection,
            caps: &gst::Caps,
            _size: usize,
            othercaps: &gst::Caps,
        ) -> Option<usize> {
            // Input and output are both single-plane 4-byte-per-pixel raw
            // video at the same dimensions, so the other side's allocatable
            // size is simply its fitted video buffer size.
            gstreamer_video::VideoInfo::from_caps(othercaps)
                .ok()
                .map(|info| info.size())
                .or_else(|| gstreamer_video::VideoInfo::from_caps(caps).ok().map(|info| info.size()))
        }

        fn transform(
            &self,
            inbuf: &gst::Buffer,
            outbuf: &mut gst::BufferRef,
        ) -> Result<gst::FlowSuccess, gst::FlowError> {
            let mut payload = self.payload.lock().expect("keyframe payload lock");
            // GObject properties are assigned after constructed(), so resolve
            // the staged payload lazily on the first buffer instead.
            if payload.is_none() {
                let clip_key = self.clip_key.load(std::sync::atomic::Ordering::Relaxed);
                *payload = payload_for(clip_key);
            }
            let Some(payload) = payload.as_ref() else {
                return self.copy_identity(inbuf, outbuf).map(|_| gst::FlowSuccess::Ok);
            };
            if payload.is_identity() {
                return self.copy_identity(inbuf, outbuf).map(|_| gst::FlowSuccess::Ok);
            }
            let pts_ns = inbuf.pts().map(|pts| pts.nseconds()).unwrap_or(0);
            let time = pts_to_ticks(pts_ns);
            let position_x = payload.sample(KeyframeProperty::PositionX, time);
            let position_y = payload.sample(KeyframeProperty::PositionY, time);
            let scale = payload.sample(KeyframeProperty::Scale, time);
            let rotation = payload.sample(KeyframeProperty::Rotation, time);
            let opacity = payload.sample(KeyframeProperty::Opacity, time).clamp(0.0, 1.0);

            // Fast path: an identity frame needs no warp (still fixes alpha).
            if position_x.abs() < 1e-9
                && position_y.abs() < 1e-9
                && (scale - 1.0).abs() < 1e-9
                && rotation.abs() < 1e-9
                && opacity >= 1.0
            {
                return self.copy_identity(inbuf, outbuf).map(|_| gst::FlowSuccess::Ok);
            }

            let geometry = self
                .geometry
                .lock()
                .expect("keyframe geometry lock")
                .ok_or(gst::FlowError::NotNegotiated)?;
            let Geometry {
                width,
                height,
                in_stride,
                out_stride,
            } = geometry;
            let center_x = width as f64 / 2.0;
            let center_y = height as f64 / 2.0;
            let alpha = (opacity * 255.0).round() as u8;

            let input = inbuf.map_readable().map_err(|_| gst::FlowError::Error)?;
            let input = input.as_slice();
            let mut output = outbuf.map_writable().map_err(|_| gst::FlowError::Error)?;
            let output = output.as_mut_slice();

            for out_y in 0..height {
                let out_row = &mut output[out_y * out_stride..out_y * out_stride + width * 4];
                for out_x in 0..width {
                    let (u, v) = crate::keyframes::inverse_map_point(
                        out_x as f64 + 0.5,
                        out_y as f64 + 0.5,
                        position_x,
                        position_y,
                        scale,
                        rotation,
                        center_x,
                        center_y,
                    );
                    // Sample with pixel centres: shift to the corner grid.
                    let u = u - 0.5;
                    let v = v - 0.5;

                    let sx = u.floor();
                    let sy = v.floor();
                    let frac_x = u - sx;
                    let frac_y = v - sy;
                    let x0 = sx as isize;
                    let y0 = sy as isize;

                    // Bilinear over the four neighbours; pixels outside the
                    // source frame contribute black and stretch no weights
                    // (the layer is opaque content with a hard edge, like CSS
                    // overflow clipping).
                    let mut red = 0.0_f64;
                    let mut green = 0.0_f64;
                    let mut blue = 0.0_f64;
                    let mut weight = 0.0_f64;
                    for (dy, wy) in [(0_isize, 1.0 - frac_y), (1_isize, frac_y)] {
                        for (dx, wx) in [(0_isize, 1.0 - frac_x), (1_isize, frac_x)] {
                            let px = x0 + dx;
                            let py = y0 + dy;
                            if px < 0 || py < 0 || px >= width as isize || py >= height as isize {
                                continue;
                            }
                            let sample = &input[py as usize * in_stride + px as usize * 4..];
                            let w = wx * wy;
                            blue += w * sample[0] as f64;
                            green += w * sample[1] as f64;
                            red += w * sample[2] as f64;
                            weight += w;
                        }
                    }
                    let pixel = &mut out_row[out_x * 4..out_x * 4 + 4];
                    if weight <= 0.0 {
                        pixel[0] = 0;
                        pixel[1] = 0;
                        pixel[2] = 0;
                        pixel[3] = 0;
                    } else {
                        pixel[0] = (blue / weight).round().clamp(0.0, 255.0) as u8;
                        pixel[1] = (green / weight).round().clamp(0.0, 255.0) as u8;
                        pixel[2] = (red / weight).round().clamp(0.0, 255.0) as u8;
                        pixel[3] = alpha;
                    }
                }
            }
            Ok(gst::FlowSuccess::Ok)
        }
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
        use crate::keyframes::build_visual_payload;
        use saturn_core::project::{InterpolationKind, Keyframe, KeyframeProperty};
        use saturn_core::time::Tick;

        fn keyframe(property: KeyframeProperty, time_ms: i64, value: f64) -> Keyframe {
            let mut keyframe = Keyframe::new(property, Tick::from_millis(time_ms), value);
            keyframe.interpolation = InterpolationKind::Linear;
            keyframe
        }

        #[test]
        fn pts_to_ticks_follows_the_audio_element_convention() {
            assert_eq!(pts_to_ticks(0), Tick(0));
            // Half a second is half of TICKS_PER_SECOND ticks.
            assert_eq!(
                pts_to_ticks(500_000_000).0,
                TICKS_PER_SECOND / 2,
                "1 s of PTS nanoseconds maps to TICKS_PER_SECOND ticks"
            );
        }

        #[test]
        fn staged_payload_is_retrievable_by_clip_key() {
            let payload =
                build_visual_payload(42, Tick::default(), &[keyframe(KeyframeProperty::Opacity, 0, 0.5)])
                    .expect("payload with keyframes");
            stage_payloads(&[payload.clone()]).expect("staging serializes");
            let round_tripped = payload_for(42).expect("staged payload retrievable");
            assert_eq!(round_tripped.clip_key, 42);
            assert!((round_tripped.opacity[0].value - 0.5).abs() < 1e-9);
        }
    }
}

glib::wrapper! {
    pub struct SaturnKfVid(ObjectSubclass<imp::SaturnKfVid>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

/// Replaces the staged per-clip payloads element instances read.
pub fn stage_keyframes(payloads: &[VisualKeyframePayload]) -> Result<(), serde_json::Error> {
    imp::stage_payloads(payloads)
}

/// Registers the element with GStreamer; call once after `gst::init`.
/// Idempotent: re-registering after an earlier registration is fine.
pub fn register() -> Result<(), glib::BoolError> {
    if gst::ElementFactory::find(ELEMENT_NAME).is_some() {
        return Ok(());
    }
    gst::Element::register(None, ELEMENT_NAME, gst::Rank::NONE, SaturnKfVid::static_type())
}
