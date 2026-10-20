"""
Daemon/Control server commands for Easy Edits Pro.

`saturn-control` is a real HTTP/WebSocket server; these commands start it as a
background process, report its health, and stop the process this CLI started.
"""

import click
from cli_anything_eep.core import errors
from cli_anything_eep.core.runtime import ManagedService, require_binary
from cli_anything_eep.output import format_output, format_success


@click.group(name='daemon')
def daemon_group():
    """Control server (--control PORT) commands."""
    pass


@daemon_group.command('start')
@click.option('--port', '-p', type=int, default=9090, help='Control server port')
@click.option('--host', '-h', default='127.0.0.1', help='Bind host')
@click.pass_context
def daemon_start(ctx: click.Context, port: int, host: str):
    """Start the control server in the background."""
    try:
        binary = require_binary("saturn-control", "SATURN_CONTROL")
        service = ManagedService("control")
        result = service.start(
            [binary, "--host", host, "--port", str(port)],
            host=host,
            port=port,
        )
        format_success(f"Control server listening on http://{host}:{port}", result, ctx.obj['output_format'])
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
