// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! One initialization boundary for decoding, discovery and timeline rendering.
use std::sync::OnceLock;

static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();

/// Initialize GStreamer, GES and every built-in timeline element before creating
/// media pipelines or discoverers, including those in the desktop application.
///
/// GES type initialization and concurrent playback-plugin loading can acquire
/// native locks in opposite orders. Complete both behind this shared boundary;
/// steady-state decoding remains concurrent. Keep the cell private so callers
/// cannot win initialization with only a subset of the runtime ready.
pub fn init_media_runtime() -> Result<(), String> {
    GST_INIT
        .get_or_init(|| {
            gstreamer::init()
                .map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
            gstreamer_editing_services::init().map_err(|error| {
                format!("Could not initialize GStreamer Editing Services: {error}")
            })?;
            crate::graphics_element::register()
                .map_err(|error| format!("Could not register the graphics element: {error}"))?;
            crate::automation_element::register().map_err(|error| {
                format!("Could not register the audio automation element: {error}")
            })?;
            crate::keyframe_element::register().map_err(|error| {
                format!("Could not register the keyframe video element: {error}")
            })?;
            crate::plugin_element::register()
                .map_err(|error| format!("Could not register the audio plugin element: {error}"))
        })
        .clone()
}
