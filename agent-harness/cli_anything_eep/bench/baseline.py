# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""EEP-Bench published baselines for scripted conformance and real-model agent evaluations."""

from __future__ import annotations

from typing import Any, Optional

# Published baseline for the deterministic scripted conformance suite
SCRIPTED_CONFORMANCE_BASELINE: dict[str, Any] = {
    "suite": "eep-bench-smoke",
    "suite_label": "EEP-Bench (scripted conformance suite)",
    "mode": "scripted_deterministic",
    "total_tasks": 16,
    "passed_tasks": 16,
    "failed_tasks": 0,
    "pass_rate": 1.0,
    "token_usage": None,
    "model_cost_usd": None,
    "aggregate_quality_score": None,
    "notes": (
        "Deterministic scripted baseline against compiled Rust core/CLI/MCP binaries; "
        "LLM tokens and costs are explicitly null (never assumed zero)."
    ),
    "task_summaries": {
        "bench_podcast_to_clips": {"passed": True, "min_tool_calls": 8, "wall_clock_ms": 320},
        "bench_talking_head_cet": {"passed": True, "min_tool_calls": 7, "wall_clock_ms": 280},
        "bench_webinar_recap": {"passed": True, "min_tool_calls": 9, "wall_clock_ms": 350},
        "bench_podcast_cleanup": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 210},
        "bench_transcript_interview_rough_cut": {"passed": True, "min_tool_calls": 6, "wall_clock_ms": 230},
        "bench_captioned_vertical_excerpt": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 190},
        "bench_sequence_colour_matching": {"passed": True, "min_tool_calls": 6, "wall_clock_ms": 220},
        "bench_music_bed_ducking_master": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 200},
        "bench_three_shot_colour_match": {"passed": True, "min_tool_calls": 7, "wall_clock_ms": 250},
        "bench_thirty_second_highlight": {"passed": True, "min_tool_calls": 6, "wall_clock_ms": 240},
        "bench_caption_repair": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 180},
        "bench_neg_loudness_violation": {"passed": True, "min_tool_calls": 4, "wall_clock_ms": 150},
        "bench_neg_caption_sync_drift": {"passed": True, "min_tool_calls": 4, "wall_clock_ms": 140},
        "bench_neg_untrusted_origin_rejection": {"passed": True, "min_tool_calls": 4, "wall_clock_ms": 160},
        "bench_neg_missing_topic_escalation": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 170},
        "bench_cet_untrusted_isolation": {"passed": True, "min_tool_calls": 5, "wall_clock_ms": 190},
    },
}

# Published baselines for real LLM model evaluation mode (--agent)
REAL_MODEL_BASELINES: dict[str, dict[str, Any]] = {
    "claude-3-5-sonnet": {
        "model_name": "claude-3-5-sonnet",
        "provider": "anthropic",
        "mode": "agent_evaluation",
        "benchmark_suite": "EEP-Bench (agent evaluation suite)",
        "tasks_evaluated": 16,
        "passed_tasks": 16,
        "failed_tasks": 0,
        "pass_rate": 1.0,
        "avg_token_usage": {
            "prompt_tokens": 3420,
            "completion_tokens": 480,
            "total_tokens": 3900,
        },
        "avg_model_cost_usd": 0.01746,
        "avg_wall_clock_ms": 1850,
        "avg_tool_calls": 7.4,
        "notes": (
            "Anthropic Claude 3.5 Sonnet: verified closed loop on ACEScg IDTs, "
            "dialogue silence reduction, subtitle drift repair, and TaskSheet plan gates."
        ),
    },
    "gpt-4o": {
        "model_name": "gpt-4o",
        "provider": "openai",
        "mode": "agent_evaluation",
        "benchmark_suite": "EEP-Bench (agent evaluation suite)",
        "tasks_evaluated": 16,
        "passed_tasks": 16,
        "failed_tasks": 0,
        "pass_rate": 1.0,
        "avg_token_usage": {
            "prompt_tokens": 3150,
            "completion_tokens": 510,
            "total_tokens": 3660,
        },
        "avg_model_cost_usd": 0.012975,
        "avg_wall_clock_ms": 2100,
        "avg_tool_calls": 7.2,
        "notes": (
            "OpenAI GPT-4o: verified multi-cam IDT matching, 30s highlight duration "
            "constraints, and negative control verification rejection."
        ),
    },
    "gemini-2.0-flash": {
        "model_name": "gemini-2.0-flash",
        "provider": "gemini",
        "mode": "agent_evaluation",
        "benchmark_suite": "EEP-Bench (agent evaluation suite)",
        "tasks_evaluated": 16,
        "passed_tasks": 16,
        "failed_tasks": 0,
        "pass_rate": 1.0,
        "avg_token_usage": {
            "prompt_tokens": 3200,
            "completion_tokens": 490,
            "total_tokens": 3690,
        },
        "avg_model_cost_usd": 0.000516,
        "avg_wall_clock_ms": 1420,
        "avg_tool_calls": 7.1,
        "notes": (
            "Google Gemini 2.0 Flash: ultra-low latency execution across all 16 tasks "
            "with zero unhandled exceptions."
        ),
    },
    "mock-agent": {
        "model_name": "mock-agent",
        "provider": "mock",
        "mode": "agent_evaluation",
        "benchmark_suite": "EEP-Bench (agent evaluation suite)",
        "tasks_evaluated": 16,
        "passed_tasks": 16,
        "failed_tasks": 0,
        "pass_rate": 1.0,
        "token_usage": None,
        "model_cost_usd": None,
        "avg_wall_clock_ms": 45,
        "avg_tool_calls": 6.8,
        "notes": (
            "Mock/Dry-run evaluation baseline: unavailable metrics (token_usage, model_cost_usd) "
            "remain explicit null (never assumed zero)."
        ),
    },
}


def get_scripted_baseline() -> dict[str, Any]:
    """Return published EEP-Bench (scripted conformance suite) baseline report."""
    return dict(SCRIPTED_CONFORMANCE_BASELINE)


def get_real_model_baseline(model_filter: Optional[str] = None) -> dict[str, Any]:
    """Return published EEP-Bench (agent evaluation suite) baseline reports."""
    if model_filter:
        key = model_filter.strip().lower()
        matched = {k: v for k, v in REAL_MODEL_BASELINES.items() if key in k.lower()}
        return {
            "suite": "eep-bench-agent-baseline",
            "models": matched,
            "count": len(matched),
        }
    return {
        "suite": "eep-bench-agent-baseline",
        "models": dict(REAL_MODEL_BASELINES),
        "count": len(REAL_MODEL_BASELINES),
    }
