# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""End-to-end CI smoke tests for TS6 flagship recipes, synthetic CET, and EEP-Bench."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

from cli_anything_eep.core.runtime import find_binary
from cli_anything_eep.recipes import (
    run_podcast_to_clips,
    run_talking_head_tutorial,
    run_webinar_recap,
)
from cli_anything_eep.recipes._common import run_cli_subcommand


def _agent_harness_root() -> str:
    return str(Path(__file__).resolve().parents[1])


def test_cet_cli_subcommands_generate_validate_propose_query(tmp_path: Path):
    cet_file = tmp_path / "demo.cet.jsonl"
    gen = run_cli_subcommand([
        "cet-gen",
        str(cet_file),
        "--scenario",
        "multi_window",
        "--duration-ms",
        "18000",
        "--seed",
        "5",
    ])
    assert gen["generated"] is True
    assert gen["valid"] is True
    assert gen["privacy_keys"] == "classes_only"
    assert gen["privacy_text"] in ("off", "redacted")

    # Also test standalone `cet-gen` binary if built in target dir.
    standalone_cet_gen = find_binary("cet-gen")
    if standalone_cet_gen:
        out2 = tmp_path / "standalone.cet.jsonl"
        proc = subprocess.run(
            [standalone_cet_gen, str(out2), "--scenario", "named_shortcuts", "--named-shortcuts"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert proc.returncode == 0, proc.stderr
        parsed = json.loads(proc.stdout)
        assert parsed["privacy_keys"] == "named_shortcuts"

    val = run_cli_subcommand(["cet-validate", str(cet_file)])
    assert val["valid"] is True
    assert val["errors"] == []

    props = run_cli_subcommand(["cet-propose", str(cet_file), "--scale", "1.45"])
    assert props["zoom_proposals"]
    assert all(z["click_in_crop"] is True for z in props["zoom_proposals"])
    assert props["chapter_proposals"]
    assert all(ch["untrusted"] is True for ch in props["chapter_proposals"])
    assert "AF5_UNTRUSTED_CET" in props["untrusted_notice"]

    qry = run_cli_subcommand(["cet-query", str(cet_file), "--types", "button,window", "--limit", "15"])
    assert "AF5_UNTRUSTED_CET" in qry["untrusted_notice"]
    assert qry["returned_events"] <= 15
    assert qry["matched_events"] >= 1


def test_recipe1_podcast_to_clips_end_to_end(tmp_path: Path):
    out = run_podcast_to_clips(
        tmp_path / "r1",
        approve_review=True,
        package_delivery=True,
    )
    assert out["recipe"] == "podcast-to-clips"
    assert out["word_boundary_all_aligned"] is True
    assert out["rejected_must_not_segments"] >= 1
    assert out["frame_fit"]["mode"] == "fill"
    assert out["frame_fit"]["scale"] > 1.0
    assert out["plan_gate_denied_to_agent"] is True
    assert out["verify"]["summary"]["failed"] == 0
    assert out["verify"]["notes"]["aggregate_score"] is None
    criteria = {f["criterion"] for f in out["selected_candidate"]["review_flags"]}
    assert {"hook", "tone", "ending"} <= criteria
    assert out["review"]["agent_close_denied"] is True
    assert out["review"]["delivery_verified"]["verified"] is True


def test_recipe2_talking_head_tutorial_cet_end_to_end(tmp_path: Path):
    out = run_talking_head_tutorial(
        tmp_path / "r2",
        scenario="tutorial_basic",
        approve_review=True,
        package_delivery=True,
    )
    assert out["recipe"] == "talking-head-tutorial"
    assert out["cet_validation"]["valid"] is True
    assert out["cet_validation"]["errors"] == []
    assert out["applied_zooms"] >= 1
    assert out["all_zoom_clicks_in_crop"] is True
    assert out["af5_untrusted_isolation_verified"] is True
    assert out["silence_cleanup"]["deleted_ranges_count"] >= 1
    assert out["filler_cleanup"]["deleted_ranges_count"] >= 1
    assert out["plan_gate_denied_to_agent"] is True
    assert out["verify"]["summary"]["failed"] == 0
    assert out["review"]["delivery_verified"]["verified"] is True


def test_recipe3_webinar_recap_end_to_end(tmp_path: Path):
    out = run_webinar_recap(
        tmp_path / "r3",
        simulate_missing_topic=True,
        approve_review=True,
        package_delivery=True,
    )
    assert out["recipe"] == "webinar-recap"
    assert out["escalation"] is not None
    assert out["escalation"]["agent_answer_denied"] is True
    assert out["escalation"]["asked"]["status"] == "needs_input"
    assert out["escalation"]["answered"]["sheet_status"] == "running"
    assert len(out["analysis"]["flagged_must_not"]) >= 1
    assert out["reel_excludes_all_must_not"] is True
    assert len(out["reel_included_segment_ids"]) >= 1
    assert out["verify_full"]["summary"]["failed"] == 0
    assert out["verify_reel"]["summary"]["failed"] == 0
    assert out["review"]["delivery_verified"]["verified"] is True


def test_eep_bench_suite_subprocess_smoke(tmp_path: Path):
    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    harness_root = _agent_harness_root()
    env["PYTHONPATH"] = f"{harness_root}:{existing}" if existing else harness_root

    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "cli_anything_eep",
            "--json",
            "bench",
            "run",
            "--workdir",
            str(tmp_path / "bench_work"),
        ],
        capture_output=True,
        text=True,
        env=env,
        timeout=240,
    )
    assert proc.returncode == 0, f"stderr:\n{proc.stderr}\nstdout:\n{proc.stdout}"
    report = json.loads(proc.stdout)
    assert report["suite"] == "eep-bench-smoke"
    assert report["mode"] == "scripted_deterministic"
    assert report["passed"] is True
    assert report["total_tasks"] == 4
    assert report["passed_tasks"] == 4
    assert report["failed_tasks"] == 0
    assert report["token_usage"] is None
    assert report["model_cost_usd"] is None
    assert report["aggregate_quality_score"] is None
    assert report["total_tool_calls"] > 0
