# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""CLI commands for EEP-Bench (scripted conformance suite) smoke tasks and telemetry."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Optional

import click

from cli_anything_eep.bench import BENCH_TASKS, list_bench_tasks, run_bench_suite, run_bench_task
from cli_anything_eep.core import errors
from cli_anything_eep.output import format_output


@click.group(name="bench")
def bench_group():
    """Run EEP-Bench (scripted conformance suite) verification tasks and report honest telemetry."""


@bench_group.command("list")
@click.pass_context
def bench_list(ctx: click.Context):
    """List EEP-Bench (scripted conformance suite) tasks."""
    format_output(
        {"suite": "EEP-Bench (scripted conformance suite)", "tasks": list_bench_tasks()},
        ctx.obj["output_format"],
    )


@bench_group.command("run")
@click.option("--task", "task_id", type=click.Choice(list(BENCH_TASKS.keys())), help="Run one specific EEP-Bench (scripted conformance suite) task")
@click.option("--workdir", type=click.Path(), help="Directory for task artifacts")
@click.option("--skip-audio-dsp", is_flag=True, help="Skip real audio_process normalization pass")
@click.pass_context
def bench_run(
    ctx: click.Context,
    task_id: Optional[str],
    workdir: Optional[str],
    skip_audio_dsp: bool,
):
    """Execute EEP-Bench (scripted conformance suite) tasks (`mode: scripted_deterministic`)."""
    target_dir = Path(workdir) if workdir else Path(tempfile.mkdtemp(prefix="eep_bench_"))
    try:
        if task_id:
            result = run_bench_task(task_id, target_dir, run_audio_dsp=not skip_audio_dsp)
            ok = bool(result.get("passed"))
        else:
            result = run_bench_suite(target_dir, run_audio_dsp=not skip_audio_dsp)
            ok = bool(result.get("passed"))
        format_output(result, ctx.obj["output_format"])
        if not ok:
            ctx.exit(1)
    except (errors.EepError, RuntimeError, KeyError) as exc:
        msg = exc.to_dict() if isinstance(exc, errors.EepError) else {"error": "BENCH_FAILED", "message": str(exc)}
        format_output(msg, ctx.obj["output_format"])
        ctx.exit(1)
