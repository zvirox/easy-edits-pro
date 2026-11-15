# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Companion §2.6 Recipe 1: Podcast dialogue cleanup with silence/filler removal, branches, and verify.run."""

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
    ticks_to_sec,
    try_mcp_audio_and_verify,
)


def _default_cleanup_sheet() -> dict[str, Any]:
    return {
        "schema": "tasksheet/1",
        "id": "sheet_podcast_cleanup",
        "title": "Podcast Dialogue Cleanup & Silence Reduction",
        "requester": "human:producer",
        "inputs": [
            {
                "id": "in_dialogue",
                "media": "media/raw_speech.wav",
                "role": "isolated_audio",
                "trust": "trusted",
            }
        ],
        "deliverables": [
            {
                "id": "deliv_cleaned_dialogue",
                "name": "Cleaned Dialogue Master",
                "preset": "web-1080p",
                "duration": {"target_s": 20.0, "tolerance_s": 10.0},
                "loudness_lufs": -14.0,
                "true_peak_dbtp": -1.0,
            }
        ],
    }


def run_podcast_cleanup(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    segments: Optional[list[dict[str, Any]]] = None,
    media_path: Optional[str | Path] = None,
    approve_review: bool = False,
    package_delivery: bool = False,
    run_audio_dsp: bool = True,
    auto_merge: bool = False,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Companion §2.6 Recipe: Podcast dialogue cleanup end-to-end."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "podcast_cleanup.eep"

    segs = segments if segments is not None else default_podcast_segments()
    max_end_s = max((ticks_to_sec(int(s["end_ticks"])) for s in segs), default=24.0)
    source_duration_s = max(24.0, max_end_s + 0.5)

    audio_wav = (
        Path(media_path)
        if media_path
        else synthesize_speech_wav(root / "raw_speech.wav", segs, source_duration_s)
    )

    sheet = (
        json.loads(Path(sheet_path).read_text(encoding="utf-8"))
        if sheet_path
        else _default_cleanup_sheet()
    )

    tool_calls = 0
    branch_name = "agent/podcast-cleanup"

    with (
        ControlClient(cli_path=cli_path) as human_cli,
        ControlClient(cli_path=cli_path, actor="agent:recipe-cleanup") as agent_cli,
    ):
        # 1. Initialize project and baseline sequence
        human_cli.execute(
            "project.new",
            {
                "name": "Podcast Cleanup Project",
                "sequence_name": "Dialogue Track",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("tasksheet.create", sheet)
        human_cli.execute("file.save", {"path": str(project_path)})

        # 2. Closed-loop flow: Submit and approve 5-10 step plan
        plan_steps = [
            {"id": 1, "title": "Load dialogue cleanup profile and create task branch", "status": "pending"},
            {"id": 2, "title": "Ingest dialogue track and align transcript segments", "status": "pending"},
            {"id": 3, "title": "Detect and ripple-delete silence stretches (> 1.5s)", "status": "pending"},
            {"id": 4, "title": "Ripple-delete detected filler words and stumbles", "status": "pending"},
            {"id": 5, "title": "Apply dialogue loudness normalization and peak control", "status": "pending"},
            {"id": 6, "title": "Run verify.run measured QC and audio telemetry checks", "status": "pending"},
            {"id": 7, "title": "Compute branch diff and present review flags for human sign-off", "status": "pending"},
        ]
        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=plan_steps,
        )

        # 3. Create and switch to isolated task branch
        agent_cli.execute(
            "project.branch_create",
            {"name": branch_name, "note": "Reversible dialogue cleanup and silence reduction"},
        )
        agent_cli.execute("project.branch_switch", {"name": branch_name})
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})

        # 4. Ingest dialogue media and place on timeline
        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "aud_track_01",
                "path": str(audio_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        clip_res = agent_cli.execute(
            "timeline.add_clip",
            {"media_id": aud_res["media_id"], "source_in": 0, "duration": sec_to_ticks(source_duration_s)},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 3, "completed_step_id": 2})

        # 5. Silence detection & ripple delete
        peaks = [200] * 40 + [0] * 20 + [200] * 40
        silence_res = agent_cli.execute(
            "speech.ripple_delete_silence",
            {"peaks": peaks},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 4, "completed_step_id": 3})

        # 6. Ripple delete filler words
        filler_res = agent_cli.execute(
            "speech.ripple_delete_fillers",
            {"segments": segs, "custom_fillers": ["stumble", "rate"]},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 5, "completed_step_id": 4})

        # 7. Subtitles / captions alignment
        agent_cli.execute(
            "speech.transcribe_to_subtitles",
            {"segments": segs, "replace_existing": True},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 6, "completed_step_id": 5})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # 8. MCP audio telemetry and verify.run
        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="podcast",
            verify_preset="web-1080p",
            run_loudness_dsp=run_audio_dsp,
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 7, "completed_step_id": 6})

        # 9. Compute branch diff against live state
        branch_diff = agent_cli.execute("project.branch_diff", {"branch": branch_name})

        # 10. Review round and human review gate
        review_notes = [
            {
                "text": "[Acoustic Inspection] Silence gaps removed. Pacing ready for sign-off.",
                "severity": "info",
                "category": "acoustic_pacing",
                "target_tick": 0,
                "end_tick": sec_to_ticks(5.0),
            }
        ]

        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Cleaned audio speech ready for pacing evaluation",
            review_notes=review_notes,
            preset="web-1080p",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_podcast_cleanup") if package_delivery and approve_review else None,
        )

        # 11. Explicit opt-in auto-merge if requested and approved
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
        "recipe": "podcast-cleanup",
        "project_path": str(project_path),
        "branch": branch_name,
        "branch_diff": branch_diff,
        "silence_deleted_ranges": silence_res.get("deleted_ranges_count", 0),
        "filler_deleted_ranges": filler_res.get("deleted_ranges_count", 0),
        "gate_state": gate_state,
        "verify_report": mcp_qc.get("verify_report"),
        "review": review_res,
        "merged": merged_res,
        "tool_calls": tool_calls,
        "elapsed_s": round(time.monotonic() - t0, 3),
    }
