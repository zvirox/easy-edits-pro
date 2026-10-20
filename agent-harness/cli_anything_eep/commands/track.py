"""
Track management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


@click.group(name='track')
def track_group():
    """Track management commands."""
    pass


@track_group.command('add')
@click.option('--kind', '-k', type=click.Choice(['video', 'audio', 'subtitle', 'submix']), required=True, help='Track kind')
@click.option('--name', '-n', help='Track name (auto-generated if not provided)')
@click.option('--index', '-i', type=int, help='Insert at index')
@click.pass_context
def track_add(ctx: click.Context, kind: str, name: str, index: int):
    """Add a new track to current timeline."""
    try:
        if index is not None:
            report_unsupported(ctx, "track.add", "The Rust command engine currently appends tracks and does not accept an insertion index.")
        cli = backend.get_saturn_cli()
        result = cli.track_add(kind=kind, name=name, index=index)
        format_success(f"{kind.title()} track added", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@track_group.command('remove')
@click.option('--id', '-i', required=True, help='Track ID')
@click.pass_context
def track_remove(ctx: click.Context, id: str):
    """Remove a track."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.track_remove(id)
        format_success(f"Track {id} removed", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@track_group.command('list')
@click.pass_context
def track_list(ctx: click.Context):
    """List all tracks in current timeline."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.track_list()
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@track_group.command('move')
@click.option('--id', '-i', required=True, help='Track ID')
@click.option('--index', '-n', type=int, required=True, help='New index position')
@click.pass_context
def track_move(ctx: click.Context, id: str, index: int):
    """Move track to new position."""
    report_unsupported(ctx, "track.move", "The Rust engine does not expose track reordering yet.")


@track_group.command('settings')
@click.option('--id', '-i', required=True, help='Track ID')
@click.option('--name', '-n', help='Track name')
@click.option('--enabled/--disabled', default=None, help='Enable/disable track')
@click.option('--locked/--unlocked', default=None, help='Lock/unlock track')
@click.option('--muted/--unmuted', default=None, help='Mute/unmute track')
@click.option('--solo/--unsolo', default=None, help='Solo/unsolo track')
@click.option('--volume', type=float, help='Volume (0.0-1.0)')
@click.option('--pan', type=float, help='Pan (-1.0 to 1.0)')
@click.option('--opacity', type=float, help='Opacity (0.0-1.0)')
@click.option('--blend-mode', type=click.Choice(['normal', 'multiply', 'screen', 'overlay', 'darken', 'lighten', 'color-dodge', 'color-burn', 'hard-light', 'soft-light', 'difference', 'exclusion', 'hue', 'saturation', 'color', 'luminosity']), help='Blend mode')
@click.pass_context
def track_settings(ctx: click.Context, id: str, name: str, enabled: bool, locked: bool, muted: bool, solo: bool, volume: float, pan: float, opacity: float, blend_mode: str):
    """Get or update track settings."""
    try:
        cli = backend.get_saturn_cli()
        if name is not None or enabled is not None or opacity is not None or blend_mode is not None:
            report_unsupported(ctx, "track.settings", "Only lock, mute, solo, gain, and pan are currently supported by the Rust command engine.")
        if all(value is None for value in (muted, solo, locked, volume, pan)):
            tracks = cli.track_list()['tracks']
            track_id = int(id) if id.isdigit() else next((item['id'] for item in tracks if item['name'] == id), None)
            result = next((item for item in tracks if item['id'] == track_id), None)
            if result is None:
                raise errors.TrackNotFoundError(id)
            format_output(result, ctx.obj['output_format'])
            return
        else:
            result = cli.track_settings(id, muted=muted, solo=solo, locked=locked, volume=volume, pan=pan)
        format_success(f"Track {id} settings updated", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
