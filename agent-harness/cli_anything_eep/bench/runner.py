# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""EEP-Bench runner supporting deterministic scripted suite and real LLM --agent evaluation mode."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable, Iterable, Optional

from cli_anything_eep.bench.agent_eval import (
    AgentConfig,
    execute_agent_eval_task,
    resolve_agent_config,
)
from cli_anything_eep.bench.tasks import BENCH_TASKS


def run_bench_task(
    task_id: str,
    workdir: str | Path,
    *,
    agent: Optional[str] = None,
    endpoint: Optional[str] = None,
    api_key: Optional[str] = None,
    dry_run: bool = False,
    max_turns: int = 10,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute one EEP-Bench task under scripted conformance or real LLM --agent mode."""
    spec = BENCH_TASKS.get(task_id)
    if spec is None:
        raise KeyError(f"Unknown EEP-Bench task `{task_id}`; available: {list(BENCH_TASKS)}")

    task_dir = Path(workdir) / task_id
    task_dir.mkdir(parents=True, exist_ok=True)

    agent_cfg = resolve_agent_config(
        agent,
        endpoint=endpoint,
        api_key=api_key,
        dry_run=dry_run,
        max_turns=max_turns,
    )

    if agent_cfg is not None:
        return execute_agent_eval_task(
            spec,
            task_dir,
            agent_cfg,
            run_audio_dsp=run_audio_dsp,
            cli_path=cli_path,
            mcp_path=mcp_path,
        )

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
    agent: Optional[str] = None,
    endpoint: Optional[str] = None,
    api_key: Optional[str] = None,
    dry_run: bool = False,
    max_turns: int = 10,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Run all (or selected) EEP-Bench tasks with honest scripted or LLM agent telemetry."""
    selected = list(task_ids) if task_ids is not None else list(BENCH_TASKS.keys())
    agent_cfg = resolve_agent_config(
        agent,
        endpoint=endpoint,
        api_key=api_key,
        dry_run=dry_run,
        max_turns=max_turns,
    )

    t0 = time.monotonic()
    results = [
        run_bench_task(
            tid,
            workdir,
            agent=agent,
            endpoint=endpoint,
            api_key=api_key,
            dry_run=dry_run,
            max_turns=max_turns,
            run_audio_dsp=run_audio_dsp,
            cli_path=cli_path,
            mcp_path=mcp_path,
        )
        for tid in selected
    ]
    total_ms = int((time.monotonic() - t0) * 1000)
    passed_count = sum(1 for r in results if r["passed"])

    if agent_cfg is not None:
        has_tokens = any(r.get("token_usage") is not None for r in results)
        if has_tokens:
            prompt_sum = sum(int(r["token_usage"]["prompt_tokens"]) for r in results if r.get("token_usage"))
            comp_sum = sum(int(r["token_usage"]["completion_tokens"]) for r in results if r.get("token_usage"))
            suite_tokens: Optional[dict[str, int]] = {
                "prompt_tokens": prompt_sum,
                "completion_tokens": comp_sum,
                "total_tokens": prompt_sum + comp_sum,
            }
            costs = [r["model_cost_usd"] for r in results if r.get("model_cost_usd") is not None]
            suite_cost: Optional[float] = round(sum(costs), 6) if costs else None
        else:
            suite_tokens = None
            suite_cost = None

        status = "live_llm_evaluation" if not agent_cfg.dry_run and agent_cfg.api_key else ("mock_evaluation" if agent_cfg.provider == "mock" else "dry_run")

        return {
            "suite": "eep-bench-agent",
            "suite_label": "EEP-Bench (agent evaluation suite)",
            "mode": "agent_evaluation",
            "agent": agent_cfg.agent_id,
            "model_name": agent_cfg.model_name,
            "provider": agent_cfg.provider,
            "status": status,
            "note": (
                "EEP-Bench (agent evaluation suite) under real LLM --agent execution mode; "
                "unavailable metrics remain explicit null (never assumed zero)."
            ),
            "total_tasks": len(results),
            "passed_tasks": passed_count,
            "failed_tasks": len(results) - passed_count,
            "passed": passed_count == len(results),
            "total_wall_clock_ms": total_ms,
            "total_tool_calls": sum(int(r.get("tool_calls", 0)) for r in results),
            "token_usage": suite_tokens,
            "model_cost_usd": suite_cost,
            "aggregate_quality_score": None,
            "tasks": results,
        }

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
