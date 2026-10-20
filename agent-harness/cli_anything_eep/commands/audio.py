"""
Audio mixing commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


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
        if all(value is None for value in (volume, pan, mute, solo)):
            raise errors.EepError.validation_error('mix', 'Provide at least one of --volume, --pan, --mute, or --solo')
        cli = backend.get_saturn_cli()
        result = cli.audio_mix(track_id=track, volume=volume, pan=pan, mute=mute, solo=solo)
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
    report_unsupported(ctx, "audio.effect", "Clip audio effects are not available in the Rust command catalogue; track DSP is supported separately.")


@audio_group.command('meter')
@click.option('--track', '-t', required=True, help='Track ID')
@click.pass_context
def audio_meter(ctx: click.Context, track: str):
    """Show current audio levels for track."""
    report_unsupported(ctx, "audio.meter", "Live meter readings are available only in the running desktop app, not through saturn-cli.")


@audio_group.command('automation')
@click.option('--track', '-t', required=True, help='Track ID')
@click.option('--param', '-p', type=click.Choice(['volume', 'pan', 'mute', 'solo']), required=True, help='Parameter to automate')
@click.option('--mode', '-m', type=click.Choice(['read', 'touch', 'latch', 'write']), default='read', help='Automation mode')
@click.pass_context
def audio_automation(ctx: click.Context, track: str, param: str, mode: str):
    """Set automation mode for track parameter."""
    try:
        result = backend.get_saturn_cli().track_automation(track, mode, param)
        format_success(f"Automation mode set for {track}.{param}: {mode}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
