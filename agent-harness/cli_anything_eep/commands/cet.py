# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Capture Event Track (CET v0.1) CLI wrapper commands (`cet-gen`, `cet-validate`, `cet-propose`, `cet-query`)."""

from __future__ import annotations

from typing import Optional

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output


def _run_cet_subcommand(ctx: click.Context, subcommand: str, args: list[str]) -> None:
    try:
        result = backend.get_saturn_cli().run_cli_json([subcommand, *args])
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@click.group(name="cet")
def cet_group():
    """Capture Event Track (CET v0.1) synthetic generation, validation, and analysis."""


@cet_group.command("generate")
@click.option("--output", "-o", type=click.Path(), help="Destination .cet.jsonl path")
@click.option(
    "--scenario",
    type=click.Choice(["tutorial_basic", "multi_window", "redacted_privacy", "named_shortcuts"]),
    default="tutorial_basic",
    show_default=True,
)
@click.option("--video", default="desktop_capture.mkv", show_default=True)
@click.option("--fps", type=float, default=30.0, show_default=True)
@click.option("--width", type=int, default=1920, show_default=True)
@click.option("--height", type=int, default=1080, show_default=True)
@click.option("--duration-ms", type=int, default=24000, show_default=True)
@click.option("--seed", type=int, default=1, show_default=True)
@click.option("--named-shortcuts/--classes-only", default=False, show_default=True)
@click.pass_context
def cet_generate(
    ctx: click.Context,
    output: Optional[str],
    scenario: str,
    video: str,
    fps: float,
    width: int,
    height: int,
    duration_ms: int,
    seed: int,
    named_shortcuts: bool,
):
    """Generate a deterministic synthetic CET v0.1 JSONL track."""
    args = [
        "--scenario", scenario,
        "--video", video,
        "--fps", str(fps),
        "--width", str(width),
        "--height", str(height),
        "--duration-ms", str(duration_ms),
        "--seed", str(seed),
    ]
    if named_shortcuts:
        args.append("--named-shortcuts")
    if output:
        args.insert(0, output)
    else:
        args.append("--json")
    _run_cet_subcommand(ctx, "cet-gen", args)


@cet_group.command("validate")
@click.argument("path", type=click.Path(exists=True))
@click.option("--max-bytes", type=int, help="Override size cap in bytes")
@click.pass_context
def cet_validate(ctx: click.Context, path: str, max_bytes: Optional[int]):
    """Validate a `.cet.jsonl` sidecar track against CET v0.1 rules."""
    args = [path]
    if max_bytes is not None:
        args.extend(["--max-bytes", str(max_bytes)])
    _run_cet_subcommand(ctx, "cet-validate", args)


@cet_group.command("propose")
@click.argument("path", type=click.Path(exists=True))
@click.option("--scale", type=float, default=1.4, show_default=True)
@click.option("--dead-air-ms", type=int, default=1200, show_default=True)
@click.option("--dwell-ms", type=int, default=250, show_default=True)
@click.option("--hold-ms", type=int, default=2200, show_default=True)
@click.pass_context
def cet_propose(
    ctx: click.Context,
    path: str,
    scale: float,
    dead_air_ms: int,
    dwell_ms: int,
    hold_ms: int,
):
    """Derive click zooms, chapters, and dead-air idle intervals from a CET track."""
    _run_cet_subcommand(
        ctx,
        "cet-propose",
        [
            path,
            "--scale", str(scale),
            "--dead-air-ms", str(dead_air_ms),
            "--dwell-ms", str(dwell_ms),
            "--hold-ms", str(hold_ms),
        ],
    )


@cet_group.command("query")
@click.argument("path", type=click.Path(exists=True))
@click.option("--from-ms", type=int)
@click.option("--to-ms", type=int)
@click.option("--types", help="Comma-separated event types")
@click.option("--window", help="Case-insensitive window/app filter")
@click.option("--limit", type=int, default=100, show_default=True)
@click.pass_context
def cet_query(
    ctx: click.Context,
    path: str,
    from_ms: Optional[int],
    to_ms: Optional[int],
    types: Optional[str],
    window: Optional[str],
    limit: int,
):
    """Query bounded CET events with AF5 untrusted field metadata."""
    args = [path, "--limit", str(limit)]
    if from_ms is not None:
        args.extend(["--from-ms", str(from_ms)])
    if to_ms is not None:
        args.extend(["--to-ms", str(to_ms)])
    if types:
        args.extend(["--types", types])
    if window:
        args.extend(["--window", window])
    _run_cet_subcommand(ctx, "cet-query", args)
