"""
MCP (Model Context Protocol) server commands for Easy Edits Pro.

`mcp start` runs `saturn-mcp` in the foreground so a client can speak MCP over
stdin/stdout. The list commands perform a real initialize/tools handshake
against the same binary instead of describing tools from memory.
"""

import subprocess

import click
from cli_anything_eep.core import errors
from cli_anything_eep.core.mcp_client import McpSession
from cli_anything_eep.core.runtime import require_binary
from cli_anything_eep.output import format_output


@click.group(name='mcp')
def mcp_group():
    """MCP server commands for AI-assisted editing."""
    pass


@mcp_group.command('start')
@click.option('--project', type=click.Path(exists=True), help='Project file the server should open')
@click.pass_context
def mcp_start(ctx: click.Context, project):
    """Run the MCP server in the foreground (stdio transport)."""
    binary = require_binary("saturn-mcp", "SATURN_MCP")
    argv = [binary]
    if project:
        argv += ["--project", project]
    try:
        ctx.exit(subprocess.call(argv))
    except KeyboardInterrupt:
        ctx.exit(130)


@mcp_group.command('tools')
@click.option('--project', type=click.Path(exists=True), help='Project file the server should open')
@click.pass_context
def mcp_tools(ctx: click.Context, project):
    """List the tools the MCP server exposes."""
    try:
        binary = require_binary("saturn-mcp", "SATURN_MCP")
        with McpSession(binary, project=project) as session:
            result = session.request("tools/list")
        format_output({"tools": result.get("tools", [])}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@mcp_group.command('resources')
@click.option('--project', type=click.Path(exists=True), help='Project file the server should open')
@click.pass_context
def mcp_resources(ctx: click.Context, project):
    """List the resources the MCP server exposes."""
    try:
        binary = require_binary("saturn-mcp", "SATURN_MCP")
        with McpSession(binary, project=project) as session:
            result = session.request("resources/list")
        format_output({"resources": result.get("resources", [])}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@mcp_group.command('prompts')
@click.option('--project', type=click.Path(exists=True), help='Project file the server should open')
@click.pass_context
def mcp_prompts(ctx: click.Context, project):
    """List the prompts the MCP server exposes."""
    try:
        binary = require_binary("saturn-mcp", "SATURN_MCP")
        with McpSession(binary, project=project) as session:
            result = session.request("prompts/list")
        format_output({"prompts": result.get("prompts", [])}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
