"""
Timeline management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


@click.group(name='timeline')
def timeline_group():
    """Timeline management commands."""
    pass


@timeline_group.command('create')
@click.option('--name', '-n', required=True, help='Timeline name')
@click.option('--fps', type=float, default=30, help='Frame rate (23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60)')
@click.option('--width', type=int, default=1920, help='Width in pixels')
@click.option('--height', type=int, default=1080, help='Height in pixels')
@click.option('--drop-frame/--no-drop-frame', default=False, help='Drop frame timecode')
@click.option('--color-space', type=click.Choice(['rec709', 'rec2020', 'p3d65', 'srgb', 'linear']), default='rec709', help='Color space')
@click.pass_context
def timeline_create(ctx: click.Context, name: str, fps: float, width: int, height: int, drop_frame: bool, color_space: str):
    """Create a new timeline in current project."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.timeline_create(name=name, fps=fps, width=width, height=height)
        format_success(f"Timeline '{name}' created", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@timeline_group.command('list')
@click.pass_context
def timeline_list(ctx: click.Context):
    """List all timelines in current project."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.timeline_list()
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@timeline_group.command('open')
@click.option('--id', '-i', required=True, help='Timeline ID')
@click.pass_context
def timeline_open(ctx: click.Context, id: str):
    """Open/select a timeline for editing."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.timeline_open(id)
        format_success(f"Timeline {id} opened", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@timeline_group.command('inspect')
@click.option('--id', '-i', required=True, help='Timeline ID')
@click.pass_context
def timeline_inspect(ctx: click.Context, id: str):
    """Inspect timeline details."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.timeline_inspect(id)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@timeline_group.command('settings')
@click.option('--id', '-i', required=True, help='Timeline ID')
@click.option('--fps', type=float, help='Change frame rate')
@click.option('--width', type=int, help='Change width')
@click.option('--height', type=int, help='Change height')
@click.option('--color-space', type=click.Choice(['rec709', 'rec2020', 'p3d65', 'srgb', 'linear']), help='Change color space')
@click.pass_context
def timeline_settings(ctx: click.Context, id: str, fps: float, width: int, height: int, color_space: str):
    """Get or update timeline settings."""
    report_unsupported(ctx, "timeline.settings", "Sequence settings are not editable through the current Rust command catalogue.")
