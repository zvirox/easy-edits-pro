# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Task Sheet, review, verification, and delivery CLI commands."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Optional

import click

from cli_anything_eep.commands import _engine
from cli_anything_eep.core import errors


@click.group(name="tasksheet")
def tasksheet_group():
    """Task Sheet lifecycle, review rounds, verification, and delivery packaging."""


@tasksheet_group.command("create")
@click.option("--file", "sheet_file", type=click.Path(exists=True), help="Load complete Task Sheet JSON file")
@click.option("--id", "sheet_id", help="Task sheet id")
@click.option("--title", help="Task sheet title")
@click.option("--requester", default="human:user", show_default=True, help="Requester actor")
@click.option("--preset", default="web-1080p", show_default=True, help="Default deliverable preset")
@click.option("--duration-s", type=float, default=60.0, show_default=True, help="Default deliverable target duration")
@click.pass_context
def tasksheet_create(
    ctx: click.Context,
    sheet_file: Optional[str],
    sheet_id: Optional[str],
    title: Optional[str],
    requester: str,
    preset: str,
    duration_s: float,
):
    """Create or load a Task Sheet on the open project."""
    if sheet_file:
        try:
            payload = json.loads(Path(sheet_file).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise errors.EepError(errors.ErrorCode.INVALID_ARGUMENT, f"Invalid sheet JSON: {exc}") from exc
    else:
        payload = {
            "id": sheet_id or "ts_default",
            "title": title or "New Task Sheet",
            "requester": requester,
            "deliverables": [
                {
                    "id": "deliv_main",
                    "name": "Main Deliverable",
                    "preset": preset,
                    "duration": {"target_s": duration_s, "tolerance_s": 10.0},
                    "loudness_lufs": -14.0 if preset != "broadcast-r128" else -23.0,
                    "true_peak_dbtp": -1.0,
                    "captions": True,
                }
            ],
        }
    _engine.run_engine(ctx, "tasksheet.create", payload, success="Task sheet created")


@tasksheet_group.command("get")
@click.pass_context
def tasksheet_get(ctx: click.Context):
    """Inspect the active Task Sheet."""
    _engine.run_engine(ctx, "tasksheet.get", {})


@tasksheet_group.command("validate")
@click.pass_context
def tasksheet_validate(ctx: click.Context):
    """Validate the active Task Sheet."""
    _engine.run_engine(ctx, "tasksheet.validate", {})


@tasksheet_group.command("submit")
@click.option("--reason", help="Optional submission reason")
@click.pass_context
def tasksheet_submit(ctx: click.Context, reason: Optional[str]):
    """Submit a draft Task Sheet to `ready`."""
    params = {"reason": reason} if reason else {}
    _engine.run_engine(ctx, "tasksheet.submit", params, success="Task sheet submitted")


@tasksheet_group.command("plan-submit")
@click.option("--steps-json", required=True, help="JSON array of plan steps")
@click.pass_context
def tasksheet_plan_submit(ctx: click.Context, steps_json: str):
    """Submit an execution plan for human approval."""
    steps = _engine.parse_json_option(steps_json, "--steps-json")
    if not isinstance(steps, list):
        raise errors.EepError(errors.ErrorCode.INVALID_ARGUMENT, "--steps-json must decode to an array")
    _engine.run_engine(ctx, "tasksheet.plan_submit", {"steps": steps}, success="Plan submitted")


@tasksheet_group.command("run-report")
@click.option("--completed", type=int, help="Completed step id")
@click.option("--current", type=int, help="Current active step id")
@click.pass_context
def tasksheet_run_report(ctx: click.Context, completed: Optional[int], current: Optional[int]):
    """Update step execution progress on the active plan."""
    params = {}
    if completed is not None:
        params["completed_step_id"] = completed
    if current is not None:
        params["current_step_id"] = current
    _engine.run_engine(ctx, "tasksheet.run_report", params, success="Run report recorded")


@tasksheet_group.command("verify")
@click.option("--preset", type=click.Choice(["web-1080p", "vertical-9x16", "broadcast-r128"]), default="web-1080p")
@click.option("--start-tick", type=int, help="Optional verification start tick")
@click.option("--end-tick", type=int, help="Optional verification end tick")
@click.pass_context
def tasksheet_verify(ctx: click.Context, preset: str, start_tick: Optional[int], end_tick: Optional[int]):
    """Run measured `verify.run` QC checks over the active sequence."""
    params: dict[str, object] = {"preset": preset}
    if start_tick is not None:
        params["start_tick"] = start_tick
    if end_tick is not None:
        params["end_tick"] = end_tick
    _engine.run_engine(ctx, "verify.run", params)


@tasksheet_group.command("review-open")
@click.option("--label", help="Review round label (e.g. v1)")
@click.option("--reason", help="Summary reason for opening review")
@click.pass_context
def tasksheet_review_open(ctx: click.Context, label: Optional[str], reason: Optional[str]):
    """Open the next review round on the active Task Sheet."""
    params = {}
    if label:
        params["label"] = label
    if reason:
        params["reason"] = reason
    _engine.run_engine(ctx, "review.round_open", params, success="Review round opened")


@tasksheet_group.command("note-add")
@click.option("--text", required=True, help="Review note text")
@click.option("--target-tick", type=int, help="Timeline start tick")
@click.option("--end-tick", type=int, help="Timeline end tick")
@click.option("--severity", type=click.Choice(["info", "warning", "blocker"]), default="info")
@click.option("--category", help="Note category (e.g. hook, tone, ending, must_not)")
@click.pass_context
def tasksheet_note_add(
    ctx: click.Context,
    text: str,
    target_tick: Optional[int],
    end_tick: Optional[int],
    severity: str,
    category: Optional[str],
):
    """Attach a timecoded review note to the active review round."""
    params: dict[str, object] = {"text": text, "severity": severity}
    if target_tick is not None:
        params["target_tick"] = target_tick
    if end_tick is not None:
        params["end_tick"] = end_tick
    if category:
        params["category"] = category
    _engine.run_engine(ctx, "review.note_add", params, success="Review note added")


@tasksheet_group.command("note-list")
@click.pass_context
def tasksheet_note_list(ctx: click.Context):
    """List review notes on the active Task Sheet."""
    _engine.run_engine(ctx, "review.note_list", {})


@tasksheet_group.command("note-resolve")
@click.option("--note-id", required=True, help="Note identifier")
@click.option("--resolution", required=True, help="Resolution explanation")
@click.pass_context
def tasksheet_note_resolve(ctx: click.Context, note_id: str, resolution: str):
    """Resolve an open review note with explanation."""
    _engine.run_engine(
        ctx,
        "review.note_resolve",
        {"note_id": note_id, "resolution": resolution},
        success="Review note resolved",
    )


@tasksheet_group.command("deliver-package")
@click.option("--path", required=True, type=click.Path(), help="Destination bundle directory")
@click.option("--require-approved/--allow-unapproved", default=True, show_default=True)
@click.option("--preset", type=click.Choice(["web-1080p", "vertical-9x16", "broadcast-r128"]), default="web-1080p")
@click.pass_context
def tasksheet_deliver_package(ctx: click.Context, path: str, require_approved: bool, preset: str):
    """Assemble a verified TS5 handoff package."""
    _engine.run_engine(
        ctx,
        "delivery.package",
        {
            "path": path,
            "require_approved": require_approved,
            "verify": {"preset": preset},
        },
        success="Delivery package built",
    )


@tasksheet_group.command("deliver-verify")
@click.option("--path", required=True, type=click.Path(exists=True), help="Package directory to verify")
@click.pass_context
def tasksheet_deliver_verify(ctx: click.Context, path: str):
    """Verify integrity and SHA-256 checksums of a delivery bundle."""
    _engine.run_engine(ctx, "delivery.verify", {"path": path})
