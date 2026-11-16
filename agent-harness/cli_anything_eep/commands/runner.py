# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""
Runner and multi-job execution commands for Easy Edits Pro (TS9).

Commands:
  cli-anything-eep runner start   - start the runner service
  cli-anything-eep runner status  - show runner status and active/pending job counts
  cli-anything-eep runner stop    - stop the runner service
  cli-anything-eep runner list    - list all runner jobs
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Any, Optional

import click

from cli_anything_eep.core import errors
from cli_anything_eep.core.runtime import ManagedService
from cli_anything_eep.output import format_output, format_success


def _control_endpoint() -> tuple[str, Optional[str]]:
    """Return (base_url, auth_token) for the active control daemon."""
    service = ManagedService("control")
    record = service.registry.get("control")
    if record is None or not record.is_alive():
        raise errors.EepError(
            errors.ErrorCode.BACKEND_ERROR,
            "Control daemon is not running. Start it first with 'eep daemon start'.",
            recoverable=False,
        )
    base_url = f"http://{record.host}:{record.port}"
    return base_url, record.token


def _runner_request(method: str, path: str, data: Optional[dict[str, Any]] = None) -> dict[str, Any]:
    """Send an HTTP request to the control daemon's runner API."""
    base_url, token = _control_endpoint()
    url = f"{base_url}{path}"
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
        headers["x-auth-token"] = token

    payload_bytes = json.dumps(data).encode("utf-8") if data is not None else None
    req = urllib.request.Request(url, data=payload_bytes, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=5.0) as resp:
            content = resp.read().decode("utf-8")
            return json.loads(content) if content else {}
    except urllib.error.HTTPError as exc:
        try:
            err_body = json.loads(exc.read().decode("utf-8"))
            err_msg = err_body.get("error", str(exc))
            err_code = err_body.get("code", "HTTPError")
        except Exception:
            err_msg = str(exc)
            err_code = "HTTPError"
        raise errors.EepError(
            errors.ErrorCode.BACKEND_ERROR,
            f"Runner API error ({exc.code}): {err_msg}",
            details={"code": err_code, "status": exc.code},
        ) from exc
    except (urllib.error.URLError, OSError) as exc:
        raise errors.EepError(
            errors.ErrorCode.BACKEND_ERROR,
            f"Could not reach control daemon at {url}: {exc}",
        ) from exc


@click.group(name="runner")
def runner_group():
    """Runner daemon service and multi-job execution commands (TS9)."""
    pass


@runner_group.command("start")
@click.option("--workers", "-w", "max_workers", type=int, default=None, help="Max concurrent worker slots")
@click.pass_context
def runner_start(ctx: click.Context, max_workers: Optional[int]):
    """Start the runner service."""
    try:
        body: dict[str, Any] = {}
        if max_workers is not None:
            body["max_concurrent_workers"] = max_workers
        result = _runner_request("POST", "/runner/start", body)
        format_success("Runner service started", result, ctx.obj["output_format"])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@runner_group.command("status")
@click.pass_context
def runner_status(ctx: click.Context):
    """Show runner service status and active/pending job counts."""
    try:
        result = _runner_request("GET", "/runner/status")
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@runner_group.command("stop")
@click.pass_context
def runner_stop(ctx: click.Context):
    """Stop the runner service and cancel active jobs."""
    try:
        result = _runner_request("POST", "/runner/stop")
        format_success("Runner service stopped", result, ctx.obj["output_format"])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@runner_group.command("list")
@click.pass_context
def runner_list(ctx: click.Context):
    """List all runner jobs."""
    try:
        result = _runner_request("GET", "/runner/jobs")
        format_output(result, ctx.obj["output_format"])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)
