"""
Command registry commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name='command')
def command_group():
    """Command registry and execution commands."""
    pass


@command_group.command('list')
@click.option('--category', '-c', help='Filter by category')
@click.pass_context
def command_list(ctx: click.Context, category: str):
    """List available commands in registry."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.commands_list()
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@command_group.command('execute')
@click.option('--id', '-i', required=True, help='Command ID')
@click.option('--params', '-p', help='Parameters as JSON')
@click.pass_context
def command_execute(ctx: click.Context, id: str, params: str):
    """Execute a command by ID with parameters."""
    import json
    try:
        cli = backend.get_saturn_cli()
        param_dict = json.loads(params) if params else {}
        result = cli.command_execute(command_id=id, params=param_dict)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except json.JSONDecodeError:
        format_output(errors.EepError.validation_error('params', 'Invalid JSON').to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@command_group.command('info')
@click.option('--id', '-i', required=True, help='Command ID')
@click.pass_context
def command_info(ctx: click.Context, id: str):
    """Get detailed command information."""
    try:
        format_output({"command_id": id, "message": "Command info would be here"}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@command_group.command('categories')
@click.pass_context
def command_categories(ctx: click.Context):
    """List command categories."""
    categories = [
        "project", "timeline", "track", "clip", "render", 
        "color", "audio", "media", "template", "effect",
        "transition", "text", "automation", "system"
    ]
    format_output(categories, ctx.obj['output_format'])