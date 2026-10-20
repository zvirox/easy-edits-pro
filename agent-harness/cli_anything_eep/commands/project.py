"""
Project management commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors, state
from cli_anything_eep.output import format_output, format_success


@click.group(name='project')
def project_group():
    """Project management commands."""
    pass


@project_group.command('create')
@click.option('--name', '-n', required=True, help='Project name')
@click.option('--template', '-t', type=click.Choice(['youtube-1080p', 'cinema-4k', 'vertical-9x16', 'broadcast', 'custom']), default='youtube-1080p', help='Project template')
@click.option('--path', '-p', type=click.Path(), help='Output path (default: ./<name>.eep)')
@click.option('--fps', type=float, default=30, help='Frame rate')
@click.option('--width', type=int, default=1920, help='Width in pixels')
@click.option('--height', type=int, default=1080, help='Height in pixels')
@click.pass_context
def project_create(ctx: click.Context, name: str, template: str, path: str, fps: float, width: int, height: int):
    """Create a new Easy Edits Pro project."""
    try:
        cli = backend.get_saturn_cli()
        path = path or f"{name.replace('/', '_').replace(chr(92), '_')}.eep"
        result = cli.project_create(name=name, template=template, path=path, fps=fps, width=width, height=height)
        ctx.obj['project_state'] = state.ProjectState.load(path)
        ctx.obj['project_state'].save()
        format_success(f"Project '{name}' created", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except ValueError as e:
        failure = errors.EepError(errors.ErrorCode.PROJECT_INVALID, str(e), details={"path": path}, recoverable=False)
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except Exception as e:
        failure = errors.EepError.from_backend_error(e, "Could not create project")
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@project_group.command('open')
@click.argument('path', type=click.Path(exists=True))
@click.pass_context
def project_open(ctx: click.Context, path: str):
    """Open an existing project."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.project_open(path)
        ctx.obj['project_state'] = state.ProjectState.load(path)
        ctx.obj['project_state'].save()
        format_success(f"Project opened", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except ValueError as e:
        failure = errors.EepError(errors.ErrorCode.PROJECT_INVALID, str(e), details={"path": path}, recoverable=False)
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)
    except Exception as e:
        failure = errors.EepError.from_backend_error(e, "Could not open project")
        format_output(failure.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@project_group.command('save')
@click.option('--path', '-p', type=click.Path(), help='Save path (default: current project)')
@click.pass_context
def project_save(ctx: click.Context, path: str):
    """Save current project."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.project_save(path)
        if path:
            ctx.obj['project_state'] = state.ProjectState.load(path)
            ctx.obj['project_state'].save()
        format_success("Project saved", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@project_group.command('close')
@click.pass_context
def project_close(ctx: click.Context):
    """Close current project."""
    failure = errors.EepError(errors.ErrorCode.COMMAND_NOT_FOUND, "Project close is not supported by the Rust editor engine yet.", recoverable=False)
    format_output(failure.to_dict(), ctx.obj['output_format'])
    ctx.exit(1)


@project_group.command('inspect')
@click.argument('path', type=click.Path(exists=True))
@click.pass_context
def project_inspect(ctx: click.Context, path: str):
    """Inspect project file metadata."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.project_inspect(path)
        format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@project_group.command('templates')
@click.pass_context
def project_templates(ctx: click.Context):
    """List available project templates."""
    templates = [
        {"name": "youtube-1080p", "description": "YouTube 1080p30", "fps": 30, "resolution": "1920x1080"},
        {"name": "cinema-4k", "description": "Cinema 4K 24fps", "fps": 24, "resolution": "4096x2160"},
        {"name": "vertical-9x16", "description": "TikTok/Reels 9:16", "fps": 30, "resolution": "1080x1920"},
        {"name": "broadcast", "description": "Broadcast 1080i/59.94", "fps": 29.97, "resolution": "1920x1080"},
    ]
    format_output(templates, ctx.obj['output_format'])
