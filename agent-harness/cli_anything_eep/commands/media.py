"""
Media management commands for Easy Edits Pro.
"""

import click
from pathlib import Path
from cli_anything_eep.commands import _engine
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


# ─────────────────────────────────────────────────────────────────────────────
# Engine-backed media verbs
# ─────────────────────────────────────────────────────────────────────────────

@media_group.command('bin-create')
@click.option('--name', '-n', required=True, help='Bin name')
@click.option('--parent-id', type=int, help='Parent bin id')
@click.option('--color', help='Bin color')
@click.pass_context
def media_bin_create(ctx: click.Context, name: str, parent_id, color):
    """Create a media bin."""
    params = {"name": name}
    if parent_id is not None:
        params["parent_id"] = parent_id
    if color is not None:
        params["color"] = color
    _engine.run_engine(ctx, "media.create_bin", params, success="Bin created")


@media_group.command('bin-rename')
@click.option('--bin', '-b', 'bin_id', type=int, required=True, help='Bin id')
@click.option('--name', '-n', required=True, help='New bin name')
@click.pass_context
def media_bin_rename(ctx: click.Context, bin_id: int, name: str):
    """Rename a media bin."""
    _engine.run_engine(ctx, "media.rename_bin", {"bin_id": bin_id, "name": name}, success="Bin renamed")


@media_group.command('bin-delete')
@click.option('--bin', '-b', 'bin_id', type=int, required=True, help='Bin id')
@click.pass_context
def media_bin_delete(ctx: click.Context, bin_id: int):
    """Delete a media bin."""
    _engine.run_engine(ctx, "media.delete_bin", {"bin_id": bin_id}, success="Bin deleted")


@media_group.command('bin-move')
@click.option('--media', '-m', 'media_id', type=int, required=True, help='Media id')
@click.option('--bin', '-b', 'bin_id', type=int, help='Target bin id (omit to move to the project root)')
@click.pass_context
def media_bin_move(ctx: click.Context, media_id: int, bin_id):
    """Move a media item into a bin."""
    params = {"media_id": media_id}
    if bin_id is not None:
        params["bin_id"] = bin_id
    _engine.run_engine(ctx, "media.move_to_bin", params, success="Media moved")


@media_group.command('update')
@click.option('--media', '-m', 'media_id', type=int, required=True, help='Media id')
@click.option('--name', '-n', help='New media name')
@click.option('--color-label', help='Color label')
@click.option('--tags', help='Comma-separated tags')
@click.pass_context
def media_update(ctx: click.Context, media_id: int, name, color_label, tags):
    """Update media metadata."""
    params = {"media_id": media_id}
    if name is not None:
        params["name"] = name
    if color_label is not None:
        params["color_label"] = color_label
    if tags is not None:
        params["tags"] = _engine.comma_list(tags)
    _engine.run_engine(ctx, "media.update_properties", params, success="Media updated")


@media_group.command('missing')
@click.pass_context
def media_missing(ctx: click.Context):
    """List media whose source file is missing."""
    _engine.run_engine(ctx, "media.check_missing", {})
