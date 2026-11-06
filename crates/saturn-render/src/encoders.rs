// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Export formats, encoding profiles, and hardware-encoder selection.
//!
//! Split out of `lib.rs` (Packet 1.1) to keep every source file under the
//! 800-line gate. The render entry points stay in `lib.rs`; this module owns
//! everything between "which container" and "which encoder element".
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_pbutils as gst_pbutils;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::automation_element;
use crate::graphics_element;
use crate::keyframe_element;
use gstreamer_editing_services as ges;
use gstreamer_pbutils::prelude::*;

pub(crate) static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareEncoderInfo {
    pub has_hardware_encoder: bool,
    pub preferred_encoder: String,
    pub encoder_type: String, // "vaapi" | "nvenc" | "qsv" | "v4l2" | "software"
    pub description: String,
    pub available_encoders: Vec<String>,
}

pub fn detect_hardware_encoders() -> HardwareEncoderInfo {
    let _ = GST_INIT.get_or_init(|| {
        gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        graphics_element::register()
            .map_err(|error| format!("Could not register the graphics element: {error}"))?;
        automation_element::register()
            .map_err(|error| format!("Could not register the audio automation element: {error}"))?;
        keyframe_element::register()
            .map_err(|error| format!("Could not register the keyframe video element: {error}"))?;
        ges::init()
            .map_err(|error| format!("Could not initialize GStreamer Editing Services: {error}"))
    });

    let candidates = [
        (
            "vah264enc",
            "vaapi",
            "VA-API Hardware H.264 (Intel / AMD GPU)",
        ),
        (
            "nvh264enc",
            "nvenc",
            "NVIDIA NVENC Hardware H.264 (NVIDIA GPU)",
        ),
        (
            "qsvh264enc",
            "qsv",
            "Intel Quick Sync Video H.264 (Intel QSV)",
        ),
        ("v4l2h264enc", "v4l2", "V4L2 Hardware Codec H.264"),
    ];

    let mut available = Vec::new();
    let mut selected = None;

    for (name, enc_type, desc) in &candidates {
        if gst::ElementFactory::find(name).is_some() {
            available.push((*name).to_string());
            if selected.is_none() {
                selected = Some((
                    (*name).to_string(),
                    (*enc_type).to_string(),
                    (*desc).to_string(),
                ));
            }
        }
    }

    if let Some((name, enc_type, desc)) = selected {
        HardwareEncoderInfo {
            has_hardware_encoder: true,
            preferred_encoder: name,
            encoder_type: enc_type,
            description: desc,
            available_encoders: available,
        }
    } else {
        let sw_name = if gst::ElementFactory::find("x264enc").is_some() {
            "x264enc"
        } else if gst::ElementFactory::find("openh264enc").is_some() {
            "openh264enc"
        } else {
            "software"
        };
        available.push(sw_name.to_string());
        HardwareEncoderInfo {
            has_hardware_encoder: false,
            preferred_encoder: sw_name.to_string(),
            encoder_type: "software".to_string(),
            description: "High-Quality CPU Software Encoding (x264 / OpenH264)".to_string(),
            available_encoders: available,
        }
    }
}


pub(crate) struct EncoderRankOverrides(Vec<(gst::ElementFactory, gst::Rank)>);

impl Drop for EncoderRankOverrides {
    fn drop(&mut self) {
        for (factory, rank) in &self.0 {
            factory.set_rank(*rank);
        }
    }
}

pub(crate) fn prefer_hardware_h264_encoder() -> Option<(String, EncoderRankOverrides)> {
    let candidates = ["vah264enc", "qsvh264enc", "nvh264enc", "v4l2h264enc"];
    let (name, selected) = candidates
        .iter()
        .find_map(|name| gst::ElementFactory::find(name).map(|factory| (*name, factory)))?;
    let mut overrides = Vec::new();
    overrides.push((selected.clone(), selected.rank()));
    selected.set_rank(gst::Rank::PRIMARY + 1);
    Some((name.to_owned(), EncoderRankOverrides(overrides)))
}

pub(crate) fn prefer_software_h264_encoder() -> EncoderRankOverrides {
    let mut overrides = Vec::new();
    for name in ["vah264enc", "qsvh264enc", "nvh264enc", "v4l2h264enc"] {
        if let Some(factory) = gst::ElementFactory::find(name) {
            overrides.push((factory.clone(), factory.rank()));
            factory.set_rank(gst::Rank::NONE);
        }
    }
    let software =
        gst::ElementFactory::find("x264enc").or_else(|| gst::ElementFactory::find("openh264enc"));
    if let Some(factory) = software {
        overrides.push((factory.clone(), factory.rank()));
        factory.set_rank(gst::Rank::PRIMARY + 1);
    }
    EncoderRankOverrides(overrides)
}

/// libav builds its AAC encoder at rank NONE, and encodebin only considers
/// encoders at rank MARGINAL and above for a caps-based profile. Hosts that
/// ship faac/vaacenc still export fine, but a sandboxed runtime with libav as
/// the only AAC encoder (the Flatpak spike) fails MP4 export with
/// "Couldn't create encoder for audio/mpeg mpegversion=4". Raise whichever
/// audio encoder the format needs, exactly like the video path above; the
/// override restores the real ranks on drop.
pub(crate) fn prefer_software_audio_encoder(format: OutputFormat) -> EncoderRankOverrides {
    let candidates: &[&[&str]] = match format {
        OutputFormat::Mp4 => &[&["avenc_aac", "faac", "voaacenc"]],
        OutputFormat::WebM => &[&["vorbisenc"]],
    };
    let mut overrides = Vec::new();
    for names in candidates {
        for name in *names {
            if let Some(factory) = gst::ElementFactory::find(name) {
                overrides.push((factory.clone(), factory.rank()));
                factory.set_rank(gst::Rank::PRIMARY);
            }
        }
    }
    EncoderRankOverrides(overrides)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Mp4,
    WebM,
}

impl OutputFormat {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("mp4") => Ok(Self::Mp4),
            Some("webm") => Ok(Self::WebM),
            _ => Err("Choose an .mp4 or .webm output file".into()),
        }
    }
}


pub(crate) fn absolute_output_path(path: &Path) -> Result<PathBuf, String> {
    let filename = path
        .file_name()
        .ok_or_else(|| "Choose a valid output filename".to_string())?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create output folder: {error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("Could not access output folder: {error}"))?;
    Ok(parent.join(filename))
}


pub(crate) fn encoding_profile(format: OutputFormat) -> Result<gst_pbutils::EncodingContainerProfile, String> {
    match format {
        OutputFormat::Mp4 => {
            let container_caps = gst::Caps::builder("video/quicktime")
                .field("variant", "iso")
                .build();
            let video_caps = gst::Caps::builder("video/x-h264").build();
            let audio_caps = gst::Caps::builder("audio/mpeg")
                .field("mpegversion", 4)
                .build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("H.264 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("AAC audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edits Pro MP4")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
        OutputFormat::WebM => {
            let container_caps = gst::Caps::builder("video/webm").build();
            let video_caps = gst::Caps::builder("video/x-vp8").build();
            let audio_caps = gst::Caps::builder("audio/x-vorbis").build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("VP8 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("Vorbis audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edits Pro WebM")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_is_selected_from_extension() {
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.MP4")),
            Ok(OutputFormat::Mp4)
        );
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.webm")),
            Ok(OutputFormat::WebM)
        );
        assert!(OutputFormat::from_path(Path::new("cut.mov")).is_err());
    }

    #[test]
    fn audio_encoder_rank_override_is_visible_to_encodebin_and_restored() {
        // The Flatpak-spike failure: encodebin looks up audio encoders at
        // rank MARGINAL and above, but libav registers avenc_aac at rank
        // NONE; the override must lift it for the render and restore the
        // real rank afterwards.
        gst::init().unwrap();
        let factory = gst::ElementFactory::find("avenc_aac");
        let factory = match factory {
            Some(factory) => factory,
            None => return, // no libav in this environment; nothing to lift
        };
        let original = factory.rank();
        {
            let _override = prefer_software_audio_encoder(OutputFormat::Mp4);
            assert!(
                factory.rank() >= gst::Rank::MARGINAL,
                "encodebin only considers >= MARGINAL; got {}",
                factory.rank()
            );
        }
        assert_eq!(factory.rank(), original, "override must restore ranks");
    }
}
