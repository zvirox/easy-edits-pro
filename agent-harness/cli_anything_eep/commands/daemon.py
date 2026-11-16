# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""
Daemon/Control server commands for Easy Edits Pro (AF9).

`saturn-control` runs an authenticated long-lived headless daemon over UDS or HTTP;
these commands start it as a background process, report its health, and stop it.
"""

from typing import Optional
import click
from cli_anything_eep.core import errors
from cli_anything_eep.core.runtime import ManagedService, require_binary
from cli_anything_eep.output import format_output, format_success


@click.group(name='daemon')
def daemon_group():
    """Control server and authenticated headless daemon commands."""
    pass


@daemon_group.command('start')
@click.option('--port', '-p', type=int, default=None, help='Control server port')
@click.option('--host', '-h', default='127.0.0.1', help='Bind host')
@click.option('--socket', '-s', 'socket_path', default=None, help='Unix Domain Socket path')
@click.option('--auth-token', '--token', 'auth_token', default=None, help='Master session token for authentication')
@click.option('--rate-limit', type=int, default=120, help='Max requests per minute per session')
@click.option('--time-budget', type=int, default=None, help='Session wall-clock time budget in seconds')
@click.option('--project-root', default=None, help='Allowed project root directory for filesystem operations')
@click.option('--audit-file', default=None, help='Path to structured audit.jsonl log file')
@click.pass_context
def daemon_start(
    ctx: click.Context,
    port: Optional[int],
    host: str,
    socket_path: Optional[str],
    auth_token: Optional[str],
    rate_limit: int,
    time_budget: Optional[int],
    project_root: Optional[str],
    audit_file: Optional[str] = None,
):
    """Start the control server or authenticated headless daemon in the background."""
    try:
        binary = require_binary("saturn-control", "SATURN_CONTROL")
        service = ManagedService("control")

        argv = [binary]
        if socket_path:
            argv.extend(["--socket", socket_path])
        if port is not None:
            argv.extend(["--host", host, "--port", str(port)])
        elif not socket_path:
            # Default to TCP port 9090 when neither socket nor port is provided
            port = 9090
            argv.extend(["--host", host, "--port", str(port)])

        if auth_token:
            argv.extend(["--auth-token", auth_token])
        if rate_limit:
            argv.extend(["--rate-limit", str(rate_limit)])
        if time_budget:
            argv.extend(["--time-budget", str(time_budget)])
        if project_root:
            argv.extend(["--project-root", project_root])
        if audit_file:
            argv.extend(["--audit-file", audit_file])

        result = service.start(
            argv,
            host=host,
            port=port or 0,
            socket_path=socket_path,
            token=auth_token,
        )
        endpoint = f"socket {socket_path}" if socket_path else f"http://{host}:{port}"
        format_success(f"Control daemon listening on {endpoint}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@daemon_group.command('stop')
@click.pass_context
def daemon_stop(ctx: click.Context):
    """Stop the control server this CLI started."""
    try:
        result = ManagedService("control").stop()
        format_success("Control server stopped", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@daemon_group.command('status')
@click.pass_context
def daemon_status(ctx: click.Context):
    """Show control server status and health."""
    try:
        result = ManagedService("control").status()
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
