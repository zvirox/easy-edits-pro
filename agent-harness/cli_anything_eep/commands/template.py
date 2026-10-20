"""
Graphics template commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


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
    report_unsupported(ctx, "template.create", "Template storage is not implemented by the Rust engine yet.")


@template_group.command('list')
@click.option('--type', '-t', type=click.Choice(['project', 'graphics', 'sequence', 'bin', 'all']), default='all', help='Filter by type')
@click.pass_context
def template_list(ctx: click.Context, type: str):
    """List available templates."""
    report_unsupported(ctx, "template.list", "The Rust engine does not persist templates yet.")


@template_group.command('apply')
@click.option('--id', '-i', required=True, help='Template ID')
@click.option('--params', '-p', help='Parameter bindings (JSON)')
@click.pass_context
def template_apply(ctx: click.Context, id: str, params: str):
    """Apply template with parameter bindings."""
    report_unsupported(ctx, "template.apply", "Template application is not implemented by the Rust engine yet.")


@template_group.command('export')
@click.option('--id', '-i', required=True, help='Template ID')
@click.option('--output', '-o', required=True, help='Output .sgt file path')
@click.pass_context
def template_export(ctx: click.Context, id: str, output: str):
    """Export template to .sgt file."""
    report_unsupported(ctx, "template.export", "Template export is not implemented by the Rust engine yet.")
