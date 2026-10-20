"""
Render commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success, report_unsupported


@click.group(name='render')
def render_group():
    """Render and export commands."""
    pass


@render_group.command('queue')
@click.option('--preset', '-p', type=click.Choice(['youtube', 'tiktok', 'cinema-4k', 'proxies', 'custom']), required=True, help='Render preset')
@click.option('--output', '-o', required=True, help='Output file path')
@click.option('--timeline', '-t', help='Timeline ID (default: current)')
@click.option('--priority', type=click.Choice(['immediate', 'high', 'normal', 'background']), default='normal', help='Queue priority')
@click.pass_context
def render_queue(ctx: click.Context, preset: str, output: str, timeline: str, priority: str):
    """Queue a render job."""
    try:
        if timeline is not None or priority != 'normal':
            report_unsupported(ctx, "render.queue", "Selecting a timeline or queue priority is not exposed by the desktop render bridge yet.")
        cli = backend.get_saturn_cli()
        result = cli.render_queue(preset=preset, output=output, timeline_id=timeline)
        format_success(f"Render queued: {output}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@render_group.command('status')
@click.option('--job-id', '-j', required=True, type=int, help='Render job ID returned by render queue')
@click.pass_context
def render_status(ctx: click.Context, job_id: int):
    """Show render queue status."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.render_status(job_id)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@render_group.command('cancel')
@click.option('--job-id', '-j', required=True, help='Job ID to cancel')
@click.pass_context
def render_cancel(ctx: click.Context, job_id: str):
    """Cancel a render job."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.render_cancel(int(job_id))
        format_success(f"Cancellation requested for render job {job_id}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except ValueError:
        failure = errors.EepError.validation_error('job_id', 'Render job IDs must be integers')
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except Exception as error:
        failure = errors.EepError.from_backend_error(error, "Could not cancel render")
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@render_group.command('presets')
@click.pass_context
def render_presets(ctx: click.Context):
    """List available render presets."""
    presets = [
        {"name": "youtube", "description": "YouTube 1080p30 H.264", "codec": "h264", "bitrate": "8M"},
        {"name": "tiktok", "description": "TikTok/Reels 9:16 H.264", "codec": "h264", "bitrate": "5M"},
        {"name": "cinema-4k", "description": "Cinema 4K ProRes", "codec": "prores", "bitrate": "lossless"},
        {"name": "proxies", "description": "Low-res proxies for editing", "codec": "h264", "bitrate": "1M"},
    ]
    format_output(presets, ctx.obj['output_format'])
