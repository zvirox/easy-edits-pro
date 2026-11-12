# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""CLI commands for running Easy Edits Pro flagship Task Sheet recipes."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Optional

import click

from cli_anything_eep.core import errors
from cli_anything_eep.output import format_output
from cli_anything_eep.recipes import RECIPES, list_recipes, run_recipe


@click.group(name="recipe")
def recipe_group():
    """Run flagship Task Sheet editing recipes against Rust CLI and MCP backends."""


@recipe_group.command("list")
@click.pass_context
def recipe_list(ctx: click.Context):
    """List available flagship recipes and their delivery presets."""
    format_output({"recipes": list_recipes()}, ctx.obj["output_format"])


@recipe_group.command("run")
@click.argument("recipe_id", type=click.Choice(list(RECIPES.keys())))
@click.option("--workdir", type=click.Path(), help="Working directory for project/media/package outputs")
@click.option("--sheet", "sheet_path", type=click.Path(exists=True), help="Optional custom Task Sheet JSON path")
@click.option("--cet", "cet_path", type=click.Path(exists=True), help="Optional CET v0.1 sidecar (.cet.jsonl) for tutorial recipe")
@click.option("--approve-review/--no-approve-review", default=False, show_default=True)
@click.option("--package-delivery/--no-package-delivery", default=False, show_default=True)
@click.option("--skip-audio-dsp", is_flag=True, help="Skip real audio_process normalization pass")
@click.pass_context
def recipe_run(
    ctx: click.Context,
    recipe_id: str,
    workdir: Optional[str],
    sheet_path: Optional[str],
    cet_path: Optional[str],
    approve_review: bool,
    package_delivery: bool,
    skip_audio_dsp: bool,
):
    """Execute a flagship recipe cleanly against compiled Rust binaries."""
    target_dir = Path(workdir) if workdir else Path(tempfile.mkdtemp(prefix=f"eep_{recipe_id}_"))
    kwargs: dict[str, object] = {
        "sheet_path": sheet_path,
        "approve_review": approve_review,
        "package_delivery": package_delivery,
        "run_audio_dsp": not skip_audio_dsp,
    }
    if cet_path and recipe_id == "talking-head-tutorial":
        kwargs["cet_path"] = cet_path
    try:
        result = run_recipe(recipe_id, target_dir, **kwargs)
        format_output(result, ctx.obj["output_format"])
    except (errors.EepError, RuntimeError, KeyError) as exc:
        msg = exc.to_dict() if isinstance(exc, errors.EepError) else {"error": "RECIPE_FAILED", "message": str(exc)}
        format_output(msg, ctx.obj["output_format"])
        ctx.exit(1)
