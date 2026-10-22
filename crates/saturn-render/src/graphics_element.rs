//! `saturngraphicsblend`: a GStreamer video filter that blends timeline
//! graphics into export frames (Phase 5 slice 5).
//!
//! The element is registered at render start and attached to the video track
//! as a track-level GES effect in [`crate::build_timeline`]. Per frame it
//! takes the buffer PTS as timeline time, rasterizes the graphics active at
//! that instant through [`crate::graphics_overlay`], and composites them over
//! the incoming RGBA/BGRA frame in place.
//!
//! Graphics reach the element through a module-level handoff
//! ([`set_pending_graphics_json`]) because `ges::Effect` instantiates the
//! wrapped element from a bin description and offers no typed property path.
//! Renders are serialized app-wide, so the staged payload is unambiguous.

use gstreamer as gst;
use gstreamer_base as gst_base;
use gstreamer_video as gst_video;

use gst::glib;
use gst::prelude::*;
use saturn_core::project::Graphic;

pub const ELEMENT_NAME: &str = "saturngraphicsblend";

mod imp {
    use std::sync::{Mutex, OnceLock};

    use gstreamer as gst;
    use gstreamer_base as gst_base;
    use gstreamer_video as gst_video;

    use gst_base::subclass::prelude::*;
    use gst_video::VideoFrameExt;
    use gst_video::subclass::prelude::*;
    use gst_video::{VideoFormat, VideoFrameRef, VideoInfo};
    use saturn_core::project::Graphic;
    use saturn_core::time::{TICKS_PER_SECOND, Tick};
    use saturn_text::TextContext;

    use super::*;
    use crate::graphics_overlay::render_graphics_overlay;

    /// JSON payload handed from `build_timeline` to the next element instance.
    static PENDING_PAYLOAD: OnceLock<Mutex<Option<String>>> = OnceLock::new();

    /// Stages the graphics JSON the next created element instance consumes.
    pub(super) fn stage_pending_graphics_json(json: String) {
        let cell = PENDING_PAYLOAD.get_or_init(|| Mutex::new(None));
        *cell.lock().expect("pending graphics lock") = Some(json);
    }

    /// Peeks without consuming: GES may instantiate the effect's element
    /// several times (probe + live instances), and every instance needs the
    /// payload. It stays staged until the next `build_timeline` overwrites it.
    fn staged_graphics_json() -> Option<String> {
        let cell = PENDING_PAYLOAD.get_or_init(|| Mutex::new(None));
        cell.lock().expect("pending graphics lock").clone()
    }

    /// Decodable payload stored on the element (forward-compatible shape).
    #[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
    pub(super) struct OverlayPayload {
        graphics: Vec<Graphic>,
    }

    /// Borrowing mirror of [`OverlayPayload`] for staging without a clone.
    #[derive(serde::Serialize)]
    pub(super) struct GraphicsPayloadRef<'a> {
        graphics: &'a [Graphic],
    }

    impl<'a> From<GraphicsPayloadRef<'a>> for OverlayPayload {
        fn from(payload: GraphicsPayloadRef<'a>) -> Self {
            Self {
                graphics: payload.graphics.to_vec(),
            }
        }
    }

    /// Serializes the project graphics into the staged JSON payload.
    pub(super) fn stage_graphics(graphics: &[Graphic]) -> Result<(), serde_json::Error> {
        let json = serde_json::to_string(&GraphicsPayloadRef { graphics })?;
        stage_pending_graphics_json(json);
        Ok(())
    }

    /// The element consumes the staged JSON into an [`OverlayPayload`].
    fn decode_payload(json: &str) -> Option<OverlayPayload> {
        match serde_json::from_str::<OverlayPayload>(json) {
            Ok(payload) => Some(payload),
            Err(error) => {
                eprintln!("saturngraphicsblend: invalid staged graphics payload: {error}");
                None
            }
        }
    }

    pub struct SaturnGraphicsBlend {
        payload: Mutex<Option<OverlayPayload>>,
        fonts: Mutex<Option<TextContext>>,
        info: Mutex<Option<VideoInfo>>,
        warned_format: std::sync::atomic::AtomicBool,
    }

    impl SaturnGraphicsBlend {
        fn new() -> Self {
            Self {
                payload: Mutex::new(None),
                fonts: Mutex::new(None),
                info: Mutex::new(None),
                warned_format: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    impl Default for SaturnGraphicsBlend {
        fn default() -> Self {
            Self::new()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SaturnGraphicsBlend {
        const NAME: &'static str = "SaturnGraphicsBlend";
        type Type = super::SaturnGraphicsBlend;
        type ParentType = gst_video::VideoFilter;

        fn with_class(_klass: &Self::Class) -> Self {
            Self::new()
        }
    }

    impl ObjectImpl for SaturnGraphicsBlend {
        fn constructed(&self) {
            self.parent_constructed();
            // Consume the payload staged by build_timeline for this render.
            let payload = staged_graphics_json().and_then(|json| decode_payload(&json));
            eprintln!(
                "saturngraphicsblend: constructed, staged graphics present={}",
                payload.is_some()
            );
            *self.payload.lock().expect("graphics payload lock") = payload;
        }
    }

    impl GstObjectImpl for SaturnGraphicsBlend {}

    impl ElementImpl for SaturnGraphicsBlend {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "Saturn graphics blend",
                    "Filter/Effect/Video",
                    "Blends timeline graphics (text, shapes) into the video frame",
                    "Project Saturn",
                )
            }))
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
            TEMPLATES.get_or_init(|| {
                let caps = gst::Caps::builder("video/x-raw")
                    .field(
                        "format",
                        gst::List::new([VideoFormat::Rgba.to_str(), VideoFormat::Bgra.to_str()]),
                    )
                    .build();
                let src = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    &caps,
                )
                .expect("saturngraphicsblend src template");
                let sink = gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &caps,
                )
                .expect("saturngraphicsblend sink template");
                vec![src, sink]
            })
        }
    }

    impl VideoFilterImpl for SaturnGraphicsBlend {
        fn set_info(
            &self,
            _incaps: &gst::Caps,
            in_info: &VideoInfo,
            _outcaps: &gst::Caps,
            _out_info: &VideoInfo,
        ) -> Result<(), gst::LoggableError> {
            *self.info.lock().expect("video info lock") = Some(in_info.clone());
            Ok(())
        }

        fn transform_frame_ip(
            &self,
            frame: &mut VideoFrameRef<&mut gst::BufferRef>,
        ) -> Result<gst::FlowSuccess, gst::FlowError> {
            let payload = self.payload.lock().expect("graphics payload lock").clone();
            let Some(payload) = payload.filter(|payload| !payload.graphics.is_empty()) else {
                return Ok(gst::FlowSuccess::Ok);
            };

            let info = self.info.lock().expect("video info lock").clone();
            let Some(info) = info else {
                return Ok(gst::FlowSuccess::Ok);
            };
            let bgra = match info.format() {
                VideoFormat::Rgba => false,
                VideoFormat::Bgra => true,
                other => {
                    // Negotiation only offers RGBA/BGRA; skip gracefully anyway.
                    if !self
                        .warned_format
                        .swap(true, std::sync::atomic::Ordering::Relaxed)
                    {
                        eprintln!(
                            "saturngraphicsblend: unsupported format {other}, skipping overlay"
                        );
                    }
                    return Ok(gst::FlowSuccess::Ok);
                }
            };

            let playhead =
                pts_to_ticks(frame.buffer().pts().map(|pts| pts.nseconds()).unwrap_or(0));
            let width = info.width();
            let height = info.height();
            let mut fonts = self.fonts.lock().expect("font context lock");
            let fonts = fonts.get_or_insert_with(TextContext::new);
            let overlay =
                render_graphics_overlay(&payload.graphics, playhead, width, height, 1.0, fonts);
            drop(fonts);
            if overlay.iter().all(|&byte| byte == 0) {
                return Ok(gst::FlowSuccess::Ok);
            }

            let mut plane = frame.plane_data_mut(0).map_err(|error| {
                eprintln!("saturngraphicsblend: could not map plane: {error}");
                gst::FlowError::Error
            })?;
            let stride = info.stride()[0] as usize;
            blend_overlay_into(&mut plane, stride, width, height, &overlay, bgra);
            Ok(gst::FlowSuccess::Ok)
        }
    }

    impl BaseTransformImpl for SaturnGraphicsBlend {
        const MODE: gst_base::subclass::BaseTransformMode =
            gst_base::subclass::BaseTransformMode::AlwaysInPlace;
        const PASSTHROUGH_ON_SAME_CAPS: bool = false;
        const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;
    }

    /// Timeline ticks from buffer PTS nanoseconds (GES timelines start at 0).
    fn pts_to_ticks(nanoseconds: u64) -> Tick {
        let ticks = (nanoseconds as i128)
            .saturating_mul(TICKS_PER_SECOND as i128)
            .saturating_div(1_000_000_000);
        Tick(ticks.clamp(0, i64::MAX as i128) as i64)
    }

    /// Source-over composites the straight-alpha overlay into the packed frame.
    fn blend_overlay_into(
        plane: &mut [u8],
        stride: usize,
        width: u32,
        height: u32,
        overlay: &[u8],
        bgra: bool,
    ) {
        for y in 0..height as usize {
            let row_start = y * stride;
            if row_start + width as usize * 4 > plane.len() {
                continue;
            }
            let row = &mut plane[row_start..row_start + width as usize * 4];
            let overlay_row = &overlay[y * width as usize * 4..(y + 1) * width as usize * 4];
            for (dst, src) in row.chunks_exact_mut(4).zip(overlay_row.chunks_exact(4)) {
                let sa = src[3] as f32 / 255.0;
                if sa <= 0.0 {
                    continue;
                }
                let da = dst[3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a <= 0.0 {
                    continue;
                }
                let mix = |s: u8, d: u8| -> u8 {
                    ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                // Overlay is always RGBA; the packed frame puts alpha first
                // for BGRA, so only the destination placement swaps.
                let (r, g, b) = (src[0], src[1], src[2]);
                if bgra {
                    dst[0] = mix(b, dst[0]);
                    dst[1] = mix(g, dst[1]);
                    dst[2] = mix(r, dst[2]);
                } else {
                    dst[0] = mix(r, dst[0]);
                    dst[1] = mix(g, dst[1]);
                    dst[2] = mix(b, dst[2]);
                }
                dst[3] = (255.0 * out_a).round() as u8;
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pts_maps_to_timeline_ticks() {
            // One second of PTS is TICKS_PER_SECOND ticks.
            assert_eq!(pts_to_ticks(1_000_000_000).0, TICKS_PER_SECOND);
            // Half a second is half the ticks.
            let half = pts_to_ticks(500_000_000).0;
            assert!(
                (half - TICKS_PER_SECOND / 2).abs() < 2,
                "half second ~half ticks: {half}"
            );
        }

        #[test]
        fn blend_overlay_composites_source_over() {
            // Opaque red overlay over an opaque blue frame (RGBA) → red.
            let mut frame = vec![10_u8, 20, 200, 255];
            let overlay = vec![255_u8, 0, 0, 255];
            blend_overlay_into(&mut frame, 4, 1, 1, &overlay, false);
            assert_eq!(frame, [255, 0, 0, 255]);

            // Half-transparent white over blue → light blue (straight alpha).
            frame = vec![10, 20, 200, 255];
            let overlay = vec![255, 255, 255, 128];
            blend_overlay_into(&mut frame, 4, 1, 1, &overlay, false);
            let a = 128.0_f32 / 255.0;
            let expected =
                |s: f32, d: f32| -> u8 { ((s * a + d * (1.0 - a)) / (a + 1.0 - a)).round() as u8 };
            assert_eq!(frame[0], expected(255.0, 10.0));
            assert_eq!(frame[3], 255);
        }

        #[test]
        fn blend_overlay_respects_stride_and_bgra() {
            // A 2-pixel row with padding: pixel 0 red, pixel 1 untouched.
            let mut plane = vec![0_u8; 2 * 12 + 4];
            let stride = 12;
            let overlay = vec![255_u8, 0, 0, 255, 255, 0, 0, 255];
            blend_overlay_into(&mut plane, stride, 2, 1, &overlay, true);
            assert_eq!(&plane[0..4], &[0, 0, 255, 255], "BGRA puts red third");
            assert_eq!(&plane[4..8], &[0, 0, 255, 255]);
            assert!(
                plane[8..].iter().all(|&byte| byte == 0),
                "padding untouched"
            );
        }
    }
}

glib::wrapper! {
    pub struct SaturnGraphicsBlend(ObjectSubclass<imp::SaturnGraphicsBlend>)
        @extends gst_video::VideoFilter, gst_base::BaseTransform, gst::Element, gst::Object;
}

/// Stages the graphics JSON the next created element instance consumes.
pub fn set_pending_graphics_json(json: String) {
    imp::stage_pending_graphics_json(json);
}

/// Stages the project's graphics for the next element instance as JSON.
pub fn stage_graphics(graphics: &[Graphic]) -> Result<(), serde_json::Error> {
    imp::stage_graphics(graphics)
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
        SaturnGraphicsBlend::static_type(),
    )
}
