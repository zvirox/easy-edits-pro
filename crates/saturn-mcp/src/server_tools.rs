// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! MCP tool catalogue: names, descriptions and JSON schemas (Packet 1.1
//! split of `server.rs` under the 800-line gate).

use rmcp::model::Tool;
use serde_json::{Value, json};
use std::sync::Arc;

fn schema(value: Value) -> Arc<serde_json::Map<String, Value>> {
    Arc::new(value.as_object().cloned().unwrap_or_default())
}

pub(super) fn tool_list() -> Vec<Tool> {
    vec![
            Tool::new(
                "command_list",
                "Discover commands supported by the editor engine, including project, media, timeline, effects and audio operations.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "filter": { "type": "string", "description": "Optional case-insensitive id or label filter." },
                        "enabled_only": { "type": "boolean", "description": "Only include commands enabled in the current project." }
                    }
                })),
            ),
            Tool::new(
                "command_run",
                "Run an editor command by id with its JSON parameters. Use command_list to discover ids.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Editor command id." },
                        "params": { "type": "object", "description": "Command-specific parameters." }
                    },
                    "required": ["id"]
                })),
            ),
            Tool::new(
                "command_batch",
                "Run editor commands in order. Each successful command is applied to the current project session.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "steps": { "type": "array", "items": { "type": "object", "properties": {
                            "id": { "type": "string" }, "params": { "type": "object" }
                        }, "required": ["id"] } },
                        "stop_on_error": { "type": "boolean", "default": true }
                    },
                    "required": ["steps"]
                })),
            ),
            Tool::new(
                "doc_inspect",
                "Inspect the current project document, including media, settings and active timeline.",
                schema(json!({ "type": "object", "properties": {} })),
            ),
            Tool::new(
                "sequence_inspect",
                "Inspect the active sequence, including its tracks and timeline clips.",
                schema(json!({ "type": "object", "properties": {} })),
            ),
            Tool::new(
                "media_search",
                "Search media already imported in the active project by name, path, and optional media kind.",
                schema(json!({ "type": "object", "properties": {
                    "query": { "type": "string" },
                    "kind": { "type": "string", "enum": ["video", "audio", "image", "title", "compoundsequence", "unknown"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500, "default": 100 }
                } })),
            ),
            Tool::new(
                "media_preview",
                "Inspect one imported media asset, its timeline placements and a real poster frame (still_frame plus an MCP image content block). For media that cannot be decoded (audio, titles, missing sources) still_frame.available is false with the reason.",
                schema(json!({ "type": "object", "properties": {
                    "media_id": { "type": "integer", "minimum": 0 }
                }, "required": ["media_id"] })),
            ),
            Tool::new(
                "render_start",
                "Start an asynchronous export and return its job_id. Poll render_status for progress or call render_cancel.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string" },
                        "use_mark_range": { "type": "boolean", "default": false },
                        "range": { "type": "object", "properties": {
                            "start_ticks": { "type": "integer", "minimum": 0 },
                            "end_ticks": { "type": "integer", "minimum": 1 }
                        }, "required": ["start_ticks", "end_ticks"] },
                        "settings": { "type": "object", "properties": {
                            "width": { "type": "integer" }, "height": { "type": "integer" },
                            "frameRateNumerator": { "type": "integer" }, "frameRateDenominator": { "type": "integer" },
                            "useHardwareEncoding": { "type": "boolean" }
                        } }
                    }, "required": ["output_path"]
                })),
            ),
            Tool::new(
                "render_status",
                "Read queued, running, completed, failed, or cancelled render state and progress.",
                schema(
                    json!({ "type": "object", "properties": { "job_id": { "type": "integer", "minimum": 1 } }, "required": ["job_id"] }),
                ),
            ),
            Tool::new(
                "render_cancel",
                "Request cancellation of a queued or active render job.",
                schema(
                    json!({ "type": "object", "properties": { "job_id": { "type": "integer", "minimum": 1 } }, "required": ["job_id"] }),
                ),
            ),
            Tool::new(
                "render_project",
                "Render the active sequence to MP4 or WebM. In bridge mode this renders the project open in the desktop app.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string", "description": "Absolute .mp4 or .webm output path." },
                        "use_mark_range": { "type": "boolean", "default": false },
                        "range": { "type": "object", "description": "Optional timeline render range in project ticks, e.g. {start_ticks: 0, end_ticks: 508032000000} for a two-second clip at 30 fps.", "properties": {
                            "start_ticks": { "type": "integer", "minimum": 0 },
                            "end_ticks": { "type": "integer", "minimum": 1 }
                        }, "required": ["start_ticks", "end_ticks"] },
                        "settings": { "type": "object", "properties": {
                            "width": { "type": "integer" }, "height": { "type": "integer" },
                            "frameRateNumerator": { "type": "integer" }, "frameRateDenominator": { "type": "integer" },
                            "useHardwareEncoding": { "type": "boolean" }
                        } }
                    },
                    "required": ["output_path"]
                })),
            ),
            Tool::new(
                "vision_extract_frame_sheet",
                "Extract a labeled contact sheet (grid of timecode-burned video frames) from one media asset for visual inspection. Returns tiles with ticks and timecodes, the PNG artifact path in the private previews directory, and an in-band JPEG image (size-capped).",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "media_id": { "type": "integer", "minimum": 0 },
                        "start_tick": { "type": "integer", "minimum": 0 },
                        "end_tick": { "type": "integer", "minimum": 1 },
                        "columns": { "type": "integer", "minimum": 1, "maximum": 8, "default": 4 },
                        "rows": { "type": "integer", "minimum": 1, "maximum": 8, "default": 4 },
                        "tile_width": { "type": "integer", "minimum": 64, "maximum": 640, "default": 320 },
                        "tile_height": { "type": "integer", "minimum": 64, "maximum": 640, "default": 180 },
                        "burn_timecode": { "type": "boolean", "default": true }
                    },
                    "required": ["media_id", "start_tick", "end_tick"]
                })),
            ),
            Tool::new(
                "render_preview_clip_proxy",
                "Render a fast, low-resolution MP4 snippet of the active sequence (at most 10 seconds) into the private previews directory for visual verification of an edit. Proxies are preview evidence, not deliverables.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "start_tick": { "type": "integer", "minimum": 0 },
                        "end_tick": { "type": "integer", "minimum": 1 },
                        "resolution": { "type": "string", "description": "WxH, e.g. 640x360; within 64..1920", "default": "640x360" },
                        "include_audio": { "type": "boolean", "default": true }
                    },
                    "required": ["start_tick", "end_tick"]
                })),
            ),
            Tool::new(
                "color_read_scopes",
                "Read measurable video scope telemetry for one frame: luma IRE percentiles (min/p10/p50/p90/max), clipped-highlight and crushed-shadow percentages, RGB parade medians, vectorscope saturation, and chroma-band skin-tone-line statistics relative to a reference shot. Pass media_id+tick to analyze a source frame, or at_tick to render and analyze the active sequence (verifies a grade actually changed the picture). Optional roi restricts the region.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "media_id": { "type": "integer", "minimum": 0 },
                        "tick": { "type": "integer", "minimum": 0, "description": "Source-frame tick (media mode)" },
                        "at_tick": { "type": "integer", "minimum": 0, "description": "Timeline tick; renders the graded sequence frame (sequence mode)" },
                        "roi": { "type": "object", "properties": {
                            "x": { "type": "integer", "minimum": 0 },
                            "y": { "type": "integer", "minimum": 0 },
                            "w": { "type": "integer", "minimum": 1 },
                            "h": { "type": "integer", "minimum": 1 }
                        }, "required": ["x", "y", "w", "h"] }
                    }
                })),
            ),
            Tool::new(
                "color_match_shot_to_reference",
                "Automated shot matcher: decode one source frame from the target clip and one from the reference hero shot, fit a primary-wheels correction (exposure, per-channel gain, gamma, offset) that aligns their luma/chroma CDFs, anchor detected skin tones to the reference's vectorscope band, and append the validated correction as a provenance-tagged color node on the target clip. Frames are measured on the clips' ungraded source media at src_tick/ref_tick (defaults: each clip's source midpoint).",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "src_track_id": { "type": "integer", "minimum": 0, "description": "Track of the clip that receives the match node" },
                        "src_clip_index": { "type": "integer", "minimum": 0 },
                        "ref_track_id": { "type": "integer", "minimum": 0, "description": "Track of the reference hero shot" },
                        "ref_clip_index": { "type": "integer", "minimum": 0 },
                        "lock_skin_tones": { "type": "boolean", "description": "Anchor detected skin-band chroma to the reference (default true)" },
                        "src_tick": { "type": "integer", "minimum": 0, "description": "In-clip source tick for the target frame" },
                        "ref_tick": { "type": "integer", "minimum": 0, "description": "In-clip source tick for the reference frame" }
                    },
                    "required": ["src_track_id", "src_clip_index", "ref_track_id", "ref_clip_index"]
                })),
            ),
            Tool::new(
                "edit_match_cut_align",
                "Analyze adjacent raw source clips and slip only the incoming source_in in one undoable edit. eye_trace is a squared-contrast centroid heuristic, motion matches measured block displacement, transient matches 40 ms attack envelopes and places the incoming onset at the cut. No-signal analysis errors. Align before detaching audio; requires enabled, unretimed, unlinked clips without effects/tracking and a hard cut. Returns application and measurement telemetry.",
                schema(json!({
                    "type": "object", "additionalProperties": false,
                    "properties": {
                        "track_id": {"type": "integer", "minimum": 0},
                        "left_clip_index": {"type": "integer", "minimum": 0},
                        "right_clip_index": {"type": "integer", "minimum": 0},
                        "mode": {"type": "string", "enum": ["eye_trace", "motion", "transient"]},
                        "search_frames": {"type": "integer", "minimum": 1, "maximum": 30, "default": 12}
                    },
                    "required": ["track_id", "left_clip_index", "right_clip_index", "mode"]
                })),
            ),
            Tool::new(
                "timeline_beat_grid_quantize",
                "Roll cuts in a continuous 2-128 clip video track onto a measured music pulse grid, preserving outer edges and source-to-timeline mapping in one undoable transaction. Analyze 4-120 seconds of one raw mono/stereo music clip covering the strip. Constant 60-200 BPM RMS-rise heuristic; no meter recognition, and half/double-tempo ambiguity is possible. Bar/half require a caller-supplied downbeat_tick and assume 4/4. Rejects gaps, overlaps, transitions, processing, linked/retimed clips, insufficient handles, collisions and stale snapshots. Quantize before detaching sound. Returns applied cut positions and measured fit telemetry.",
                schema(json!({
                    "type": "object", "additionalProperties": false,
                    "properties": {
                        "music_track_id": {"type": "integer", "minimum": 0},
                        "video_track_id": {"type": "integer", "minimum": 0},
                        "subdivision": {"type": "string", "enum": ["bar", "half", "quarter"]},
                        "strength": {"type": "number", "minimum": 0, "maximum": 1},
                        "downbeat_tick": {"type": "integer", "minimum": 0, "description": "Known bar-start timeline tick; must align with a measured beat"}
                    },
                    "required": ["music_track_id", "video_track_id", "subdivision", "strength"],
                    "allOf": [{"if": {"properties": {"subdivision": {"enum": ["bar", "half"]}}}, "then": {"required": ["downbeat_tick"]}}]
                })),
            ),
            Tool::new(
                "audio_process",
                "Prepare and atomically apply an undoable offline audio edit to a saved project (maximum 120 seconds). phase_align: two single-clip audio tracks, raw-source GCC-PHAT with fractional correction. room_tone: caller-selected speech-free profile, spectral gap synthesis with in-gap fades. duck: energy-keyed dialogue ducking, requires empty music volume and pan automation lanes. normalize_master: render the full mix to float WAV, measure/limit/normalize, retain and mute original tracks, add an audible master bounce. asymmetric_trim: select the incoming video clip of an adjacent pair, roll the picture cut by video_delta ticks, move sound by audio_delta ticks, and bake a centered constant-power fade (default 40 ms) to an editable audio track. Requires plain unretimed video clips with source handles and mono/stereo audio; both deltas are relative to the original cut. Derived media is stored beside the project; stale analyses are rejected. Cinema stem separation is not available.",
                schema(json!({
                    "type": "object", "required": ["operation"],
                    "properties": {
                        "operation": {"type": "string", "enum": ["phase_align", "room_tone", "duck", "normalize_master", "asymmetric_trim"]},
                        "clip_index": {"type": "integer", "minimum": 0},
                        "video_delta": {"type": "integer"},
                        "audio_delta": {"type": "integer"},
                        "fade_ticks": {"type": "integer", "minimum": 0},
                        "ref_track_id": {"type": "integer", "minimum": 0},
                        "target_track_id": {"type": "integer", "minimum": 0},
                        "track_id": {"type": "integer", "minimum": 0},
                        "dialogue_track_id": {"type": "integer", "minimum": 0},
                        "music_track_id": {"type": "integer", "minimum": 0},
                        "room_tone_media_id": {"type": "integer", "minimum": 0},
                        "profile_start_tick": {"type": "integer", "minimum": 0},
                        "profile_end_tick": {"type": "integer", "minimum": 1},
                        "min_gap_ms": {"type": "integer", "minimum": 1, "maximum": 120000},
                        "crossfade_ms": {"type": "integer", "minimum": 0, "maximum": 5000},
                        "duck_db": {"type": "number", "minimum": -60, "maximum": 0},
                        "lookahead_ms": {"type": "integer", "minimum": 0, "maximum": 1000},
                        "hold_ms": {"type": "integer", "minimum": 0, "maximum": 5000},
                        "release_ms": {"type": "integer", "minimum": 10, "maximum": 5000},
                        "threshold_dbfs": {"type": "number", "minimum": -80, "maximum": 0},
                        "standard": {"type": "string", "enum": ["ebu_r128", "streaming", "atsc_a85"]},
                        "true_peak_dbtp": {"type": "number", "minimum": -12, "maximum": -0.1}
                    }
                })),
            ),
            Tool::new(
                "audio_read_telemetry",
                "Read measurable audio telemetry for one range: full EBU R128 / BS.1770-4 loudness block (momentary, short-term, integrated LUFS, LRA, true peak dBTP), a 10-band octave spectrum (31 Hz..16 kHz), stereo phase correlation (−1..+1), and a dialogue-to-background SNR estimate. Pass media_id to analyze a source file, or start_tick+end_tick to render and analyze the active sequence mix (verifies a mix change actually landed). Optional standard selects the loudness target (ebu_r128, streaming, atsc_a85).",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "media_id": { "type": "integer", "minimum": 0 },
                        "start_tick": { "type": "integer", "minimum": 0 },
                        "end_tick": { "type": "integer", "minimum": 1 },
                        "standard": { "type": "string", "enum": ["ebu_r128", "streaming", "atsc_a85"], "default": "ebu_r128" },
                        "track_id": { "type": "integer", "minimum": 0, "description": "Reserved: track-scoped telemetry is not implemented; analysis covers the range's full mix" }
                    }
                })),
            ),
            Tool::new(
                "media_semantic_query",
                "Search the project's semantic media index with a natural-language query (SigLIP zero-shot image/text matching, offline local inference). Returns the stored frames whose embedded vectors are most cosine-similar to the query — media_id, source tick, and the exact similarity. Requires the sidecar index <project>.saturn-index.sqlite to be populated (index media on the desktop first); matches only vectors from the current embedding-model revision.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "minLength": 1 },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 8 },
                        "media_id": { "type": "integer", "minimum": 0, "description": "Restrict results to one media asset (matched on its indexed id)" }
                    },
                    "required": ["query"]
                })),
            ),
            Tool::new(
                "edit_evaluate_cut_continuity",
                "Measure the continuity of one hard cut in a source media file: luma jump (|ΔIRE|), CIEDE2000 mean-color change, luminance-centroid shift (eye-trace proxy, not measured gaze), within-shot motion-direction difference (None when either side is near-static), audio boundary pop (None when silent/absent), screen-direction always None (the 180° rule needs a subject-orientation model that is not implemented). heuristic_continuity_score is an unvalidated weighted combination — treat it as advisory only.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "media_id": { "type": "integer", "minimum": 0 },
                        "cut_tick": { "type": "integer", "minimum": 1, "description": "Source-media tick of the first incoming frame" },
                        "neighbor_step_ms": { "type": "integer", "minimum": 1, "maximum": 500, "default": 40, "description": "Gap to the same-shot neighbor frames used for motion direction; defaults to ≈1 frame at 25 fps (media fps used when known)" },
                        "include_audio": { "type": "boolean", "default": true }
                    },
                    "required": ["media_id", "cut_tick"]
                })),
            ),
    ]
}
