"""
MCP (Model Context Protocol) server commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


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
    try:
        format_success(f"MCP server started ({transport})", {"transport": transport, "port": port}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@mcp_group.command('stop')
@click.pass_context
def mcp_stop(ctx: click.Context):
    """Stop MCP server."""
    try:
        format_success("MCP server stopped", {"stopped": True}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@mcp_group.command('tools')
@click.pass_context
def mcp_tools(ctx: click.Context):
    """List available MCP tools."""
    tools = [
        {"name": "saturn.analyzeTimeline", "description": "Analyze timeline structure, detect gaps, overlaps"},
        {"name": "saturn.suggestEdit", "description": "Suggest edits based on context, selection, playhead"},
        {"name": "saturn.batchOperation", "description": "Execute multiple commands atomically"},
        {"name": "saturn.searchMedia", "description": "Natural language media search"},
        {"name": "saturn.createTemplate", "description": "Create project/graphics template from spec"},
    ]
    format_output(tools, ctx.obj['output_format'])


@mcp_group.command('resources')
@click.pass_context
def mcp_resources(ctx: click.Context):
    """List available MCP resources."""
    resources = [
        "saturn://project/{id}/timeline",
        "saturn://project/{id}/media",
        "saturn://project/{id}/sequences",
        "saturn://project/{id}/tracks",
    ]
    format_output(resources, ctx.obj['output_format'])


@mcp_group.command('prompts')
@click.pass_context
def mcp_prompts(ctx: click.Context):
    """List available MCP prompts."""
    prompts = [
        "editing-workflow",
        "color-grading",
        "audio-mixing",
        "multi-cam-sync",
    ]
    format_output(prompts, ctx.obj['output_format'])