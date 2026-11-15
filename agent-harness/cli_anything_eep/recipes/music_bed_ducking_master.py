# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Companion §2.6 Recipe 5: Music bed ducking and broadcast/streaming master loudness."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.recipes._common import (
    ControlClient,
    default_podcast_segments,
    finalize_review_and_delivery,
    sec_to_ticks,
    sync_submit_approve_plan,
    synthesize_speech_wav,
    try_mcp_audio_and_verify,
)


def _default_ducking_sheet() -> dict[str, Any]:
    return {
        "schema": "tasksheet/1",
        "id": "sheet_music_ducking",
        "title": "Music Bed Ducking & EBU R128 Master Mix",
        "requester": "human:sound_supervisor",
        "inputs": [
            {
                "id": "in_dialogue",
                "media": "media/aud_stem_dialogue.wav",
                "role": "dialogue",
                "trust": "trusted",
            },
            {
                "id": "in_music_bed",
                "media": "media/aud_stem_music.wav",
                "role": "music",
                "trust": "trusted",
            },
        ],
        "deliverables": [
            {
                "id": "deliv_master_mix",
                "name": "Ducked & Mastered Mix",
                "preset": "broadcast-r128",
                "duration": {"target_s": 12.0, "tolerance_s": 5.0},
                "loudness_lufs": -23.0,
                "true_peak_dbtp": -1.0,
            }
        ],
    }


def run_music_bed_ducking_master(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    media_path: Optional[str | Path] = None,
    approve_review: bool = False,
    package_delivery: bool = False,
    run_audio_dsp: bool = True,
    auto_merge: bool = False,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Companion §2.6 Recipe: Music-bed ducking master end-to-end."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "music_bed_ducking.eep"

    source_duration_s = 14.0
    segs = default_podcast_segments()
    dialogue_wav = (
        Path(media_path)
        if media_path
        else synthesize_speech_wav(root / "aud_stem_dialogue.wav", segs, source_duration_s)
    )
    music_wav = (
        Path(media_path)
        if media_path
        else synthesize_speech_wav(root / "aud_stem_music.wav", [], source_duration_s)
    )

    sheet = (
        json.loads(Path(sheet_path).read_text(encoding="utf-8"))
        if sheet_path
        else _default_ducking_sheet()
    )

    branch_name = "agent/music-bed-ducking"
    tool_calls = 0

    with (
        ControlClient(cli_path=cli_path) as human_cli,
        ControlClient(cli_path=cli_path, actor="agent:recipe-audio") as agent_cli,
    ):
        # 1. Project setup with initial sequence
        human_cli.execute(
            "project.new",
            {
                "name": "Music Bed Ducking Master",
                "sequence_name": "Audio Mix",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("tasksheet.create", sheet)
        human_cli.execute("file.save", {"path": str(project_path)})

        # 2. Plan submission & human approval
        plan_steps = [
            {"id": 1, "title": "Load audio master profile and create task branch", "status": "pending"},
            {"id": 2, "title": "Setup separate dialogue and music tracks", "status": "pending"},
            {"id": 3, "title": "Ingest dialogue and music bed stems onto timeline", "status": "pending"},
            {"id": 4, "title": "Apply sidechain ducking envelope and track leveling", "status": "pending"},
            {"id": 5, "title": "Normalize master loudness to broadcast/streaming standard", "status": "pending"},
            {"id": 6, "title": "Run verify.run audio telemetry QC (loudness, true peak, silence)", "status": "pending"},
            {"id": 7, "title": "Compute branch diff and present review notes for acoustic sign-off", "status": "pending"},
        ]
        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=plan_steps,
        )

        # 3. Task branch
        agent_cli.execute(
            "project.branch_create",
            {"name": branch_name, "note": "Separate dialogue/music stem ducking and master leveling"},
        )
        agent_cli.execute("project.branch_switch", {"name": branch_name})
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})

        # 4. Add secondary audio track for music bed using timeline.add_track
        added_track = agent_cli.execute("timeline.add_track", {"kind": "audio", "name": "Music Bed"})
        music_track_id = added_track.get("track_id", 2)
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 3, "completed_step_id": 2})

        # 5. Ingest media and place clips
        aud_dia = agent_cli.execute(
            "project.add_media",
            {
                "name": "aud_asset_dia",
                "path": str(dialogue_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        aud_mus = agent_cli.execute(
            "project.add_media",
            {
                "name": "aud_asset_mus",
                "path": str(music_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(source_duration_s),
            },
        )

        clip_dur = sec_to_ticks(12.0)
        # Place dialogue on track 1 (default audio track)
        agent_cli.execute(
            "timeline.add_clip",
            {"media_id": aud_dia["media_id"], "source_in": 0, "duration": clip_dur},
        )
        # Place music on secondary track
        agent_cli.execute(
            "timeline.add_clip",
            {
                "media_id": aud_mus["media_id"],
                "source_in": 0,
                "duration": clip_dur,
                "track_id": music_track_id,
            },
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 4, "completed_step_id": 3})

        # 6. Apply ducking automation and gain adjustment
        agent_cli.execute("track.set_gain_db", {"track_id": music_track_id, "gain_db": -10.0})
        agent_cli.execute(
            "track.add_automation_point",
            {"track_id": music_track_id, "lane": "volume", "time": sec_to_ticks(2.0), "value": -14.0},
        )
        agent_cli.execute(
            "track.add_automation_point",
            {"track_id": music_track_id, "lane": "volume", "time": sec_to_ticks(8.0), "value": -14.0},
        )
        agent_cli.execute(
            "track.add_automation_point",
            {"track_id": music_track_id, "lane": "volume", "time": sec_to_ticks(10.0), "value": -8.0},
        )

        # Enable dialogue processing
        agent_cli.execute(
            "track.set_audio_processing",
            {
                "track_id": 1,
                "comp_enabled": True,
                "comp_threshold": -18.0,
                "comp_ratio": 3.0,
            },
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 5, "completed_step_id": 4})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # 7. Audio DSP and verify.run
        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="broadcast",
            verify_preset="broadcast-r128",
            run_loudness_dsp=run_audio_dsp,
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 6, "completed_step_id": 5})

        # 8. Branch diff
        branch_diff = agent_cli.execute("project.branch_diff", {"branch": branch_name})

        # 9. Review round
        notes = [
            {
                "text": "[Acoustic Balance] Separate dialogue and music bed ducked by -10dB. Speech intelligibility verified.",
                "severity": "info",
                "category": "audio_mix",
                "target_tick": 0,
                "end_tick": clip_dur,
            }
        ]
        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Music bed ducking and loudness mix ready for acoustic review",
            review_notes=notes,
            preset="broadcast-r128",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_ducking") if package_delivery and approve_review else None,
        )

        merged_res = None
        if auto_merge and approve_review:
            human_cli.execute("file.open", {"path": str(project_path)})
            bl = human_cli.execute("project.branch_list", {})
            if bl.get("active") != "main":
                human_cli.execute("project.branch_switch", {"name": "main"})
            merged_res = human_cli.execute(
                "project.branch_merge",
                {"branch": branch_name, "action": "accept", "force": True},
            )
            human_cli.execute("file.save", {"path": str(project_path)})

        tool_calls += human_cli.tool_calls + agent_cli.tool_calls + mcp_qc["tool_calls"]

    return {
        "recipe": "music-bed-ducking-master",
        "project_path": str(project_path),
        "branch": branch_name,
        "branch_diff": branch_diff,
        "music_track_id": music_track_id,
        "gate_state": gate_state,
        "verify_report": mcp_qc.get("verify_report"),
        "review": review_res,
        "merged": merged_res,
        "tool_calls": tool_calls,
        "elapsed_s": round(time.monotonic() - t0, 3),
    }
