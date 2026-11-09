// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Tool payload assembly for the vision/preview MCP surface (Packet 1.1)
//! and the Packet 2.2 perception tools (semantic search, cut continuity).
//!
//! Shared by the headless MCP server (`saturn-mcp`) and the desktop bridge
//! (`saturn-tauri/src/mcp_bridge.rs`) so both seams return byte-identical
//! shapes. These functions are synchronous and decode-bound: callers run
//! them on a blocking thread.
//!
//! Write containment (plan §10.2): generated artifacts live only in the
//! isolated 0700 previews directory — a caller can never point these tools
//! at an arbitrary path.

use base64::Engine as _;
use saturn_core::project::{MediaKind, ProjectDocument};
use saturn_core::{FrameRate, Tick};
use serde_json::{Value, json};

use crate::frame_extractor::{
    self, MAX_INLINE_JPEG_BYTES, SheetRequest, StillOptions, default_poster_tick,
};
use crate::audio_pcm;
use crate::audio_telemetry;
use crate::loudness::LoudnessStandard;
use crate::render_project_range_with_settings;
use crate::scopes_gpu;
use crate::{RenderRange, RenderSettings, render_frame};

use base64::engine::general_purpose::STANDARD as BASE64;

/// The in-band image convention: tool results carry an `image_block` object
/// that MCP servers strip into a real image content block and plain JSON
/// clients can still read.
fn image_block(bytes: &[u8], mime: &str) -> Value {
    json!({
        "data": BASE64.encode(bytes),
        "mime_type": mime,
    })
}

/// Serialized [`MediaAsset`] look-up with a typed error (not a placeholder).
fn find_asset<'a>(document: &'a ProjectDocument, id: u64) -> Result<&'a saturn_core::project::MediaAsset, String> {
    document
        .project
        .media
        .iter()
        .find(|asset| asset.id == id)
        .ok_or_else(|| format!("Media asset {id} was not found"))
}

fn decode_kind_allowed(kind: &MediaKind) -> Option<&'static str> {
    match kind {
        MediaKind::Video | MediaKind::Image => None, // decodable
        MediaKind::Audio => Some("audio media has no video frames to preview"),
        MediaKind::Title => Some("title media renders from text; no source frame exists"),
        MediaKind::CompoundSequence => Some("compound sequences are rendered, not decoded; no source frame exists"),
        MediaKind::Unknown => Some("unknown media kind; nothing to decode"),
    }
}

/// Honest `media_preview` upgrade: real poster frame when the source can be
/// decoded, a real reason (never a path placeholder) when it cannot.
pub fn media_preview_payload(
    document: &ProjectDocument,
    media_id: u64,
    poster_tick: Option<Tick>,
) -> Result<Value, String> {
    let asset = find_asset(document, media_id)?;
    let asset_json = serde_json::to_value(asset).map_err(|error| error.to_string())?;
    let placements = document
        .project
        .sequence
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .clips
                .iter()
                .filter(move |clip| clip.media_id == media_id)
                .map(move |clip| {
                    json!({
                        "track_id": track.id,
                        "track_name": track.name,
                        "clip": clip,
                    })
                })
        })
        .collect::<Vec<_>>();

    let mut payload = json!({
        "media": asset_json,
        "timeline_placements": placements,
    });

    let still = || -> Result<Value, String> {
        if asset.is_missing {
            return Ok(json!({
                "available": false,
                "reason": "the asset is marked missing; relink it before previewing",
            }));
        }
        if let Some(reason) = decode_kind_allowed(&asset.kind) {
            return Ok(json!({ "available": false, "reason": reason }));
        }
        if !asset.path.is_file() {
            return Ok(json!({
                "available": false,
                "reason": format!("source file is missing on disk: {}", asset.path.display()),
            }));
        }
        let tick = poster_tick.unwrap_or_else(|| default_poster_tick(asset.duration));
        let cached = frame_extractor::cached_still(&asset.path, StillOptions { at_tick: tick, max_width: 512 })?;
        let mut still = json!({
            "available": true,
            "tick": tick.0,
            "width": cached.width,
            "height": cached.height,
            "mime": "image/jpeg",
            "path": cached.path.to_string_lossy(),
            "from_cache": cached.from_cache,
        });
        if cached.bytes.len() <= MAX_INLINE_JPEG_BYTES {
            still["data_base64"] = json!(BASE64.encode(&cached.bytes));
        } else {
            still["inline_omitted"] = json!("payload exceeds the in-band size cap; read the file at `path`");
        }
        Ok(still)
    };

    let still = still().unwrap_or_else(|error| {
        json!({ "available": false, "reason": format!("frame extraction failed: {error}") })
    });
    payload["still_frame"] = still.clone();
    if still["available"] == json!(true) {
        // Top-level marker the MCP server turns into a real image block.
        payload["image_block"] = json!({
            "data": still["data_base64"],
            "mime_type": "image/jpeg",
        });
    }
    Ok(payload)
}

fn parse_tick(params: &Value, key: &str) -> Result<Tick, String> {
    let raw = params
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{} requires a tick integer", key))?;
    Ok(Tick(raw))
}

fn asset_frame_rate(document: &ProjectDocument, media_id: u64) -> FrameRate {
    let settings_rate = document.project.settings.frame_rate;
    document
        .project
        .media
        .iter()
        .find(|asset| asset.id == media_id)
        .and_then(|asset| asset.fps)
        .and_then(|fps| {
            if fps > 0.1 && fps <= 240.0 {
                FrameRate::new(fps.round() as u32, 1)
            } else {
                None
            }
        })
        .unwrap_or(settings_rate)
}

/// `vision_extract_frame_sheet`: tiled contact sheet of one media asset with
/// burned-in timecode labels; PNG artifact in the previews dir, size-capped
/// JPEG in-band.
pub fn frame_sheet_payload(document: &ProjectDocument, params: &Value) -> Result<Value, String> {
    let media_id = params
        .get("media_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "vision_extract_frame_sheet requires a media_id".to_owned())?;
    let asset = find_asset(document, media_id)?;
    if let Some(reason) = decode_kind_allowed(&asset.kind) {
        return Err(format!("vision_extract_frame_sheet: {reason}"));
    }
    if !asset.path.is_file() {
        return Err(format!(
            "vision_extract_frame_sheet: source file is missing: {}",
            asset.path.display()
        ));
    }
    let start = parse_tick(params, "start_tick")?;
    let end = parse_tick(params, "end_tick")?;
    let base = SheetRequest::well_ranged(start, end);
    let request = SheetRequest {
        columns: params.get("columns").and_then(Value::as_u64).unwrap_or(base.columns as u64) as u32,
        rows: params.get("rows").and_then(Value::as_u64).unwrap_or(base.rows as u64) as u32,
        tile_width: params.get("tile_width").and_then(Value::as_u64).unwrap_or(base.tile_width as u64) as u32,
        tile_height: params.get("tile_height").and_then(Value::as_u64).unwrap_or(base.tile_height as u64) as u32,
        burn_timecode: params.get("burn_timecode").and_then(Value::as_bool).unwrap_or(true),
        ..base
    };
    // The media span bounds the requested range: clips never reach outside
    // the source file, and a fully out-of-range request is a caller error.
    let span_limit = asset.duration.map(|duration| Tick(duration.0.max(start.0 + 1)));
    if let Some(limit) = span_limit {
        if end.0 > limit.0 {
            return Err(format!(
                "end_tick is past the media duration ({} ticks); clamp the range",
                limit.0
            ));
        }
    }
    let (request, warnings) = request.clamped();
    let sheet = frame_extractor::extract_frame_sheet(&asset.path, request, asset_frame_rate(document, media_id))?;
    let tiles = sheet
        .tiles
        .iter()
        .map(|tile| {
            json!({
                "index": tile.index,
                "tick": tile.tick.0,
                "timecode": tile.timecode,
            })
        })
        .collect::<Vec<_>>();
    let mut payload = json!({
        "media_id": media_id,
        "width": sheet.width,
        "height": sheet.height,
        "columns": request.columns,
        "rows": request.rows,
        "tile": { "width": request.tile_width, "height": request.tile_height },
        "tiles": tiles,
        "png_path": sheet.png_path.to_string_lossy(),
        "warnings": warnings,
    });
    if let Some(jpeg) = &sheet.inline_jpeg {
        payload["image_block"] = image_block(jpeg, "image/jpeg");
    } else {
        payload["inline_omitted"] = json!("sheet exceeds the in-band size cap; read the PNG at `png_path`");
    }
    Ok(payload)
}

/// Parse `"WxH"` resolutions like `"640x360"`.
fn parse_resolution(raw: &str) -> Result<(u32, u32), String> {
    let (w, h) = raw
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("resolution must look like `640x360`, got `{raw}`"))?;
    let width: u32 = w
        .trim()
        .parse()
        .map_err(|_| format!("resolution width is not a number: `{w}`"))?;
    let height: u32 = h
        .trim()
        .parse()
        .map_err(|_| format!("resolution height is not a number: `{h}`"))?;
    if !(64..=1920).contains(&width) || !(64..=1920).contains(&height) {
        return Err("proxy resolution must stay within 64..1920 pixels".into());
    }
    Ok((width, height))
}

const MAX_PROXY_SPAN_TICKS: i64 = 10 * saturn_core::time::TICKS_PER_SECOND;

/// `render_preview_clip_proxy`: fast low-resolution MP4 snippet of the active
/// sequence around an edit point, written into the previews directory.
/// `include_audio: false` mutes the audio tracks in the proxy only.
pub fn preview_clip_proxy_payload(document: &ProjectDocument, params: &Value) -> Result<Value, String> {
    let start = parse_tick(params, "start_tick")?;
    let end = parse_tick(params, "end_tick")?;
    if end.0 <= start.0 {
        return Err("render_preview_clip_proxy: end_tick must be after start_tick".into());
    }
    if end.0 - start.0 > MAX_PROXY_SPAN_TICKS {
        return Err(
            "render_preview_clip_proxy: keep the snippet at or under 10 seconds; use \
             render_start/render_project for full exports"
                .into(),
        );
    }
    let (width, height) = match params.get("resolution").and_then(Value::as_str) {
        Some(raw) => parse_resolution(raw)?,
        None => (640, 360),
    };
    let include_audio = params.get("include_audio").and_then(Value::as_bool).unwrap_or(true);

    let mut proxy_document = document.clone();
    if !include_audio {
        for track in &mut proxy_document.project.sequence.tracks {
            track.gain_db = -120.0;
        }
    }
    if proxy_document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("render_preview_clip_proxy: the timeline has no clips".into());
    }

    let previews = frame_extractor::previews_dir()?;
    let output = previews.join(format!(
        "proxy-{}-{}-{}x{}{}.mp4",
        start.0,
        end.0,
        width,
        height,
        if include_audio { "" } else { "-mute" }
    ));
    let settings = RenderSettings {
        width,
        height,
        frame_rate_numerator: document.project.settings.frame_rate.numerator,
        frame_rate_denominator: document.project.settings.frame_rate.denominator,
        // Proxies are preview evidence, not deliverables: always the fast
        // deterministic software path.
        use_hardware_encoding: false,
        include_audio,
    };
    render_project_range_with_settings(
        &proxy_document,
        &output,
        Some(RenderRange { start, end }),
        settings,
        |_| {},
    )
    .map_err(|error| format!("render_preview_clip_proxy: {error}"))?;
    let bytes = std::fs::metadata(&output)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(json!({
        "output_path": output.to_string_lossy(),
        "width": width,
        "height": height,
        "include_audio": include_audio,
        "start_tick": start.0,
        "end_tick": end.0,
        "bytes": bytes,
    }))
}

/// Parse optional `{x, y, w, h}` ROI parameters into a scopes [`scopes_gpu::Roi`].
fn parse_roi(params: &Value) -> Result<Option<scopes_gpu::Roi>, String> {
    let Some(roi) = params.get("roi") else {
        return Ok(None);
    };
    let field = |name: &str| -> Result<u32, String> {
        roi.get(name)
            .and_then(Value::as_u64)
            .map(|value| value.min(u32::MAX as u64) as u32)
            .filter(|value| {
                // reject negatives up front (as_u64 already did) and zeros
                // are invalid for w/h — validated with the frame later
                *value > 0 || (name != "w" && name != "h")
            })
            .ok_or_else(|| format!("roi.{name} must be a non-negative integer"))
    };
    if roi.get("x").is_none() || roi.get("y").is_none() || roi.get("w").is_none() || roi.get("h").is_none() {
        return Err("roi must carry x, y, w and h".into());
    }
    Ok(Some(scopes_gpu::Roi { x: field("x")?, y: field("y")?, w: field("w")?, h: field("h")? }))
}

/// `color_read_scopes`: measurable scope telemetry over one frame.
///
/// Two modes:
/// - `media_id` present (+ optional `tick`): analyze a source frame;
/// - else `at_tick`: render the active sequence at that tick (graded and
///   composited) into the previews directory and analyze it — this is the
///   mode that verifies a grade actually changed the picture.
///
/// Every reported number is a direct pixel measurement (plan-review §4.4:
/// no invented aggregate score).
pub fn color_scopes_payload(document: &ProjectDocument, params: &Value) -> Result<Value, String> {
    let roi = parse_roi(params)?;
    let (width, height, rgb, at_tick, source): (u32, u32, Vec<u8>, Tick, String) =
        if let Some(media_id) = params.get("media_id").and_then(Value::as_u64) {
            let asset = find_asset(document, media_id)?;
            if let Some(reason) = decode_kind_allowed(&asset.kind) {
                return Err(format!("color_read_scopes: {reason}"));
            }
            if !asset.path.is_file() {
                return Err(format!(
                    "color_read_scopes: source file is missing: {}",
                    asset.path.display()
                ));
            }
            let tick = params
                .get("tick")
                .and_then(Value::as_i64)
                .map(Tick)
                .unwrap_or_else(|| default_poster_tick(asset.duration));
            let (width, height, rgb) = frame_extractor::decode_frame_rgb(&asset.path, tick)?;
            (width, height, rgb, tick, format!("media {}", asset.name))
        } else {
            let at_tick = Tick(
                params
                    .get("at_tick")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| {
                        "color_read_scopes requires media_id+tick or at_tick".to_owned()
                    })?,
            );
            let previews = frame_extractor::previews_dir()?;
            let png_path = previews.join(format!("scopes-frame-{}.png", at_tick.0));
            render_frame(document, at_tick, &png_path)?;
            let (width, height, rgb) = frame_extractor::decode_png_rgb(&png_path)?;
            let _ = std::fs::remove_file(&png_path); // analysis temp, not an artifact
            (width, height, rgb, at_tick, "sequence render".to_owned())
        };

    let report = scopes_gpu::analyze_rgb_scopes(&rgb, width, height, roi, at_tick)?;
    let report_value = serde_json::to_value(&report).map_err(|error| error.to_string())?;
    let report = report_value.as_object().cloned().unwrap_or_default();
    Ok(json!({
        "source": source,
        "analysis": report,
        "notes": {
            "ire_scale": "Rec.709 luma mapped to 0..100 IRE; 8-bit RGB quantizes 100 IRE to raw 255",
            "skin_tone": "chroma-band telemetry against the 123deg flesh-tone reference; compare to a reference shot, never a group target",
            "aggregate_score": null,
        },
    }))
}

/// Parse the optional loudness standard string into a
/// [`LoudnessStandard`] (`ebu_r128` default).
fn parse_standard(params: &Value) -> Result<LoudnessStandard, String> {
    match params.get("standard").and_then(Value::as_str) {
        None | Some("ebu_r128") | Some("ebu") => Ok(LoudnessStandard::EbuR128),
        Some("streaming") => Ok(LoudnessStandard::Streaming),
        Some("atsc_a85") | Some("atsc") => Ok(LoudnessStandard::AtscA85),
        Some(other) => Err(format!(
            "Unknown loudness standard {other:?}: use ebu_r128, streaming or atsc_a85"
        )),
    }
}

/// Parse optional `start_tick`/`end_tick` (both default-free; they bound
/// the analyzed region in project ticks).
fn parse_tick_range(params: &Value) -> Result<(Option<Tick>, Option<Tick>), String> {
    let start = params
        .get("start_tick")
        .and_then(Value::as_i64)
        .map(Tick)
        .filter(|tick| tick.0 >= 0);
    if params.get("start_tick").is_some() && start.is_none() {
        return Err("start_tick must be a non-negative integer".into());
    }
    let end = params
        .get("end_tick")
        .and_then(Value::as_i64)
        .map(Tick)
        .filter(|tick| tick.0 >= 0);
    if params.get("end_tick").is_some() && end.is_none() {
        return Err("end_tick must be a non-negative integer".into());
    }
    Ok((start, end))
}

/// `audio_read_telemetry`: measurable audio telemetry (E. R128 loudness,
/// octave-band spectrum, phase correlation, dialogue SNR) over one range.
///
/// Two modes, mirroring `color_read_scopes`:
/// - `media_id` present: analyze a source file's audio (default: whole
///   file, clamped to the analysis cap; `start_tick`/`end_tick` narrow it);
/// - else `start_tick`+`end_tick`: render the active sequence mix for the
///   range (the mixed, ducked, automated result — what the audience hears)
///   into the previews directory and analyze that.
///
/// `standard` selects the loudness target (`ebu_r128` default). A provided
/// `track_id` currently has no effect — telemetry always covers the
/// analyzed range's full mix; the tool errors rather than pretending to
/// scope it (plan §4.4 honesty).
pub fn audio_telemetry_payload(
    document: &ProjectDocument,
    params: &Value,
) -> Result<Value, String> {
    if params.get("track_id").is_some() {
        return Err(
            "audio_read_telemetry: track-scoped telemetry is not implemented; \
             analysis covers the range's full mix"
                .into(),
        );
    }
    let standard = parse_standard(params)?;
    let report = if let Some(media_id) = params.get("media_id").and_then(Value::as_u64) {
        let asset = find_asset(document, media_id)?;
        match &asset.kind {
            MediaKind::Audio | MediaKind::Video => {}
            other => {
                return Err(format!(
                    "audio_read_telemetry: {other:?} media has no audio track to analyze"
                ));
            }
        }
        if !asset.path.is_file() {
            return Err(format!(
                "audio_read_telemetry: source file is missing: {}",
                asset.path.display()
            ));
        }
        let (start, end) = parse_tick_range(params)?;
        let start = start.unwrap_or(Tick(0));
        let decoded = audio_pcm::decode_audio_pcm(&asset.path, start, end)?;
        let frames = (decoded.samples.len() / decoded.channels.max(1)) as i64;
        let end_tick = match end {
            Some(end) => end,
            None => Tick(
                start.0 + frames * saturn_core::time::TICKS_PER_SECOND as i64
                    / decoded.sample_rate.max(1) as i64,
            ),
        };
        audio_telemetry::analyze_pcm_audio_telemetry(
            &decoded.samples,
            decoded.channels,
            decoded.sample_rate,
            standard,
            start,
            end_tick,
        )
    } else {
        // Sequence mode: render the range's mix, decode, analyze.
        let (Some(start), Some(end)) = parse_tick_range(params)? else {
            return Err(
                "audio_read_telemetry requires media_id, or start_tick+end_tick for \
                 sequence mode"
                    .into(),
            );
        };
        if end.0 <= start.0 {
            return Err("end_tick must come after start_tick".into());
        }
        let previews = frame_extractor::previews_dir()?;
        let snippet = previews.join(format!("audio-telemetry-{}-{}.mp4", start.0, end.0));
        let result = render_project_range_with_settings(
            document,
            &snippet,
            Some(RenderRange { start, end }),
            RenderSettings {
                use_hardware_encoding: false,
                ..RenderSettings::from_project(document)
            },
            |_| {},
        );
        let decoded = result
            .and_then(|()| audio_pcm::decode_audio_pcm(&snippet, Tick(0), None))
            .map_err(|error| {
                let _ = std::fs::remove_file(&snippet);
                format!("Could not render/decode the sequence audio: {error}")
            });
        let _ = std::fs::remove_file(&snippet); // analysis temp, never an artifact
        let decoded = decoded?;
        audio_telemetry::analyze_pcm_audio_telemetry(
            &decoded.samples,
            decoded.channels,
            decoded.sample_rate,
            standard,
            start,
            end,
        )
    }?;

    let report_value = serde_json::to_value(&report).map_err(|error| error.to_string())?;
    let report_object = report_value.as_object().cloned().unwrap_or_default();
    Ok(json!({
        "source": if params.get("media_id").is_some() {
            "media file".to_owned()
        } else {
            "sequence mix".to_owned()
        },
        "report": report_object,
        "notes": {
            "snr_and_warnings": "heuristics over band energies; no ASR transcript, no invented aggregate score",
            "aggregate_score": null,
            "sequence_mode_encoding": "analyzed through the preview snippet's audio encoder chain",
        },
    }))
}

#[cfg(test)]
mod tests {    use super::*;

    #[test]
    fn resolution_parsing_is_strict() {
        assert_eq!(parse_resolution("640x360").unwrap(), (640, 360));
        assert_eq!(parse_resolution("1280X720").unwrap(), (1280, 720));
        assert!(parse_resolution("wide").is_err());
        assert!(parse_resolution("4000x2000").is_err());
    }

    #[test]
    fn media_preview_reports_honest_absence_per_kind() {
        let engine = saturn_core::EditorEngine::default();
        let document = engine.document().clone();
        // Unknown id → typed error.
        assert!(media_preview_payload(&document, 42, None).is_err());
    }
}

// ---------------------------------------------------------------------------
// Packet 2.2: semantic media search + cut continuity (perception tools)
// ---------------------------------------------------------------------------

/// `media_semantic_query`: embed the natural-language query with the local
/// SigLIP text encoder and cosine-scan the project's semantic index sidecar.
/// Nothing is ranked by heuristics here — hits carry the exact stored cosine
/// similarity, filtered to the current embedding-model revision.
pub fn semantic_query_payload(
    document: &ProjectDocument,
    project_path: Option<&std::path::Path>,
    arguments: &Value,
) -> Result<Value, String> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .ok_or_else(|| "media_semantic_query requires a non-empty string \"query\"".to_owned())?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 50) as usize;
    let media_filter = arguments
        .get("media_id")
        .and_then(Value::as_u64)
        .map(|id| id.to_string());
    let project_path = project_path.ok_or_else(|| {
        "media_semantic_query needs a saved project (the index sidecar lives next to \
         the project file); save the project first"
            .to_owned()
    })?;
    let sidecar = saturn_perception::SemanticIndex::open_for_project(project_path)?;
    // Only treat a *populated* index as usable; zero stored vectors is the
    // honest "nothing indexed yet" state, not an empty result.
    if sidecar.stored_vector_count()? == 0 {
        return Err(format!(
            "The semantic index at {} has no vectors yet — run the desktop \
             \"Index media for semantic search\" command first (it embeds one \
             sample frame every ~2 s of each media file)",
            sidecar.path().display()
        ));
    }
    let mut embedder = saturn_perception::SiglipEmbedder::load()?;
    // When filtering to one media, scan everything and truncate after the
    // filter, so the limit applies to what the caller actually receives.
    let scan_limit = if media_filter.is_some() { 10_000 } else { limit };
    let hits =
        saturn_perception::query_media_index(&mut embedder, &sidecar, query, scan_limit)?;
    let hit_values: Vec<Value> = hits
        .into_iter()
        .filter(|hit| match media_filter.as_deref() {
            Some(id) => hit.media_id == id,
            None => true,
        })
        .take(limit)
        .map(|hit| {
            let asset = hit
                .media_id
                .parse::<u64>()
                .ok()
                .and_then(|id| document.project.media.iter().find(|asset| asset.id == id));
            json!({
                "media_id": hit.media_id,
                "name": asset.map(|asset| asset.name.clone()),
                "source_path": asset.map(|asset| asset.path.display().to_string()),
                "tick": hit.tick_ticks,
                "similarity": hit.similarity,
            })
        })
        .collect();
    Ok(json!({
        "query": query,
        "hitCount": hit_values.len(),
        "hits": hit_values,
        "similarityMeaning": "cosine similarity in the SigLIP space of the pinned model revision; not a human relevance judgment",
        "revision": saturn_perception::media_search::semantic_index_revision(),
        "sidecar": sidecar.path().display().to_string(),
    }))
}

/// `edit_evaluate_cut_continuity`: decode the boundary frames (plus same-shot
/// neighbors, and ~150 ms of audio on each side when requested and present)
/// and run the measured cut-continuity report.
pub fn cut_continuity_payload(document: &ProjectDocument, arguments: &Value) -> Result<Value, String> {
    let media_id = arguments
        .get("media_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "edit_evaluate_cut_continuity requires media_id".to_owned())?;
    let cut_tick = arguments
        .get("cut_tick")
        .and_then(Value::as_i64)
        .ok_or_else(|| "edit_evaluate_cut_continuity requires cut_tick (source-media tick of the incoming frame)".to_owned())?;
    if cut_tick < 1 {
        return Err("cut_tick must be at least 1 (the incoming frame must exist)".into());
    }
    let include_audio = arguments
        .get("include_audio")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let asset = find_asset(document, media_id)?;
    match decode_kind_allowed(&asset.kind) {
        None => {}
        Some(reason) => {
            return Err(format!(
                "edit_evaluate_cut_continuity: {reason}"
            ));
        }
    }
    if !asset.path.is_file() {
        return Err(format!(
            "edit_evaluate_cut_continuity: source file is missing: {}",
            asset.path.display()
        ));
    }
    // Neighbor gap: one frame when fps is known, else ≈25 fps.
    let neighbor_step_ms = arguments
        .get("neighbor_step_ms")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| {
            asset
                .fps
                .filter(|fps| *fps > 0.0)
                .map(|fps| (1000.0 / fps).round() as i64)
                .unwrap_or(40)
                .clamp(1, 500)
        });
    let source = asset.path.clone();
    let mut decode = |tick: Tick| -> Result<(u32, u32, Vec<u8>), String> {
        frame_extractor::decode_frame_rgb(&source, tick)
    };
    let frames =
        saturn_perception::decode_cut_frames(Tick(cut_tick), Tick(neighbor_step_ms), &mut decode)?;
    // ~150 ms of mono 16 kHz audio on each side; a decode problem keeps the
    // pop honestly None instead of failing a video-only review.
    let (audio_before, audio_after): (Vec<f32>, Vec<f32>) = if include_audio {
        let before = audio_pcm::decode_audio_pcm_mono_16k(
            &asset.path,
            Tick((cut_tick - 150).max(0)),
            Some(Tick(cut_tick)),
        )
        .unwrap_or_default();
        let after = audio_pcm::decode_audio_pcm_mono_16k(
            &asset.path,
            Tick(cut_tick),
            Some(Tick(cut_tick + 150)),
        )
        .unwrap_or_default();
        (before, after)
    } else {
        (Vec::new(), Vec::new())
    };
    let report = saturn_perception::shot_detect::evaluate_cut_continuity(
        &frames.outgoing_last,
        &frames.incoming_first,
        &saturn_perception::shot_detect::CutContext {
            audio_before: &audio_before,
            audio_after: &audio_after,
            outgoing_prev: frames.outgoing_prev.as_ref(),
            incoming_next: frames.incoming_next.as_ref(),
        },
        Tick(cut_tick),
    )?;
    let mut value = serde_json::to_value(&report)
        .map_err(|error| format!("could not serialize the cut report: {error}"))?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "notes".into(),
            json!({
                "heuristic_continuity_score": "unvalidated weighted combination (plan §4.4) — advisory only",
                "focal_centroid_distance_norm": "luminance-centroid proxy, not measured eye-trace",
                "screen_direction_preserved": "always null in this packet: the 180° rule needs a subject-orientation model",
                "audioMeasured": include_audio && (!audio_before.is_empty() || !audio_after.is_empty()),
            }),
        );
    }
    Ok(value)
}
