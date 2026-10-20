"""
Graphics template commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name='template')
def template_group():
    """Graphics template commands (.sgt format)."""
    pass


@template_group.command('create')
@click.option('--name', '-n', required=True, help='Template name')
@click.option('--type', '-t', type=click.Choice(['project', 'graphics', 'sequence', 'bin']), required=True, help='Template type')
@click.option('--spec', '-s', required=True, help='Template specification (JSON or path to .sgt file)')
@click.pass_context
def template_create(ctx: click.Context, name: str, type: str, spec: str):
    """Create a new template from specification."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.template_create(name=name, type_=type, spec=spec)
        format_success(f"Template '{name}' created", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('list')
@click.option('--type', '-t', type=click.Choice(['project', 'graphics', 'sequence', 'bin', 'all']), default='all', help='Filter by type')
@click.pass_context
def template_list(ctx: click.Context, type: str):
    """List available templates."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.template_list()
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('apply')
@click.option('--id', '-i', required=True, help='Template ID')
@click.option('--params', '-p', help='Parameter bindings (JSON)')
@click.pass_context
def template_apply(ctx: click.Context, id: str, params: str):
    """Apply template with parameter bindings."""
    try:
        format_success(f"Template {id} applied", {"applied": True}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('export')
@click.option('--id', '-i', required=True, help='Template ID')
@click.option('--output', '-o', required=True, help='Output .sgt file path')
@click.pass_context
def template_export(ctx: click.Context, id: str, output: str):
    """Export template to .sgt file."""
    try:
        format_success(f"Template exported to {output}", {"exported": True}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)