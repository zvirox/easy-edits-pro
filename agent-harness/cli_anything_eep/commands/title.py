"""Title item commands for Easy Edits Pro."""

import click

from cli_anything_eep.commands import _engine


@click.group(name='title')
def title_group():
    """Title item commands."""
    pass


@title_group.command('update')
@click.option('--media-id', '-m', type=int, required=True, help='Title media id')
@click.option('--text', help='New primary text')
@click.option('--subtitle', help='New secondary text')
@click.option('--style', help='New style preset')
@click.option('--font-size', type=float, help='Font size in points')
@click.option('--color', help='Text color')
@click.option('--bg-color', help='Background color')
@click.pass_context
def title_update(ctx: click.Context, media_id: int, text, subtitle, style, font_size, color, bg_color):
    """Update a title item's content and styling."""
    params = {"media_id": media_id}
    for key, value in (
        ("text", text),
        ("subtitle", subtitle),
        ("style", style),
        ("font_size", font_size),
        ("color", color),
        ("bg_color", bg_color),
    ):
        if value is not None:
            params[key] = value
    _engine.run_engine(ctx, "title.update", params, success="Title updated")
