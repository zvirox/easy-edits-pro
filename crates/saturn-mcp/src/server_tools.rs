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
