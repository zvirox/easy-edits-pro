"""Speech analysis commands for Easy Edits Pro."""

import click

from cli_anything_eep.commands import _engine


@click.group(name='speech')
def speech_group():
    """Transcription and speech-driven edits."""
    pass


@speech_group.command('transcribe')
@click.option('--segments', required=True, help='JSON array of SpeechSegment objects')
@click.option('--replace/--append', default=True, help='Replace existing subtitles')
@click.pass_context
def speech_transcribe(ctx: click.Context, segments: str, replace: bool):
    """Turn speech segments into subtitles."""
    _engine.run_engine(
        ctx,
        "speech.transcribe_to_subtitles",
        {
            "segments": _engine.parse_json_option(segments, '--segments'),
            "replace_existing": replace,
        },
        success="Subtitles generated",
    )


@speech_group.command('ripple-delete-silence')
@click.option('--peaks', required=True, help='JSON array of waveform peak bytes')
@click.option('--options', help='SilenceDetectOptions JSON')
@click.pass_context
def speech_ripple_delete_silence(ctx: click.Context, peaks: str, options):
    """Remove detected silence from the timeline."""
    params = {"peaks": _engine.parse_json_option(peaks, '--peaks')}
    if options is not None:
        params["options"] = _engine.parse_json_option(options, '--options')
    _engine.run_engine(ctx, "speech.ripple_delete_silence", params, success="Silence removed")


@speech_group.command('ripple-delete-fillers')
@click.option('--segments', required=True, help='JSON array of SpeechSegment objects')
@click.option('--custom-fillers', help='Comma-separated extra filler words')
@click.pass_context
def speech_ripple_delete_fillers(ctx: click.Context, segments: str, custom_fillers):
    """Remove filler words from the timeline."""
    params = {"segments": _engine.parse_json_option(segments, '--segments')}
    if custom_fillers is not None:
        params["custom_fillers"] = _engine.comma_list(custom_fillers)
    _engine.run_engine(ctx, "speech.ripple_delete_fillers", params, success="Filler words removed")
