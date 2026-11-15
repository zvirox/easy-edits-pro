# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Companion §2.6 Recipe 4: Sequence colour matching with ACEScg/OCIO pipeline and IDTs."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.recipes._common import (
    ControlClient,
    finalize_review_and_delivery,
    sec_to_ticks,
    sync_submit_approve_plan,
    synthesize_test_video,
    try_mcp_audio_and_verify,
)


def _default_colour_sheet() -> dict[str, Any]:
    return {
        "schema": "tasksheet/1",
        "id": "sheet_colour_matching",
        "title": "Sequence Multi-Cam Colour Matching & OCIO ACEScg Setup",
        "requester": "human:colorist",
        "inputs": [
            {
                "id": "in_cam_a",
                "media": "media/vid_source_a.mp4",
                "role": "main_video",
                "trust": "trusted",
            },
            {
                "id": "in_cam_b",
                "media": "media/vid_source_b.mp4",
                "role": "secondary_video",
                "trust": "trusted",
            },
        ],
        "deliverables": [
            {
                "id": "deliv_graded_sequence",
                "name": "ACEScg Graded Master",
                "preset": "web-1080p",
                "duration": {"target_s": 10.0, "tolerance_s": 5.0},
            }
        ],
    }


def run_sequence_colour_matching(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    media_path: Optional[str | Path] = None,
    approve_review: bool = False,
    package_delivery: bool = False,
    auto_merge: bool = False,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Companion §2.6 Recipe: Sequence colour matching end-to-end."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "colour_matching.eep"

    source_duration_s = 12.0
    cam_a_dummy = str(Path(media_path) if media_path else synthesize_test_video(root / "vid_source_a.mp4", source_duration_s))
    cam_b_dummy = str(Path(media_path) if media_path else synthesize_test_video(root / "vid_source_b.mp4", source_duration_s))

    sheet = (
        json.loads(Path(sheet_path).read_text(encoding="utf-8"))
        if sheet_path
        else _default_colour_sheet()
    )

    branch_name = "agent/colour-matching"
    tool_calls = 0

    with (
        ControlClient(cli_path=cli_path) as human_cli,
        ControlClient(cli_path=cli_path, actor="agent:recipe-color") as agent_cli,
    ):
        # 1. Project setup
        human_cli.execute(
            "project.new",
            {
                "name": "Colour Matching Project",
                "sequence_name": "Graded Assembly",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("tasksheet.create", sheet)
        human_cli.execute("file.save", {"path": str(project_path)})

        # 2. Plan submission & human approval
        plan_steps = [
            {"id": 1, "title": "Load color profile and create task branch", "status": "pending"},
            {"id": 2, "title": "Configure OCIO ACEScg working pipeline and display ODT", "status": "pending"},
            {"id": 3, "title": "Assign camera IDTs to ARRI and Sony media assets", "status": "pending"},
            {"id": 4, "title": "Assemble clips on timeline and apply color grading transforms", "status": "pending"},
            {"id": 5, "title": "Run verify.run video QC (clipped highlights, crushed blacks)", "status": "pending"},
            {"id": 6, "title": "Compute branch diff and present color consistency review notes", "status": "pending"},
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
            {"name": branch_name, "note": "ACEScg colour management and multi-cam IDT matching"},
        )
        agent_cli.execute("project.branch_switch", {"name": branch_name})
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})

        # 4. Configure OCIO pipeline
        pipeline_res = agent_cli.execute(
            "ocio.set_pipeline",
            {
                "working_space": "aces_cg",
                "display_odt": "rec709",
                "default_idt": "rec709_video",
            },
        )
        agent_cli.execute("tasksheet.run_report", {"current_step_id": 3, "completed_step_id": 2})

        # 5. Ingest media and assign IDTs
        med_a = agent_cli.execute(
            "project.add_media",
            {
                "name": "vid_asset_a",
                "path": cam_a_dummy,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        idt_a = agent_cli.execute(
            "ocio.set_media_idt",
            {"media_id": med_a["media_id"], "idt": "arri_logc4"},
        )

        med_b = agent_cli.execute(
            "project.add_media",
            {
                "name": "vid_asset_b",
                "path": cam_b_dummy,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(source_duration_s),
            },
        )
        idt_b = agent_cli.execute(
            "ocio.set_media_idt",
            {"media_id": med_b["media_id"], "idt": "sony_slog3"},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 4, "completed_step_id": 3})

        # 6. Place clips on timeline
        clip_dur = sec_to_ticks(5.0)
        agent_cli.execute(
            "timeline.add_clip",
            {"media_id": med_a["media_id"], "source_in": 0, "duration": clip_dur},
        )
        agent_cli.execute(
            "timeline.add_clip",
            {"media_id": med_b["media_id"], "source_in": 0, "duration": clip_dur},
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 5, "completed_step_id": 4})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # 7. QC verification with verify.run
        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="web-1080p",
            run_loudness_dsp=False,
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 6, "completed_step_id": 5})

        # 8. Branch diff
        branch_diff = agent_cli.execute("project.branch_diff", {"branch": branch_name})

        # 9. Review round
        notes = [
            {
                "text": "[Grading Inspection] Working space and camera transforms assigned. Scopes ready for sign-off.",
                "severity": "info",
                "category": "color_science",
                "target_tick": 0,
                "end_tick": clip_dur * 2,
            }
        ]
        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Color management and IDT setup ready for grading sign-off",
            review_notes=notes,
            preset="web-1080p",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_colour") if package_delivery and approve_review else None,
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
        "recipe": "sequence-colour-matching",
        "project_path": str(project_path),
        "branch": branch_name,
        "branch_diff": branch_diff,
        "pipeline": pipeline_res,
        "idt_a": idt_a,
        "idt_b": idt_b,
        "gate_state": gate_state,
        "verify_report": mcp_qc.get("verify_report"),
        "review": review_res,
        "merged": merged_res,
        "tool_calls": tool_calls,
        "elapsed_s": round(time.monotonic() - t0, 3),
    }
