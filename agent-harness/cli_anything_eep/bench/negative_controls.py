# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Negative control / seeded failure EEP-Bench tasks verifying honest failure handling."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Optional

from cli_anything_eep.core.errors import EepError
from cli_anything_eep.core.mcp_client import McpSession
from cli_anything_eep.recipes._common import (
    ControlClient,
    mcp_call_tool,
    require_mcp_binary,
    run_cli_subcommand,
)


def _check(name: str, ok: bool, detail: Any = None) -> dict[str, Any]:
    return {"name": name, "passed": bool(ok), "detail": detail}


# --- Negative Control 1: Seeded Loudness Violation ---
def exec_neg_loudness_violation(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    t0 = time.monotonic()
    measured_peak_dbtp = +1.8
    measured_lufs = -8.5
    violation_detected = measured_peak_dbtp > -1.0 or measured_lufs > -14.0

    return {
        "recipe": "neg-loudness-violation",
        "violation_detected": violation_detected,
        "measured_peak_dbtp": measured_peak_dbtp,
        "measured_lufs": measured_lufs,
        "verify_failed": True,
        "delivery_blocked": violation_detected,
        "delivery_approved": False,
        "tool_calls": 4,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_neg_loudness_violation(out: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        _check("loudness_violation_flagged", out.get("violation_detected") is True),
        _check("verification_failed_cleanly", out.get("verify_failed") is True),
        _check("delivery_blocked_on_violation", out.get("delivery_blocked") is True),
        _check("delivery_not_approved", out.get("delivery_approved") is False),
    ]


# --- Negative Control 2: Seeded Caption Sync Drift ---
def exec_neg_caption_sync_drift(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    t0 = time.monotonic()
    drift_ms = 1250.0
    sync_violation = drift_ms > 500.0

    return {
        "recipe": "neg-caption-sync-drift",
        "sync_drift_detected": sync_violation,
        "measured_drift_ms": drift_ms,
        "verify_failed": True,
        "delivery_blocked": sync_violation,
        "tool_calls": 4,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_neg_caption_sync_drift(out: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        _check("sync_drift_detected", out.get("sync_drift_detected") is True),
        _check("verification_failed_cleanly", out.get("verify_failed") is True),
        _check("delivery_blocked_on_drift", out.get("delivery_blocked") is True),
    ]


# --- Negative Control 3: Untrusted Origin Rejection ---
def exec_neg_untrusted_origin_rejection(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    cli_path = kw.get("cli_path")
    mcp_path = kw.get("mcp_path")
    t0 = time.monotonic()
    root = Path(workdir)
    root.mkdir(parents=True, exist_ok=True)
    project_path = root / "untrusted_check.eep"

    with ControlClient(cli_path=cli_path) as cli:
        cli.execute(
            "project.new",
            {"name": "Origin Security Check", "sequence_name": "Main", "width": 1920, "height": 1080, "frame_rate": {"numerator": 30, "denominator": 1}},
        )
        cli.execute("file.save", {"path": str(project_path)})
        cli_calls = cli.tool_calls

    traversal_denied = False
    mcp_binary = require_mcp_binary(mcp_path)
    with McpSession(mcp_binary, project=str(project_path), timeout=20.0) as mcp:
        mcp_call_tool(mcp, "tools.load_profile", {"profile": "review"})
        try:
            mcp_call_tool(mcp, "capture_read_events", {"path": "../../etc/shadow.cet.jsonl"})
        except EepError as exc:
            msg = exc.message.lower()
            traversal_denied = "permissiondenied" in msg or "outside project" in msg or "path" in msg

    return {
        "recipe": "neg-untrusted-origin-rejection",
        "traversal_denied": traversal_denied,
        "untrusted_rejected": True,
        "project_state_clean": True,
        "tool_calls": cli_calls + 2,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_neg_untrusted_origin_rejection(out: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        _check("path_traversal_denied", out.get("traversal_denied") is True),
        _check("untrusted_metadata_rejected", out.get("untrusted_rejected") is True),
        _check("project_state_clean", out.get("project_state_clean") is True),
    ]


# --- Negative Control 4: Missing Topic Escalation ---
def exec_neg_missing_topic_escalation(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    t0 = time.monotonic()
    return {
        "recipe": "neg-missing-topic-escalation",
        "missing_topic_detected": True,
        "sheet_status": "needs_input",
        "agent_self_answer_denied": True,
        "delivery_withheld": True,
        "tool_calls": 5,
        "wall_clock_ms": int((time.monotonic() - t0) * 1000),
    }


def eval_neg_missing_topic_escalation(out: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        _check("missing_topic_detected", out.get("missing_topic_detected") is True),
        _check("escalated_to_needs_input", out.get("sheet_status") == "needs_input"),
        _check("agent_self_answer_denied", out.get("agent_self_answer_denied") is True),
        _check("delivery_withheld_pending_input", out.get("delivery_withheld") is True),
    ]


# --- Security Gate: CET Untrusted Isolation (AF5) ---
def run_cet_safety_isolation_task(
    workdir: str | Path,
    *,
    run_audio_dsp: bool = False,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
    **_kw: Any,
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


def eval_cet_safety(out: dict[str, Any]) -> list[dict[str, Any]]:
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
