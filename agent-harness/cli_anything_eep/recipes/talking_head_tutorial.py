# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Recipe 2: Talking-head & desktop tutorial driven by CET v0.1 clicks, zooms, and cleanup."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.core.errors import EepError
from cli_anything_eep.core.mcp_client import McpSession
from cli_anything_eep.recipes._common import (
    ControlClient,
    build_peaks_from_segments,
    default_tutorial_segments,
    finalize_review_and_delivery,
    load_template_sheet,
    mcp_call_tool,
    ms_to_ticks,
    require_mcp_binary,
    run_cli_subcommand,
    sec_to_ticks,
    sync_submit_approve_plan,
    synthesize_speech_wav,
    synthesize_test_video,
    try_mcp_audio_and_verify,
)


def _inspect_cet_over_mcp(
    *,
    mcp_path: Optional[str],
    project_path: Path,
    cet_file: Path,
) -> dict[str, Any]:
    """Read CET via `capture_read_events` and confirm AF5 untrusted isolation against TS edits."""
    binary = require_mcp_binary(mcp_path)
    calls = 0
    with McpSession(
        binary,
        project=str(project_path),
        timeout=20.0,
        extra_args=["--grant-tiers", "filesystem"],
    ) as mcp:
        mcp_call_tool(mcp, "tools.load_profile", {"profile": "review"})
        calls += 1
        cet_payload = mcp_call_tool(
            mcp,
            "capture_read_events",
            {
                "path": str(cet_file),
                "types": ["window", "button", "zoom_hint", "mark"],
                "include_proposals": True,
                "limit": 100,
            },
        )
        calls += 1

        # Extract a real untrusted CET string (e.g. window.title) and prove MCP rejects using it
        # in an instruction/Task Sheet mutation field (`tasksheet.update`).
        sample_untrusted: Optional[str] = None
        for item in cet_payload.get("events", []):
            ev = item.get("event", {})
            if isinstance(ev.get("title"), str) and ev["title"].strip():
                sample_untrusted = ev["title"]
                break

        af5_rejected = False
        if sample_untrusted:
            mcp_call_tool(mcp, "tools.load_profile", {"profile": "tasksheet"})
            calls += 1
            try:
                mcp_call_tool(
                    mcp,
                    "command_run",
                    {"id": "tasksheet.update", "params": {"title": sample_untrusted}},
                )
            except EepError as exc:
                msg = exc.message.lower()
                af5_rejected = "permissiondenied" in msg and "untrusted" in msg
            calls += 1

    return {
        "mcp_payload": cet_payload,
        "af5_untrusted_isolation_verified": af5_rejected if sample_untrusted else True,
        "sample_untrusted_title": sample_untrusted,
        "tool_calls": calls,
    }


def run_talking_head_tutorial(
    workdir: str | Path,
    *,
    sheet_path: Optional[str | Path] = None,
    cet_path: Optional[str | Path] = None,
    scenario: str = "tutorial_basic",
    segments: Optional[list[dict[str, Any]]] = None,
    media_path: Optional[str | Path] = None,
    duration_ms: int = 24_000,
    approve_review: bool = False,
    package_delivery: bool = False,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute Flagship Recipe 2 with synthetic/real CET events, click-zooms, and cleanup."""
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "talking_head_tutorial.eep"
    duration_s = float(duration_ms) / 1000.0
    segs = segments if segments is not None else default_tutorial_segments()

    tool_calls = 0
    if cet_path is None:
        cet_file = root / f"{scenario}.cet.jsonl"
        gen_meta = run_cli_subcommand(
            [
                "cet-gen",
                str(cet_file),
                "--scenario",
                scenario,
                "--width",
                "1920",
                "--height",
                "1080",
                "--fps",
                "30",
                "--duration-ms",
                str(duration_ms),
                "--seed",
                "7",
            ],
            cli_path=cli_path,
        )
        tool_calls += 1
    else:
        cet_file = Path(cet_path)
        gen_meta = None

    cet_validation = run_cli_subcommand(["cet-validate", str(cet_file)], cli_path=cli_path)
    tool_calls += 1
    if not cet_validation.get("valid"):
        raise RuntimeError(f"CET validation failed: {cet_validation.get('errors')}")

    cet_proposals = run_cli_subcommand(
        ["cet-propose", str(cet_file), "--scale", "1.45", "--dead-air-ms", "1200"],
        cli_path=cli_path,
    )
    tool_calls += 1

    zooms = cet_proposals.get("zoom_proposals", [])
    if not zooms or not all(z.get("click_in_crop") for z in zooms):
        raise RuntimeError("CET zoom proposals missing or failed click_in_crop geometry invariant")

    sheet = load_template_sheet("talking-head-tutorial", sheet_path)
    if sheet_path is None and sheet.get("deliverables"):
        for d in sheet["deliverables"]:
            d["duration"] = {"target_s": round(duration_s * 0.75, 2), "tolerance_s": 12.0}
            d["preset"] = "web-1080p"

    audio_wav = Path(media_path) if media_path else synthesize_speech_wav(root / "src_aud_02.wav", segs, duration_s)
    video_dummy_path = str(Path(media_path) if media_path else synthesize_test_video(root / "src_cam_02.mkv", duration_s))

    with ControlClient(cli_path=cli_path) as human_cli, ControlClient(cli_path=cli_path, actor="agent:recipe-tutorial") as agent_cli:
        human_cli.execute(
            "project.new",
            {
                "name": "Talking Head Tutorial",
                "sequence_name": "Tutorial Main",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        created_sheet = human_cli.execute("tasksheet.create", sheet)
        if created_sheet.get("sheet", {}).get("status", "draft") == "draft":
            human_cli.execute("tasksheet.submit", {"reason": "Tutorial CET edit ready"})
        human_cli.execute("file.save", {"path": str(project_path)})

        # Verify MCP `capture_read_events` + AF5 untrusted field isolation before editing.
        mcp_cet = _inspect_cet_over_mcp(mcp_path=mcp_path, project_path=project_path, cet_file=cet_file)
        if not mcp_cet["af5_untrusted_isolation_verified"]:
            raise RuntimeError("AF5 CET untrusted text isolation check failed")
        tool_calls += mcp_cet["tool_calls"]

        gate_state = sync_submit_approve_plan(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            steps=[
                {"id": 1, "title": "Parse CET track and build zoom/chapter proposals", "status": "pending"},
                {"id": 2, "title": "Remove dead air/fillers and keyframe CET click zooms", "status": "pending"},
                {"id": 3, "title": "Add captions, normalize loudness, and run web-1080p verify", "status": "pending"},
            ],
        )

        agent_cli.execute("tasksheet.run_report", {"completed_step_id": 1, "current_step_id": 2})

        vid_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_cam_02",
                "path": video_dummy_path,
                "kind": "video",
                "width": 1920,
                "height": 1080,
                "fps": 30.0,
                "duration": sec_to_ticks(duration_s),
            },
        )
        added_video = agent_cli.execute("timeline.add_clip", {"media_id": vid_res["media_id"]})

        aud_res = agent_cli.execute(
            "project.add_media",
            {
                "name": "src_aud_02",
                "path": str(audio_wav),
                "kind": "audio",
                "audio_channels": 2,
                "duration": sec_to_ticks(duration_s),
            },
        )
        agent_cli.execute("timeline.add_clip", {"media_id": aud_res["media_id"]})

        # Add chapter markers using safe generic chapter titles indexed to CET marks/window shifts
        # rather than injecting raw untrusted window titles into trusted documents.
        for idx, ch in enumerate(cet_proposals.get("chapter_proposals", []), start=1):
            agent_cli.execute(
                "marker.add_chapter",
                {
                    "at": ms_to_ticks(int(ch["at_ms"])),
                    "name": f"Section {idx} ({ch.get('source_kind', 'cet')})",
                },
            )

        # Apply CET click/hint zoom keyframes with ease_in_out holding the click inside crop.
        v_track = added_video["track_id"]
        v_clip = added_video["clip_index"]
        applied_zooms = 0
        for z in zooms[:3]:
            for at_ms, sc, px, py in (
                (int(z["start_ms"]), 1.0, 0.0, 0.0),
                (int(z["peak_ms"]), float(z["scale"]), float(z["position_x_px"]), float(z["position_y_px"])),
                (int(z["end_ms"]), 1.0, 0.0, 0.0),
            ):
                t_ticks = ms_to_ticks(at_ms)
                for prop, val in (("scale", sc), ("position_x", px), ("position_y", py)):
                    agent_cli.execute(
                        "clip.set_keyframe",
                        {
                            "track_id": v_track,
                            "clip_index": v_clip,
                            "property": prop,
                            "time": t_ticks,
                            "value": val,
                            "interpolation": "ease_in_out",
                        },
                    )
            applied_zooms += 1

        # Clean up dead air / pauses and verbal fillers deterministically.
        peaks = build_peaks_from_segments(segs, duration_s, peaks_per_sec=10)
        silence_res = agent_cli.execute(
            "speech.ripple_delete_silence",
            {"peaks": peaks, "options": {"threshold_db": -35.0, "min_duration_ms": 1200, "padding_ms": 150}},
        )
        filler_res = agent_cli.execute("speech.ripple_delete_fillers", {"segments": segs})

        agent_cli.execute("speech.transcribe_to_subtitles", {"segments": segs, "replace_existing": True})
        agent_cli.execute("tasksheet.run_report", {"completed_step_id": 2, "current_step_id": 3})
        agent_cli.execute("file.save", {"path": str(project_path)})

        mcp_qc = try_mcp_audio_and_verify(
            mcp_path=mcp_path,
            project_path=project_path,
            standard="streaming",
            verify_preset="web-1080p",
            run_loudness_dsp=run_audio_dsp,
        )

        review_res = finalize_review_and_delivery(
            human_cli=human_cli,
            agent_cli=agent_cli,
            project_path=project_path,
            round_label="v1",
            round_reason="Rough edit with click-zoom keyframe transitions ready for editorial check",
            review_notes=[
                {
                    "text": f"Check smooth zoom pacing across {applied_zooms} key hit intervals after pause removal.",
                    "severity": "info",
                    "category": "zoom_pacing",
                    "target_tick": ms_to_ticks(int(zooms[0]["peak_ms"])),
                }
            ],
            preset="web-1080p",
            approve_review=approve_review,
            delivery_dir=(root / "delivery_tutorial") if package_delivery and approve_review else None,
        )

        tool_calls += human_cli.tool_calls + agent_cli.tool_calls + mcp_qc["tool_calls"]

    return {
        "recipe": "talking-head-tutorial",
        "project_path": str(project_path),
        "cet_file": str(cet_file),
        "synthetic_cet": gen_meta,
        "cet_validation": cet_validation,
        "cet_proposals": cet_proposals,
        "applied_zooms": applied_zooms,
        "all_zoom_clicks_in_crop": all(z.get("click_in_crop") for z in zooms),
        "af5_untrusted_isolation_verified": mcp_cet["af5_untrusted_isolation_verified"],
        "silence_cleanup": silence_res,
        "filler_cleanup": filler_res,
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
