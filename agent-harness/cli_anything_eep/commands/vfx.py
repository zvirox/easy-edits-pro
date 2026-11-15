# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Visual effects (VFX) commands returning structured errors per A4 rule."""

from __future__ import annotations

import click

from cli_anything_eep.output import report_unsupported


@click.group(name="vfx")
def vfx_group():
    """VFX operations (deferred after Phase 6 per A4 hard rule)."""


@vfx_group.command("composite")
@click.option("--layer-a", help="Foreground layer ID")
@click.option("--layer-b", help="Background layer ID")
@click.pass_context
def vfx_composite(ctx: click.Context, layer_a, layer_b):
    """Composite visual layers (deferred after Phase 6 per A4 rule)."""
    report_unsupported(
        ctx,
        "vfx.composite",
        "VFX compositing operations are deferred after Phase 6 per project plan and hard rule A4.",
    )


@vfx_group.command("matte")
@click.option("--clip", help="Target clip ID")
@click.pass_context
def vfx_matte(ctx: click.Context, clip):
    """Generate neural alpha matte (deferred after Phase 6 per A4 rule)."""
    report_unsupported(
        ctx,
        "vfx.matte",
        "Neural rotoscope/matting operations are deferred after Phase 6 per project plan and hard rule A4.",
    )


@vfx_group.command("depth")
@click.option("--clip", help="Target clip ID")
@click.pass_context
def vfx_depth(ctx: click.Context, clip):
    """Estimate depth map (deferred after Phase 6 per A4 rule)."""
    report_unsupported(
        ctx,
        "vfx.depth",
        "Depth estimation operations are deferred after Phase 6 per project plan and hard rule A4.",
    )


@vfx_group.command("track")
@click.option("--clip", help="Target clip ID")
@click.pass_context
def vfx_track(ctx: click.Context, clip):
    """Planar or point feature tracking (deferred after Phase 6 per A4 rule)."""
    report_unsupported(
        ctx,
        "vfx.track",
        "Point and planar motion tracking operations are deferred after Phase 6 per project plan and hard rule A4.",
    )


@vfx_group.command("effects")
@click.option("--clip", help="Target clip ID")
@click.pass_context
def vfx_effects(ctx: click.Context, clip):
    """Apply visual effects pipeline (deferred after Phase 6 per A4 rule)."""
    report_unsupported(
        ctx,
        "vfx.effects",
        "Visual effects node pipelines are deferred after Phase 6 per project plan and hard rule A4.",
    )
