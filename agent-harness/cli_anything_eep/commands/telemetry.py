# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Audio and video telemetry inspection commands (mapped to `verify.run` AF3)."""

from __future__ import annotations

import json
from typing import Optional, Sequence

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


def _parse_json_metrics(metrics_str: Optional[str], label: str) -> Optional[dict]:
    if not metrics_str:
        return None
    try:
        data = json.loads(metrics_str)
        if not isinstance(data, dict):
            raise ValueError(f"{label} must be a JSON object")
        return data
    except Exception as exc:
        raise errors.EepError(
            errors.ErrorCode.INVALID_ARGUMENT,
            f"Failed to parse {label}: {exc}",
            details={"raw": metrics_str},
        )


@click.group(name="telemetry")
def telemetry_group():
    """Inspect measured audio and video telemetry via the `verify.run` subsystem."""


@telemetry_group.command("audio")
@click.option(
    "--preset",
    "-p",
    default="web-1080p",
    help="Delivery preset (web-1080p, vertical-9x16, broadcast-r128)",
)
@click.option("--start-tick", type=int, default=None, help="Start tick for analysis window")
@click.option("--end-tick", type=int, default=None, help="End tick for analysis window")
@click.option("--metrics", default=None, help="Optional raw audio measurement JSON string")
@click.pass_context
def telemetry_audio(
    ctx: click.Context,
    preset: str,
    start_tick: Optional[int],
    end_tick: Optional[int],
    metrics: Optional[str],
):
    """Inspect audio telemetry (integrated loudness, true peak, silence/dead air)."""
    try:
        cli = backend.get_saturn_cli()
        params: dict = {
            "preset": preset,
            "checks": ["audio.loudness", "audio.true_peak", "project.timeline_silence"],
        }
        if start_tick is not None:
            params["start_tick"] = start_tick
        if end_tick is not None:
            params["end_tick"] = end_tick
        parsed_metrics = _parse_json_metrics(metrics, "audio_metrics")
        if parsed_metrics is not None:
            params["audio_metrics"] = parsed_metrics

        report = cli.execute("verify.run", params, record_history=False)
        # Extract audio checks and summary
        audio_checks = [
            c for c in report.get("checks", [])
            if c.get("check_id") in {"audio.loudness", "audio.true_peak", "project.timeline_silence"}
        ]
        result = {
            "preset": report.get("preset", preset),
            "analyzed_range": report.get("analyzed_range"),
            "audio_checks": audio_checks,
            "summary": {
                "total": len(audio_checks),
                "passed": sum(1 for c in audio_checks if c.get("verdict") == "pass"),
                "warned": sum(1 for c in audio_checks if c.get("verdict") == "warn"),
                "failed": sum(1 for c in audio_checks if c.get("verdict") == "fail"),
                "unavailable": sum(1 for c in audio_checks if c.get("status") == "unavailable"),
            },
            "snapshot_id": report.get("snapshot_id"),
        }
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@telemetry_group.command("video")
@click.option(
    "--preset",
    "-p",
    default="web-1080p",
    help="Delivery preset (web-1080p, vertical-9x16, broadcast-r128)",
)
@click.option("--start-tick", type=int, default=None, help="Start tick for analysis window")
@click.option("--end-tick", type=int, default=None, help="End tick for analysis window")
@click.option("--metrics", default=None, help="Optional raw video measurement JSON string")
@click.pass_context
def telemetry_video(
    ctx: click.Context,
    preset: str,
    start_tick: Optional[int],
    end_tick: Optional[int],
    metrics: Optional[str],
):
    """Inspect video telemetry (clipped highlights, crushed blacks, frozen frames, aspect)."""
    try:
        cli = backend.get_saturn_cli()
        params: dict = {
            "preset": preset,
            "checks": [
                "video.clipped_highlights",
                "video.crushed_blacks",
                "video.black_and_frozen_frames",
                "project.resolution_aspect",
            ],
        }
        if start_tick is not None:
            params["start_tick"] = start_tick
        if end_tick is not None:
            params["end_tick"] = end_tick
        parsed_metrics = _parse_json_metrics(metrics, "video_metrics")
        if parsed_metrics is not None:
            params["video_metrics"] = parsed_metrics

        report = cli.execute("verify.run", params, record_history=False)
        video_checks = [
            c for c in report.get("checks", [])
            if c.get("check_id") in {
                "video.clipped_highlights",
                "video.crushed_blacks",
                "video.black_and_frozen_frames",
                "project.resolution_aspect",
            }
        ]
        result = {
            "preset": report.get("preset", preset),
            "analyzed_range": report.get("analyzed_range"),
            "video_checks": video_checks,
            "summary": {
                "total": len(video_checks),
                "passed": sum(1 for c in video_checks if c.get("verdict") == "pass"),
                "warned": sum(1 for c in video_checks if c.get("verdict") == "warn"),
                "failed": sum(1 for c in video_checks if c.get("verdict") == "fail"),
                "unavailable": sum(1 for c in video_checks if c.get("status") == "unavailable"),
            },
            "snapshot_id": report.get("snapshot_id"),
        }
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@telemetry_group.command("inspect")
@click.option(
    "--preset",
    "-p",
    default="web-1080p",
    help="Delivery preset (web-1080p, vertical-9x16, broadcast-r128)",
)
@click.option("--start-tick", type=int, default=None, help="Start tick for analysis window")
@click.option("--end-tick", type=int, default=None, help="End tick for analysis window")
@click.option(
    "--check",
    "checks",
    multiple=True,
    help="Filter to specific check IDs (can specify multiple times)",
)
@click.pass_context
def telemetry_inspect(
    ctx: click.Context,
    preset: str,
    start_tick: Optional[int],
    end_tick: Optional[int],
    checks: Sequence[str],
):
    """Run full measured verification report for the active sequence."""
    try:
        cli = backend.get_saturn_cli()
        params: dict = {"preset": preset}
        if start_tick is not None:
            params["start_tick"] = start_tick
        if end_tick is not None:
            params["end_tick"] = end_tick
        if checks:
            params["checks"] = list(checks)

        report = cli.execute("verify.run", params, record_history=False)
        format_output(report, ctx.obj["output_format"])
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)
