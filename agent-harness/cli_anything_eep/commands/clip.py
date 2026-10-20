"""
Clip management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.commands import _engine
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


# ─────────────────────────────────────────────────────────────────────────────
# Engine-backed clip verbs
# ─────────────────────────────────────────────────────────────────────────────

@clip_group.command('add-effect')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--kind', '-k', required=True, type=click.Choice(['color_correction', 'gaussian_blur', 'crop', 'vignette', 'chroma_key', 'lut', 'custom_shader']), help='Effect kind')
@click.option('--name', '-n', help='Effect display name')
@click.pass_context
def clip_add_effect(ctx: click.Context, track_id: int, clip_index: int, kind: str, name):
    """Add an effect to a clip."""
    params = {"track_id": track_id, "clip_index": clip_index, "kind": kind}
    if name is not None:
        params["name"] = name
    _engine.run_engine(ctx, "clip.add_effect", params, success="Effect added")


@clip_group.command('remove-effect')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--effect-id', '-e', type=int, required=True, help='Effect id')
@click.pass_context
def clip_remove_effect(ctx: click.Context, track_id: int, clip_index: int, effect_id: int):
    """Remove an effect from a clip."""
    _engine.run_engine(
        ctx,
        "clip.remove_effect",
        {"track_id": track_id, "clip_index": clip_index, "effect_id": effect_id},
        success="Effect removed",
    )


@clip_group.command('toggle-effect')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--effect-id', '-e', type=int, required=True, help='Effect id')
@click.option('--enabled/--disabled', default=None, help='Force the effect on or off (omit to toggle)')
@click.pass_context
def clip_toggle_effect(ctx: click.Context, track_id: int, clip_index: int, effect_id: int, enabled):
    params = {"track_id": track_id, "clip_index": clip_index, "effect_id": effect_id}
    if enabled is not None:
        params["enabled"] = enabled
    _engine.run_engine(ctx, "clip.toggle_effect", params, success="Effect state changed")


@clip_group.command('reorder-effects')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--from-index', type=int, required=True, help='Current effect index')
@click.option('--to-index', type=int, required=True, help='Target effect index')
@click.pass_context
def clip_reorder_effects(ctx: click.Context, track_id: int, clip_index: int, from_index: int, to_index: int):
    """Move an effect within the clip's effect chain."""
    _engine.run_engine(
        ctx,
        "clip.reorder_effects",
        {"track_id": track_id, "clip_index": clip_index, "from_index": from_index, "to_index": to_index},
        success="Effects reordered",
    )


@clip_group.command('keyframe-set')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--property', '-p', required=True, type=click.Choice(['position_x', 'position_y', 'scale', 'rotation', 'opacity']), help='Animated property')
@click.option('--time', required=True, type=int, help='Keyframe time in ticks')
@click.option('--value', required=True, type=float, help='Property value')
@click.option('--interpolation', type=click.Choice(['linear', 'ease_in', 'ease_out', 'ease_in_out', 'bezier']), help='Interpolation')
@click.pass_context
def clip_keyframe_set(ctx: click.Context, track_id: int, clip_index: int, property: str, time: int, value: float, interpolation):
    """Set a keyframe on a clip property."""
    params = {
        "track_id": track_id,
        "clip_index": clip_index,
        "property": property,
        "time": time,
        "value": value,
    }
    if interpolation is not None:
        params["interpolation"] = interpolation
    _engine.run_engine(ctx, "clip.set_keyframe", params, success="Keyframe set")


@clip_group.command('keyframe-remove')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--property', '-p', required=True, type=click.Choice(['position_x', 'position_y', 'scale', 'rotation', 'opacity']), help='Animated property')
@click.option('--time', required=True, type=int, help='Keyframe time in ticks')
@click.pass_context
def clip_keyframe_remove(ctx: click.Context, track_id: int, clip_index: int, property: str, time: int):
    """Remove a keyframe from a clip property."""
    _engine.run_engine(
        ctx,
        "clip.remove_keyframe",
        {"track_id": track_id, "clip_index": clip_index, "property": property, "time": time},
        success="Keyframe removed",
    )


@clip_group.command('pin')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--pinned-clip-id', type=int, help='Clip id to pin to (omit to unpin)')
@click.pass_context
def clip_pin(ctx: click.Context, track_id: int, clip_index: int, pinned_clip_id):
    """Pin a clip to another clip's timeline position."""
    params = {"track_id": track_id, "clip_index": clip_index}
    if pinned_clip_id is not None:
        params["pinned_clip_id"] = pinned_clip_id
    _engine.run_engine(ctx, "clip.pin_to_track", params, success="Clip pin updated")


@clip_group.command('motion-blur')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--enabled/--disabled', default=True, help='Enable or disable motion blur')
@click.option('--shutter-angle', type=float, default=180.0, help='Shutter angle in degrees')
@click.option('--samples', type=int, default=8, help='Motion samples')
@click.pass_context
def clip_motion_blur(ctx: click.Context, track_id: int, clip_index: int, enabled: bool, shutter_angle: float, samples: int):
    """Configure motion blur for a clip."""
    _engine.run_engine(
        ctx,
        "clip.set_motion_blur",
        {
            "track_id": track_id,
            "clip_index": clip_index,
            "motion_blur": {
                "enabled": enabled,
                "shutter_angle_deg": shutter_angle,
                "samples": samples,
            },
        },
        success="Motion blur updated",
    )


@clip_group.command('effect-update')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--effect-id', '-e', type=int, required=True, help='Effect id')
@click.option('--payload', required=True, help='EffectPayload JSON')
@click.pass_context
def clip_effect_update(ctx: click.Context, track_id: int, clip_index: int, effect_id: int, payload: str):
    """Replace an effect's payload."""
    _engine.run_engine(
        ctx,
        "clip.update_effect",
        {
            "track_id": track_id,
            "clip_index": clip_index,
            "effect_id": effect_id,
            "payload": _engine.parse_json_option(payload, '--payload'),
        },
        success="Effect updated",
    )


@clip_group.command('color-wheels')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--wheels', required=True, help='ColorWheels JSON (lift/gamma/gain/offset)')
@click.pass_context
def clip_color_wheels(ctx: click.Context, track_id: int, clip_index: int, wheels: str):
    """Set primary color wheels."""
    _engine.run_engine(
        ctx,
        "clip.set_color_wheels",
        {"track_id": track_id, "clip_index": clip_index, "wheels": _engine.parse_json_option(wheels, '--wheels')},
        success="Color wheels updated",
    )


@clip_group.command('rgb-curves')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--curves', required=True, help='RgbCurves JSON')
@click.pass_context
def clip_rgb_curves(ctx: click.Context, track_id: int, clip_index: int, curves: str):
    """Set RGB curves."""
    _engine.run_engine(
        ctx,
        "clip.update_rgb_curves",
        {"track_id": track_id, "clip_index": clip_index, "curves": _engine.parse_json_option(curves, '--curves')},
        success="RGB curves updated",
    )


@clip_group.command('hsl-qualifier')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--qualifier', required=True, help='HslQualifier JSON')
@click.pass_context
def clip_hsl_qualifier(ctx: click.Context, track_id: int, clip_index: int, qualifier: str):
    """Set an HSL qualifier."""
    _engine.run_engine(
        ctx,
        "clip.update_hsl_qualifier",
        {"track_id": track_id, "clip_index": clip_index, "qualifier": _engine.parse_json_option(qualifier, '--qualifier')},
        success="HSL qualifier updated",
    )


@clip_group.command('shader-add')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--name', help='Shader preset name')
@click.option('--code', help='Shader source')
@click.option('--preset', 'preset_id', help='Built-in preset id')
@click.option('--uniforms', help='Uniform values as JSON')
@click.pass_context
def clip_shader_add(ctx: click.Context, track_id: int, clip_index: int, name, code, preset_id, uniforms):
    """Add a custom shader effect."""
    params = {"track_id": track_id, "clip_index": clip_index}
    if name is not None:
        params["name"] = name
    if code is not None:
        params["shader_code"] = code
    if preset_id is not None:
        params["preset_id"] = preset_id
    if uniforms is not None:
        params["uniforms"] = _engine.parse_json_option(uniforms, '--uniforms')
    _engine.run_engine(ctx, "clip.add_custom_shader", params, success="Custom shader added")


@clip_group.command('shader-update')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--effect-id', '-e', type=int, required=True, help='Shader effect id')
@click.option('--name', help='New preset name')
@click.option('--code', help='New shader source')
@click.option('--preset', 'preset_id', help='New built-in preset id')
@click.option('--uniforms', help='Uniform values as JSON')
@click.pass_context
def clip_shader_update(ctx: click.Context, track_id: int, clip_index: int, effect_id: int, name, code, preset_id, uniforms):
    """Update a custom shader effect."""
    params = {"track_id": track_id, "clip_index": clip_index, "effect_id": effect_id}
    if name is not None:
        params["name"] = name
    if code is not None:
        params["shader_code"] = code
    if preset_id is not None:
        params["preset_id"] = preset_id
    if uniforms is not None:
        params["uniforms"] = _engine.parse_json_option(uniforms, '--uniforms')
    _engine.run_engine(ctx, "clip.update_custom_shader", params, success="Custom shader updated")


@clip_group.command('motion-tracking')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--tracking', required=True, help='MotionTrackingData JSON')
@click.pass_context
def clip_motion_tracking(ctx: click.Context, track_id: int, clip_index: int, tracking: str):
    """Set motion tracking data for a clip."""
    _engine.run_engine(
        ctx,
        "clip.set_motion_tracking",
        {"track_id": track_id, "clip_index": clip_index, "tracking": _engine.parse_json_option(tracking, '--tracking')},
        success="Motion tracking updated",
    )


@clip_group.command('speed-ramp')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--ramp', required=True, help='SpeedRamp JSON')
@click.pass_context
def clip_speed_ramp(ctx: click.Context, track_id: int, clip_index: int, ramp: str):
    """Set a speed ramp."""
    _engine.run_engine(
        ctx,
        "clip.set_speed_ramp",
        {"track_id": track_id, "clip_index": clip_index, "speed_ramp": _engine.parse_json_option(ramp, '--ramp')},
        success="Speed ramp updated",
    )
