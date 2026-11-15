# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Timeline branch management commands for Easy Edits Pro."""

from __future__ import annotations

from typing import Optional

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name="branch")
def branch_group():
    """Manage isolated timeline branches and review diffs before merge."""


@branch_group.command("create")
@click.option("--name", "-n", required=True, help="New branch name")
@click.option("--note", default="", help="Optional note describing the branch purpose")
@click.pass_context
def branch_create(ctx: click.Context, name: str, note: str):
    """Create a new timeline branch without disturbing the live state."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute("project.branch_create", {"name": name, "note": note})
        format_success(f"Branch '{name}' created", result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@branch_group.command("list")
@click.pass_context
def branch_list(ctx: click.Context):
    """List timeline branches and report the currently active branch."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute("project.branch_list", {}, record_history=False)
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@branch_group.command("switch")
@click.option("--name", "-n", required=True, help="Branch name to switch to")
@click.pass_context
def branch_switch(ctx: click.Context, name: str):
    """Switch active timeline branch, parking current modifications."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute("project.branch_switch", {"name": name})
        format_success(f"Switched to branch '{name}'", result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@branch_group.command("diff")
@click.option("--branch", "-b", required=True, help="Branch name to compare against live state")
@click.pass_context
def branch_diff(ctx: click.Context, branch: str):
    """Compare a branch against the live timeline and show modified/added items."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute("project.branch_diff", {"branch": branch}, record_history=False)
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@branch_group.command("merge")
@click.option("--branch", "-b", required=True, help="Branch name to merge")
@click.option(
    "--action",
    "-a",
    type=click.Choice(["accept", "reject"]),
    default="accept",
    show_default=True,
    help="Merge action: accept whole branch or reject and discard",
)
@click.option(
    "--force",
    is_flag=True,
    default=False,
    help="Force merge ignoring conflicting baseline revisions",
)
@click.pass_context
def branch_merge(ctx: click.Context, branch: str, action: str, force: bool):
    """Merge a timeline branch into the live timeline or reject it."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute(
            "project.branch_merge",
            {"branch": branch, "action": action, "force": force},
        )
        format_success(f"Branch '{branch}' merged ({action})", result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)
