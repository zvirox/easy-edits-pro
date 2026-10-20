"""Interchange import commands for Easy Edits Pro."""

from pathlib import Path

import click

from cli_anything_eep.commands import _engine
from cli_anything_eep.core import errors


def _read_input(file, text, label):
    if file is not None:
        return Path(file).read_text(encoding="utf-8")
    if text is not None:
        return text
    raise errors.EepError(
        errors.ErrorCode.INVALID_ARGUMENT,
        f"{label} requires --file or --text",
    )


@click.group(name='interop')
def interop_group():
    """Import EDL, FCPXML, OTIO, and Premiere XML timelines."""
    pass


@interop_group.command('edl')
@click.option('--file', type=click.Path(exists=True, dir_okay=False), help='EDL file')
@click.option('--text', help='EDL contents')
@click.option('--project-name', help='Project name override')
@click.pass_context
def interop_edl(ctx: click.Context, file, text, project_name):
    """Import a CMX EDL."""
    params = {"edl": _read_input(file, text, "interop edl")}
    if project_name is not None:
        params["project_name"] = project_name
    _engine.run_engine(ctx, "interop.import_edl", params, success="EDL imported")


@interop_group.command('fcpxml')
@click.option('--file', type=click.Path(exists=True, dir_okay=False), help='FCPXML file')
@click.option('--text', help='FCPXML contents')
@click.pass_context
def interop_fcpxml(ctx: click.Context, file, text):
    """Import a Final Cut Pro XML timeline."""
    _engine.run_engine(
        ctx,
        "interop.import_fcpxml",
        {"xml": _read_input(file, text, "interop fcpxml")},
        success="FCPXML imported",
    )


@interop_group.command('premiere')
@click.option('--file', type=click.Path(exists=True, dir_okay=False), help='Premiere XML file')
@click.option('--text', help='Premiere XML contents')
@click.pass_context
def interop_premiere(ctx: click.Context, file, text):
    """Import a Premiere Pro XML timeline."""
    _engine.run_engine(
        ctx,
        "interop.import_premiere_xml",
        {"xml": _read_input(file, text, "interop premiere")},
        success="Premiere XML imported",
    )


@interop_group.command('otio')
@click.option('--file', type=click.Path(exists=True, dir_okay=False), help='OTIO JSON file')
@click.option('--text', help='OTIO JSON contents')
@click.option('--project-name', help='Project name override')
@click.pass_context
def interop_otio(ctx: click.Context, file, text, project_name):
    """Import an OpenTimelineIO document."""
    params = {"content": _read_input(file, text, "interop otio")}
    if project_name is not None:
        params["project_name"] = project_name
    _engine.run_engine(ctx, "interop.import_otio", params, success="OTIO imported")
