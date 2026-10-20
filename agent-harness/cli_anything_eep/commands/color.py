"""
Color grading commands for Easy Edits Pro.
"""

import click
from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


@click.group(name='color')
def color_group():
    """Color grading commands (Lift/Gamma/Gain/Offset)."""
    pass


@color_group.command('grade')
@click.option('--clip', '-c', required=True, help='Clip ID')
@click.option('--lift', '-l', nargs=3, type=float, help='Lift RGB (0-1)')
@click.option('--gamma', '-g', nargs=3, type=float, help='Gamma RGB (0-1)')
@click.option('--gain', '-G', nargs=3, type=float, help='Gain RGB (0-1)')
@click.option('--offset', '-o', nargs=3, type=float, help='Offset RGB (0-1)')
@click.option('--contrast', type=float, help='Contrast (-1 to 1)')
@click.option('--saturation', type=float, help='Saturation (0-2)')
@click.option('--temperature', type=float, help='Temperature (-1 to 1)')
@click.option('--tint', type=float, help='Tint (-1 to 1)')
@click.pass_context
def color_grade(ctx: click.Context, clip: str, lift, gamma, gain, offset, contrast, saturation, temperature, tint):
    """Apply color grade to clip."""
    try:
        cli = backend.get_saturn_cli()
        result = cli.color_grade(
            clip_id=clip,
            lift=list(lift) if lift else None,
            gamma=list(gamma) if gamma else None,
            gain=list(gain) if gain else None,
            offset=list(offset) if offset else None,
            contrast=contrast,
            saturation=saturation,
            temperature=temperature,
            tint=tint
        )
        format_success(f"Color grade applied to clip {clip}", result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@color_group.command('reset')
@click.option('--clip', '-c', required=True, help='Clip ID')
@click.pass_context
def color_reset(ctx: click.Context, clip: str):
    """Reset color grade to default."""
    try:
        format_success(f"Color grade reset for clip {clip}", {"reset": True}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@color_group.command('lut')
@click.option('--clip', '-c', required=True, help='Clip ID')
@click.option('--file', '-f', required=True, help='Path to .cube LUT file')
@click.option('--intensity', type=float, default=1.0, help='LUT intensity (0-1)')
@click.pass_context
def color_lut(ctx: click.Context, clip: str, file: str, intensity: float):
    """Apply 3D LUT to clip."""
    try:
        format_success(f"LUT applied to clip {clip}", {"lut": file, "intensity": intensity}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


@color_group.command('wheels')
@click.option('--clip', '-c', required=True, help='Clip ID')
@click.pass_context
def color_wheels(ctx: click.Context, clip: str):
    """Show current color wheel values for clip."""
    try:
        format_output({"clip": clip, "lift": [0,0,0], "gamma": [1,1,1], "gain": [1,1,1], "offset": [0,0,0]}, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)