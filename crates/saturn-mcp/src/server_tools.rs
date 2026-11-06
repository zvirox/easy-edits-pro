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
    ]
}
