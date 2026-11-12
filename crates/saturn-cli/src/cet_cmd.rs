// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Headless CLI subcommands for Capture Event Track (`cet-gen`, `cet-validate`,
//! `cet-propose`, `cet-query`) and flagship recipe transcript analyzers.

use std::collections::HashMap;
use std::path::PathBuf;

use saturn_core::tasksheet::{
    analyze_webinar_recap, derive_cet_proposals_from_jsonl, generate_synthetic_cet,
    query_cet_events, rank_podcast_clips, validate_cet_jsonl, CetProposalOptions, CetQueryParams,
    CetScenario, PodcastClipRankParams, SyntheticCetOptions, WebinarAnalyzeParams,
};
use saturn_core::tasksheet::SheetMustNot;
use saturn_core::SpeechSegment;
use serde_json::{json, Value};

/// Shared flag parser supporting `--key=value`, `--key value`, and bare flags.
pub fn parse_cli_flags(tokens: Vec<String>) -> (Vec<String>, HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        if let Some(name) = token.strip_prefix("--") {
            if let Some((key, value)) = name.split_once('=') {
                flags.insert(key.to_string(), value.to_string());
            } else if matches!(
                name,
                "json" | "named-shortcuts" | "help" | "include-proposals"
            ) {
                flags.insert(name.to_string(), "true".to_string());
            } else if let Some(next) = iter.peek() {
                if next.starts_with("--") {
                    flags.insert(name.to_string(), "true".to_string());
                } else {
                    flags.insert(name.to_string(), iter.next().unwrap_or_default());
                }
            } else {
                flags.insert(name.to_string(), "true".to_string());
            }
        } else {
            positional.push(token);
        }
    }
    (positional, flags)
}

fn parse_u64(flags: &HashMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    flags
        .get(key)
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| format!("invalid unsigned integer for --{key}: {s}"))
        })
        .transpose()
}

fn parse_u32(flags: &HashMap<String, String>, key: &str) -> Result<Option<u32>, String> {
    flags
        .get(key)
        .map(|s| {
            s.parse::<u32>()
                .map_err(|_| format!("invalid integer for --{key}: {s}"))
        })
        .transpose()
}

fn parse_f64(flags: &HashMap<String, String>, key: &str) -> Result<Option<f64>, String> {
    flags
        .get(key)
        .map(|s| {
            s.parse::<f64>()
                .map_err(|_| format!("invalid float for --{key}: {s}"))
        })
        .transpose()
}

fn print_json(value: &Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    Ok(())
}

/// Runs `cet-gen` (`saturn-cli cet-gen ...` and standalone `cet-gen ...`).
pub fn run_cet_gen_cli(tokens: Vec<String>) -> Result<(), String> {
    let (positional, flags) = parse_cli_flags(tokens);
    if flags.contains_key("help") {
        println!(
            "usage: cet-gen [output.cet.jsonl] [--scenario tutorial_basic|multi_window|redacted_privacy|named_shortcuts] [--video <file>] [--fps 30] [--width 1920] [--height 1080] [--duration-ms 24000] [--seed 1] [--named-shortcuts] [--json]"
        );
        return Ok(());
    }

    let scenario = flags
        .get("scenario")
        .map(|s| CetScenario::parse(s))
        .transpose()?
        .unwrap_or_default();

    let defaults = SyntheticCetOptions::default();
    let opts = SyntheticCetOptions {
        scenario,
        video_file: flags.get("video").cloned().unwrap_or(defaults.video_file),
        fps: parse_f64(&flags, "fps")?.unwrap_or(defaults.fps),
        width: parse_u32(&flags, "width")?.unwrap_or(defaults.width),
        height: parse_u32(&flags, "height")?.unwrap_or(defaults.height),
        duration_ms: parse_u64(&flags, "duration-ms")?.unwrap_or(defaults.duration_ms),
        seed: parse_u64(&flags, "seed")?.unwrap_or(defaults.seed),
        named_shortcuts: flags
            .get("named-shortcuts")
            .is_some_and(|v| v == "true"),
    };

    let jsonl = generate_synthetic_cet(&opts)?;
    let report = validate_cet_jsonl(&jsonl, None);

    if let Some(output_path) = positional.first().or_else(|| flags.get("output")) {
        let out = PathBuf::from(output_path);
        if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create directory {}: {e}", parent.display()))?;
        }
        std::fs::write(&out, &jsonl)
            .map_err(|e| format!("could not write {}: {e}", out.display()))?;
        print_json(&json!({
            "generated": true,
            "path": out,
            "scenario": opts.scenario.as_str(),
            "event_count": report.event_count,
            "duration_ms": report.duration_ms,
            "privacy_keys": report.privacy_keys,
            "privacy_text": report.privacy_text,
            "valid": report.valid,
        }))?;
    } else if flags.contains_key("json") {
        print_json(&json!({
            "generated": true,
            "scenario": opts.scenario.as_str(),
            "report": report,
            "jsonl": jsonl,
        }))?;
    } else {
        print!("{jsonl}");
    }

    Ok(())
}

/// Validates a `.cet.jsonl` sidecar file and prints a JSON summary.
pub fn run_cet_validate(tokens: Vec<String>) -> Result<(), String> {
    let (positional, flags) = parse_cli_flags(tokens);
    let path = positional
        .first()
        .ok_or("usage: saturn-cli cet-validate <file.cet.jsonl> [--max-bytes N]")?;
    let max_bytes = parse_u64(&flags, "max-bytes")?.map(|v| v as usize);
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read CET file {path}: {e}"))?;
    let report = validate_cet_jsonl(&raw, max_bytes);
    print_json(&serde_json::to_value(&report).map_err(|e| e.to_string())?)?;
    if !report.valid {
        return Err(report.errors.join("; "));
    }
    Ok(())
}

/// Derives zoom, chapter, focus, and idle proposals from a `.cet.jsonl` file.
pub fn run_cet_propose(tokens: Vec<String>) -> Result<(), String> {
    let (positional, flags) = parse_cli_flags(tokens);
    let path = positional
        .first()
        .ok_or("usage: saturn-cli cet-propose <file.cet.jsonl> [--scale 1.4] [--dead-air-ms 1200]")?;
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read CET file {path}: {e}"))?;

    let defaults = CetProposalOptions::default();
    let opts = CetProposalOptions {
        target_zoom_scale: parse_f64(&flags, "scale")?.unwrap_or(defaults.target_zoom_scale),
        zoom_dwell_ms: parse_u64(&flags, "dwell-ms")?.unwrap_or(defaults.zoom_dwell_ms),
        zoom_hold_ms: parse_u64(&flags, "hold-ms")?.unwrap_or(defaults.zoom_hold_ms),
        dead_air_threshold_ms: parse_u64(&flags, "dead-air-ms")?.unwrap_or(defaults.dead_air_threshold_ms),
        min_window_dwell_ms: parse_u64(&flags, "min-window-ms")?.unwrap_or(defaults.min_window_dwell_ms),
    };
    let report = derive_cet_proposals_from_jsonl(&raw, &opts)?;
    print_json(&serde_json::to_value(&report).map_err(|e| e.to_string())?)
}

/// Filters CET events with AF5 untrusted field metadata (`cet-query`).
pub fn run_cet_query(tokens: Vec<String>) -> Result<(), String> {
    let (positional, flags) = parse_cli_flags(tokens);
    let path = positional
        .first()
        .ok_or("usage: saturn-cli cet-query <file.cet.jsonl> [--from-ms N] [--to-ms N] [--types a,b] [--window substr] [--limit N]")?;
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read CET file {path}: {e}"))?;
    let types = flags
        .get("types")
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let params = CetQueryParams {
        from_ms: parse_u64(&flags, "from-ms")?,
        to_ms: parse_u64(&flags, "to-ms")?,
        types,
        window: flags.get("window").cloned(),
        limit: parse_u64(&flags, "limit")?.map(|v| v as usize).unwrap_or(100),
    };
    let result = query_cet_events(&raw, &params)?;
    print_json(&serde_json::to_value(&result).map_err(|e| e.to_string())?)
}

fn parse_segments_and_must_not(root: &Value) -> Result<(Vec<SpeechSegment>, Vec<String>, Option<SheetMustNot>), String> {
    let segments: Vec<SpeechSegment> = serde_json::from_value(
        root.get("segments")
            .cloned()
            .ok_or("input JSON requires `segments` array")?,
    )
    .map_err(|e| format!("invalid `segments` array: {e}"))?;
    let must_include: Vec<String> = root
        .get("must_include")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("invalid `must_include`: {e}"))?
        .unwrap_or_default();
    let must_not: Option<SheetMustNot> = root
        .get("must_not")
        .filter(|v| !v.is_null())
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("invalid `must_not`: {e}"))?;
    Ok((segments, must_include, must_not))
}

/// Runs deterministic core recipe content analyzers (`podcast` or `webinar`).
pub fn run_analyze_recipe(tokens: Vec<String>) -> Result<(), String> {
    let (positional, _) = parse_cli_flags(tokens);
    let recipe = positional
        .first()
        .map(String::as_str)
        .ok_or("usage: saturn-cli analyze-recipe <podcast|webinar> <input.json>")?;
    let input_path = positional
        .get(1)
        .ok_or("missing input JSON path for analyze-recipe")?;
    let raw = std::fs::read_to_string(input_path)
        .map_err(|e| format!("could not read {input_path}: {e}"))?;
    let root: Value =
        serde_json::from_str(&raw).map_err(|e| format!("invalid input JSON: {e}"))?;
    let (segments, must_include, must_not) = parse_segments_and_must_not(&root)?;

    match recipe {
        "podcast" | "podcast-to-clips" => {
            let defaults = PodcastClipRankParams::default();
            let hints: Vec<f64> = root
                .get("segment_energy_hints")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()
                .map_err(|e| format!("invalid `segment_energy_hints`: {e}"))?
                .unwrap_or_default();
            let params = PodcastClipRankParams {
                target_duration_s: root
                    .get("target_duration_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(defaults.target_duration_s),
                tolerance_s: root
                    .get("tolerance_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(defaults.tolerance_s),
                top_k: root
                    .get("top_k")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize)
                    .unwrap_or(defaults.top_k),
                must_include,
                must_not,
                segment_energy_hints: hints,
            };
            let report = rank_podcast_clips(&segments, &params)?;
            print_json(&serde_json::to_value(&report).map_err(|e| e.to_string())?)
        }
        "webinar" | "webinar-recap" => {
            let defaults = WebinarAnalyzeParams::default();
            let params = WebinarAnalyzeParams {
                reel_target_s: root
                    .get("reel_target_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(defaults.reel_target_s),
                reel_tolerance_s: root
                    .get("reel_tolerance_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(defaults.reel_tolerance_s),
                chapter_gap_s: root
                    .get("chapter_gap_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(defaults.chapter_gap_s),
                must_include,
                must_not,
            };
            let report = analyze_webinar_recap(&segments, &params)?;
            print_json(&serde_json::to_value(&report).map_err(|e| e.to_string())?)
        }
        other => Err(format!(
            "unknown recipe analyzer `{other}`; expected `podcast` or `webinar`"
        )),
    }
}
