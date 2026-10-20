"""Multicam commands for Easy Edits Pro."""

import click

from cli_anything_eep.commands import _engine


@click.group(name='multicam')
def multicam_group():
    """Multicam creation, sync, and angle switching."""
    pass


@multicam_group.command('create')
@click.option('--name', '-n', required=True, help='Multicam clip name')
@click.option('--angles', required=True, help='JSON array of MultiCamAngle objects')
@click.option('--sync-mode', type=click.Choice(['timecode', 'audio_waveform', 'marker']), default='timecode', help='Sync mode')
@click.pass_context
def multicam_create(ctx: click.Context, name: str, angles: str, sync_mode: str):
    """Create a multicam source from angles."""
    _engine.run_engine(
        ctx,
        "multicam.create",
        {"name": name, "angles": _engine.parse_json_option(angles, '--angles'), "sync_mode": sync_mode},
        success="Multicam created",
    )


@multicam_group.command('sync')
@click.option('--multicam-id', type=int, required=True, help='Multicam id')
@click.option('--sync-mode', type=click.Choice(['timecode', 'audio_waveform', 'marker']), required=True, help='Sync mode')
@click.option('--offsets', help='JSON object of angle offsets')
@click.pass_context
def multicam_sync(ctx: click.Context, multicam_id: int, sync_mode: str, offsets):
    """Re-sync a multicam clip."""
    params = {"multicam_id": multicam_id, "sync_mode": sync_mode}
    if offsets is not None:
        params["offsets"] = _engine.parse_json_option(offsets, '--offsets')
    _engine.run_engine(ctx, "multicam.sync", params, success="Multicam synced")


@multicam_group.command('switch')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Multicam clip index')
@click.option('--angle-id', type=int, required=True, help='Angle to show')
@click.pass_context
def multicam_switch(ctx: click.Context, track_id: int, clip_index: int, angle_id: int):
    """Switch the active angle of a multicam clip."""
    _engine.run_engine(
        ctx,
        "multicam.switch_angle",
        {"track_id": track_id, "clip_index": clip_index, "angle_id": angle_id},
        success="Multicam angle switched",
    )
