"""Subtitle track commands for Easy Edits Pro."""

import click

from cli_anything_eep.commands import _engine
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output


@click.group(name='subtitle')
def subtitle_group():
    """Subtitle track commands."""
    pass


@subtitle_group.command('add')
@click.option('--start', required=True, type=int, help='Start tick')
@click.option('--end', required=True, type=int, help='End tick')
@click.option('--text', required=True, help='Subtitle text')
@click.pass_context
def subtitle_add(ctx: click.Context, start: int, end: int, text: str):
    """Add a subtitle."""
    _engine.run_engine(
        ctx,
        "subtitle.add",
        {"start_ticks": start, "end_ticks": end, "text": text},
        success="Subtitle added",
    )


@subtitle_group.command('update')
@click.option('--id', '-i', 'subtitle_id', type=int, required=True, help='Subtitle id')
@click.option('--start', type=int, help='New start tick')
@click.option('--end', type=int, help='New end tick')
@click.option('--text', help='New subtitle text')
@click.pass_context
def subtitle_update(ctx: click.Context, subtitle_id: int, start, end, text):
    """Update a subtitle's timing or text."""
    params = {"id": subtitle_id}
    if start is not None:
        params["start_ticks"] = start
    if end is not None:
        params["end_ticks"] = end
    if text is not None:
        params["text"] = text
    _engine.run_engine(ctx, "subtitle.update", params, success="Subtitle updated")


@subtitle_group.command('remove')
@click.option('--id', '-i', 'subtitle_id', type=int, required=True, help='Subtitle id')
@click.pass_context
def subtitle_remove(ctx: click.Context, subtitle_id: int):
    """Remove a subtitle."""
    _engine.run_engine(ctx, "subtitle.remove", {"id": subtitle_id}, success="Subtitle removed")


@subtitle_group.command('clear')
@click.pass_context
def subtitle_clear(ctx: click.Context):
    """Remove every subtitle in the active sequence."""
    _engine.run_engine(ctx, "subtitle.clear", {}, success="Subtitles cleared")


@subtitle_group.command('list')
@click.pass_context
def subtitle_list(ctx: click.Context):
    """List subtitles in the active sequence."""
    try:
        document = backend.get_saturn_cli().execute("project.inspect", {}, record_history=False)
        sequence = (document or {}).get("project", {}).get("sequence", {})
        format_output({"subtitles": sequence.get("subtitles", [])}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
