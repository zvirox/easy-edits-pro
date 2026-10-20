"""
Timeline management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.commands import _engine
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


# ─────────────────────────────────────────────────────────────────────────────
# Engine-backed editing verbs (each maps to one registry command id)
# ─────────────────────────────────────────────────────────────────────────────

@timeline_group.command('split')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--at', type=int, required=True, help='Split position in ticks')
@click.pass_context
def timeline_split(ctx: click.Context, track_id: int, clip_index: int, at: int):
    """Split a clip at a timeline tick position."""
    _engine.run_engine(
        ctx,
        "timeline.split_clip",
        {"track_id": track_id, "clip_index": clip_index, "at": at},
        success="Clip split",
    )


@timeline_group.command('ripple-delete')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.pass_context
def timeline_ripple_delete(ctx: click.Context, track_id: int, clip_index: int):
    """Delete a clip and close the gap on its track."""
    _engine.run_engine(
        ctx,
        "timeline.ripple_delete",
        {"track_id": track_id, "clip_index": clip_index},
        success="Clip ripple-deleted",
    )


@timeline_group.command('slip')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--delta', '-d', type=int, required=True, help='Source shift in ticks')
@click.pass_context
def timeline_slip(ctx: click.Context, track_id: int, clip_index: int, delta: int):
    """Slip a clip's source without moving it on the timeline."""
    _engine.run_engine(
        ctx,
        "timeline.slip_clip",
        {"track_id": track_id, "clip_index": clip_index, "delta": delta},
        success="Clip slipped",
    )


@timeline_group.command('slide')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--delta', '-d', type=int, required=True, help='Timeline shift in ticks')
@click.pass_context
def timeline_slide(ctx: click.Context, track_id: int, clip_index: int, delta: int):
    """Slide a clip and its neighbours on the timeline."""
    _engine.run_engine(
        ctx,
        "timeline.slide_clip",
        {"track_id": track_id, "clip_index": clip_index, "delta": delta},
        success="Clip slid",
    )


@timeline_group.command('roll')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--left-clip-index', '-c', type=int, required=True, help='Index of the left clip at the cut')
@click.option('--delta', '-d', type=int, required=True, help='Cut shift in ticks')
@click.pass_context
def timeline_roll(ctx: click.Context, track_id: int, left_clip_index: int, delta: int):
    """Move an edit point between two adjacent clips."""
    _engine.run_engine(
        ctx,
        "timeline.roll_edit",
        {"track_id": track_id, "left_clip_index": left_clip_index, "delta": delta},
        success="Edit point rolled",
    )


@timeline_group.command('rate-stretch')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Clip index on the track')
@click.option('--duration', type=int, help='New duration in ticks')
@click.option('--speed', type=float, help='Playback speed multiplier')
@click.pass_context
def timeline_rate_stretch(ctx: click.Context, track_id: int, clip_index: int, duration, speed):
    """Change a clip's duration or speed."""
    params = {"track_id": track_id, "clip_index": clip_index}
    if duration is not None:
        params["new_duration"] = duration
    if speed is not None:
        params["speed"] = speed
    _engine.run_engine(ctx, "timeline.rate_stretch", params, success="Clip rate stretched")


@timeline_group.command('add-transition')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--cut-at', type=int, required=True, help='Cut position in ticks')
@click.option('--duration', type=int, help='Transition duration in ticks')
@click.option('--kind', type=click.Choice(['crossfade', 'dip_to_black', 'dip_to_white', 'wipe_left', 'wipe_right', 'slide_left', 'slide_right']), default='crossfade', help='Transition kind')
@click.pass_context
def timeline_add_transition(ctx: click.Context, track_id: int, cut_at: int, duration, kind: str):
    """Add a transition at a cut."""
    params = {"track_id": track_id, "cut_at": cut_at, "kind": kind}
    if duration is not None:
        params["duration"] = duration
    _engine.run_engine(ctx, "timeline.add_transition", params, success="Transition added")


@timeline_group.command('remove-transition')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--transition-id', type=int, required=True, help='Transition id')
@click.pass_context
def timeline_remove_transition(ctx: click.Context, track_id: int, transition_id: int):
    """Remove a transition by id."""
    _engine.run_engine(
        ctx,
        "timeline.remove_transition",
        {"track_id": track_id, "transition_id": transition_id},
        success="Transition removed",
    )


@timeline_group.command('add-title')
@click.option('--text', required=True, help='Title text')
@click.option('--subtitle', help='Secondary title text')
@click.option('--style', help='Title style preset')
@click.option('--duration', type=int, help='Title duration in ticks')
@click.option('--track', '-t', 'track_id', type=int, help='Target track id')
@click.option('--at', type=int, help='Timeline start in ticks')
@click.pass_context
def timeline_add_title(ctx: click.Context, text: str, subtitle, style, duration, track_id, at):
    """Create a title item and place it on a track."""
    params = {"text": text}
    if subtitle is not None:
        params["subtitle"] = subtitle
    if style is not None:
        params["style"] = style
    if duration is not None:
        params["duration"] = duration
    if track_id is not None:
        params["track_id"] = track_id
    if at is not None:
        params["timeline_start"] = at
    _engine.run_engine(ctx, "timeline.add_title", params, success="Title added")


@timeline_group.command('mark-in')
@click.option('--at', type=int, required=True, help='Mark position in ticks')
@click.pass_context
def timeline_mark_in(ctx: click.Context, at: int):
    """Set the sequence in point."""
    _engine.run_engine(ctx, "timeline.set_mark_in", {"at": at}, success="In point set")


@timeline_group.command('mark-out')
@click.option('--at', type=int, required=True, help='Mark position in ticks')
@click.pass_context
def timeline_mark_out(ctx: click.Context, at: int):
    """Set the sequence out point."""
    _engine.run_engine(ctx, "timeline.set_mark_out", {"at": at}, success="Out point set")


@timeline_group.command('clear-marks')
@click.pass_context
def timeline_clear_marks(ctx: click.Context):
    """Clear the sequence in and out points."""
    _engine.run_engine(ctx, "timeline.clear_marks", {}, success="Marks cleared")


@timeline_group.command('ripple-delete-gap')
@click.option('--at', type=int, required=True, help='Tick inside the gap')
@click.option('--track', '-t', 'track_id', type=int, help='Restrict to one track')
@click.pass_context
def timeline_ripple_delete_gap(ctx: click.Context, at: int, track_id):
    """Close the gap at a tick position."""
    params = {"at_tick": at}
    if track_id is not None:
        params["track_id"] = track_id
    _engine.run_engine(ctx, "timeline.ripple_delete_gap", params, success="Gap closed")


@timeline_group.command('ripple-delete-ranges')
@click.option('--ranges', required=True, help='Comma-separated start:end tick ranges')
@click.pass_context
def timeline_ripple_delete_ranges(ctx: click.Context, ranges: str):
    """Delete several tick ranges and ripple the timeline."""
    _engine.run_engine(
        ctx,
        "timeline.ripple_delete_ranges",
        {"ranges": _engine.tick_ranges(ranges)},
        success="Ranges deleted",
    )


@timeline_group.command('link')
@click.option('--clips', required=True, help='Comma-separated track:index references')
@click.pass_context
def timeline_link(ctx: click.Context, clips: str):
    """Link clips so they move together."""
    _engine.run_engine(ctx, "timeline.link_clips", {"clips": _engine.clip_refs(clips)}, success="Clips linked")


@timeline_group.command('unlink')
@click.option('--clips', required=True, help='Comma-separated track:index references')
@click.pass_context
def timeline_unlink(ctx: click.Context, clips: str):
    """Unlink previously linked clips."""
    _engine.run_engine(ctx, "timeline.unlink_clips", {"clips": _engine.clip_refs(clips)}, success="Clips unlinked")


@timeline_group.command('move-batch')
@click.option('--moves', required=True, help='JSON array of {track_id, clip_index, timeline_start}')
@click.pass_context
def timeline_move_batch(ctx: click.Context, moves: str):
    """Move several clips in one atomic edit."""
    _engine.run_engine(ctx, "timeline.move_clips_batch", {"moves": _engine.parse_json_option(moves, '--moves')}, success="Clips moved")


@timeline_group.command('compound')
@click.option('--name', default='Compound Clip', help='Compound clip name')
@click.option('--target-track', type=int, help='Track that receives the compound clip')
@click.option('--clips', required=True, help='Comma-separated track:index references')
@click.pass_context
def timeline_compound(ctx: click.Context, name: str, target_track, clips: str):
    """Collapse clips into a compound clip."""
    params = {"name": name, "clips": _engine.clip_refs(clips)}
    if target_track is not None:
        params["target_track_id"] = target_track
    _engine.run_engine(ctx, "timeline.create_compound_clip", params, success="Compound clip created")


@timeline_group.command('decompose')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--clip-index', '-c', type=int, required=True, help='Compound clip index')
@click.pass_context
def timeline_decompose(ctx: click.Context, track_id: int, clip_index: int):
    """Expand a compound clip back into its source clips."""
    _engine.run_engine(
        ctx,
        "timeline.decompose_compound_clip",
        {"track_id": track_id, "clip_index": clip_index},
        success="Compound clip decomposed",
    )


@timeline_group.command('group-create')
@click.option('--name', '-n', required=True, help='Group name')
@click.option('--color', help='Group color')
@click.pass_context
def timeline_group_create(ctx: click.Context, name: str, color):
    """Create a track group."""
    params = {"name": name}
    if color is not None:
        params["color"] = color
    _engine.run_engine(ctx, "timeline.create_track_group", params, success="Track group created")


@timeline_group.command('group-assign')
@click.option('--track', '-t', 'track_id', type=int, required=True, help='Track id')
@click.option('--group', '-g', 'group_id', type=int, help='Group id (omit to ungroup)')
@click.pass_context
def timeline_group_assign(ctx: click.Context, track_id: int, group_id):
    params = {"track_id": track_id}
    if group_id is not None:
        params["group_id"] = group_id
    _engine.run_engine(ctx, "timeline.assign_track_group", params, success="Track group assignment updated")


@timeline_group.command('group-delete')
@click.option('--group', '-g', 'group_id', type=int, required=True, help='Group id')
@click.pass_context
def timeline_group_delete(ctx: click.Context, group_id: int):
    _engine.run_engine(ctx, "timeline.delete_track_group", {"group_id": group_id}, success="Track group deleted")


@timeline_group.command('group-toggle')
@click.option('--group', '-g', 'group_id', type=int, required=True, help='Group id')
@click.pass_context
def timeline_group_toggle(ctx: click.Context, group_id: int):
    _engine.run_engine(
        ctx,
        "timeline.toggle_track_group_collapsed",
        {"group_id": group_id},
        success="Track group collapsed state toggled",
    )


@timeline_group.command('group-properties')
@click.option('--group', '-g', 'group_id', type=int, required=True, help='Group id')
@click.option('--name', help='New group name')
@click.option('--color', help='New group color')
@click.option('--muted/--unmuted', default=None, help='Mute every track in the group')
@click.option('--solo/--unsolo', default=None, help='Solo every track in the group')
@click.option('--locked/--unlocked', default=None, help='Lock every track in the group')
@click.pass_context
def timeline_group_properties(ctx: click.Context, group_id: int, name, color, muted, solo, locked):
    params = {"group_id": group_id}
    for key, value in (("name", name), ("color", color), ("muted", muted), ("solo", solo), ("locked", locked)):
        if value is not None:
            params[key] = value
    _engine.run_engine(ctx, "timeline.set_track_group_properties", params, success="Track group updated")


@timeline_group.command('ripple-delete-batch')
@click.option('--clips', required=True, help='Comma-separated track:index references')
@click.option('--ripple/--no-ripple', default=True, help='Close the resulting gaps')
@click.pass_context
def timeline_ripple_delete_batch(ctx: click.Context, clips: str, ripple: bool):
    """Delete several clips in one atomic edit."""
    _engine.run_engine(
        ctx,
        "timeline.ripple_delete_batch",
        {"clips": _engine.clip_refs(clips), "ripple": ripple},
        success="Clips deleted",
    )
