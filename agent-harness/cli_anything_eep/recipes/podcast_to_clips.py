# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Recipe 1: Podcast to vertical 9:16 social clips with word-boundary cuts, captions, and review flags."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.recipes._common import (
    ControlClient,
    default_podcast_segments,
    finalize_review_and_delivery,
    load_template_sheet,
    run_cli_subcommand,
    sec_to_ticks,
    sync_submit_approve_plan,
    synthesize_speech_wav,
    synthesize_test_video,
    ticks_to_sec,
    try_mcp_audio_and_verify,
)


def _rebase_segments_to_clip(
    segments: list[dict[str, Any]],
    clip_start_ticks: int,
    clip_end_ticks: int,
) -> list[dict[str, Any]]:
    """Trim and rebase transcript segments to timeline-relative ticks `[0, duration]`."""
    rebased: list[dict[str, Any]] = []
    for seg in segments:
        s0 = int(seg["start_ticks"])
        e0 = int(seg["end_ticks"])
        if e0 <= clip_start_ticks or s0 >= clip_end_ticks:
            continue
        words = [
            w for w in seg.get("words", [])
            if int(w["start_ticks"]) >= clip_start_ticks and int(w["end_ticks"]) <= clip_end_ticks
        ]
        start = max(s0, clip_start_ticks) - clip_start_ticks
        end = min(e0, clip_end_ticks) - clip_start_ticks
        if end <= start:
            continue
        rebased.append({
            "id": seg["id"],
            "start_ticks": start,
            "end_ticks": end,
            "text": " ".join(w["word"] for w in words) if words else seg["text"],
            "speaker": seg.get("speaker"),
            "confidence": seg.get("confidence", 0.95),
            "words": [
                {
                    **w,
                    "start_ticks": int(w["start_ticks"]) - clip_start_ticks,
                    "end_ticks": int(w["end_ticks"]) - clip_start_ticks,
                }
                for w in words
            ],
        })
    return rebased


def run_podcast_to_clips(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    segments: Optional[list[dict[str, Any]]] = None,
    media_path: Optional[str | Path] = None,
    top_k: int = 2,
    approve_review: bool = False,
    package_delivery: bool = False,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Flagship Recipe 1 end-to-end against real `saturn-cli` and `saturn-mcp` binaries."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "podcast_to_clips.eep"

    segs = segments if segments is not None else default_podcast_segments()
    max_end_s = max((ticks_to_sec(int(s["end_ticks"])) for s in segs), default=24.0)
    source_duration_s = max(24.0, max_end_s + 0.5)

    sheet = load_template_sheet("podcast-social-clips", sheet_path)
    must_not = sheet.get("must_not")
    must_include = []
    if sheet.get("deliverables"):
        must_include = list(sheet["deliverables"][0].get("must_include", []))

    analyze_input = {
        "segments": segs,
        "target_duration_s": 13.6 if segments is None else 45.0,
        "tolerance_s": 6.0 if segments is None else 15.0,
        "top_k": top_k,
        "must_include": ["pricing teardown", "benchmarks", *must_include],
        "must_not": must_not,
    }
    input_json = root / "podcast_analyze_input.json"
    input_json.write_text(json.dumps(analyze_input, indent=2), encoding="utf-8")
    rank_report = run_cli_subcommand(["analyze-recipe", "podcast", str(input_json)], cli_path=cli_path)

    candidates = rank_report.get("candidates", [])
    if not candidates:
        raise RuntimeError("Podcast recipe produced zero ranked clip candidates")
    top = candidates[0]
    clip_start = int(top["start_ticks"])
    clip_end = int(top["end_ticks"])
    clip_dur = clip_end - clip_start
    clip_dur_s = ticks_to_sec(clip_dur)

    # Align synthetic demo sheet duration constraint cleanly with selected clip window.
    if sheet_path is None and sheet.get("deliverables"):
        for d in sheet["deliverables"]:
            d["duration"] = {"target_s": round(clip_dur_s, 2), "tolerance_s": 10.0}
            d["preset"] = "vertical-9x16"

    audio_wav = Path(media_path) if media_path else synthesize_speech_wav(root / "src_aud_01.wav", segs, source_duration_s)
    video_dummy_path = str(Path(media_path) if media_path else synthesize_test_video(root / "src_cam_01.mp4", source_duration_s))

    tool_calls = 1  # analyze-recipe
    with ControlClient(cli_path=cli_path) as human_cli, ControlClient(cli_path=cli_path, actor="agent:recipe-podcast") as agent_cli:
        human_cli.execute(
            "project.new",
            {
                "name": "Podcast Social Clips",
                "sequence_name": "Social Clip 1",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        created_sheet = human_cli.execute("tasksheet.create", sheet)
        if created_sheet.get("sheet", {}).get("status", "draft") == "draft":
            human_cli.execute("tasksheet.submit", {"reason": "Ready for clip ranking"})
        human_cli.execute("file.save", {"path": str(project_path)})

        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=[
                {"id": 1, "title": "Rank candidate clips on word boundaries", "status": "pending"},
                {"id": 2, "title": "Reframe 9:16 vertical, caption, and normalize audio", "status": "pending"},
                {"id": 3, "title": "Run verify.run QC and flag hook/tone/ending review", "status": "pending"},
            ],
        )

        agent_cli.execute("tasksheet.run_report", {"current_step_id": 2, "completed_step_id": 1})
        agent_cli.execute("sequence.settings", {"width": 1080, "height": 1920, "frame_rate": {"numerator": 30, "denominator": 1}})

        vid_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_cam_01",
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
        fit_res = agent_cli.execute(
            "clip.frame_fit",
            {"track_id": added_video["track_id"], "clip_index": added_video["clip_index"], "mode": "fill"},
        )

        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_aud_01",
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

        clip_captions = _rebase_segments_to_clip(segs, clip_start, clip_end)
        if clip_captions:
            agent_cli.execute("speech.transcribe_to_subtitles", {"segments": clip_captions, "replace_existing": True})

        agent_cli.execute("tasksheet.run_report", {"completed_step_id": 2, "current_step_id": 3})
        agent_cli.execute("file.save", {"path": str(project_path)})

        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="vertical-9x16",
            run_loudness_dsp=run_audio_dsp,
        )

        # Record human-judgement review flags (hook, tone, ending) in review round v1.
        flags = top.get("review_flags", [])
        notes = [
            {
                "text": f"[{f['criterion']}] {f['prompt']}",
                "severity": "info",
                "category": f["criterion"],
                "target_tick": max(0, int(f["target_tick"]) - clip_start),
                "end_tick": max(1, int(f["end_tick"]) - clip_start),
            }
            for f in flags
        ]
        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Vertical 9:16 social clip ready for editorial hook/tone/ending review",
            review_notes=notes,
            preset="vertical-9x16",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_podcast") if package_delivery and approve_review else None,
        )

        tool_calls += human_cli.tool_calls + agent_cli.tool_calls + mcp_qc["tool_calls"]

    return {
        "recipe": "podcast-to-clips",
        "project_path": str(project_path),
        "candidates": candidates,
        "selected_candidate": top,
        "word_boundary_all_aligned": all(c.get("word_boundary_aligned") for c in candidates),
        "rejected_must_not_segments": rank_report.get("rejected_must_not_segments", 0),
        "frame_fit": fit_res,
        "loudness": {
            "applied": mcp_qc["loudness_applied"],
            "measurement": mcp_qc["loudness_measurement"],
            "note": mcp_qc["loudness_note"],
        },
        "verify": mcp_qc["verify_report"],
        "plan_gate_denied_to_agent": gate_state["agent_gate_denied"],
        "review": review_res,
        "tool_calls": tool_calls,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }
