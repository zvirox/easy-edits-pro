"""
Clip management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


@click.group(name='clip')
def clip_group():
    """Clip management commands."""
    pass


@clip_group.command('add')
@click.option('--source', '-s', required=True, help='Media file path or media ID')
@click.option('--track', '-t', required=True, help='Target track ID')
@click.option('--at', '-a', 'timeline_start', required=True, type=int, help='Timeline position in ticks')
@click.option('--duration', '-d', type=int, help='Timeline duration in ticks (default: full clip)')
@click.option('--source-in', 'source_in', type=int, default=0, help='Source in point in ticks')
@click.option('--source-out', 'source_out', type=int, help='Source out point in ticks')
@click.option('--speed', type=float, default=1.0, help='Playback speed')
@click.option('--reverse/--no-reverse', default=False, help='Reverse playback')
@click.pass_context
def clip_add(ctx: click.Context, source: str, track: str, timeline_start: int, duration: int, source_in: int, source_out: int, speed: float, reverse: bool):
    """Add a clip to a track."""
    try:
        if duration is not None and duration <= 0:
            raise errors.EepError.validation_error('duration', 'Must be greater than zero ticks')
        if source_in != 0 or source_out is not None or speed != 1.0 or reverse:
            report_unsupported(ctx, "clip.add", "Source trimming, speed, and reverse are not supported by the current Rust command engine.")
        cli = backend.get_saturn_cli()
        result = cli.clip_add(source=source, track_id=track, timeline_start=timeline_start, timeline_duration=duration)
        format_success(f"Clip added to track {track}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@clip_group.command('move')
@click.option('--id', '-i', required=True, help='Clip ID')
@click.option('--track', '-t', required=True, help='Target track ID')
@click.option('--at', '-a', 'timeline_start', required=True, type=int, help='New timeline position in ticks')
@click.pass_context
def clip_move(ctx: click.Context, id: str, track: str, timeline_start: int):
    """Move clip to different track/position."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.clip_move(clip_id=id, track_id=track, timeline_start=timeline_start)
        format_success(f"Clip {id} moved", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@clip_group.command('trim')
@click.option('--id', '-i', required=True, help='Clip ID')
@click.option('--in', 'source_in', required=True, type=int, help='New source in point in ticks')
@click.option('--out', 'source_out', required=True, type=int, help='New source out point in ticks')
@click.pass_context
def clip_trim(ctx: click.Context, id: str, source_in: int, source_out: int):
    """Trim clip source in/out points."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.clip_trim(clip_id=id, source_in=source_in, source_out=source_out)
        format_success(f"Clip {id} trimmed", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@clip_group.command('remove')
@click.option('--id', '-i', required=True, help='Clip ID')
@click.pass_context
def clip_remove(ctx: click.Context, id: str):
    """Remove clip from timeline."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.clip_remove(id)
        format_success(f"Clip {id} removed", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@clip_group.command('speed')
@click.option('--id', '-i', required=True, help='Clip ID')
@click.option('--speed', '-s', required=True, type=float, help='Playback speed (0.01-100)')
@click.pass_context
def clip_speed(ctx: click.Context, id: str, speed: float):
    """Change clip playback speed."""
    failure = errors.EepError(errors.ErrorCode.COMMAND_NOT_FOUND, "Clip speed editing is not wired to the Rust engine yet.", details={"clip": id, "speed": speed}, recoverable=False)
    format_output(failure.to_dict(), ctx.obj['output_format'])
    ctx.exit(1)


@clip_group.command('reverse')
@click.option('--id', '-i', required=True, help='Clip ID')
@click.option('--enable/--disable', default=True, help='Enable or disable reverse')
@click.pass_context
def clip_reverse(ctx: click.Context, id: str, enable: bool):
    """Toggle clip reverse playback."""
    failure = errors.EepError(errors.ErrorCode.COMMAND_NOT_FOUND, "Clip reverse playback is not wired to the Rust engine yet.", details={"clip": id, "reverse": enable}, recoverable=False)
    format_output(failure.to_dict(), ctx.obj['output_format'])
    ctx.exit(1)


@clip_group.command('list')
@click.option('--track', '-t', help='Filter by track ID')
@click.pass_context
def clip_list(ctx: click.Context, track: str):
    """List clips in current timeline."""
    try:
        cli = backend.get_saturn_cli()
        format_output(cli.clip_list(track), ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
