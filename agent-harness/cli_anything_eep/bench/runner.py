# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""EEP-Bench (scripted conformance suite) runner with honest scripted telemetry (null LLM cost/tokens)."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable, Iterable, Optional

from cli_anything_eep.bench.tasks import BENCH_TASKS


def run_bench_task(
    task_id: str,
    workdir: str | Path,
    *,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute one EEP-Bench (scripted conformance suite) task and evaluate its verifiable invariants."""
    spec = BENCH_TASKS.get(task_id)
    if spec is None:
        raise KeyError(f"Unknown EEP-Bench (scripted conformance suite) task `{task_id}`; available: {list(BENCH_TASKS)}")

    task_dir = Path(workdir) / task_id
    task_dir.mkdir(parents=True, exist_ok=True)
    executor: Callable[..., dict[str, Any]] = spec["executor"]
    evaluator: Callable[[dict[str, Any]], list[dict[str, Any]]] = spec["evaluator"]

    t0 = time.monotonic()
    outcome = executor(
        task_dir,
        run_audio_dsp=run_audio_dsp,
        cli_path=cli_path,
        mcp_path=mcp_path,
    )
    elapsed_ms = int((time.monotonic() - t0) * 1000)
    checks = evaluator(outcome)
    verify = outcome.get("verify") or outcome.get("verify_reel") or {}

    return {
        "task_id": task_id,
        "recipe": spec["recipe"],
        "mode": "scripted_deterministic",
        "passed": all(bool(c.get("passed")) for c in checks),
        "checks": checks,
        "wall_clock_ms": outcome.get("wall_clock_ms", elapsed_ms),
        "tool_calls": outcome.get("tool_calls", 0),
        "verify_summary": verify.get("summary") if isinstance(verify, dict) else None,
        "token_usage": None,
        "model_cost_usd": None,
        "aggregate_quality_score": None,
        "project_path": outcome.get("project_path"),
    }


def run_bench_suite(
    workdir: str | Path,
    *,
    task_ids: Optional[Iterable[str]] = None,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Run all (or selected) EEP-Bench (scripted conformance suite) tasks and aggregate verifiable telemetry."""
    selected = list(task_ids) if task_ids is not None else list(BENCH_TASKS.keys())
    t0 = time.monotonic()
    results = [
        run_bench_task(
            tid,
            workdir,
            run_audio_dsp=run_audio_dsp,
            cli_path=cli_path,
            mcp_path=mcp_path,
        )
        for tid in selected
    ]
    total_ms = int((time.monotonic() - t0) * 1000)
    passed_count = sum(1 for r in results if r["passed"])

    return {
        "suite": "eep-bench-smoke",
        "suite_label": "EEP-Bench (scripted conformance suite)",
        "mode": "scripted_deterministic",
        "note": (
            "EEP-Bench (scripted conformance suite) against compiled Rust core/CLI/MCP binaries; "
            "token_usage, model_cost_usd, and aggregate_quality_score are explicitly null."
        ),
        "total_tasks": len(results),
        "passed_tasks": passed_count,
        "failed_tasks": len(results) - passed_count,
        "passed": passed_count == len(results),
        "total_wall_clock_ms": total_ms,
        "total_tool_calls": sum(int(r.get("tool_calls", 0)) for r in results),
        "token_usage": None,
        "model_cost_usd": None,
        "aggregate_quality_score": None,
        "tasks": results,
    }
