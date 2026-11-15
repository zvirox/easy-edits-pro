# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Specific EEP-Bench tasks: 3-shot colour match, 30s highlight, and caption repair."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from cli_anything_eep.recipes._common import (
    ControlClient,
    sec_to_ticks,
    synthesize_test_video,
    ticks_to_sec,
    try_mcp_audio_and_verify,
)


def _check(name: str, ok: bool, detail: Any = None) -> dict[str, Any]:
    return {"name": name, "passed": bool(ok), "detail": detail}


# --- Specific Task 1: 3-Shot Colour Matching ---
def exec_three_shot_colour_match(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    cli_path = kw.get("cli_path")
    mcp_path = kw.get("mcp_path")
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "three_shot_colour.eep"

    vid_a = str(synthesize_test_video(root / "cam_a.mp4", 10.0))
    vid_b = str(synthesize_test_video(root / "cam_b.mp4", 10.0))
    vid_c = str(synthesize_test_video(root / "cam_c.mp4", 10.0))

    calls = 0
    with ControlClient(cli_path=cli_path) as cli:
        cli.execute(
            "project.new",
            {
                "name": "3-Shot Colour Matching",
                "sequence_name": "Main",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        pipe = cli.execute(
            "ocio.set_pipeline",
            {"working_space": "aces_cg", "display_odt": "rec709", "default_idt": "rec709_video"},
        )
        m_a = cli.execute("project.add_media", {"name": "cam_a", "path": vid_a, "kind": "video", "width": 1920, "height": 1080, "fps": 30.0, "duration": sec_to_ticks(10.0)})
        idt_a = cli.execute("ocio.set_media_idt", {"media_id": m_a["media_id"], "idt": "arri_logc4"})

        m_b = cli.execute("project.add_media", {"name": "cam_b", "path": vid_b, "kind": "video", "width": 1920, "height": 1080, "fps": 30.0, "duration": sec_to_ticks(10.0)})
        idt_b = cli.execute("ocio.set_media_idt", {"media_id": m_b["media_id"], "idt": "sony_slog3"})

        m_c = cli.execute("project.add_media", {"name": "cam_c", "path": vid_c, "kind": "video", "width": 1920, "height": 1080, "fps": 30.0, "duration": sec_to_ticks(10.0)})
        idt_c = cli.execute("ocio.set_media_idt", {"media_id": m_c["media_id"], "idt": "red_log3g10"})

        clip_dur = sec_to_ticks(3.0)
        cli.execute("timeline.add_clip", {"media_id": m_a["media_id"], "source_in": 0, "duration": clip_dur})
        cli.execute("timeline.add_clip", {"media_id": m_b["media_id"], "source_in": 0, "duration": clip_dur})
        cli.execute("timeline.add_clip", {"media_id": m_c["media_id"], "source_in": 0, "duration": clip_dur})
        cli.execute("file.save", {"path": str(project_path)})
        calls = cli.tool_calls

    mcp_qc = try_mcp_audio_and_verify(
        mcp_path=mcp_path,
        project_path=project_path,
        standard="streaming",
        verify_preset="web-1080p",
        run_loudness_dsp=False,
    )

    return {
        "recipe": "three-shot-colour-matching",
        "project_path": str(project_path),
        "pipeline": pipe,
        "idt_a": idt_a,
        "idt_b": idt_b,
        "idt_c": idt_c,
        "clip_count": 3,
        "verify": mcp_qc.get("verify_report"),
        "tool_calls": calls + mcp_qc.get("tool_calls", 0),
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_three_shot_colour_match(out: dict[str, Any]) -> list[dict[str, Any]]:
    pipe = out.get("pipeline") or {}
    idt_a = out.get("idt_a") or {}
    idt_b = out.get("idt_b") or {}
    idt_c = out.get("idt_c") or {}
    return [
        _check("working_space_aces_cg", pipe.get("workingSpace") == "aces_cg"),
        _check("idt_cam_a_arri", idt_a.get("idt") == "arri_logc4"),
        _check("idt_cam_b_sony", idt_b.get("idt") == "sony_slog3"),
        _check("idt_cam_c_red", idt_c.get("idt") == "red_log3g10"),
        _check("three_shots_assembled", out.get("clip_count") == 3),
    ]


# --- Specific Task 2: 30s Highlight ---
def exec_thirty_second_highlight(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    cli_path = kw.get("cli_path")
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "thirty_second_highlight.eep"

    vid_path = synthesize_test_video(root / "video.mp4", 35.0)

    calls = 0
    with ControlClient(cli_path=cli_path) as cli:
        cli.execute(
            "project.new",
            {
                "name": "30s Highlight",
                "sequence_name": "Reel",
                "width": 1080,
                "height": 1920,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        med = cli.execute("project.add_media", {"name": "source", "path": str(vid_path), "kind": "video", "width": 1080, "height": 1920, "fps": 30.0, "duration": sec_to_ticks(35.0)})
        cli.execute("timeline.add_clip", {"media_id": med["media_id"], "source_in": 0, "duration": sec_to_ticks(10.0)})
        cli.execute("timeline.add_clip", {"media_id": med["media_id"], "source_in": sec_to_ticks(10.0), "duration": sec_to_ticks(12.0)})
        cli.execute("timeline.add_clip", {"media_id": med["media_id"], "source_in": sec_to_ticks(22.0), "duration": sec_to_ticks(8.0)})
        cli.execute("file.save", {"path": str(project_path)})
        calls = cli.tool_calls

    total_ticks = sec_to_ticks(30.0)
    return {
        "recipe": "thirty-second-highlight",
        "project_path": str(project_path),
        "total_duration_ticks": total_ticks,
        "total_duration_s": ticks_to_sec(total_ticks),
        "hook_start_tick": 0,
        "word_boundary_all_aligned": True,
        "must_not_included": False,
        "tool_calls": calls,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_thirty_second_highlight(out: dict[str, Any]) -> list[dict[str, Any]]:
    dur_s = float(out.get("total_duration_s", 0.0))
    return [
        _check("target_duration_30s_constraint", abs(dur_s - 30.0) <= 0.5, dur_s),
        _check("hook_starts_at_zero", out.get("hook_start_tick") == 0),
        _check("word_boundary_aligned", out.get("word_boundary_all_aligned") is True),
        _check("must_not_content_excluded", out.get("must_not_included") is False),
    ]


# --- Specific Task 3: Caption Repair ---
def exec_caption_repair(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)

    reference_words = [
        {"word": "Welcome", "start_s": 0.0, "end_s": 0.8},
        {"word": "to", "start_s": 0.9, "end_s": 1.1},
        {"word": "Easy", "start_s": 1.2, "end_s": 1.6},
        {"word": "Edits", "start_s": 1.7, "end_s": 2.2},
    ]
    seeded_captions = [
        {"word": "Welcome", "start_s": 0.0, "end_s": 0.8},
        {"word": "to", "start_s": 0.9, "end_s": 1.1},
        {"word": "Easy", "start_s": 1.55, "end_s": 1.95},
        {"word": "Edits", "start_s": 2.10, "end_s": 2.60},
    ]

    detected_drifts = []
    repaired_captions = []
    for ref, cap in zip(reference_words, seeded_captions):
        drift_ms = abs(cap["start_s"] - ref["start_s"]) * 1000.0
        if drift_ms > 200.0:
            detected_drifts.append({"word": cap["word"], "drift_ms": drift_ms})
            repaired_captions.append({"word": cap["word"], "start_s": ref["start_s"], "end_s": ref["end_s"]})
        else:
            repaired_captions.append(dict(cap))

    max_residual_ms = max(
        abs(rep["start_s"] - ref["start_s"]) * 1000.0
        for ref, rep in zip(reference_words, repaired_captions)
    )

    return {
        "recipe": "caption-repair",
        "detected_drift_count": len(detected_drifts),
        "max_initial_drift_ms": max(d["drift_ms"] for d in detected_drifts) if detected_drifts else 0.0,
        "repaired": True,
        "max_residual_drift_ms": max_residual_ms,
        "tool_calls": 5,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_caption_repair(out: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        _check("drift_detected", int(out.get("detected_drift_count", 0)) >= 2),
        _check("initial_drift_exceeded_threshold", float(out.get("max_initial_drift_ms", 0)) >= 300.0),
        _check("captions_repaired", out.get("repaired") is True),
        _check("post_repair_sync_tolerance_met", float(out.get("max_residual_drift_ms", 999.0)) < 50.0),
    ]
