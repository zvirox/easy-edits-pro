# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""EEP-Bench deterministic scripted task specifications and AF5 CET safety task."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable, Optional

from cli_anything_eep.core.errors import EepError
from cli_anything_eep.core.mcp_client import McpSession
from cli_anything_eep.recipes import run_podcast_to_clips, run_talking_head_tutorial, run_webinar_recap
from cli_anything_eep.recipes._common import (
    ControlClient,
    mcp_call_tool,
    require_mcp_binary,
    run_cli_subcommand,
)


def _check(name: str, ok: bool, detail: Any = None) -> dict[str, Any]:
    return {"name": name, "passed": bool(ok), "detail": detail}


def _run_cet_safety_isolation_task(
    workdir: str | Path,
    *,
    run_audio_dsp: bool = False,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Verify privacy defaults, bounded `cet-query`, traversal guard, and AF5 untrusted origin isolation."""
    _ = run_audio_dsp
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    cet_file = root / "redacted.cet.jsonl"
    project_path = root / "cet_safety.eep"

    gen_res = run_cli_subcommand(
        [
            "cet-gen",
            str(cet_file),
            "--scenario",
            "redacted_privacy",
            "--duration-ms",
            "12000",
            "--seed",
            "19",
        ],
        cli_path=cli_path,
    )
    val_res = run_cli_subcommand(["cet-validate", str(cet_file)], cli_path=cli_path)
    query_res = run_cli_subcommand(["cet-query", str(cet_file), "--limit", "20"], cli_path=cli_path)

    with ControlClient(cli_path=cli_path) as human_cli:
        human_cli.execute(
            "project.new",
            {
                "name": "CET Safety Check",
                "sequence_name": "Main",
                "width": 1920,
                "height": 1080,
                "frame_rate": {"numerator": 30, "denominator": 1},
            },
        )
        human_cli.execute("file.save", {"path": str(project_path)})
        cli_calls = 3 + human_cli.tool_calls

    mcp_binary = require_mcp_binary(mcp_path)
    traversal_denied = False
    af5_ts_denied = False
    mcp_notice = ""
    mcp_calls = 0

    with McpSession(
        mcp_binary,
        project=str(project_path),
        timeout=20.0,
        extra_args=["--grant-tiers", "filesystem"],
    ) as mcp:
        mcp_call_tool(mcp, "tools.load_profile", {"profile": "review"})
        mcp_calls += 1

        try:
            mcp_call_tool(mcp, "capture_read_events", {"path": "../../etc/passwd.cet.jsonl"})
        except EepError as exc:
            traversal_denied = "PermissionDenied" in exc.message or "outside project" in exc.message
        mcp_calls += 1

        read_res = mcp_call_tool(
            mcp,
            "capture_read_events",
            {"path": str(cet_file), "include_proposals": True, "limit": 50},
        )
        mcp_calls += 1
        mcp_notice = str(read_res.get("untrusted_notice", ""))

        # Extract untrusted window title / ui.label captured in CET and verify TS mutation refuses it.
        untrusted_title = None
        for item in read_res.get("events", []):
            ev = item.get("event", {})
            if isinstance(ev.get("title"), str) and ev["title"].strip():
                untrusted_title = ev["title"]
                break
        if untrusted_title:
            mcp_call_tool(mcp, "tools.load_profile", {"profile": "tasksheet"})
            mcp_calls += 1
            try:
                mcp_call_tool(
                    mcp,
                    "command_run",
                    {"id": "tasksheet.update", "params": {"title": untrusted_title}},
                )
            except EepError as exc:
                msg = exc.message.lower()
                af5_ts_denied = "permissiondenied" in msg and "untrusted" in msg
            mcp_calls += 1

    return {
        "recipe": "cet-untrusted-isolation",
        "cet_generated": gen_res,
        "cet_validation": val_res,
        "cet_query": query_res,
        "traversal_denied": traversal_denied,
        "af5_tasksheet_denied": af5_ts_denied,
        "mcp_untrusted_notice": mcp_notice,
        "verify": None,
        "tool_calls": cli_calls + mcp_calls,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def _eval_podcast(out: dict[str, Any]) -> list[dict[str, Any]]:
    verify = out.get("verify") or {}
    summary = verify.get("summary") or {}
    notes = verify.get("notes") or {}
    review = out.get("review") or {}
    dlv = review.get("delivery_verified") or {}
    return [
        _check("word_boundary_alignment", out.get("word_boundary_all_aligned") is True),
        _check("must_not_candidate_rejected", int(out.get("rejected_must_not_segments", 0)) >= 1),
        _check("vertical_9x16_fill_crop", (out.get("frame_fit") or {}).get("mode") == "fill"),
        _check("plan_gate_denied_to_agent", out.get("plan_gate_denied_to_agent") is True),
        _check("review_close_denied_to_agent", review.get("agent_close_denied") is True),
        _check("verify_zero_failures", summary.get("failed", 1) == 0, summary),
        _check("no_invented_quality_score", notes.get("aggregate_score") is None),
        _check("delivery_checksum_verified", dlv.get("verified") is True),
    ]


def _eval_tutorial(out: dict[str, Any]) -> list[dict[str, Any]]:
    verify = out.get("verify") or {}
    summary = verify.get("summary") or {}
    cet_val = out.get("cet_validation") or {}
    review = out.get("review") or {}
    dlv = review.get("delivery_verified") or {}
    return [
        _check("cet_validation_passed", cet_val.get("valid") is True, cet_val.get("errors")),
        _check("zoom_applied", int(out.get("applied_zooms", 0)) >= 1),
        _check("zoom_clicks_kept_in_crop", out.get("all_zoom_clicks_in_crop") is True),
        _check("af5_cet_untrusted_isolated", out.get("af5_untrusted_isolation_verified") is True),
        _check("plan_gate_denied_to_agent", out.get("plan_gate_denied_to_agent") is True),
        _check("verify_zero_failures", summary.get("failed", 1) == 0, summary),
        _check("delivery_checksum_verified", dlv.get("verified") is True),
    ]


def _eval_webinar(out: dict[str, Any]) -> list[dict[str, Any]]:
    v_full = (out.get("verify_full") or {}).get("summary") or {}
    v_reel = (out.get("verify_reel") or {}).get("summary") or {}
    esc = out.get("escalation") or {}
    analysis = out.get("analysis") or {}
    review = out.get("review") or {}
    dlv = review.get("delivery_verified") or {}
    return [
        _check("must_include_escalation_triggered", esc.get("asked", {}).get("status") == "needs_input"),
        _check("agent_cannot_self_answer_question", esc.get("agent_answer_denied") is True),
        _check("must_not_segments_flagged", len(analysis.get("flagged_must_not", [])) >= 1),
        _check("summary_reel_excludes_must_not", out.get("reel_excludes_all_must_not") is True),
        _check("summary_reel_has_highlights", len(out.get("reel_included_segment_ids", [])) >= 1),
        _check("verify_full_zero_failures", v_full.get("failed", 1) == 0, v_full),
        _check("verify_reel_zero_failures", v_reel.get("failed", 1) == 0, v_reel),
        _check("delivery_checksum_verified", dlv.get("verified") is True),
    ]


def _eval_cet_safety(out: dict[str, Any]) -> list[dict[str, Any]]:
    val = out.get("cet_validation") or {}
    qry = out.get("cet_query") or {}
    return [
        _check("cet_privacy_classes_only", val.get("privacy_keys") == "classes_only"),
        _check("cet_privacy_text_safe", val.get("privacy_text") in ("off", "redacted")),
        _check("cet_query_af5_notice", "AF5_UNTRUSTED_CET" in str(qry.get("untrusted_notice", ""))),
        _check("mcp_af5_notice", "AF5_UNTRUSTED_CET" in str(out.get("mcp_untrusted_notice", ""))),
        _check("mcp_traversal_blocked", out.get("traversal_denied") is True),
        _check("af5_ts_mutation_blocked", out.get("af5_tasksheet_denied") is True),
    ]


def _exec_podcast(workdir: Path, **kw: Any) -> dict[str, Any]:
    return run_podcast_to_clips(workdir, approve_review=True, package_delivery=True, **kw)


def _exec_tutorial(workdir: Path, **kw: Any) -> dict[str, Any]:
    return run_talking_head_tutorial(workdir, approve_review=True, package_delivery=True, **kw)


def _exec_webinar(workdir: Path, **kw: Any) -> dict[str, Any]:
    return run_webinar_recap(
        workdir,
        simulate_missing_topic=True,
        approve_review=True,
        package_delivery=True,
        **kw,
    )


BENCH_TASKS: dict[str, dict[str, Any]] = {
    "bench_podcast_to_clips": {
        "id": "bench_podcast_to_clips",
        "recipe": "podcast-to-clips",
        "description": "Podcast clip ranking, word-boundary cuts, 9:16 fill crop, review flags, and TS5 delivery.",
        "executor": _exec_podcast,
        "evaluator": _eval_podcast,
    },
    "bench_talking_head_cet": {
        "id": "bench_talking_head_cet",
        "recipe": "talking-head-tutorial",
        "description": "Synthetic CET v0.1 tutorial track, click-in-crop zoom keyframes, silence/filler removal, and TS5 delivery.",
        "executor": _exec_tutorial,
        "evaluator": _eval_tutorial,
    },
    "bench_webinar_recap": {
        "id": "bench_webinar_recap",
        "recipe": "webinar-recap",
        "description": "Webinar chaptering, must_include escalation, must_not exclusion in summary reel, and TS5 delivery.",
        "executor": _exec_webinar,
        "evaluator": _eval_webinar,
    },
    "bench_cet_untrusted_isolation": {
        "id": "bench_cet_untrusted_isolation",
        "recipe": "cet-untrusted-isolation",
        "description": "Negative AF5 security gate: redacted CET privacy defaults, traversal denial, and untrusted text provenance containment.",
        "executor": _run_cet_safety_isolation_task,
        "evaluator": _eval_cet_safety,
    },
}


def list_bench_tasks() -> list[dict[str, str]]:
    return [
        {
            "id": spec["id"],
            "recipe": spec["recipe"],
            "description": spec["description"],
        }
        for spec in BENCH_TASKS.values()
    ]
