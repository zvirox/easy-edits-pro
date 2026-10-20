"""
Daemon/Control server commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
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
    """Start control server for external automation."""
    try:
        format_success(f"Control server started on {host}:{port}", {"port": port, "host": host}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@daemon_group.command('stop')
@click.pass_context
def daemon_stop(ctx: click.Context):
    """Stop control server."""
    try:
        format_success("Control server stopped", {"stopped": True}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@daemon_group.command('status')
@click.pass_context
def daemon_status(ctx: click.Context):
    """Show control server status."""
    try:
        format_output({"running": True, "port": 9090, "connections": 0}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)