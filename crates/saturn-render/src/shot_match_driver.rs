// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Decode-owning driver for `color.match_shot_to_reference`. The engine
//! command in `saturn-core` is decode-free by design (the core owns project
//! state, not media access), so this module plays the caller role the plan
//! assigns to it: resolve both clips' source media, decode one frame each,
//! measure and fit, and produce the exact command request the bridge then
//! applies through the engine's validated, checkpointed command channel.
//!
//! The measurement is deliberately taken on the clips' **source media**
//! frames (ungraded): the matcher models the camera/pipeline mismatch
//! between the two shots, which the primary-wheels model can correct
//! globally. Corrections already on either clip are not part of the
//! measurement domain and are reported as a caveat in the response.

use crate::frame_extractor;
use crate::shot_match::{fit_match, measurement};
use saturn_core::project::{MediaKind, ProjectDocument};
use saturn_core::time::Tick;
use serde_json::{Value, json};

/// Provenance expected on the node the engine command creates; must match
/// `saturn_core`'s `SHOT_MATCH_ORIGIN`.
pub const SHOT_MATCH_ORIGIN: &str = "shot-match";

/// Compute the shot-match fit for one clip against a reference clip.
///
/// Parameters: `src_track_id`, `src_clip_index` (the clip that receives the
/// match node), `ref_track_id`, `ref_clip_index` (the anchor hero shot),
/// `lock_skin_tones` (default true), and optional `src_tick` / `ref_tick`
/// in-clip ticks. Tick defaults fall at the middle of each clip's source
/// range; the caller picks another frame when the middle is unsuitable.
///
/// Returns the ready-to-execute engine `command` plus the fit `telemetry`.
/// Frame decode is expensive — run on a blocking pool (the bridge does).
pub fn match_shot_payload(document: &ProjectDocument, params: &Value) -> Result<Value, String> {
    let src_track_id = u64_param(params, "src_track_id")?;
    let src_clip_index = usize_param(params, "src_clip_index")?;
    let ref_track_id = u64_param(params, "ref_track_id")?;
    let ref_clip_index = usize_param(params, "ref_clip_index")?;
    let lock_skin_tones = params
        .get("lock_skin_tones")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let (src_clip, src_source) = clip_frame_source(document, src_track_id, src_clip_index)?;
    let (ref_clip, ref_source) = clip_frame_source(document, ref_track_id, ref_clip_index)?;

    let src_tick = params
        .get("src_tick")
        .and_then(Value::as_i64)
        .map(Tick)
        .unwrap_or_else(|| mid_tick(src_clip.source_in, src_clip.duration));
    let ref_tick = params
        .get("ref_tick")
        .and_then(Value::as_i64)
        .map(Tick)
        .unwrap_or_else(|| mid_tick(ref_clip.source_in, ref_clip.duration));

    let (src_w, src_h, src_rgb) = frame_extractor::decode_frame_rgb(&src_source.path, src_tick)?;
    let (ref_w, ref_h, ref_rgb) = frame_extractor::decode_frame_rgb(&ref_source.path, ref_tick)?;

    let target_m = measurement(src_w, src_h, &src_rgb)
        .map_err(|error| format!("color match: target frame: {error}"))?;
    let reference_m = measurement(ref_w, ref_h, &ref_rgb)
        .map_err(|error| format!("color match: reference frame: {error}"))?;
    let fit = fit_match(&target_m, &reference_m, lock_skin_tones)
        .map_err(|error| format!("color match: {error}"))?;
    let wheels = json!(fit.wheels_payload());

    Ok(json!({
        "command": {
            "id": "color.match_shot_to_reference",
            "params": {
                "src_track_id": src_track_id,
                "src_clip_index": src_clip_index,
                "ref_track_id": ref_track_id,
                "ref_clip_index": ref_clip_index,
                "lock_skin_tones": lock_skin_tones,
                "wheels": wheels,
            },
        },
        "telemetry": {
            "match": fit,
            "frames": {
                "target": frame_report(&src_source.name, src_tick, src_w, src_h),
                "reference": frame_report(&ref_source.name, ref_tick, ref_w, ref_h),
            },
            "measured_on": "ungraded source media frames; corrections already on either clip are outside the fit's measurement domain",
        },
    }))
}

/// Resolve one timeline clip and its decodable source media by
/// `track_id` + `clip_index`. References the asset rather than cloning it.
fn clip_frame_source<'a>(
    document: &'a ProjectDocument,
    track_id: u64,
    clip_index: usize,
) -> Result<
    (
        &'a saturn_core::project::Clip,
        &'a saturn_core::project::MediaAsset,
    ),
    String,
> {
    let track = document
        .project
        .sequence
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| format!("Track {track_id} was not found"))?;
    let clip = track.clips.get(clip_index).ok_or_else(|| {
        format!("Timeline clip does not exist: track {track_id} index {clip_index}")
    })?;
    let asset = document
        .project
        .media
        .iter()
        .find(|asset| asset.id == clip.media_id)
        .ok_or_else(|| format!("Clip media {} was not found", clip.media_id))?;
    if !matches!(asset.kind, MediaKind::Video | MediaKind::Image) {
        return Err(format!(
            "color match: media \"{}\" ({:?}) has no video frames to measure",
            asset.name, asset.kind
        ));
    }
    if !asset.path.is_file() {
        return Err(format!(
            "color match: source file is missing: {}",
            asset.path.display()
        ));
    }
    Ok((clip, asset))
}

/// Default analysis tick: the middle of the clip's source range.
fn mid_tick(source_in: Tick, duration: Tick) -> Tick {
    Tick(source_in.0.saturating_add(duration.0 / 2))
}

fn frame_report(name: &str, tick: Tick, width: u32, height: u32) -> Value {
    json!({ "media": name, "tick": tick.0, "width": width, "height": height })
}

fn u64_param(params: &Value, key: &str) -> Result<u64, String> {
    params
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("color match requires '{key}' as an unsigned integer"))
}

fn usize_param(params: &Value, key: &str) -> Result<usize, String> {
    params
        .get(key)
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .ok_or_else(|| format!("color match requires '{key}' as an unsigned integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with_clips() -> (ProjectDocument, u64, u64) {
        let mut engine = saturn_core::EditorEngine::default();
        // Decoding needs no real media for these tests: the anchor
        // resolution errors they assert are unreachable only while the
        // source file is missing, so both assets point at stub files.
        let stub_a = std::env::temp_dir().join("eep-match-a.mp4");
        let stub_b = std::env::temp_dir().join("eep-match-b.mp4");
        std::fs::write(&stub_a, b"stub").unwrap();
        std::fs::write(&stub_b, b"stub").unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                json!({"name": "a.mp4", "path": stub_a, "kind": "video"}),
            ))
            .unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                json!({"name": "b.mp4", "path": stub_b, "kind": "video"}),
            ))
            .unwrap();
        let media_a = engine.document().project.media[0].id;
        let media_b = engine.document().project.media[1].id;
        engine
            .execute(saturn_core::CommandRequest::new(
                "timeline.add_clip",
                json!({"media_id": media_a, "track_id": 1, "timeline_start": 0}),
            ))
            .unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "timeline.add_clip",
                json!({"media_id": media_b, "track_id": 1, "timeline_start": 0}),
            ))
            .unwrap();
        let document = engine.document().clone();
        let track_id = document.project.sequence.tracks[0].id;
        (document, track_id, track_id)
    }

    fn param_error(document: &ProjectDocument, t0: u64, t1: u64, patch: Value) -> String {
        let params = json!({
            "src_track_id": t0,
            "src_clip_index": 0,
            "ref_track_id": t1,
            "ref_clip_index": 1,
        });
        let params = match patch {
            Value::Null => params,
            patch => {
                let mut params = params;
                let (key, value) = patch.as_object().unwrap().iter().next().unwrap();
                params[key.as_str()] = value.clone();
                params
            }
        };
        match_shot_payload(document, &params).unwrap_err()
    }

    #[test]
    fn missing_media_file_is_a_typed_error() {
        let (document, t0, t1) = document_with_clips();
        // Rename the stub away: a missing source must refuse with its path.
        let stub = std::env::temp_dir().join("eep-match-a.mp4");
        let hidden = std::env::temp_dir().join("eep-match-a-hidden.mp4");
        std::fs::rename(&stub, &hidden).unwrap();
        let error = param_error(&document, t0, t1, Value::Null);
        std::fs::rename(&hidden, &stub).unwrap();
        assert!(error.contains("source file is missing"), "{error}");
        assert!(error.contains("eep-match-a.mp4"), "{error}");
    }

    #[test]
    fn source_errors_are_typed_before_any_decode() {
        let (document, t0, t1) = document_with_clips();
        // A decode failure (stub file has no media) is still a typed error
        // naming the operation, not a panic or a fake success.
        let params = json!({
            "src_track_id": t0,
            "src_clip_index": 0,
            "ref_track_id": t1,
            "ref_clip_index": 1,
        });
        let error = match_shot_payload(&document, &params).unwrap_err();
        assert!(!error.contains("panicked"), "{error}");

        let (document, t0, t1) = document_with_clips();
        let error = param_error(&document, t0, t1, json!({"src_clip_index": 9}));
        assert!(error.contains("does not exist"), "{error}");
        let error = param_error(&document, t0, t1, json!({"src_track_id": 999}));
        assert!(error.contains("was not found"), "{error}");
        let error = param_error(&document, t0, t1, json!({"ref_track_id": 42}));
        assert!(error.contains("was not found"), "{error}");
    }

    #[test]
    fn audio_clips_refuse_with_a_reason() {
        let mut engine = saturn_core::EditorEngine::default();
        let stub = std::env::temp_dir().join("eep-match-audio.wav");
        std::fs::write(&stub, b"stub").unwrap();
        engine
            .execute(saturn_core::CommandRequest::new(
                "project.add_media",
                json!({"name": "a.wav", "path": stub, "kind": "audio"}),
            ))
            .unwrap();
        let media_a = engine.document().project.media[0].id;
        engine
            .execute(saturn_core::CommandRequest::new(
                "timeline.add_clip",
                json!({"media_id": media_a, "track_id": 1, "timeline_start": 0}),
            ))
            .unwrap();
        let document = engine.document().clone();
        let track_id = document.project.sequence.tracks[0].id;
        let params = json!({
            "src_track_id": track_id,
            "src_clip_index": 0,
            "ref_track_id": track_id,
            "ref_clip_index": 0,
        });
        let error = match_shot_payload(&document, &params).unwrap_err();
        assert!(error.contains("no video frames"), "{error}");
    }
}
