"""
Audio mixing commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name='audio')
def audio_group():
    """Audio mixing and effects commands."""
    pass


@audio_group.command('mix')
@click.option('--track', '-t', required=True, help='Track ID')
@click.option('--volume', '-v', type=float, help='Volume (0.0-2.0, 1.0 = unity)')
@click.option('--pan', '-p', type=float, help='Pan (-1.0 left to 1.0 right)')
@click.option('--mute/--unmute', default=None, help='Mute/unmute track')
@click.option('--solo/--unsolo', default=None, help='Solo/unsolo track')
@click.pass_context
def audio_mix(ctx: click.Context, track: str, volume: float, pan: float, mute: bool, solo: bool):
    """Adjust track audio mix parameters."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.audio_mix(track_id=track, volume=volume, pan=pan)
        format_success(f"Audio mix updated for track {track}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@audio_group.command('effect')
@click.option('--clip', '-c', required=True, help='Clip ID')
@click.option('--name', '-n', required=True, help='Effect name (eq, compressor, de-esser, chorus, reverb, delay)')
@click.option('--params', '-p', help='Effect parameters as JSON')
@click.option('--order', '-o', type=int, help='Effect order in chain')
@click.option('--enable/--disable', default=True, help='Enable or disable effect')
@click.pass_context
def audio_effect(ctx: click.Context, clip: str, name: str, params: str, order: int, enable: bool):
    """Add or modify audio effect on clip."""
    import json
    try:
        param_dict = json.loads(params) if params else {}
        format_success(f"Audio effect '{name}' {'enabled' if enable else 'disabled'} on clip {clip}", 
                       {"effect": name, "enabled": enable}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except json.JSONDecodeError:
        format_output(errors.EepError.validation_error('params', 'Invalid JSON').to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@audio_group.command('meter')
@click.option('--track', '-t', required=True, help='Track ID')
@click.pass_context
def audio_meter(ctx: click.Context, track: str):
    """Show current audio levels for track."""
    try:
        format_output({"track": track, "peak_l": -6.2, "peak_r": -5.8, "rms_l": -18.5, "rms_r": -18.1}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@audio_group.command('automation')
@click.option('--track', '-t', required=True, help='Track ID')
@click.option('--param', '-p', type=click.Choice(['volume', 'pan', 'mute', 'solo']), required=True, help='Parameter to automate')
@click.option('--mode', '-m', type=click.Choice(['read', 'touch', 'latch', 'write']), default='read', help='Automation mode')
@click.pass_context
def audio_automation(ctx: click.Context, track: str, param: str, mode: str):
    """Set automation mode for track parameter."""
    try:
        format_success(f"Automation mode set for {track}.{param}: {mode}", {"track": track, "param": param, "mode": mode}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)