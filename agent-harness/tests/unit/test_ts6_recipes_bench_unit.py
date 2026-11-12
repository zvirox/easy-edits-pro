# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Unit tests for TS6 recipe catalog, WAV/peak synthesis, CLI group wiring, and honest telemetry."""

from __future__ import annotations

import json
import wave
from pathlib import Path

from click.testing import CliRunner

from cli_anything_eep.bench import BENCH_TASKS, list_bench_tasks, run_bench_suite
from cli_anything_eep.cli import cli
from cli_anything_eep.recipes import RECIPES, list_recipes
from cli_anything_eep.recipes._common import (
    TICKS_PER_SECOND,
    build_peaks_from_segments,
    default_podcast_segments,
    load_template_sheet,
    ms_to_ticks,
    sec_to_ticks,
    synthesize_speech_wav,
)


def test_tick_conversion_and_templates():
    assert TICKS_PER_SECOND == 254_016_000_000
    assert sec_to_ticks(1.0) == TICKS_PER_SECOND
    assert ms_to_ticks(500) == TICKS_PER_SECOND // 2

    for name in ("podcast-social-clips", "talking-head-tutorial", "webinar-recap"):
        sheet = load_template_sheet(name)
        assert sheet["id"]
        assert sheet["deliverables"]


def test_recipe_and_bench_registries():
    recipe_ids = [r["id"] for r in list_recipes()]
    assert recipe_ids == ["podcast-to-clips", "talking-head-tutorial", "webinar-recap"]
    assert set(RECIPES.keys()) == set(recipe_ids)

    bench_ids = [b["id"] for b in list_bench_tasks()]
    assert set(bench_ids) == {
        "bench_podcast_to_clips",
        "bench_talking_head_cet",
        "bench_webinar_recap",
        "bench_cet_untrusted_isolation",
    }
    assert set(BENCH_TASKS.keys()) == set(bench_ids)


def test_wav_and_peak_synthesis(tmp_path: Path):
    segs = default_podcast_segments()
    wav_path = synthesize_speech_wav(tmp_path / "test.wav", segs, 6.0)
    assert wav_path.exists()
    with wave.open(str(wav_path), "rb") as wf:
        assert wf.getnchannels() == 2
        assert wf.getframerate() == 48_000
        assert wf.getnframes() == 48_000 * 6

    peaks = build_peaks_from_segments(segs, 6.0, peaks_per_sec=10)
    assert len(peaks) == 60
    assert max(peaks) >= 50
    assert min(peaks) <= 5


def test_cli_groups_registered_and_honest_list_output():
    for group_name in ("tasksheet", "cet", "recipe", "bench"):
        assert group_name in cli.commands

    runner = CliRunner()
    res_recipe = runner.invoke(cli, ["--json", "recipe", "list"])
    assert res_recipe.exit_code == 0, res_recipe.output
    data_recipe = json.loads(res_recipe.output)
    assert len(data_recipe["recipes"]) == 3

    res_bench = runner.invoke(cli, ["--json", "bench", "list"])
    assert res_bench.exit_code == 0, res_bench.output
    data_bench = json.loads(res_bench.output)
    assert len(data_bench["tasks"]) == 4


def test_bench_runner_telemetry_never_invents_tokens_or_quality(monkeypatch, tmp_path: Path):
    fake_outcome = {
        "recipe": "cet-untrusted-isolation",
        "cet_validation": {"privacy_keys": "classes_only", "privacy_text": "redacted"},
        "cet_query": {"untrusted_notice": "AF5_UNTRUSTED_CET notice"},
        "mcp_untrusted_notice": "AF5_UNTRUSTED_CET notice",
        "traversal_denied": True,
        "af5_tasksheet_denied": True,
        "tool_calls": 4,
        "wall_clock_ms": 17,
    }
    monkeypatch.setitem(
        BENCH_TASKS["bench_cet_untrusted_isolation"],
        "executor",
        lambda *_a, **_kw: fake_outcome,
    )
    report = run_bench_suite(tmp_path, task_ids=["bench_cet_untrusted_isolation"], run_audio_dsp=False)
    assert report["passed"] is True
    assert report["mode"] == "scripted_deterministic"
    assert report["token_usage"] is None
    assert report["model_cost_usd"] is None
    assert report["aggregate_quality_score"] is None
    assert report["tasks"][0]["token_usage"] is None
    assert report["tasks"][0]["model_cost_usd"] is None
    assert report["tasks"][0]["aggregate_quality_score"] is None
