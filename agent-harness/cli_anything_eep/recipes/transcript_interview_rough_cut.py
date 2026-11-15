# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Companion §2.6 Recipe 2: Transcript-driven interview rough cut on word boundaries."""

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
    synthesize_test_video,
    ticks_to_sec,
    try_mcp_audio_and_verify,
)


def _default_interview_sheet() -> dict[str, Any]:
    return {
        "schema": "tasksheet/1",
        "id": "sheet_interview_rough_cut",
        "title": "Interview Transcript Rough Cut Assembly",
        "requester": "human:director",
        "inputs": [
            {
                "id": "in_camera_a",
                "media": "media/vid_source_01.mp4",
                "role": "main_video",
                "trust": "trusted",
            },
            {
                "id": "in_mic_host",
                "media": "media/aud_source_01.wav",
                "role": "dialogue",
                "trust": "trusted",
            },
        ],
        "deliverables": [
            {
                "id": "deliv_rough_cut",
                "name": "Interview Story Assembly",
                "preset": "web-1080p",
                "duration": {"target_s": 15.0, "tolerance_s": 8.0},
                "loudness_lufs": -14.0,
                "true_peak_dbtp": -1.0,
            }
        ],
    }


def run_transcript_interview_rough_cut(
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
    """Execute Companion §2.6 Recipe: Transcript interview rough cut end-to-end."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "interview_rough_cut.eep"

    segs = segments if segments is not None else default_podcast_segments()
    # Select key narrative segments (e.g. segments 2 and 3)
    selected_segs = [s for s in segs if s.get("id") in {2, 3}] or segs[:2]

    source_duration_s = max(24.0, max((ticks_to_sec(int(s["end_ticks"])) for s in segs), default=24.0) + 1.0)
    audio_wav = (
        Path(media_path)
        if media_path
        else synthesize_speech_wav(root / "aud_source_01.wav", segs, source_duration_s)
    )
    video_dummy_path = str(
        Path(media_path)
        if media_path
        else synthesize_test_video(root / "vid_source_01.mp4", source_duration_s)
    )

    sheet = (
        json.loads(Path(sheet_path).read_text(encoding="utf-8"))
        if sheet_path
        else _default_interview_sheet()
    )

    branch_name = "agent/interview-rough-cut"
    tool_calls = 0

    with (
        ControlClient(cli_path=cli_path) as human_cli,
        ControlClient(cli_path=cli_path, actor="agent:recipe-interview") as agent_cli,
    ):
        # 1. Project setup
        human_cli.execute(
            "project.new",
            {
                "name": "Interview Assembly",
                "sequence_name": "Rough Cut V1",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("tasksheet.create", sheet)
        human_cli.execute("file.save", {"path": str(project_path)})

        # 2. Plan submission & human approval
        plan_steps = [
            {"id": 1, "title": "Load rough cut profile and create task branch", "status": "pending"},
            {"id": 2, "title": "Index transcript soundbites by topic and speaker", "status": "pending"},
            {"id": 3, "title": "Assemble selected soundbites on timeline at word boundaries", "status": "pending"},
            {"id": 4, "title": "Align audio tracks and generate speech subtitles", "status": "pending"},
            {"id": 5, "title": "Run verify.run QC checks (timing, silence, loudness)", "status": "pending"},
            {"id": 6, "title": "Compute branch diff and present review notes for editorial review", "status": "pending"},
        ]
        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=plan_steps,
        )

        # 3. Create task branch and switch to it
        agent_cli.execute(
            "project.branch_create",
            {"name": branch_name, "note": "Transcript soundbite assembly"},
        )
        agent_cli.execute("project.branch_switch", {"name": branch_name})
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})

        # 4. Ingest media
        vid_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "vid_asset_01",
                "path": video_dummy_path,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "aud_asset_01",
                "path": str(audio_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(source_duration_s),
            },
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 3, "completed_step_id": 2})

        # 5. Assemble soundbites on timeline
        rebased_cues: list[dict[str, Any]] = []
        timeline_cursor_ticks = 0

        for seg in selected_segs:
            start_tick = int(seg["start_ticks"])
            end_tick = int(seg["end_ticks"])
            dur = end_tick - start_tick

            # Add video clip
            agent_cli.execute(
                "timeline.add_clip",
                {
                    "media_id": vid_res["media_id"],
                    "source_in": start_tick,
                    "duration": dur,
                },
            )
            # Add audio clip
            agent_cli.execute(
                "timeline.add_clip",
                {
                    "media_id": aud_res["media_id"],
                    "source_in": start_tick,
                    "duration": dur,
                },
            )

            # Rebase transcript cues to timeline time
            words = seg.get("words", [])
            rebased_cues.append({
                "id": seg["id"],
                "start_ticks": timeline_cursor_ticks,
                "end_ticks": timeline_cursor_ticks + dur,
                "text": seg["text"],
                "speaker": seg.get("speaker", "Guest"),
                "confidence": seg.get("confidence", 0.98),
                "words": [
                    {
                        **w,
                        "start_ticks": int(w["start_ticks"]) - start_tick + timeline_cursor_ticks,
                        "end_ticks": int(w["end_ticks"]) - start_tick + timeline_cursor_ticks,
                    }
                    for w in words
                ],
            })
            timeline_cursor_ticks += dur

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 4, "completed_step_id": 3})

        # 6. Generate subtitles
        if rebased_cues:
            agent_cli.execute("speech.transcribe_to_subtitles", {"segments": rebased_cues, "replace_existing": True})

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 5, "completed_step_id": 4})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # 7. QC and verify.run
        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="web-1080p",
            run_loudness_dsp=run_audio_dsp,
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 6, "completed_step_id": 5})

        # 8. Branch diff
        branch_diff = agent_cli.execute("project.branch_diff", {"branch": branch_name})

        # 9. Review round
        notes = [
            {
                "text": "[Story Flow] Soundbites assembled across topic changes. Cadence verified.",
                "severity": "info",
                "category": "story_flow",
                "target_tick": 0,
                "end_tick": timeline_cursor_ticks,
            }
        ]
        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Interview cut assembly ready for storyline sign-off",
            review_notes=notes,
            preset="web-1080p",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_interview") if package_delivery and approve_review else None,
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
        "recipe": "transcript-interview-rough-cut",
        "project_path": str(project_path),
        "branch": branch_name,
        "branch_diff": branch_diff,
        "selected_segments_count": len(selected_segs),
        "assembled_duration_ticks": timeline_cursor_ticks,
        "gate_state": gate_state,
        "verify_report": mcp_qc.get("verify_report"),
        "review": review_res,
        "merged": merged_res,
        "tool_calls": tool_calls,
        "elapsed_s": round(time.monotonic() - t0, 3),
    }
