"""
Graphics template commands for Easy Edits Pro (.sgt format).

The Rust `saturn-graphics` crate owns the binary `.sgt` format. These commands
call `saturn-cli template ...` so the harness and the engine share one parser.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success

CATEGORIES = ["lower-third", "title", "transition", "background", "overlay", "bug", "full-screen", "custom"]


@click.group(name='template')
def template_group():
    """Graphics template commands (.sgt format)."""
    pass


@template_group.command('create')
@click.option('--name', '-n', required=True, help='Template name')
@click.option('--type', '-t', 'category', type=click.Choice(CATEGORIES), default='custom', help='Template category')
@click.option('--output', '-o', required=True, type=click.Path(dir_okay=False), help='Output .sgt file path')
@click.option('--description', '-d', default='', help='Template description')
@click.option('--author', '-a', default='', help='Template author')
@click.option('--tags', default='', help='Comma-separated tags')
@click.pass_context
def template_create(ctx: click.Context, name: str, category: str, output: str, description: str, author: str, tags: str):
    """Create a new .sgt graphics template."""
    try:
        args = [
            "template", "create", output,
            "--name", name,
            "--category", category,
            "--description", description,
            "--author", author,
        ]
        if tags:
            args += ["--tags", tags]
        result = backend.get_saturn_cli().run_cli_json(args)
        format_success(f"Template '{name}' created at {output}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('list')
@click.option('--dir', '-d', 'directory', default='.', type=click.Path(file_okay=False), help='Directory to scan for .sgt files')
@click.option('--type', '-t', 'category', type=click.Choice(CATEGORIES), help='Filter by category')
@click.pass_context
def template_list(ctx: click.Context, directory: str, category):
    """List .sgt templates in a directory."""
    try:
        templates = backend.get_saturn_cli().run_cli_json(["template", "list", directory])
        if category:
            wanted = _category_name(category)
            templates = [item for item in templates if item.get("category") == wanted]
        format_output({"templates": templates}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('inspect')
@click.argument('path', type=click.Path(exists=True, dir_okay=False))
@click.pass_context
def template_inspect(ctx: click.Context, path: str):
    """Show one .sgt template."""
    try:
        result = backend.get_saturn_cli().run_cli_json(["template", "inspect", path])
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('export')
@click.argument('path', type=click.Path(exists=True, dir_okay=False))
@click.option('--output', '-o', required=True, type=click.Path(dir_okay=False), help='Output .sgt file path')
@click.pass_context
def template_export(ctx: click.Context, path: str, output: str):
    """Validate and re-write a template to another location."""
    try:
        result = backend.get_saturn_cli().run_cli_json(
            ["template", "export", path, "--output", output]
        )
        format_success(f"Template exported to {output}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@template_group.command('apply')
@click.argument('path', type=click.Path(exists=True, dir_okay=False))
@click.option('--params', '-p', help='Parameter bindings (JSON)')
@click.pass_context
def template_apply(ctx: click.Context, path: str, params):
    """Apply a template to the current sequence (not supported yet)."""
    try:
        backend.get_saturn_cli().run_cli_json(["template", "apply", path])
        format_success("Template applied", {"path": path}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


def _category_name(value: str) -> str:
    """Map the CLI spelling to the serde name used by saturn-graphics."""
    return {
        "lower-third": "LowerThird",
        "full-screen": "FullScreen",
    }.get(value, value.capitalize())
