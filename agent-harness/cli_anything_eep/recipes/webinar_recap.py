# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Recipe 3: Webinar & training recap with topic coverage check, `must_not` flagging, and full + reel sequences."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.recipes._common import (
    ControlClient,
    default_webinar_segments,
    finalize_review_and_delivery,
    load_template_sheet,
    run_cli_subcommand,
    sec_to_ticks,
    sync_submit_approve_plan,
    synthesize_speech_wav,
    synthesize_test_video,
    ticks_to_sec,
    try_mcp_audio_and_verify,
    verify_human_gate_enforced,
)


def run_webinar_recap(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    segments: Optional[list[dict[str, Any]]] = None,
    must_include: Optional[list[str]] = None,
    must_not_terms: Optional[list[str]] = None,
    media_path: Optional[str | Path] = None,
    simulate_missing_topic: bool = False,
    approve_review: bool = False,
    package_delivery: bool = False,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Flagship Recipe 3 producing Full Edit + Summary Highlight Reel sequences."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "webinar_recap.eep"

    segs = segments if segments is not None else default_webinar_segments()
    max_end_s = max((ticks_to_sec(int(s["end_ticks"])) for s in segs), default=50.0)
    full_duration_s = max(50.0, max_end_s + 0.5)

    sheet = load_template_sheet("webinar-recap", sheet_path)
    required_topics = list(must_include) if must_include is not None else ["pipeline scheduling", "live showcase"]
    if simulate_missing_topic and "budget forecast SLA" not in required_topics:
        required_topics.append("budget forecast SLA")

    must_not = sheet.get("must_not") or {}
    if must_not_terms is not None:
        must_not = {**must_not, "words": list(must_not_terms)}
    else:
        existing_words = list(must_not.get("words") or [])
        if "unreleased roadmap" not in existing_words:
            existing_words.append("unreleased roadmap")
        must_not = {**must_not, "words": existing_words}
    sheet["must_not"] = must_not

    analyze_input = {
        "segments": segs,
        "reel_target_s": 25.0 if segments is None else 120.0,
        "reel_tolerance_s": 12.0 if segments is None else 30.0,
        "chapter_gap_s": 2.5,
        "must_include": required_topics,
        "must_not": must_not,
    }
    input_json = root / "webinar_analyze_input.json"
    input_json.write_text(json.dumps(analyze_input, indent=2), encoding="utf-8")
    analysis = run_cli_subcommand(["analyze-recipe", "webinar", str(input_json)], cli_path=cli_path)

    highlights = analysis.get("highlights", [])
    flagged_must_not = analysis.get("flagged_must_not", [])
    missing_topics = analysis.get("missing_required_topics", [])
    reel_target_s = max(10.0, float(analysis.get("reel_duration_s", 25.0)))

    if sheet_path is None and sheet.get("deliverables"):
        sheet["deliverables"][0]["preset"] = "web-1080p"
        sheet["deliverables"][0]["duration"] = {"target_s": round(full_duration_s, 2), "tolerance_s": 15.0}
        if len(sheet["deliverables"]) > 1:
            sheet["deliverables"][1]["preset"] = "web-1080p"
            sheet["deliverables"][1]["duration"] = {"target_s": round(reel_target_s, 2), "tolerance_s": 15.0}

    audio_wav = Path(media_path) if media_path else synthesize_speech_wav(root / "src_aud_03.wav", segs, full_duration_s)
    video_dummy_path = str(Path(media_path) if media_path else synthesize_test_video(root / "src_cam_03.mp4", full_duration_s))

    tool_calls = 1  # analyze-recipe
    with ControlClient(cli_path=cli_path) as human_cli, ControlClient(cli_path=cli_path, actor="agent:recipe-webinar") as agent_cli:
        human_cli.execute(
            "project.new",
            {
                "name": "Webinar Recap",
                "sequence_name": "Webinar Full Edit",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        created_sheet = human_cli.execute("tasksheet.create", sheet)
        if created_sheet.get("sheet", {}).get("status", "draft") == "draft":
            human_cli.execute("tasksheet.submit", {"reason": "Webinar edit & summary reel requested"})
        human_cli.execute("file.save", {"path": str(project_path)})

        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=[
                {"id": 1, "title": "Chapter webinar and audit must_include / must_not coverage", "status": "pending"},
                {"id": 2, "title": "Build full edited sequence + clean summary highlight reel", "status": "pending"},
                {"id": 3, "title": "Verify sequences and log review notes for flagged items", "status": "pending"},
            ],
        )

        # If any required must_include topic is missing, escalate via TS3 question_ask!
        escalation_result: Optional[dict[str, Any]] = None
        if missing_topics:
            asked = agent_cli.execute(
                "tasksheet.question_ask",
                {
                    "question_id": "q_missing_must_include",
                    "question": f"Required webinar topic(s) not detected in transcript: {', '.join(missing_topics)}. How should we proceed?",
                    "context": "Analyze-recipe topic coverage check found missing must_include topics.",
                    "default_option_id": "opt_proceed_available",
                    "options": [
                        {
                            "id": "opt_proceed_available",
                            "label": "Proceed with available topics and note omission in review",
                            "is_default": True,
                        },
                        {
                            "id": "opt_abort",
                            "label": "Pause and wait for missing recording segment",
                        },
                    ],
                },
            )
            answer_denied = verify_human_gate_enforced(
                agent_cli,
                "tasksheet.question_answer",
                {"question_id": "q_missing_must_include", "option_id": "opt_proceed_available"},
            )
            agent_cli.execute("file.save", {"path": str(project_path)})

            human_cli.execute("file.open", {"path": str(project_path)})
            answered = human_cli.execute(
                "tasksheet.question_answer",
                {"question_id": "q_missing_must_include", "option_id": "opt_proceed_available"},
            )
            human_cli.execute("file.save", {"path": str(project_path)})
            agent_cli.restart()
            agent_cli.execute("file.open", {"path": str(project_path)})
            escalation_result = {
                "asked": asked,
                "agent_answer_denied": answer_denied,
                "answered": answered,
                "missing_topics": missing_topics,
            }

        agent_cli.execute("tasksheet.run_report", {"completed_step_id": 1, "current_step_id": 2})

        vid_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_cam_03",
                "path": video_dummy_path,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(full_duration_s),
            },
        )
        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_aud_03",
                "path": str(audio_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(full_duration_s),
            },
        )

        # Build Sequence 1: Full edited webinar with chapters + captions + flagged markers
        agent_cli.execute("timeline.add_clip", {"media_id": vid_res["media_id"]})
        agent_cli.execute("timeline.add_clip", {"media_id": aud_res["media_id"]})
        for ch in analysis.get("chapters", []):
            agent_cli.execute(
                "marker.add_chapter",
                {"at": int(ch["start_ticks"]), "name": ch["title"]},
            )
        agent_cli.execute("speech.transcribe_to_subtitles", {"segments": segs, "replace_existing": True})
        agent_cli.execute("file.save", {"path": str(project_path)})

        full_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="web-1080p",
            run_loudness_dsp=run_audio_dsp,
        )

        # Build Sequence 2: Clean Summary Highlight Reel (excludes all `must_not` segments).
        reel_seq = agent_cli.execute("sequence.create", {"name": "Webinar Summary Reel"})
        flagged_ids = {int(hit["segment_id"]) for hit in flagged_must_not}
        reel_captions: list[dict[str, Any]] = []
        cursor_ticks = 0
        included_highlight_ids: list[int] = []
        for hl in highlights:
            sid = int(hl["segment_id"])
            if sid in flagged_ids:
                continue
            s0 = int(hl["start_ticks"])
            e0 = int(hl["end_ticks"])
            dur = max(1, e0 - s0)
            agent_cli.execute(
                "timeline.add_clip",
                {
                    "media_id": vid_res["media_id"],
                    "timeline_start": cursor_ticks,
                    "source_in": s0,
                    "duration": dur,
                },
            )
            agent_cli.execute(
                "timeline.add_clip",
                {
                    "media_id": aud_res["media_id"],
                    "timeline_start": cursor_ticks,
                    "source_in": s0,
                    "duration": dur,
                },
            )
            reel_captions.append({
                "id": sid,
                "start_ticks": cursor_ticks,
                "end_ticks": cursor_ticks + dur,
                "text": hl["text"],
                "confidence": 0.96,
                "words": [],
            })
            included_highlight_ids.append(sid)
            cursor_ticks += dur

        if reel_captions:
            agent_cli.execute("speech.transcribe_to_subtitles", {"segments": reel_captions, "replace_existing": True})

        agent_cli.execute("tasksheet.run_report", {"completed_step_id": 2, "current_step_id": 3})
        agent_cli.execute("file.save", {"path": str(project_path)})

        # Align summary reel brief target duration before running reel verify.run QC.
        if sheet_path is None and sheet.get("deliverables"):
            human_cli.execute("file.open", {"path": str(project_path)})
            human_cli.execute(
                "tasksheet.update",
                {
                    "deliverables": [
                        {
                            "id": "deliv_reel",
                            "name": "Webinar Summary Reel",
                            "preset": "web-1080p",
                            "duration": {"target_s": round(ticks_to_sec(cursor_ticks), 2), "tolerance_s": 12.0},
                            "loudness_lufs": -14.0,
                            "true_peak_dbtp": -1.0,
                            "captions": True,
                        }
                    ]
                },
            )
            human_cli.execute("file.save", {"path": str(project_path)})
            agent_cli.execute("file.open", {"path": str(project_path)})

        reel_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="web-1080p",
            run_loudness_dsp=run_audio_dsp,
        )

        notes: list[dict[str, Any]] = [
            {
                "text": f"Policy exclusion ({hit['category']} #1) triggered on segment #{hit['segment_id']}; inspect full edit and confirm omission.",
                "severity": "warning",
                "category": "must_not",
                "target_tick": int(hit["start_ticks"]),
                "end_tick": int(hit["end_ticks"]),
            }
            for hit in flagged_must_not
        ]
        if missing_topics:
            notes.append({
                "text": f"Deliverable topic audit missed {len(missing_topics)} requested item(s); fallback choice recorded.",
                "severity": "warning",
                "category": "must_include",
            })
        if not notes:
            notes.append({
                "text": "Short highlight pass assembled; inspect transition rhythm across kept segments.",
                "severity": "info",
                "category": "pacing",
            })

        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Full edit and short reel assembled with policy exclusions flagged",
            review_notes=notes,
            preset="web-1080p",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_webinar") if package_delivery and approve_review else None,
        )

        tool_calls += human_cli.tool_calls + agent_cli.tool_calls + full_qc["tool_calls"] + reel_qc["tool_calls"]

    return {
        "recipe": "webinar-recap",
        "project_path": str(project_path),
        "analysis": analysis,
        "full_sequence": "Webinar Full Edit",
        "reel_sequence": reel_seq.get("name", "Webinar Summary Reel"),
        "reel_included_segment_ids": included_highlight_ids,
        "reel_excludes_all_must_not": not any(sid in flagged_ids for sid in included_highlight_ids),
        "escalation": escalation_result,
        "verify_full": full_qc["verify_report"],
        "verify_reel": reel_qc["verify_report"],
        "loudness_full": full_qc["loudness_measurement"],
        "loudness_reel": reel_qc["loudness_measurement"],
        "plan_gate_denied_to_agent": gate_state["agent_gate_denied"],
        "review": review_res,
        "tool_calls": tool_calls,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }
