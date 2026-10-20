"""
Command registry commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


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
        command = next((item for item in backend.get_saturn_cli().commands_list() if item.get('id') == id), None)
        if command is None:
            raise errors.EepError(errors.ErrorCode.COMMAND_NOT_FOUND, f"Command not found: {id}", recoverable=False)
        format_output(command, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@command_group.command('categories')
@click.pass_context
def command_categories(ctx: click.Context):
    """List command categories."""
    categories = sorted({item['id'].split('.', 1)[0] for item in backend.get_saturn_cli().commands_list()})
    format_output(categories, ctx.obj['output_format'])
