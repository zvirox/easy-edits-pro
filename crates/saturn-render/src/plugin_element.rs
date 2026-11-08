// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! `saturnaudioplug`: a GStreamer audio filter that runs a track's CLAP
//! plugin chain in the dedicated `saturn-plugin-worker` sandbox (Packet 4.1).
//!
//! The element is registered at render start and attached per audio clip in
//! [`crate::ges_timeline`] when the owning track has a non-empty
//! `dsp_plugin_chain`. Per buffer it hands the interleaved F32 samples to the
//! sandbox session over the `/dev/shm` ring transport and receives them back
//! processed by the worker's plugin chain — the identical chain the full
//! export render uses, which is the Packet 4.1 parity invariant.
//!
//! The chain reaches the element through a module-level handoff
//! ([`stage_plugin_chains`]) keyed by the track's `track-id` element
//! property, mirroring `automation_element` / `keyframe_element`: `ges::Effect`
//! instantiates the wrapped element from a bin description with no typed
//! property path, and GES may instantiate it several times per clip, so the
//! staged payloads are peeked (never consumed), per instance.
//!
//! Error policy: a track with no staged chain passes through untouched, but
//! a configured chain must not degrade silently — a worker spawn or process
//! failure fails the stream (`FlowError::Error`) with the detail on stderr.

use gstreamer as gst;
use gstreamer_base as gst_base;

use gst::glib;
use gst::prelude::*;
use saturn_core::project::DspPluginSlot;

pub const ELEMENT_NAME: &str = "saturnaudioplug";

/// Per-track plugin-chain payload staged for element instances.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PluginChainPayload {
    pub track_id: u64,
    pub slots: Vec<DspPluginSlot>,
}

mod imp {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    use gstreamer as gst;
    use gstreamer_base as gst_base;

    use gst_base::subclass::prelude::*;

    use saturn_audio_dsp::sandbox::SandboxSession;

    use super::*;

    /// Track-keyed payloads staged by `build_timeline` for this render.
    /// Peeked (not consumed) because GES instantiates effect elements several
    /// times per clip and every instance needs its track's payload.
    static STAGED_PAYLOADS: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();

    fn staged_payloads() -> &'static Mutex<HashMap<u64, String>> {
        STAGED_PAYLOADS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Stages (replaces) the per-track plugin-chain payloads for this render.
    pub(super) fn stage_payloads(
        payloads: &[PluginChainPayload],
    ) -> Result<(), serde_json::Error> {
        let mut staged = staged_payloads().lock().expect("plugin staging lock");
        staged.clear();
        for payload in payloads {
            staged.insert(payload.track_id, serde_json::to_string(payload)?);
        }
        Ok(())
    }

    fn payload_for(track_id: u64) -> Option<PluginChainPayload> {
        let staged = staged_payloads().lock().expect("plugin staging lock");
        let json = staged.get(&track_id)?;
        match serde_json::from_str::<PluginChainPayload>(json) {
            Ok(payload) => Some(payload),
            Err(error) => {
                eprintln!("saturnaudioplug: invalid staged payload: {error}");
                None
            }
        }
    }

    #[derive(Default)]
    pub struct SaturnAudioPlug {
        track_id: AtomicU64,
        payload: Mutex<Option<PluginChainPayload>>,
        // (sample rate, channels) the current session was spawned with.
        format: Mutex<Option<(u32, usize)>>,
        session: Mutex<Option<SandboxSession>>,
    }

    impl std::fmt::Debug for SaturnAudioPlug {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // SandboxSession's worker process state is not Debug; the
            // track id and negotiated format are what matters here.
            f.debug_struct("SaturnAudioPlug")
                .field("track_id", &self.track_id.load(Ordering::Relaxed))
                .field("format", &self.format.lock().expect("plugin format lock"))
                .finish_non_exhaustive()
        }
    }

    impl SaturnAudioPlug {
        fn new() -> Self {
            Self::default()
        }

        /// The slots the chain will actually run: disabled slots are skipped
        /// by the host, and a chain of only disabled slots is a passthrough.
        fn active_slots(payload: &PluginChainPayload) -> usize {
            payload.slots.iter().filter(|slot| slot.enabled).count()
        }

        /// Drops the current session (stopping the worker) if rates or
        /// channel counts changed under us.
        fn reset_session_if_stale(
            &self,
            rate: u32,
            channels: usize,
        ) -> Result<(), gst::FlowError> {
            let mut session = self.session.lock().expect("plugin session lock");
            let matches = self
                .format
                .lock()
                .expect("plugin format lock")
                .is_some_and(|(stale_rate, stale_channels)| {
                    stale_rate == rate && stale_channels == channels
                });
            if !matches {
                // SandboxSession::Drop stops and deactivates the chain.
                *session = None;
                *self.format.lock().expect("plugin format lock") = Some((rate, channels));
            }
            if session.is_some() {
                return Ok(());
            }
            let payload = self.payload.lock().expect("plugin payload lock");
            let Some(payload) = payload.as_ref() else {
                // No chain: nothing to spawn; the passthrough path handles it.
                return Ok(());
            };
            match SandboxSession::spawn(&payload.slots, rate, channels) {
                Ok(spawned) => {
                    *session = Some(spawned);
                    Ok(())
                }
                Err(error) => {
                    eprintln!(
                        "saturnaudioplug: could not spawn the plugin worker for track {}: {error}",
                        payload.track_id
                    );
                    Err(gst::FlowError::Error)
                }
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SaturnAudioPlug {
        const NAME: &'static str = "SaturnAudioPlug";
        type Type = super::SaturnAudioPlug;
        type ParentType = gst_base::BaseTransform;

        fn with_class(_klass: &Self::Class) -> Self {
            Self::new()
        }
    }

    impl ObjectImpl for SaturnAudioPlug {
        fn constructed(&self) {
            self.parent_constructed();
            let track_id = self.track_id.load(Ordering::Relaxed);
            *self.payload.lock().expect("plugin payload lock") = payload_for(track_id);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![glib::ParamSpecUInt64::builder("track-id")
                    .nick("Track id")
                    .blurb("Timeline track whose staged plugin chain applies")
                    .build()]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "track-id" => self
                    .track_id
                    .store(value.get().unwrap_or(0), Ordering::Relaxed),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "track-id" => self.track_id.load(Ordering::Relaxed).to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl GstObjectImpl for SaturnAudioPlug {}

    impl ElementImpl for SaturnAudioPlug {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "Saturn audio plugin chain",
                    "Filter/Effect/Audio",
                    "Runs a track's CLAP plugin chain in the saturn-plugin-worker sandbox",
                    "Project Saturn",
                )
            }))
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
            TEMPLATES.get_or_init(|| {
                // The sandbox transport is F32LE interleaved only; channel
                // counts above the host chain's expectation are handled per
                // spawn (the worker is configured with the negotiated count).
                let caps = gst::Caps::builder("audio/x-raw")
                    .field("format", "F32LE")
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
                .expect("saturnaudioplug src template");
                let sink = gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &caps,
                )
                .expect("saturnaudioplug sink template");
                vec![src, sink]
            })
        }
    }

    impl BaseTransformImpl for SaturnAudioPlug {
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
                gst::loggable_error!(gst::CAT_RUST, "saturnaudioplug: caps without a structure")
            })?;
            let format: String = structure.get("format").unwrap_or_default();
            if format != "F32LE" {
                return Err(gst::loggable_error!(
                    gst::CAT_RUST,
                    "saturnaudioplug: unsupported format {format}"
                ));
            }
            let channels: i32 = structure.get("channels").unwrap_or(2);
            let rate: i32 = structure.get("rate").unwrap_or(48_000);
            *self.format.lock().expect("plugin format lock") =
                Some((rate.max(1) as u32, channels.max(1) as usize));
            Ok(())
        }

        fn transform_ip(
            &self,
            buf: &mut gst::BufferRef,
        ) -> Result<gst::FlowSuccess, gst::FlowError> {
            // Resolve (or look up) the staged chain, then release the guard:
            // spawning the worker below locks the payload state again.
            let has_active_chain = {
                let mut payload = self.payload.lock().expect("plugin payload lock");
                // GObject properties are assigned after constructed(), so
                // resolve the staged payload lazily on the first buffer.
                if payload.is_none() {
                    let track_id = self.track_id.load(Ordering::Relaxed);
                    *payload = payload_for(track_id);
                }
                payload
                    .as_ref()
                    .is_some_and(|payload| Self::active_slots(payload) > 0)
            };
            if !has_active_chain {
                // No chain configured (or staged but fully disabled):
                // passthrough, matching the host.
                return Ok(gst::FlowSuccess::Ok);
            }

            let Some((rate, channels)) = self.format.lock().expect("plugin format lock").clone()
            else {
                // set_caps always runs before the first buffer; defensive.
                return Ok(gst::FlowSuccess::Ok);
            };
            self.reset_session_if_stale(rate, channels)?;

            let mut session = self.session.lock().expect("plugin session lock");
            let Some(session) = session.as_mut() else {
                // reset_session_if_stale leaves no session only when no chain
                // is staged for this track: passthrough.
                return Ok(gst::FlowSuccess::Ok);
            };

            let mut data = buf.map_writable().map_err(|_| gst::FlowError::Error)?;
            let bytes = data.as_mut_slice();
            let frame_count = bytes.len() / std::mem::size_of::<f32>();
            // SAFETY: the negotiated caps pin F32LE interleaved, so re-laying
            // the mapped bytes onto an f32 slice is layout-correct, and the
            // buffer is exclusively mapped for this call.
            let samples = unsafe {
                std::slice::from_raw_parts_mut(
                    bytes.as_mut_ptr().cast::<f32>(),
                    frame_count,
                )
            };

            if let Err(error) = session.process(samples) {
                eprintln!("saturnaudioplug: plugin chain failed: {error}");
                return Err(gst::FlowError::Error);
            }
            Ok(gst::FlowSuccess::Ok)
        }
    }
}

glib::wrapper! {
    pub struct SaturnAudioPlug(ObjectSubclass<imp::SaturnAudioPlug>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

/// Stages (replaces) the per-track plugin-chain payloads element instances read.
pub fn stage_plugin_chains(payloads: &[PluginChainPayload]) -> Result<(), serde_json::Error> {
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
        SaturnAudioPlug::static_type(),
    )
}
