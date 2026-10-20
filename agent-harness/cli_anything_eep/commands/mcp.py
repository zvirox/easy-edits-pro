"""
MCP (Model Context Protocol) server commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


@click.group(name='mcp')
def mcp_group():
    """MCP server commands for AI-assisted editing."""
    pass


@mcp_group.command('start')
@click.option('--transport', type=click.Choice(['stdio', 'tcp']), default='stdio', help='Transport type')
@click.option('--port', '-p', type=int, default=3000, help='TCP port (if transport=tcp)')
@click.pass_context
def mcp_start(ctx: click.Context, transport: str, port: int):
    """Start MCP server for AI-assisted editing."""
    report_unsupported(ctx, "mcp.start", "MCP servers are launched by the MCP client; this CLI cannot own an MCP client transport.")


@mcp_group.command('stop')
@click.pass_context
def mcp_stop(ctx: click.Context):
    """Stop MCP server."""
    report_unsupported(ctx, "mcp.stop", "The CLI does not manage the MCP client's server process.")


@mcp_group.command('tools')
@click.pass_context
def mcp_tools(ctx: click.Context):
    """List available MCP tools."""
    report_unsupported(ctx, "mcp.tools", "Tool discovery is available through an MCP client connection, not this CLI command group.")


@mcp_group.command('resources')
@click.pass_context
def mcp_resources(ctx: click.Context):
    """List available MCP resources."""
    report_unsupported(ctx, "mcp.resources", "The current MCP server does not expose MCP resources.")


@mcp_group.command('prompts')
@click.pass_context
def mcp_prompts(ctx: click.Context):
    """List available MCP prompts."""
    report_unsupported(ctx, "mcp.prompts", "The current MCP server does not expose MCP prompts.")
