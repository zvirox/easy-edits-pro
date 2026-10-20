"""
Daemon/Control server commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


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
    report_unsupported(ctx, "daemon.start", "The Rust control interface is stdin/stdout based and does not expose this TCP daemon command.")


@daemon_group.command('stop')
@click.pass_context
def daemon_stop(ctx: click.Context):
    """Stop control server."""
    report_unsupported(ctx, "daemon.stop", "No standalone control daemon is running through this CLI.")


@daemon_group.command('status')
@click.pass_context
def daemon_status(ctx: click.Context):
    """Show control server status."""
    report_unsupported(ctx, "daemon.status", "The Rust CLI does not expose control-daemon status.")
