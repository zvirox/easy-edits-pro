// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Random-access still-frame and contact-sheet extraction (Packet 1.1).
//!
//! One pre-warmed GStreamer decode pipeline per source (`uridecodebin ->
//! videoconvert -> appsink`, raw RGB) serves both the single poster frame and
//! the tiled contact sheet: each tile is an accurate seek + pull, so a full
//! sheet costs one decode instead of one render.
//!
//! Plan-review constraints (docs/audit/eep-ai-native-plan-review.md §4.2):
//! there is **no `/dev/shm` ring** — a language model cannot read shared
//! memory. Frames leave the process as size-capped JPEG bytes (MCP image
//! content blocks) and/or PNG files inside the isolated previews runtime
//! directory (`$XDG_RUNTIME_DIR/easy-edits-pro/previews/`, mode `0700`, see
//! [`previews_dir`]). Callers that need a different destination must go
//! through the write-grant layer, never hand this module a raw path.
//!
//! All outputs carry a token budget: tile count, tile size, and JPEG quality
//! are clamped by [`SheetRequest::clamped`], and in-band base64 payloads are
//! capped before a tool response bloats the model context (oversized
//! artifacts stay on disk in the previews directory).

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use saturn_core::interop::ticks_to_timecode;
use saturn_core::media_fingerprint;
use saturn_core::{FrameRate, Tick};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Longest wait for a decoded frame before the extractor gives up. Keyframe
/// seeks on far-away timestamps can decode several seconds of stream first.
const SEEK_TIMEOUT: Duration = Duration::from_secs(20);

/// Token-budget ceilings: a sheet is fed to a model, so its pixel budget is
/// bounded like any other model input.
pub const MAX_SHEET_TILES: usize = 64;
pub const MAX_TILE_WIDTH: u32 = 640;
pub const MAX_TILE_HEIGHT: u32 = 640;
pub const MAX_POSTER_WIDTH: u32 = 1280;
/// Cap on any in-band base64 payload (the poster still or the sheet preview).
/// Oversized artifacts stay on disk in the previews directory instead.
pub const MAX_INLINE_JPEG_BYTES: usize = 512 * 1024;

// ---------------------------------------------------------------------------
// previews runtime directory (0700) and its pruning cache
// ---------------------------------------------------------------------------

/// Isolated, auto-pruned previews directory: mode `0700`, under
/// `$XDG_RUNTIME_DIR/easy-edits-pro/previews/` when a runtime dir exists
/// (tmpfs, wiped at logout), else under the user's cache dir. Every extractor
/// output that is not granted an explicit target is written here; nothing is
/// ever written to a caller-supplied path by this module.
pub fn previews_dir() -> Result<PathBuf, String> {
    // Test/harness override: keeps test cache traffic out of the user's real
    // runtime directory. Same trust level as the harnesses' own env vars.
    let dir = if let Some(overridden) = std::env::var_os("SATURN_PREVIEWS_DIR") {
        PathBuf::from(overridden)
    } else {
        match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(runtime_dir) => PathBuf::from(runtime_dir).join("easy-edits-pro"),
            None => {
                let home = std::env::var_os("HOME").ok_or_else(|| {
                    "No XDG_RUNTIME_DIR or HOME: cannot resolve the previews directory".to_owned()
                })?;
                PathBuf::from(home)
                    .join(".cache")
                    .join("easy-edits-pro")
                    .join("runtime")
            }
        }
        .join("previews")
    };
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "Could not create the previews directory {}: {error}",
            dir.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).map_err(
            |error| format!("Could not restrict the previews directory to 0700: {error}"),
        )?;
    }
    prune_previews(&dir);
    Ok(dir)
}

/// Auto-prune: preview artifacts older than a week are deleted. The previews
/// dir is runtime scratch, not a data store; the cache re-extracts on demand.
fn prune_previews(dir: &Path) {
    const MAX_AGE: std::time::Duration = Duration::from_secs(7 * 24 * 3600);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else { continue };
        let age = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok());
        if age.is_some_and(|age| age > MAX_AGE) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Stable cache identity for a media file: BLAKE3 fingerprint (Packet 0.2) —
/// head/tail slices plus size. Good enough for a *preview cache* (a stale hit
/// shows a changed middle-of-file at the wrong pixels; trade-off documented).
fn cache_key(source: &Path) -> Result<String, String> {
    let fingerprint = media_fingerprint::compute_file(source)?;
    let head = fingerprint.head.get(..16).unwrap_or(&fingerprint.head);
    let tail = fingerprint.tail.get(..16).unwrap_or(&fingerprint.tail);
    Ok(format!("{head}-{tail}-{}", fingerprint.size))
}

// ---------------------------------------------------------------------------
// decode core
// ---------------------------------------------------------------------------

/// A tightly packed RGB frame pulled off the pipeline.
struct DecodedFrame {
    width: u32,
    height: u32,
    /// `width * height * 3`, row-major RGB.
    data: Vec<u8>,
}

/// One pre-warmed decode pipeline for one source: built once, seeked many
/// times (poster + every sheet tile), then dropped. GStreamer elements are
/// created, used and dropped on the calling thread.
struct FrameDecoder {
    pipeline: gst::Pipeline,
    appsink: gst_app::AppSink,
}

impl FrameDecoder {
    fn open(source: &Path) -> Result<Self, String> {
        if !source.is_file() {
            return Err(format!("Source media is missing: {}", source.display()));
        }
        crate::init_media_runtime()?;
        let uri = gst::glib::filename_to_uri(source, None)
            .map_err(|error| format!("Could not read media path {}: {error}", source.display()))?;
        let decodebin = gst::ElementFactory::make("uridecodebin")
            .name("eep-frame-decoder")
            .property("uri", &uri)
            .build()
            .map_err(|error| format!("GStreamer uridecodebin is unavailable: {error}"))?;
        let convert = gst::ElementFactory::make("videoconvert")
            .name("eep-frame-convert")
            .build()
            .map_err(|error| format!("GStreamer videoconvert is unavailable: {error}"))?;
        let appsink = gst_app::AppSink::builder()
            .name("eep-frame-sink")
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "RGB")
                    .build(),
            )
            .max_buffers(1)
            .drop(false)
            .sync(false)
            .build();
        let pipeline = gst::Pipeline::new();
        pipeline
            .add_many([&decodebin, &convert])
            .map_err(|error| format!("Could not build the decode pipeline: {error}"))?;
        pipeline
            .add(&appsink)
            .map_err(|error| format!("Could not add the frame sink: {error}"))?;
        convert
            .link(&appsink)
            .map_err(|error| format!("Could not link the frame pipeline: {error}"))?;
        // Only the video pad feeds the converter; audio pads stay unlinked.
        let convert_sink = convert
            .static_pad("sink")
            .expect("videoconvert has a static sink pad");
        decodebin.connect_pad_added(move |_decodebin, pad| {
            let caps = pad.current_caps().or_else(|| Some(pad.query_caps(None)));
            let is_video = caps.is_some_and(|caps| {
                caps.iter().any(|structure| structure.name().starts_with("video/"))
            });
            if is_video && convert_sink.can_link(pad) {
                let _ = pad.link(&convert_sink);
            }
        });
        // Stay paused: a non-clocked Playing sink can race past the requested
        // frame. Each flushing accurate seek produces exactly one preroll.
        // Own the cleanup guard before startup so failures also stop decoding.
        let decoder = Self { pipeline, appsink };
        decoder.pipeline
            .set_state(gst::State::Paused)
            .map_err(|error| format!("Could not start the decode pipeline: {error}"))?;
        let (_, state, _) = decoder.pipeline.state(gst::ClockTime::from_seconds(SEEK_TIMEOUT.as_secs()));
        if state != gst::State::Paused {
            return Err("Timed out while prerolling the frame pipeline".into());
        }
        Ok(decoder)
    }

    /// Seek to `position_ns` and pull one RGB frame. Accurate seeks so
    /// contact-sheet tiles land on distinct moments.
    fn pull(&self, position_ns: u64) -> Result<DecodedFrame, String> {
        // Drain anything queued from a previous pull so the frame we get can
        // never be a stale pre-seek one.
        let _ = self.appsink.try_pull_preroll(gst::ClockTime::ZERO);
        let position = gst::ClockTime::from_nseconds(position_ns);
        let seeked = self.pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            position,
        );
        if let Err(error) = seeked {
            return Err(format!("Could not seek to the frame: {error}"));
        }
        let sample = self
            .appsink
            .try_pull_preroll(gst::ClockTime::from_seconds(SEEK_TIMEOUT.as_secs()))
            .ok_or_else(|| "Timed out waiting for the decoded frame".to_owned())?;
        let caps = sample
            .caps()
            .ok_or_else(|| "Frame sample arrived without caps".to_owned())?;
        let info = gst_video::VideoInfo::from_caps(caps)
            .map_err(|error| format!("Frame sample caps are not raw video: {error}"))?;
        let buffer = sample
            .buffer()
            .ok_or_else(|| "Frame sample arrived without a buffer".to_owned())?;
        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
            .map_err(|error| format!("Could not map the decoded frame: {error}"))?;
        let width = u32::from(info.width());
        let height = u32::from(info.height());
        let stride = info.stride()[0] as usize;
        let plane = frame
            .plane_data(0)
            .map_err(|error| format!("Could not read the decoded frame: {error}"))?;
        let row_bytes = width as usize * 3;
        let mut data = Vec::with_capacity(row_bytes * height as usize);
        for row in 0..height as usize {
            let start = row * stride;
            data.extend_from_slice(&plane[start..start + row_bytes]);
        }
        Ok(DecodedFrame { width, height, data })
    }
}

impl Drop for FrameDecoder {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// First frame position of a media file (poster default): half a second in,
/// clamped into the file so black-frame firsts and fade-ins are avoided.
pub fn default_poster_tick(duration: Option<Tick>) -> Tick {
    let half_second = Tick(saturn_core::time::TICKS_PER_SECOND / 2);
    match duration {
        Some(duration) if duration.0 > 0 => Tick(half_second.0.min(duration.0 - 1)),
        _ => half_second,
    }
}

/// Decode one frame at `at_tick` as raw RGB — the lossless input scope
/// analysis needs (the JPEG still path quantizes, which would corrupt
/// histogram measurements like clip/crush percentages).
pub fn decode_frame_rgb(source: &Path, at_tick: Tick) -> Result<(u32, u32, Vec<u8>), String> {
    let decoder = FrameDecoder::open(source)?;
    let frame = decoder.pull(position_ns(at_tick)?)?;
    Ok((frame.width, frame.height, frame.data))
}

/// Bounded raw-source sampling for match cuts. Reuses one decoder and retains
/// only normalized 160x90 frames; differing source aspect ratios use normalized
/// screen coordinates. No grade or timeline effect is applied here.
pub(crate) fn match_cut_frames(source: &Path, ticks: &[Tick]) -> Result<Vec<saturn_perception::SampledFrame>, String> {
    if ticks.is_empty() || ticks.len() > 64 { return Err("Match-cut sampling requires 1 to 64 frames".into()); }
    let started = std::time::Instant::now();
    let decoder = FrameDecoder::open(source)?;
    let mut frames = Vec::with_capacity(ticks.len());
    for &tick in ticks {
        if started.elapsed() > Duration::from_secs(120) { return Err("Match-cut frame analysis exceeded 120 seconds".into()); }
        let frame = decoder.pull(position_ns(tick)?)?;
        frames.push(saturn_perception::SampledFrame { tick, width: 160, height: 90, rgb: scale_rgb(&frame, 160, 90) });
    }
    Ok(frames)
}

/// Decode a PNG written by `render_frame` back into raw RGB (packs the
/// possibly-strided image back into tight rows). Used for scope analysis of
/// a rendered timeline frame.
pub fn decode_png_rgb(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Could not read the rendered frame {path:?}: {error}"))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("Could not parse the rendered frame: {error}"))?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("Could not decode the rendered frame: {error}"))?;
    let width = info.width;
    let height = info.height;
    let channels = info.color_type.samples();
    if !(3..=4).contains(&channels) {
        return Err(format!(
            "The rendered frame has an unsupported channel count: {}",
            channels
        ));
    }
    let stride = width as usize * channels;
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for row in 0..height as usize {
        for col in 0..width as usize {
            let source = row * stride + col * channels;
            rgb.push(buffer[source]);
            rgb.push(buffer[source + 1]);
            rgb.push(buffer[source + 2]);
        }
    }
    Ok((width, height, rgb))
}

fn position_ns(tick: Tick) -> Result<u64, String> {
    tick.as_nanoseconds()
        .ok_or_else(|| "Frame position cannot be negative".to_owned())
}

// ---------------------------------------------------------------------------
// encoding helpers
// ---------------------------------------------------------------------------

fn encode_png_rgb(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut png_bytes = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        let mut encoder = png::Encoder::new(&mut cursor, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(data))
            .map_err(|error| format!("Could not encode the PNG frame: {error}"))?;
    }
    Ok(png_bytes)
}

fn encode_jpeg_rgb(width: u32, height: u32, data: &[u8], quality: u8) -> Result<Vec<u8>, String> {
    let mut jpeg_bytes = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut jpeg_bytes, quality.clamp(30, 95));
    encoder
        .encode(data, width as u16, height as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|error| format!("Could not encode the JPEG frame: {error}"))?;
    Ok(jpeg_bytes)
}

/// Nearest-neighbour scale of a tightly packed RGB buffer.
fn scale_rgb(frame: &DecodedFrame, target_width: u32, target_height: u32) -> Vec<u8> {
    if target_width == frame.width && target_height == frame.height {
        return frame.data.clone();
    }
    let mut out = vec![0u8; target_width as usize * target_height as usize * 3];
    for ty in 0..target_height {
        let sy = (u64::from(ty) * u64::from(frame.height) / u64::from(target_height)) as u32;
        for tx in 0..target_width {
            let sx = (u64::from(tx) * u64::from(frame.width) / u64::from(target_width)) as u32;
            let src = ((sy as usize * frame.width as usize) + sx as usize) * 3;
            let dst = ((ty as usize * target_width as usize) + tx as usize) * 3;
            out[dst..dst + 3].copy_from_slice(&frame.data[src..src + 3]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// burn-in timecode label
// ---------------------------------------------------------------------------

/// 4×6 bitmap glyphs for the burn-in label (digits, colon, dash). Drawn by
/// hand — no font dependency for eight shapes of pixel art.
const GLYPHS: &[(&char, [u8; 6])] = &[
    (&'0', [0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110]),
    (&'1', [0b0010, 0b0110, 0b0010, 0b0010, 0b0010, 0b0111]),
    (&'2', [0b0110, 0b1001, 0b0001, 0b0010, 0b0100, 0b1111]),
    (&'3', [0b1110, 0b0001, 0b0110, 0b0001, 0b1001, 0b0110]),
    (&'4', [0b0010, 0b0110, 0b1010, 0b1111, 0b0010, 0b0010]),
    (&'5', [0b1111, 0b1000, 0b1110, 0b0001, 0b1001, 0b0110]),
    (&'6', [0b0110, 0b1000, 0b1110, 0b1001, 0b1001, 0b0110]),
    (&'7', [0b1111, 0b0001, 0b0010, 0b0100, 0b0100, 0b0100]),
    (&'8', [0b0110, 0b1001, 0b0110, 0b1001, 0b1001, 0b0110]),
    (&'9', [0b0110, 0b1001, 0b1001, 0b0111, 0b0001, 0b0110]),
    (&':', [0b0000, 0b0010, 0b0000, 0b0000, 0b0010, 0b0000]),
    (&'-', [0b0000, 0b0000, 0b1111, 0b0000, 0b0000, 0b0000]),
];

fn glyph_rows(ch: char) -> Option<&'static [u8; 6]> {
    GLYPHS.iter().find(|(key, _)| **key == ch).map(|(_, rows)| rows)
}

/// Burn a timecode label into the bottom-left of an RGB canvas.
pub fn burn_in_label(canvas: &mut [u8], width: u32, height: u32, label: &str) {
    let glyph_scale = (width / 160).clamp(1, 3) as usize;
    let cell = 4 * glyph_scale;
    let padding = (4 * glyph_scale) as i64;
    for (row, ch) in label.chars().enumerate() {
        let Some(rows) = glyph_rows(ch) else { continue };
        let origin_x = padding + (row * (cell + glyph_scale)) as i64;
        // Right-most labels must stay on-canvas: skip glyphs past the edge.
        if origin_x + cell as i64 > width as i64 {
            break;
        }        let origin_y = height as i64 - padding - (6 * glyph_scale) as i64;
        for (gy, bits) in rows.iter().enumerate() {
            for gx in 0..4 {
                if bits & (0b1000 >> gx) == 0 {
                    continue;
                }
                for dy in 0..glyph_scale {
                    for dx in 0..glyph_scale {
                        // Black outline look: darken one pixel around first.
                        for (ox, oy, color) in [
                            (0, 1, [0u8, 0, 0]),
                            (1, 0, [0u8, 0, 0]),
                            (1, 1, [0u8, 0, 0]),
                            (0, 0, [255u8, 255, 255]),
                        ] {
                            let px = origin_x + (gx * glyph_scale + dx) as i64 + ox;
                            let py = origin_y + (gy * glyph_scale + dy) as i64 + oy;
                            if px < 0 || py < 0 || px >= width as i64 || py >= height as i64 {
                                continue;
                            }
                            let index = ((py as usize * width as usize) + px as usize) * 3;
                            if index + 3 <= canvas.len() {
                                canvas[index..index + 3].copy_from_slice(&color);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn timecode_label(tick: Tick, fps: FrameRate) -> String {
    ticks_to_timecode(tick, fps)
}

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

/// A single poster frame, ready to hand to a model or write to the cache.
#[derive(Clone, Debug)]
pub struct StillFrame {
    pub tick: Tick,
    pub width: u32,
    pub height: u32,
    /// `image/jpeg` — the only in-band encoding (size-capped).
    pub mime: &'static str,
    /// Encoded still, capped at [`MAX_INLINE_JPEG_BYTES`].
    pub bytes: Vec<u8>,
}

/// Options for [`extract_still`]. `max_width` is clamped to
/// [`MAX_POSTER_WIDTH`] so a poster can never dominate a model context.
#[derive(Clone, Copy, Debug)]
pub struct StillOptions {
    pub at_tick: Tick,
    pub max_width: u32,
}

impl Default for StillOptions {
    fn default() -> Self {
        Self { at_tick: Tick(0), max_width: 512 }
    }
}

/// Extract one poster frame from `source` at `options.at_tick`, scaled to fit
/// `options.max_width` (aspect preserved). Decodes exactly one frame.
pub fn extract_still(source: &Path, options: StillOptions) -> Result<StillFrame, String> {
    let max_width = options.max_width.clamp(64, MAX_POSTER_WIDTH);
    let decoder = FrameDecoder::open(source)?;
    let frame = decoder.pull(position_ns(options.at_tick)?)?;
    let scale = max_width as f32 / frame.width.max(1) as f32;
    let scale = scale.min(1.0);
    let width = ((frame.width as f32 * scale).round() as u32).max(1).min(frame.width.max(1));
    let height =
        ((frame.height as f32 * scale).round() as u32).max(1).min(frame.height.max(1));
    let scaled = scale_rgb(&frame, width, height);
    let bytes = encode_jpeg_rgb(width, height, &scaled, 80)?;
    Ok(StillFrame {
        tick: options.at_tick,
        width,
        height,
        mime: "image/jpeg",
        bytes,
    })
}

/// A cached poster still inside the previews directory. `bytes` may be `None`
/// when the payload exceeded [`MAX_INLINE_JPEG_BYTES`]; the file on disk is
/// always the source of truth.
#[derive(Clone, Debug)]
pub struct CachedStill {
    pub path: PathBuf,
    pub from_cache: bool,
    pub bytes: Vec<u8>,
    pub tick: Tick,
    pub width: u32,
    pub height: u32,
}

/// Extract a poster still (or lift it from the previews cache keyed by the
/// file's BLAKE3 fingerprint) and park a copy in the previews directory. The
/// cache file is written atomically so a crash never leaves a half image.
pub fn cached_still(source: &Path, options: StillOptions) -> Result<CachedStill, String> {
    let still = extract_still(source, options)?;
    let previews = previews_dir()?;
    let key = cache_key(source)?;
    let dir = previews.join(&key);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create the preview cache dir: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let path = dir.join(format!("poster-{}-{}.jpg", options.at_tick.0, still.width));
    if !path.exists() {
        let tmp = dir.join(format!(".poster-{}.tmp", std::process::id()));
        std::fs::write(&tmp, &still.bytes)
            .map_err(|error| format!("Could not write the preview cache: {error}"))?;
        std::fs::rename(&tmp, &path)
            .map_err(|error| format!("Could not finalize the preview cache: {error}"))?;
        return Ok(CachedStill {
            path,
            from_cache: false,
            bytes: still.bytes,
            tick: still.tick,
            width: still.width,
            height: still.height,
        });
    }
    let cached = std::fs::read(&path)
        .map_err(|error| format!("Could not read the preview cache: {error}"))?;
    Ok(CachedStill {
        path,
        from_cache: true,
        bytes: cached,
        tick: still.tick,
        width: still.width,
        height: still.height,
    })
}

/// Request for [`extract_frame_sheet`]. Unvalidated; use
/// [`SheetRequest::clamped`] to apply the token budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SheetRequest {
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub columns: u32,
    pub rows: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    /// Burn `HH:MM:SS:FF` into every tile's bottom-left corner.
    pub burn_timecode: bool,
}

impl SheetRequest {
    /// Grid of 4×4 tiles at 320×180 with burned-in labels — small enough for
    /// a model context and large enough to read a scene.
    pub fn well_ranged(start: Tick, end: Tick) -> Self {
        Self {
            start_tick: start,
            end_tick: end,
            columns: 4,
            rows: 4,
            tile_width: 320,
            tile_height: 180,
            burn_timecode: true,
        }
    }

    /// Apply the token budget: at most [`MAX_SHEET_TILES`] tiles, at most
    /// [`MAX_TILE_WIDTH`]×[`MAX_TILE_HEIGHT`] pixels each, and a valid,
    /// forward time range. Returns the constraint violations it fixed so the
    /// tool response can report them honestly.
    pub fn clamped(mut self) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        if self.end_tick.0 <= self.start_tick.0 {
            warnings.push("end_tick must be after start_tick; a one-tick span was used".into());
            self.end_tick = Tick(self.start_tick.0 + 1);
        }
        let mut clamp_dim = |value: &mut u32, max: u32, name: &str| {
            if *value < 64 {
                warnings.push(format!("{name} below 64 was clamped to 64"));
                *value = 64;
            }
            if *value > max {
                warnings.push(format!("{name} above the token budget was clamped to {max}"));
                *value = max;
            }
        };
        clamp_dim(&mut self.tile_width, MAX_TILE_WIDTH, "tile_width");
        clamp_dim(&mut self.tile_height, MAX_TILE_HEIGHT, "tile_height");
        if self.columns < 1 {
            self.columns = 1;
        }
        if self.rows < 1 {
            self.rows = 1;
        }
        let (mut columns, mut rows) = (self.columns as usize, self.rows as usize);
        while columns * rows > MAX_SHEET_TILES {
            if rows >= columns && rows > 1 {
                rows /= 2;
                warnings.push(format!(
                    "tile count over the token budget: rows clamped to {rows}"
                ));
            } else if columns > 1 {
                columns /= 2;
                warnings.push(format!(
                    "tile count over the token budget: columns clamped to {columns}"
                ));
            } else {
                break;
            }
        }
        self.columns = columns as u32;
        self.rows = rows as u32;
        (self, warnings)
    }
}

/// One labeled tile of a produced sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetTile {
    pub index: usize,
    pub tick: Tick,
    pub timecode: String,
}

/// A finished contact sheet.
#[derive(Clone, Debug)]
pub struct FrameSheet {
    pub width: u32,
    pub height: u32,
    /// In-band JPEG preview, `None` when over [`MAX_INLINE_JPEG_BYTES`]
    /// (the PNG artifact stays on disk in the previews directory).
    pub inline_jpeg: Option<Vec<u8>>,
    /// Lossless PNG artifact inside the previews directory (mode 0700).
    pub png_path: PathBuf,
    pub tiles: Vec<SheetTile>,
}

/// Render a `columns × rows` contact sheet of `source` across
/// `request.start_tick..end_tick` with optional burned-in timecode labels.
/// One decode pipeline serves the whole sheet: each tile is one seek + pull.
pub fn extract_frame_sheet(
    source: &Path,
    request: SheetRequest,
    fps: FrameRate,
) -> Result<FrameSheet, String> {
    let (request, _warnings) = request.clamped();
    let columns = request.columns as usize;
    let rows = request.rows as usize;
    let count = columns * rows;

    let span = (request.end_tick.0 - request.start_tick.0).max(1);
    // Tile centres, not gaps: the first tile sits a half-step inside the
    // range so a start-at-zero range does not ask for a negative position.
    let step = span / count as i64;
    let ticks: Vec<i64> = (0..count)
        .map(|index| request.start_tick.0 + step * index as i64 + (step / 2).max(0))
        .collect();

    let decoder = FrameDecoder::open(source)?;
    let tile_w = request.tile_width;
    let tile_h = request.tile_height;
    let sheet_width = tile_w * columns as u32;
    let sheet_height = tile_h * rows as u32;
    if sheet_width.checked_mul(sheet_height).is_none() {
        return Err("Sheet dimensions overflow".into());
    }
    let mut canvas = vec![0u8; sheet_width as usize * sheet_height as usize * 3];
    let mut tiles = Vec::with_capacity(count);

    for (index, &tick) in ticks.iter().enumerate() {
        let frame = decoder.pull(position_ns(Tick(tick))?)?;
        let mut tile = scale_rgb(&frame, tile_w, tile_h);
        let timecode = timecode_label(Tick(tick), fps);
        if request.burn_timecode {
            burn_in_label(&mut tile, tile_w, tile_h, &timecode);
        }
        let column = index % columns;
        let row = index / columns;
        for y in 0..tile_h as usize {
            let src = y * tile_w as usize * 3;
            let dst = ((row * tile_h as usize + y) * sheet_width as usize + column * tile_w as usize) * 3;
            canvas[dst..dst + tile_w as usize * 3]
                .copy_from_slice(&tile[src..src + tile_w as usize * 3]);
        }
        tiles.push(SheetTile {
            index,
            tick: Tick(tick),
            timecode,
        });
    }

    let previews = previews_dir()?;
    let key = cache_key(source)?;
    let dir = previews.join(&key);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create the preview cache dir: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let png_path = dir.join(format!(
        "sheet-{}x{}-{}-{}.png",
        request.start_tick.0, request.end_tick.0, columns, rows
    ));
    let png_bytes = encode_png_rgb(sheet_width, sheet_height, &canvas)?;
    let tmp = dir.join(format!(".sheet-{}.tmp", std::process::id()));
    std::fs::write(&tmp, &png_bytes)
        .map_err(|error| format!("Could not write the contact sheet: {error}"))?;
    std::fs::rename(&tmp, &png_path)
        .map_err(|error| format!("Could not finalize the contact sheet: {error}"))?;

    // The in-band preview is the JPEG remix of the same canvas: bounded bytes
    // for the model, lossless PNG stays on disk.
    let jpeg = encode_jpeg_rgb(sheet_width, sheet_height, &canvas, 70)?;
    let inline_jpeg = if jpeg.len() <= MAX_INLINE_JPEG_BYTES {
        Some(jpeg)
    } else {
        None
    };

    Ok(FrameSheet {
        width: sheet_width,
        height: sheet_height,
        inline_jpeg,
        png_path,
        tiles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_request_clamps_the_token_budget() {
        let base = SheetRequest::well_ranged(Tick(0), Tick(1_000));
        // 16 tiles fit; nothing changes.
        let (clamped, warnings) = base.clamped();
        assert_eq!(clamped, base);
        assert!(warnings.is_empty());

        // 10×10 = 100 tiles overflows 64; both directions get halved.
        let big = SheetRequest { columns: 10, rows: 10, ..base };
        let (clamped, warnings) = big.clamped();
        assert!(
            (clamped.columns * clamped.rows) as usize <= MAX_SHEET_TILES,
            "tiles must respect the budget: {}x{}",
            clamped.columns,
            clamped.rows
        );
        assert!(!warnings.is_empty());

        // Oversized tiles get clamped.
        let wide = SheetRequest { tile_width: 5000, tile_height: 5000, ..base };
        let (clamped, _) = wide.clamped();
        assert_eq!(clamped.tile_width, MAX_TILE_WIDTH);
        assert_eq!(clamped.tile_height, MAX_TILE_HEIGHT);
    }
}
