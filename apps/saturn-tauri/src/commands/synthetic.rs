//! Synthetic media generation for FC File > New items: solid-colour stills
//! (black video, colour matte, transparent video) written as hand-encoded
//! RGBA PNGs, and SMPTE bars + 1 kHz tone as a real MP4 via GStreamer.
//! Pure Rust, no new dependencies.

use gstreamer as gst;
use gstreamer::prelude::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

use crate::AppState;

// --- checksums (exact algorithms, no external crates) ---

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut low, mut high) = (1u32, 0u32);
    for &byte in data {
        low = (low + u32::from(byte)) % 65_521;
        high = (high + low) % 65_521;
    }
    (high << 16) | low
}

// --- PNG encoding (RGBA, filter 0, zlib stored blocks) ---

/// zlib stream of `data` using stored (uncompressed) deflate blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut chunks = data.chunks(65_535).peekable();
    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    }
    while let Some(chunk) = chunks.next() {
        let last = chunks.peek().is_none();
        out.push(u8::from(last));
        let length = chunk.len() as u16;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let mut crc_input = Vec::with_capacity(4 + body.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(body);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Write a solid-colour RGBA PNG; returns the byte length written.
pub fn write_solid_rgba_png(
    path: &Path,
    width: u32,
    height: u32,
    color: [u8; 4],
) -> Result<usize, String> {
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err(format!("Unsupported still size {width}x{height}"));
    }
    // Raw scanlines: one filter byte (0 = None) per row of RGBA pixels.
    let stride = 1 + 4 * width as usize;
    let mut raw = Vec::with_capacity(stride * height as usize);
    let mut row = vec![0u8; stride];
    for pixel in row.iter_mut().skip(1) {
        *pixel = 0; // placeholder; fill below
    }
    for _ in 0..height {
        row[0] = 0;
        for x in 0..width as usize {
            let offset = 1 + 4 * x;
            row[offset..offset + 4].copy_from_slice(&color);
        }
        raw.extend_from_slice(&row);
    }

    let mut png = Vec::with_capacity(raw.len() / 2);
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    let ihdr: [u8; 13] = [
        (width >> 24) as u8,
        (width >> 16) as u8,
        (width >> 8) as u8,
        width as u8,
        (height >> 24) as u8,
        (height >> 16) as u8,
        (height >> 8) as u8,
        height as u8,
        8, // bit depth
        6, // colour type: RGBA
        0, 0, 0,
    ];
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    png_chunk(&mut png, b"IEND", &[]);

    std::fs::write(path, &png).map_err(|error| format!("Cannot write still image: {error}"))?;
    Ok(png.len())
}

// --- Bars and tone (GStreamer) ---

fn generate_bars_tone_mp4(path: &Path, width: u32, height: u32, duration_secs: f64) -> Result<(), String> {
    gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
    let frames = (duration_secs * 30.0).ceil().max(1.0) as u64;
    let audio_buffers = (duration_secs * 48_000.0).ceil().max(1.0) as u64 / 4_800 + 1;
    // videotestsrc's smpte pattern + 1 kHz sine, AAC audio, H.264 video.
    let element = gst::parse::launch(&format!(
        "videotestsrc pattern=smpte num-buffers={frames} ! \
         video/x-raw,width={width},height={height},framerate=30/1 ! videoconvert ! \
         x264enc speed-preset=ultrafast bitrate=2000 key-int-max=30 ! \
         h264parse ! queue ! mp4mux name=mux ! \
         filesink location=\"{}\" \
         audiotestsrc wave=sine freq=1000 volume=0.6 num-buffers={audio_buffers} samplesperbuffer=4800 ! \
         audio/x-raw,rate=48000,channels=2 ! audioconvert ! \
         avenc_aac bitrate=128000 ! aacparse ! queue ! mux.",
        path.display()
    ))
    .map_err(|error| format!("Could not build bars-and-tone pipeline: {error}"))?;
    let pipeline = element
        .downcast::<gst::Pipeline>()
        .map_err(|_| "Bars-and-tone description is not a pipeline".to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| format!("Could not start bars pipeline: {error}"))?;
    let bus = pipeline.bus().expect("pipeline bus");
    let _ = bus.timed_pop_filtered(
        gst::ClockTime::from_seconds(120),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    let _ = pipeline.set_state(gst::State::Null);
    if !path.is_file() {
        return Err("Bars-and-tone render produced no file".into());
    }
    Ok(())
}

// --- colour parsing ---

fn parse_hex_color(color: &str) -> Result<[u8; 4], String> {
    let text = color.trim().trim_start_matches('#');
    let channels = (0..3)
        .map(|index| {
            let slice = text
                .get(index * 2..index * 2 + 2)
                .ok_or_else(|| format!("Color must be RRGGBB: {color}"))?;
            u8::from_str_radix(slice, 16)
                .map_err(|_| format!("Color must be RRGGBB: {color}"))
        })
        .collect::<Result<Vec<u8>, _>>()?;
    Ok([channels[0], channels[1], channels[2], 255])
}

// --- Tauri command ---

#[tauri::command]
pub async fn generate_synthetic_media(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    color: Option<String>,
    duration_secs: Option<f64>,
) -> Result<Value, String> {
    let (width, height) = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        let settings = &engine.document().project.settings;
        (settings.width, settings.height)
    };
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot resolve app data dir: {error}"))?
        .join("synthetic");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Cannot create synthetic media dir: {error}"))?;
    let destination: PathBuf = directory.join(format!(
        "{}-{}.{}",
        kind,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_millis())
            .unwrap_or_default(),
        match kind.as_str() {
            "bars_and_tone" => "mp4",
            _ => "png",
        }
    ));

    let result = match kind.as_str() {
        "black_video" => write_solid_rgba_png(&destination, width, height, [0, 0, 0, 255]),
        "color_matte" => write_solid_rgba_png(
            &destination,
            width,
            height,
            parse_hex_color(color.as_deref().unwrap_or("#808080"))?,
        ),
        // Transparent video = a fully transparent still; the alpha channel is
        // the point, compositors read it as an on-top layer.
        "transparent_video" => write_solid_rgba_png(&destination, width, height, [0, 0, 0, 0]),
        "bars_and_tone" => {
            generate_bars_tone_mp4(&destination, width, height, duration_secs.unwrap_or(5.0))
                .map(|_| 0)
        }
        other => return Err(format!("Unknown synthetic media kind: {other}")),
    };
    let bytes = result.map_err(|error| error)?;
    Ok(json!({
        "path": destination.to_string_lossy(),
        "kind": kind,
        "bytes": bytes,
        "width": width,
        "height": height,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_bodies(png: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        let mut chunks = Vec::new();
        let mut offset = 8; // signature
        while offset + 8 <= png.len() {
            let length = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
            let mut kind = [0u8; 4];
            kind.copy_from_slice(&png[offset + 4..offset + 8]);
            let body = png[offset + 8..offset + 8 + length].to_vec();
            chunks.push((kind, body));
            offset += 12 + length; // length + kind + body + crc
        }
        chunks
    }

    #[test]
    fn solid_png_is_structurally_valid_with_exact_dimensions_and_pixel() {
        let dir = std::env::temp_dir().join(format!("eep-synthetic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("matte.png");
        let bytes = write_solid_rgba_png(&path, 7, 5, [0x12, 0x34, 0x56, 0xFF]).unwrap();
        let png = std::fs::read(&path).unwrap();
        assert_eq!(png.len(), bytes);
        assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        let chunks = chunk_bodies(&png);
        assert_eq!(chunks.first().unwrap().0, *b"IHDR");
        assert_eq!(chunks.last().unwrap().0, *b"IEND");
        let ihdr = &chunks.first().unwrap().1;
        assert_eq!(&ihdr[0..4], &[0, 0, 0, 7], "width");
        assert_eq!(&ihdr[4..8], &[0, 0, 0, 5], "height");
        assert_eq!(ihdr[8], 8, "bit depth");
        assert_eq!(ihdr[9], 6, "RGBA colour type");

        let idat = &chunks
            .iter()
            .find(|(kind, _)| kind == b"IDAT")
            .expect("IDAT")
            .1;
        // Stored-deflate framing: 0x78 0x01, then one final block with the
        // raw scanlines (145 bytes: 5 rows * (1 + 28 RGBA bytes)), then adler32.
        assert_eq!(&idat[0..2], &[0x78, 0x01]);
        assert_eq!(idat[2], 0x01, "single final stored block");
        assert_eq!(&idat[3..5], &145u16.to_le_bytes());
        let stored = &idat[7..7 + 145];
        // First pixel of the first scanline carries the colour.
        assert_eq!(&stored[1..5], &[0x12, 0x34, 0x56, 0xFF]);
        // Every row starts with filter byte 0.
        for row in 0..5 {
            assert_eq!(stored[row * 29], 0, "row {row} filter byte");
        }
        let checksum = u32::from_be_bytes(idat[idat.len() - 4..].try_into().unwrap());
        assert_eq!(checksum, adler32(stored), "zlib adler32 over raw scanlines");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hex_parsing_rejects_bad_input() {
        assert!(parse_hex_color("#808080").is_ok());
        assert!(parse_hex_color("ff0080").is_ok());
        assert!(parse_hex_color("#f0f").is_err());
        assert!(parse_hex_color("#XYZXYZ").is_err());
        assert!(parse_hex_color("").is_err());
    }

    #[test]
    fn zlib_stored_framing_multiblock() {
        let data: Vec<u8> = (0..150_000u32).map(|value| value as u8).collect();
        let stream = zlib_stored(&data);
        assert_eq!(&stream[0..2], &[0x78, 0x01]);
        // First block non-final with max length.
        assert_eq!(stream[2], 0x00);
        assert_eq!(&stream[3..5], &65_535u16.to_le_bytes());
        // Adler over the whole payload ends the stream.
        let checksum = u32::from_be_bytes(stream[stream.len() - 4..].try_into().unwrap());
        assert_eq!(checksum, adler32(&data));
    }

    #[test]
    #[ignore] // encoder-dependent smoke test; run with --ignored on gst machines
    fn bars_and_tone_mp4_smoke() {
        let dir = std::env::temp_dir().join("eep-synthetic-bars");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bars.mp4");
        generate_bars_tone_mp4(&path, 320, 180, 1.0).unwrap();
        assert!(path.metadata().unwrap().len() > 1024);
        std::fs::remove_dir_all(&dir).ok();
    }
}
