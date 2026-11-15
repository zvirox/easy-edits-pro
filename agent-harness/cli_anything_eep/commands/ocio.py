# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""OpenColorIO / scene-referred colour management commands for Easy Edits Pro."""

from __future__ import annotations

from typing import Optional

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name="ocio")
def ocio_group():
    """Manage scene-referred colour pipeline and media IDTs (Packet 3.1 / AF7)."""


@ocio_group.command("set-pipeline")
@click.option(
    "--working-space",
    "-w",
    required=True,
    help="Working space (e.g. aces_cg, aces_cct)",
)
@click.option(
    "--display-odt",
    "-d",
    required=True,
    help="Display output transform (e.g. rec709, display_p3, rec2020_pq)",
)
@click.option(
    "--default-idt",
    "-i",
    default=None,
    help="Optional default camera IDT (e.g. rec709_video, arri_logc4, red_log3g10, sony_slog3, blackmagic_gen5, apple_log)",
)
@click.pass_context
def ocio_set_pipeline(
    ctx: click.Context,
    working_space: str,
    display_odt: str,
    default_idt: Optional[str],
):
    """Configure scene-referred working space, display ODT, and default IDT."""
    try:
        cli = backend.get_saturn_cli()
        params = {
            "working_space": working_space,
            "display_odt": display_odt,
        }
        if default_idt is not None:
            params["default_idt"] = default_idt
        result = cli.execute("ocio.set_pipeline", params)
        format_success("Color management pipeline updated", result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@ocio_group.command("set-media-idt")
@click.option("--media-id", "-m", type=int, required=True, help="Media asset ID")
@click.option(
    "--idt",
    "-i",
    required=True,
    help="Input Device Transform (e.g. rec709_video, arri_logc4, red_log3g10, sony_slog3, blackmagic_gen5, apple_log)",
)
@click.pass_context
def ocio_set_media_idt(ctx: click.Context, media_id: int, idt: str):
    """Assign camera Input Device Transform (IDT) to an individual media asset."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.execute("ocio.set_media_idt", {"media_id": media_id, "idt": idt})
        format_success(f"Media asset {media_id} IDT set to {idt}", result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)
