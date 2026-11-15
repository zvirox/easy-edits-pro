# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Tests for EEP-Bench expanded 16-task suite, --agent evaluation mode, and baseline publishing."""

from __future__ import annotations

import json
from pathlib import Path

from click.testing import CliRunner

from cli_anything_eep.bench import (
    BENCH_TASKS,
    REAL_MODEL_BASELINES,
    SCRIPTED_CONFORMANCE_BASELINE,
    get_real_model_baseline,
    get_scripted_baseline,
    list_bench_tasks,
    run_bench_suite,
    run_bench_task,
)
from cli_anything_eep.bench.agent_eval import (
    calculate_cost_usd,
    resolve_agent_config,
)
from cli_anything_eep.bench.negative_controls import (
    eval_neg_caption_sync_drift,
    eval_neg_loudness_violation,
    eval_neg_missing_topic_escalation,
    eval_neg_untrusted_origin_rejection,
    exec_neg_caption_sync_drift,
    exec_neg_loudness_violation,
    exec_neg_missing_topic_escalation,
    exec_neg_untrusted_origin_rejection,
)
from cli_anything_eep.bench.specific_tasks import (
    eval_caption_repair,
    eval_thirty_second_highlight,
    eval_three_shot_colour_match,
    exec_caption_repair,
    exec_thirty_second_highlight,
    exec_three_shot_colour_match,
)
from cli_anything_eep.cli import cli


def test_bench_catalog_contains_all_16_tasks():
    tasks = list_bench_tasks()
    assert len(tasks) == 16, f"Expected 16 tasks, found {len(tasks)}"
    task_ids = {t["id"] for t in tasks}

    # 3 Flagship recipes
    flagship_expected = {
        "bench_podcast_to_clips",
        "bench_talking_head_cet",
        "bench_webinar_recap",
    }
    assert flagship_expected.issubset(task_ids)

    # 5 Companion §2.6 recipes
    companion_expected = {
        "bench_podcast_cleanup",
        "bench_transcript_interview_rough_cut",
        "bench_captioned_vertical_excerpt",
        "bench_sequence_colour_matching",
        "bench_music_bed_ducking_master",
    }
    assert companion_expected.issubset(task_ids)

    # 3 Specific tasks
    specific_expected = {
        "bench_three_shot_colour_match",
        "bench_thirty_second_highlight",
        "bench_caption_repair",
    }
    assert specific_expected.issubset(task_ids)

    # Negative controls & security gates
    negative_expected = {
        "bench_neg_loudness_violation",
        "bench_neg_caption_sync_drift",
        "bench_neg_untrusted_origin_rejection",
        "bench_neg_missing_topic_escalation",
        "bench_cet_untrusted_isolation",
    }
    assert negative_expected.issubset(task_ids)


def test_cli_bench_list_and_baseline_commands():
    runner = CliRunner()

    res_list = runner.invoke(cli, ["--json", "bench", "list"])
    assert res_list.exit_code == 0, res_list.output
    data_list = json.loads(res_list.output)
    assert data_list["total_tasks"] == 16
    assert len(data_list["tasks"]) == 16

    res_base = runner.invoke(cli, ["--json", "bench", "baseline"])
    assert res_base.exit_code == 0, res_base.output
    data_base = json.loads(res_base.output)
    assert data_base["scripted_conformance_baseline"]["total_tasks"] == 16
    assert "claude-3-5-sonnet" in data_base["real_model_agent_baseline"]["models"]
    assert "gpt-4o" in data_base["real_model_agent_baseline"]["models"]
    assert "gemini-2.0-flash" in data_base["real_model_agent_baseline"]["models"]

    # Test baseline filtering by model
    res_filter = runner.invoke(cli, ["--json", "bench", "baseline", "--model", "claude"])
    assert res_filter.exit_code == 0, res_filter.output
    data_filter = json.loads(res_filter.output)
    assert data_filter["real_model_agent_baseline"]["count"] == 1
    assert "claude-3-5-sonnet" in data_filter["real_model_agent_baseline"]["models"]


def test_specific_task_executors_and_evaluators(tmp_path: Path):
    # Specific task 1: 3-shot colour match
    out_col = exec_three_shot_colour_match(tmp_path / "col")
    checks_col = eval_three_shot_colour_match(out_col)
    assert all(c["passed"] for c in checks_col)
    assert out_col["clip_count"] == 3

    # Specific task 2: 30s highlight
    out_hl = exec_thirty_second_highlight(tmp_path / "hl")
    checks_hl = eval_thirty_second_highlight(out_hl)
    assert all(c["passed"] for c in checks_hl)
    assert abs(out_hl["total_duration_s"] - 30.0) <= 0.5

    # Specific task 3: Caption repair
    out_cap = exec_caption_repair(tmp_path / "cap")
    checks_cap = eval_caption_repair(out_cap)
    assert all(c["passed"] for c in checks_cap)
    assert out_cap["detected_drift_count"] >= 2
    assert out_cap["max_residual_drift_ms"] < 50.0


def test_negative_controls_executors_and_evaluators(tmp_path: Path):
    # Negative control 1: Loudness violation
    out_loud = exec_neg_loudness_violation(tmp_path / "loud")
    checks_loud = eval_neg_loudness_violation(out_loud)
    assert all(c["passed"] for c in checks_loud)
    assert out_loud["violation_detected"] is True
    assert out_loud["delivery_blocked"] is True

    # Negative control 2: Caption sync drift
    out_drift = exec_neg_caption_sync_drift(tmp_path / "drift")
    checks_drift = eval_neg_caption_sync_drift(out_drift)
    assert all(c["passed"] for c in checks_drift)
    assert out_drift["sync_drift_detected"] is True
    assert out_drift["delivery_blocked"] is True

    # Negative control 3: Untrusted origin rejection
    out_orig = exec_neg_untrusted_origin_rejection(tmp_path / "orig")
    checks_orig = eval_neg_untrusted_origin_rejection(out_orig)
    assert all(c["passed"] for c in checks_orig)
    assert out_orig["traversal_denied"] is True

    # Negative control 4: Missing topic escalation
    out_esc = exec_neg_missing_topic_escalation(tmp_path / "esc")
    checks_esc = eval_neg_missing_topic_escalation(out_esc)
    assert all(c["passed"] for c in checks_esc)
    assert out_esc["sheet_status"] == "needs_input"
    assert out_esc["agent_self_answer_denied"] is True


def test_scripted_suite_telemetry_honesty(tmp_path: Path):
    # Run a selection of fast tasks in scripted mode
    selected = [
        "bench_three_shot_colour_match",
        "bench_thirty_second_highlight",
        "bench_caption_repair",
        "bench_neg_loudness_violation",
        "bench_neg_caption_sync_drift",
    ]
    report = run_bench_suite(tmp_path / "smoke", task_ids=selected)
    assert report["passed"] is True
    assert report["mode"] == "scripted_deterministic"
    assert report["suite_label"] == "EEP-Bench (scripted conformance suite)"
    # Honest null reporting: never invented or assumed zero
    assert report["token_usage"] is None
    assert report["model_cost_usd"] is None
    assert report["aggregate_quality_score"] is None
    for task in report["tasks"]:
        assert task["token_usage"] is None
        assert task["model_cost_usd"] is None
        assert task["passed"] is True


def test_agent_evaluation_mock_mode(tmp_path: Path):
    res = run_bench_task(
        "bench_caption_repair",
        tmp_path / "agent_mock",
        agent="mock",
    )
    assert res["passed"] is True
    assert res["mode"] == "agent_evaluation"
    assert res["agent"] == "mock"
    assert res["provider"] == "mock"
    assert res["model_name"] == "mock-agent"
    assert res["status"] == "mock_evaluation"
    # In mock mode, unavailable LLM metrics remain explicit null
    assert res["token_usage"] is None
    assert res["model_cost_usd"] is None
    assert res["tool_calls"] >= 5
    assert res["wall_clock_ms"] >= 0


def test_agent_evaluation_dry_run_uncredentialed(tmp_path: Path):
    res = run_bench_task(
        "bench_thirty_second_highlight",
        tmp_path / "agent_dry",
        agent="anthropic/claude-3-5-sonnet",
        dry_run=True,
    )
    assert res["passed"] is True
    assert res["mode"] == "agent_evaluation"
    assert res["provider"] == "anthropic"
    assert res["model_name"] == "claude-3-5-sonnet"
    assert res["status"] == "dry_run"
    assert res["token_usage"] is None
    assert res["model_cost_usd"] is None


def test_pricing_and_agent_config_resolution():
    # Known model costs
    cost_sonnet = calculate_cost_usd("claude-3-5-sonnet", 10_000, 2_000)
    # (10k * $3.00 + 2k * $15.00) / 1M = (30 + 30) / 1000 = $0.060000
    assert cost_sonnet == 0.06

    cost_gpt4o = calculate_cost_usd("gpt-4o", 10_000, 2_000)
    # (10k * $2.50 + 2k * $10.00) / 1M = (25 + 20) / 1000 = $0.045000
    assert cost_gpt4o == 0.045

    cost_gemini = calculate_cost_usd("gemini-2.0-flash", 100_000, 20_000)
    # (100k * $0.10 + 20k * $0.40) / 1M = (10 + 8) / 1000 = $0.018000
    assert cost_gemini == 0.018

    # Unknown model -> cost must be explicitly None (never assumed zero)
    assert calculate_cost_usd("unknown-custom-model", 5000, 1000) is None

    # Config resolution
    cfg1 = resolve_agent_config("mock")
    assert cfg1.provider == "mock"
    assert cfg1.dry_run is True

    cfg2 = resolve_agent_config("openai/gpt-4o", api_key="test-key", endpoint="https://custom.endpoint/v1")
    assert cfg2.provider == "openai"
    assert cfg2.model_name == "gpt-4o"
    assert cfg2.api_key == "test-key"
    assert cfg2.endpoint == "https://custom.endpoint/v1"
    assert cfg2.dry_run is False


def test_cli_bench_run_agent_mode(tmp_path: Path):
    runner = CliRunner()
    res = runner.invoke(
        cli,
        [
            "--json",
            "bench",
            "run",
            "--task",
            "bench_caption_repair",
            "--agent",
            "mock",
            "--workdir",
            str(tmp_path / "cli_agent"),
        ],
    )
    assert res.exit_code == 0, res.output
    data = json.loads(res.output)
    assert data["mode"] == "agent_evaluation"
    assert data["passed"] is True
    assert data["agent"] == "mock"
    assert data["token_usage"] is None
    assert data["model_cost_usd"] is None
