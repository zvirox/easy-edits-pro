# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Companion §2.6 Recipe 3: Captioned vertical 9:16 excerpt with frame-fit and CPS check."""

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


def _default_vertical_sheet() -> dict[str, Any]:
    return {
        "schema": "tasksheet/1",
        "id": "sheet_vertical_excerpt",
        "title": "Captioned Vertical Excerpt (9:16)",
        "requester": "human:social_lead",
        "inputs": [
            {
                "id": "in_source_video",
                "media": "media/vid_source_excerpt.mp4",
                "role": "main_video",
                "trust": "trusted",
            },
            {
                "id": "in_source_audio",
                "media": "media/aud_source_excerpt.wav",
                "role": "dialogue",
                "trust": "trusted",
            },
        ],
        "deliverables": [
            {
                "id": "deliv_vertical_hook",
                "name": "Social Hook 9:16",
                "preset": "vertical-9x16",
                "duration": {"target_s": 10.0, "tolerance_s": 5.0},
                "loudness_lufs": -14.0,
                "true_peak_dbtp": -1.0,
                "captions": True,
            }
        ],
    }


def run_captioned_vertical_excerpt(
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
    """Execute Companion §2.6 Recipe: Captioned vertical 9:16 excerpt end-to-end."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "vertical_excerpt.eep"

    segs = segments if segments is not None else default_podcast_segments()
    # Choose highlight segment (e.g. segment 3)
    highlight_seg = next((s for s in segs if s.get("id") == 3), segs[0])
    clip_start = int(highlight_seg["start_ticks"])
    clip_end = int(highlight_seg["end_ticks"])
    clip_dur = clip_end - clip_start
    source_duration_s = max(24.0, ticks_to_sec(clip_end) + 1.0)

    audio_wav = (
        Path(media_path)
        if media_path
        else synthesize_speech_wav(root / "aud_source_excerpt.wav", segs, source_duration_s)
    )
    video_dummy_path = str(
        Path(media_path)
        if media_path
        else synthesize_test_video(root / "vid_source_excerpt.mp4", source_duration_s)
    )

    sheet = (
        json.loads(Path(sheet_path).read_text(encoding="utf-8"))
        if sheet_path
        else _default_vertical_sheet()
    )

    branch_name = "agent/vertical-excerpt"
    tool_calls = 0

    with (
        ControlClient(cli_path=cli_path) as human_cli,
        ControlClient(cli_path=cli_path, actor="agent:recipe-vertical") as agent_cli,
    ):
        # 1. Project setup with 9:16 sequence (1080x1920)
        human_cli.execute(
            "project.new",
            {
                "name": "Vertical Highlights",
                "sequence_name": "Vertical 9x16 Master",
                "width": 1080,
                "height": 1920,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("tasksheet.create", sheet)
        human_cli.execute("file.save", {"path": str(project_path)})

        # 2. Plan submission & human approval
        plan_steps = [
            {"id": 1, "title": "Load vertical profile and create task branch", "status": "pending"},
            {"id": 2, "title": "Reframe sequence to 9:16 portrait (1080x1920)", "status": "pending"},
            {"id": 3, "title": "Add media and apply frame-fit fill crop", "status": "pending"},
            {"id": 4, "title": "Generate burned captions with safe area margin", "status": "pending"},
            {"id": 5, "title": "Run verify.run vertical-9x16 QC (CPS, safe area, loudness)", "status": "pending"},
            {"id": 6, "title": "Compute branch diff and present review notes for human check", "status": "pending"},
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
            {"name": branch_name, "note": "Vertical 9:16 excerpt reframing and captions"},
        )
        agent_cli.execute("project.branch_switch", {"name": branch_name})
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})

        # 4. Confirm sequence settings 1080x1920
        agent_cli.execute(
            "sequence.settings",
            {"width": 1080, "height": 1920, "frame_rate": {"numerator": 30, "denominator": 1}},
        )
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 3, "completed_step_id": 2})

        # 5. Add video media and apply frame-fit fill
        vid_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "vid_clip_01",
                "path": video_dummy_path,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        added_video = agent_cli.execute(
            "timeline.add_clip",
            {"media_id": vid_res["media_id"], "source_in": clip_start, "duration": clip_dur},
        )
        agent_cli.execute(
            "clip.frame_fit",
            {
                "track_id": added_video["track_id"],
                "clip_index": added_video["clip_index"],
                "mode": "fill",
            },
        )

        # Add audio
        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "aud_clip_01",
                "path": str(audio_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        agent_cli.execute(
            "timeline.add_clip",
            {"media_id": aud_res["media_id"], "source_in": clip_start, "duration": clip_dur},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 4, "completed_step_id": 3})

        # 6. Rebase and burn captions
        rebased_cues = [
            {
                "id": highlight_seg["id"],
                "start_ticks": 0,
                "end_ticks": clip_dur,
                "text": highlight_seg["text"],
                "speaker": highlight_seg.get("speaker", "Host"),
                "confidence": 0.98,
                "words": [
                    {
                        **w,
                        "start_ticks": int(w["start_ticks"]) - clip_start,
                        "end_ticks": int(w["end_ticks"]) - clip_start,
                    }
                    for w in highlight_seg.get("words", [])
                ],
            }
        ]
        agent_cli.execute(
            "speech.transcribe_to_subtitles",
            {"segments": rebased_cues, "replace_existing": True},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 5, "completed_step_id": 4})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # 7. QC with verify.run vertical-9x16 preset
        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="vertical-9x16",
            run_loudness_dsp=run_audio_dsp,
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 6, "completed_step_id": 5})

        # 8. Branch diff
        branch_diff = agent_cli.execute("project.branch_diff", {"branch": branch_name})

        # 9. Review round
        notes = [
            {
                "text": "[Framing Inspection] 9:16 crop applied. Verify subject center and safe margins.",
                "severity": "info",
                "category": "visual_framing",
                "target_tick": 0,
                "end_tick": clip_dur,
            }
        ]
        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Vertical 9:16 excerpt ready for mobile crop and caption review",
            review_notes=notes,
            preset="vertical-9x16",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_vertical") if package_delivery and approve_review else None,
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
        "recipe": "captioned-vertical-excerpt",
        "project_path": str(project_path),
        "branch": branch_name,
        "branch_diff": branch_diff,
        "clip_duration_ticks": clip_dur,
        "gate_state": gate_state,
        "verify_report": mcp_qc.get("verify_report"),
        "review": review_res,
        "merged": merged_res,
        "tool_calls": tool_calls,
        "elapsed_s": round(time.monotonic() - t0, 3),
    }
