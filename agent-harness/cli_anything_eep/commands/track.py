"""
Track management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.commands import _engine
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


# ─────────────────────────────────────────────────────────────────────────────
# Engine-backed track verbs
# ─────────────────────────────────────────────────────────────────────────────

@track_group.command('route')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--bus', '-b', required=True, type=click.Choice(['master', 'submix1', 'submix2', 'aux_fx']), help='Output bus')
@click.pass_context
def track_route(ctx: click.Context, track_id: int, bus: str):
    """Route an audio track to a bus."""
    _engine.run_engine(ctx, "track.set_bus_route", {"track_id": track_id, "bus": bus}, success="Bus route updated")


@track_group.command('processing')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--eq/--no-eq', default=None, help='Enable the channel EQ')
@click.option('--eq-low', type=float, help='Low band gain in dB')
@click.option('--eq-mid', type=float, help='Mid band gain in dB')
@click.option('--eq-high', type=float, help='High band gain in dB')
@click.option('--compressor/--no-compressor', default=None, help='Enable the compressor')
@click.option('--threshold', type=float, help='Compressor threshold in dB')
@click.option('--ratio', type=float, help='Compressor ratio')
@click.option('--denoise/--no-denoise', default=None, help='Enable denoise')
@click.option('--denoise-amount', type=float, help='Denoise amount')
@click.pass_context
def track_processing(ctx: click.Context, track_id: int, eq, eq_low, eq_mid, eq_high, compressor, threshold, ratio, denoise, denoise_amount):
    """Configure channel-strip processing for an audio track."""
    params = {"track_id": track_id}
    for key, value in (
        ("eq_enabled", eq),
        ("eq_low_gain", eq_low),
        ("eq_mid_gain", eq_mid),
        ("eq_high_gain", eq_high),
        ("comp_enabled", compressor),
        ("comp_threshold", threshold),
        ("comp_ratio", ratio),
        ("denoise_enabled", denoise),
        ("denoise_amount", denoise_amount),
    ):
        if value is not None:
            params[key] = value
    _engine.run_engine(ctx, "track.set_audio_processing", params, success="Track processing updated")


@track_group.command('plugin-add')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--plugin', '-p', required=True, help='AudioPluginInstance JSON')
@click.pass_context
def track_plugin_add(ctx: click.Context, track_id: int, plugin: str):
    """Insert an audio plugin instance on a track."""
    _engine.run_engine(
        ctx,
        "track.add_audio_plugin",
        {"track_id": track_id, "plugin": _engine.parse_json_option(plugin, '--plugin')},
        success="Audio plugin added",
    )


@track_group.command('plugin-toggle')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--plugin-id', type=int, required=True, help='Plugin instance id')
@click.option('--enabled/--disabled', default=True, help='Enable or bypass the plugin')
@click.pass_context
def track_plugin_toggle(ctx: click.Context, track_id: int, plugin_id: int, enabled: bool):
    """Enable or bypass an audio plugin."""
    _engine.run_engine(
        ctx,
        "track.toggle_audio_plugin",
        {"track_id": track_id, "plugin_id": plugin_id, "enabled": enabled},
        success="Audio plugin state changed",
    )


@track_group.command('bus-gain')
@click.option('--bus', '-b', required=True, type=click.Choice(['master', 'submix1', 'submix2', 'aux_fx']), help='Bus')
@click.option('--gain-db', type=float, required=True, help='Gain in dB')
@click.pass_context
def track_bus_gain(ctx: click.Context, bus: str, gain_db: float):
    """Set a bus output gain."""
    _engine.run_engine(ctx, "bus.set_gain_db", {"target": bus, "gain_db": gain_db}, success="Bus gain updated")
