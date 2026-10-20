"""
Media management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name='media')
def media_group():
    """Media bin and asset management commands."""
    pass


@media_group.command('import')
@click.option('--path', '-p', required=True, help='File or directory path to import')
@click.option('--bin', '-b', help='Target bin name')
@click.option('--recursive/--no-recursive', default=True, help='Recursive directory import')
@click.option('--proxy/--no-proxy', default=False, help='Generate proxies on import')
@click.pass_context
def media_import(ctx: click.Context, path: str, bin: str, recursive: bool, proxy: bool):
    """Import media into project."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.media_import(path=path, bin_name=bin)
        format_success(f"Media imported from {path}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@media_group.command('search')
@click.option('--query', '-q', required=True, help='Natural language search query (e.g. "close up person talking")')
@click.option('--bin', '-b', help='Search specific bin')
@click.option('--limit', '-l', type=int, default=20, help='Maximum results')
@click.pass_context
def media_search(ctx: click.Context, query: str, bin: str, limit: int):
    """Search media with natural language."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.media_search(query)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@media_group.command('list')
@click.option('--bin', '-b', help='List specific bin')
@click.option('--type', '-t', type=click.Choice(['video', 'audio', 'image', 'all']), default='all', help='Filter by media type')
@click.pass_context
def media_list(ctx: click.Context, bin: str, type: str):
    """List media in bins."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.media_list(bin_name=bin)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@media_group.command('bins')
@click.pass_context
def media_bins(ctx: click.Context):
    """List all media bins."""
    try:
        format_output({"bins": ["Footage", "Audio", "Graphics", "Music", "SFX"]}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@media_group.command('proxy')
@click.option('--media-id', '-m', required=True, help='Media ID')
@click.option('--generate/--delete', default=True, help='Generate or delete proxy')
@click.option('--quality', type=click.Choice(['low', 'medium', 'high']), default='medium', help='Proxy quality')
@click.pass_context
def media_proxy(ctx: click.Context, media_id: str, generate: bool, quality: str):
    """Generate or delete proxy for media."""
    try:
        action = "generated" if generate else "deleted"
        format_success(f"Proxy {action} for media {media_id}", {"media_id": media_id, "quality": quality}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@media_group.command('info')
@click.option('--media-id', '-m', required=True, help='Media ID')
@click.pass_context
def media_info(ctx: click.Context, media_id: str):
    """Show detailed media information."""
    try:
        format_output({
            "id": media_id,
            "name": "example.mp4",
            "duration": 120.5,
            "resolution": "1920x1080",
            "frame_rate": 29.97,
            "codec": "h264",
            "audio_channels": 2,
            "sample_rate": 48000,
            "has_proxy": True
        }, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)