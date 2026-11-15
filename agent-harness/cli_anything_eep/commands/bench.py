# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""CLI commands for EEP-Bench smoke tasks, --agent evaluation, and baseline reports."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Optional

import click

from cli_anything_eep.bench import (
    BENCH_TASKS,
    get_real_model_baseline,
    get_scripted_baseline,
    list_bench_tasks,
    run_bench_suite,
    run_bench_task,
)
from cli_anything_eep.core import errors
from cli_anything_eep.output import format_output


@click.group(name="bench")
def bench_group():
    """Run EEP-Bench tasks, evaluate real LLM agents, and inspect published baselines."""


@bench_group.command("list")
@click.pass_context
def bench_list(ctx: click.Context):
    """List all EEP-Bench benchmark tasks."""
    format_output(
        {
            "suite": "EEP-Bench (conformance and agent evaluation suite)",
            "total_tasks": len(BENCH_TASKS),
            "tasks": list_bench_tasks(),
        },
        ctx.obj["output_format"],
    )


@bench_group.command("baseline")
@click.option("--model", "model_filter", type=str, help="Filter published baseline by model name")
@click.pass_context
def bench_baseline(ctx: click.Context, model_filter: Optional[str]):
    """Display published baselines for scripted conformance and real LLM models."""
    scripted = get_scripted_baseline()
    agent_baseline = get_real_model_baseline(model_filter)
    format_output(
        {
            "scripted_conformance_baseline": scripted,
            "real_model_agent_baseline": agent_baseline,
        },
        ctx.obj["output_format"],
    )


@bench_group.command("run")
@click.option(
    "--task",
    "task_id",
    type=click.Choice(list(BENCH_TASKS.keys())),
    help="Run one specific EEP-Bench task",
)
@click.option("--workdir", type=click.Path(), help="Directory for task artifacts")
@click.option("--skip-audio-dsp", is_flag=True, help="Skip real audio_process normalization pass")
@click.option(
    "--agent",
    type=str,
    help="Evaluate using LLM agent (e.g. claude-3-5-sonnet, gpt-4o, gemini-2.0-flash, mock)",
)
@click.option("--endpoint", type=str, help="Custom LLM API endpoint URL")
@click.option("--api-key", type=str, help="LLM API key (or set via env var)")
@click.option("--dry-run", is_flag=True, help="Simulate agent evaluation without making external network calls")
@click.option("--max-turns", type=int, default=10, help="Maximum agent execution turns")
@click.pass_context
def bench_run(
    ctx: click.Context,
    task_id: Optional[str],
    workdir: Optional[str],
    skip_audio_dsp: bool,
    agent: Optional[str],
    endpoint: Optional[str],
    api_key: Optional[str],
    dry_run: bool,
    max_turns: int,
):
    """Execute EEP-Bench tasks (`mode: scripted_deterministic` or `--agent` evaluation)."""
    target_dir = Path(workdir) if workdir else Path(tempfile.mkdtemp(prefix="eep_bench_"))
    try:
        if task_id:
            result = run_bench_task(
                task_id,
                target_dir,
                agent=agent,
                endpoint=endpoint,
                api_key=api_key,
                dry_run=dry_run,
                max_turns=max_turns,
                run_audio_dsp=not skip_audio_dsp,
            )
            ok = bool(result.get("passed"))
        else:
            result = run_bench_suite(
                target_dir,
                agent=agent,
                endpoint=endpoint,
                api_key=api_key,
                dry_run=dry_run,
                max_turns=max_turns,
                run_audio_dsp=not skip_audio_dsp,
            )
            ok = bool(result.get("passed"))
        format_output(result, ctx.obj["output_format"])
        if not ok:
            ctx.exit(1)
    except (errors.EepError, RuntimeError, KeyError) as exc:
        msg = (
            exc.to_dict()
            if isinstance(exc, errors.EepError)
            else {"error": "BENCH_FAILED", "message": str(exc)}
        )
        format_output(msg, ctx.obj["output_format"])
        ctx.exit(1)
