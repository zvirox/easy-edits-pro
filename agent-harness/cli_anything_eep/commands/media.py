"""
Media management commands for Easy Edits Pro.
"""

import click
from pathlib import Path
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


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
        if bin is not None or proxy:
            report_unsupported(ctx, "media.import", "Bins and proxy generation are not supported by the current Rust command engine.")
        cli = backend.get_saturn_cli()
        if Path(path).is_dir():
            report_unsupported(ctx, "media.import", "Directory import is not supported yet; select individual media files.")
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
        if limit < 1:
            raise errors.EepError.validation_error('limit', 'Must be at least 1')
        if bin is not None:
            report_unsupported(ctx, "media.search", "Filtering search results by bin is not supported yet.")
        cli = backend.get_saturn_cli()
        result = cli.media_search(query)
        result['media'] = result.get('media', [])[:limit]
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
        if bin is not None or type != 'all':
            report_unsupported(ctx, "media.list", "Bin and type filters are not supported by the current Rust media-list command.")
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
        format_output(backend.get_saturn_cli().media_bins(), ctx.obj['output_format'])
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
    report_unsupported(ctx, "media.proxy", "Proxy generation is not implemented by the Rust CLI yet.")


@media_group.command('info')
@click.option('--media-id', '-m', required=True, help='Media ID')
@click.pass_context
def media_info(ctx: click.Context, media_id: str):
    """Show detailed media information."""
    try:
        format_output(backend.get_saturn_cli().media_info(media_id), ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
