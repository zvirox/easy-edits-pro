# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Perception inspection and offline speech/scene analysis commands for Easy Edits Pro."""

from __future__ import annotations

from typing import Optional

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, report_unsupported


@click.group(name="perception")
def perception_group():
    """Transcript and scene inspection; unbacked actions return structured errors per A4 rule."""


@perception_group.command("transcript")
@click.option("--track-index", type=int, default=0, help="Caption track index to inspect")
@click.option("--limit", type=int, default=100, help="Maximum number of cues to return")
@click.pass_context
def perception_transcript(ctx: click.Context, track_index: int, limit: int):
    """Inspect transcript and caption cues in the active sequence."""
    try:
        cli = backend.get_saturn_cli()
        doc = cli.execute("project.inspect", {}, record_history=False)
        sequence = (doc or {}).get("project", {}).get("sequence", {})
        tracks = sequence.get("caption_tracks") or []
        if track_index >= len(tracks):
            format_output({"cues": [], "total": 0, "track_index": track_index}, ctx.obj["output_format"])
            return
        cues = tracks[track_index].get("cues", [])
        subset = cues[:limit]
        format_output(
            {
                "cues": subset,
                "total": len(cues),
                "displayed": len(subset),
                "track_index": track_index,
            },
            ctx.obj["output_format"],
        )
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@perception_group.command("scenes")
@click.option("--track-id", type=str, default=None, help="Video track ID to inspect for scene cuts")
@click.pass_context
def perception_scenes(ctx: click.Context, track_id: Optional[str]):
    """Inspect scene boundaries and cut transitions along sequence video tracks."""
    try:
        cli = backend.get_saturn_cli()
        doc = cli.execute("project.inspect", {}, record_history=False)
        sequence = (doc or {}).get("project", {}).get("sequence", {})
        tracks = sequence.get("tracks") or []
        video_tracks = [t for t in tracks if t.get("kind") == "video"]
        if track_id:
            target_tracks = [t for t in video_tracks if str(t.get("id")) == str(track_id)]
        else:
            target_tracks = video_tracks

        cuts = []
        for track in target_tracks:
            tid = track.get("id")
            tname = track.get("name", f"Track {tid}")
            clips = sorted(track.get("clips", []), key=lambda c: c.get("place", {}).get("timeline_in", 0))
            for i, clip in enumerate(clips):
                place = clip.get("place", {})
                start = place.get("timeline_in", 0)
                dur = place.get("duration", 0)
                end = start + dur
                cuts.append({
                    "track_id": tid,
                    "track_name": tname,
                    "clip_index": i,
                    "clip_id": clip.get("id"),
                    "start_tick": start,
                    "end_tick": end,
                    "duration_ticks": dur,
                })
        format_output(
            {"cuts": cuts, "total_cuts": len(cuts), "video_tracks": len(target_tracks)},
            ctx.obj["output_format"],
        )
    except errors.EepError as exc:
        format_output(exc.to_dict(), ctx.obj["output_format"])
        ctx.exit(1)


@perception_group.command("realtime")
@click.pass_context
def perception_realtime(ctx: click.Context):
    """Real-time live neural perception streaming (unbacked per A4 rule)."""
    report_unsupported(
        ctx,
        "perception.realtime",
        "Real-time streaming neural perception is unbacked; only offline timeline inspection is supported per A4 rule.",
    )


@perception_group.command("voice-activity")
@click.pass_context
def perception_voice_activity(ctx: click.Context):
    """Hardware voice-activity detection (unbacked per A4 rule)."""
    report_unsupported(
        ctx,
        "perception.voice_activity",
        "Hardware VAD stream inspection is unbacked without live hardware capture daemon per A4 rule.",
    )


@perception_group.command("face-mesh")
@click.pass_context
def perception_face_mesh(ctx: click.Context):
    """3D facial landmark mesh perception (unbacked per A4 rule)."""
    report_unsupported(
        ctx,
        "perception.face_mesh",
        "3D face mesh perception is unbacked in Phase 6 per A4 rule.",
    )
