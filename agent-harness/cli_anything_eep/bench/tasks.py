# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""EEP-Bench task specifications covering flagships, §2.6 companions, specific tasks, and negative controls."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cli_anything_eep.bench.negative_controls import (
    eval_cet_safety,
    eval_neg_caption_sync_drift,
    eval_neg_loudness_violation,
    eval_neg_missing_topic_escalation,
    eval_neg_untrusted_origin_rejection,
    exec_neg_caption_sync_drift,
    exec_neg_loudness_violation,
    exec_neg_missing_topic_escalation,
    exec_neg_untrusted_origin_rejection,
    run_cet_safety_isolation_task,
)
from cli_anything_eep.bench.specific_tasks import (
    eval_caption_repair,
    eval_thirty_second_highlight,
    eval_three_shot_colour_match,
    exec_caption_repair,
    exec_thirty_second_highlight,
    exec_three_shot_colour_match,
)
from cli_anything_eep.recipes import (
    run_captioned_vertical_excerpt,
    run_music_bed_ducking_master,
    run_podcast_cleanup,
    run_podcast_to_clips,
    run_sequence_colour_matching,
    run_talking_head_tutorial,
    run_transcript_interview_rough_cut,
    run_webinar_recap,
)


def _check(name: str, ok: bool, detail: Any = None) -> dict[str, Any]:
    return {"name": name, "passed": bool(ok), "detail": detail}


# --- Flagship 1: Podcast to Clips ---
def _exec_podcast(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    return run_podcast_to_clips(workdir, approve_review=True, package_delivery=True, **kw)


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


# --- Flagship 2: Talking Head Tutorial (CET) ---
def _exec_tutorial(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    return run_talking_head_tutorial(workdir, approve_review=True, package_delivery=True, **kw)


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


# --- Flagship 3: Webinar Recap ---
def _exec_webinar(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    return run_webinar_recap(
        workdir,
        simulate_missing_topic=True,
        approve_review=True,
        package_delivery=True,
        **kw,
    )


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


# --- Companion §2.6 Recipe 1: Podcast Cleanup ---
def _exec_podcast_cleanup(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    kw.pop("run_audio_dsp", None)
    return run_podcast_cleanup(workdir, approve_review=True, package_delivery=True, auto_merge=True, **kw)


def _eval_podcast_cleanup(out: dict[str, Any]) -> list[dict[str, Any]]:
    gate_state = out.get("gate_state") or {}
    review = out.get("review") or {}
    merged = out.get("merged") or {}
    return [
        _check("silence_deleted_ranges", int(out.get("silence_deleted_ranges", 0)) >= 1),
        _check("filler_deleted_ranges", int(out.get("filler_deleted_ranges", 0)) >= 1),
        _check("plan_gate_denied_to_agent", gate_state.get("agent_gate_denied") is True),
        _check("human_review_approved", review.get("human_approved") is True),
        _check("branch_merged_accept", merged.get("merged") == "accept"),
    ]


# --- Companion §2.6 Recipe 2: Transcript Interview Rough Cut ---
def _exec_transcript_interview_rough_cut(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    kw.pop("run_audio_dsp", None)
    return run_transcript_interview_rough_cut(workdir, approve_review=True, package_delivery=True, auto_merge=True, **kw)


def _eval_transcript_interview_rough_cut(out: dict[str, Any]) -> list[dict[str, Any]]:
    review = out.get("review") or {}
    merged = out.get("merged") or {}
    return [
        _check("selected_segments_count", int(out.get("selected_segments_count", 0)) >= 1),
        _check("assembled_duration_positive", int(out.get("assembled_duration_ticks", 0)) > 0),
        _check("human_review_approved", review.get("human_approved") is True),
        _check("branch_merged_accept", merged.get("merged") == "accept"),
    ]


# --- Companion §2.6 Recipe 3: Captioned Vertical Excerpt ---
def _exec_captioned_vertical_excerpt(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    kw.pop("run_audio_dsp", None)
    return run_captioned_vertical_excerpt(workdir, approve_review=True, package_delivery=True, **kw)


def _eval_captioned_vertical_excerpt(out: dict[str, Any]) -> list[dict[str, Any]]:
    review = out.get("review") or {}
    return [
        _check("clip_duration_positive", int(out.get("clip_duration_ticks", 0)) > 0),
        _check("human_review_approved", review.get("human_approved") is True),
        _check("verify_report_present", out.get("verify_report") is not None),
    ]


# --- Companion §2.6 Recipe 4: Sequence Colour Matching ---
def _exec_sequence_colour_matching(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    kw.pop("run_audio_dsp", None)
    return run_sequence_colour_matching(workdir, approve_review=True, package_delivery=False, **kw)


def _eval_sequence_colour_matching(out: dict[str, Any]) -> list[dict[str, Any]]:
    pipeline = out.get("pipeline") or {}
    idt_a = out.get("idt_a") or {}
    idt_b = out.get("idt_b") or {}
    return [
        _check("working_space_aces_cg", pipeline.get("workingSpace") == "aces_cg"),
        _check("idt_a_arri_logc4", idt_a.get("idt") == "arri_logc4"),
        _check("idt_b_sony_slog3", idt_b.get("idt") == "sony_slog3"),
        _check("verify_report_present", out.get("verify_report") is not None),
    ]


# --- Companion §2.6 Recipe 5: Music Bed Ducking Master ---
def _exec_music_bed_ducking_master(workdir: Path, **kw: Any) -> dict[str, Any]:
    kw.pop("agent_config", None)
    kw.pop("run_audio_dsp", None)
    return run_music_bed_ducking_master(workdir, approve_review=True, package_delivery=False, **kw)


def _eval_music_bed_ducking_master(out: dict[str, Any]) -> list[dict[str, Any]]:
    review = out.get("review") or {}
    return [
        _check("music_track_created", int(out.get("music_track_id", 0)) >= 1),
        _check("human_review_approved", review.get("human_approved") is True),
        _check("verify_report_present", out.get("verify_report") is not None),
    ]


# --- BENCH_TASKS Registry (16 Tasks) ---
BENCH_TASKS: dict[str, dict[str, Any]] = {
    "bench_podcast_to_clips": {
        "id": "bench_podcast_to_clips",
        "recipe": "podcast-to-clips",
        "category": "flagship",
        "description": "Podcast clip ranking, word-boundary cuts, 9:16 fill crop, review flags, and TS5 delivery.",
        "executor": _exec_podcast,
        "evaluator": _eval_podcast,
    },
    "bench_talking_head_cet": {
        "id": "bench_talking_head_cet",
        "recipe": "talking-head-tutorial",
        "category": "flagship",
        "description": "Synthetic CET v0.1 tutorial track, click-in-crop zoom keyframes, silence/filler removal, and TS5 delivery.",
        "executor": _exec_tutorial,
        "evaluator": _eval_tutorial,
    },
    "bench_webinar_recap": {
        "id": "bench_webinar_recap",
        "recipe": "webinar-recap",
        "category": "flagship",
        "description": "Webinar chaptering, must_include escalation, must_not exclusion in summary reel, and TS5 delivery.",
        "executor": _exec_webinar,
        "evaluator": _eval_webinar,
    },
    "bench_podcast_cleanup": {
        "id": "bench_podcast_cleanup",
        "recipe": "podcast-cleanup",
        "category": "companion",
        "description": "Podcast dialogue cleanup, silence/filler reduction in branch, human review, and branch merge.",
        "executor": _exec_podcast_cleanup,
        "evaluator": _eval_podcast_cleanup,
    },
    "bench_transcript_interview_rough_cut": {
        "id": "bench_transcript_interview_rough_cut",
        "recipe": "transcript-interview-rough-cut",
        "category": "companion",
        "description": "Transcript-driven interview rough cut assembly, speaker selection, human review, and merge.",
        "executor": _exec_transcript_interview_rough_cut,
        "evaluator": _eval_transcript_interview_rough_cut,
    },
    "bench_captioned_vertical_excerpt": {
        "id": "bench_captioned_vertical_excerpt",
        "recipe": "captioned-vertical-excerpt",
        "category": "companion",
        "description": "Vertical 9:16 highlight excerpt with synchronized burn-in captions and QC verification.",
        "executor": _exec_captioned_vertical_excerpt,
        "evaluator": _eval_captioned_vertical_excerpt,
    },
    "bench_sequence_colour_matching": {
        "id": "bench_sequence_colour_matching",
        "recipe": "sequence-colour-matching",
        "category": "companion",
        "description": "OCIO ACEScg pipeline setup, ARRI/Sony camera IDT mapping, timeline assembly, and scope verification.",
        "executor": _exec_sequence_colour_matching,
        "evaluator": _eval_sequence_colour_matching,
    },
    "bench_music_bed_ducking_master": {
        "id": "bench_music_bed_ducking_master",
        "recipe": "music-bed-ducking-master",
        "category": "companion",
        "description": "Music bed ducking under speech, EBU R128 mastering, and delivery packaging.",
        "executor": _exec_music_bed_ducking_master,
        "evaluator": _eval_music_bed_ducking_master,
    },
    "bench_three_shot_colour_match": {
        "id": "bench_three_shot_colour_match",
        "recipe": "three-shot-colour-matching",
        "category": "specific",
        "description": "Multi-cam 3-shot colour matching across ARRI LogC4, Sony S-Log3, and RED Log3G10 in ACEScg.",
        "executor": exec_three_shot_colour_match,
        "evaluator": eval_three_shot_colour_match,
    },
    "bench_thirty_second_highlight": {
        "id": "bench_thirty_second_highlight",
        "recipe": "thirty-second-highlight",
        "category": "specific",
        "description": "Precise 30s highlight reel assembly with duration constraint (30.0s ± 0.5s) and hook at head.",
        "executor": exec_thirty_second_highlight,
        "evaluator": eval_thirty_second_highlight,
    },
    "bench_caption_repair": {
        "id": "bench_caption_repair",
        "recipe": "caption-repair",
        "category": "specific",
        "description": "Speech-to-caption drift detection (> 200ms) and automatic word timestamp repair (< 50ms tolerance).",
        "executor": exec_caption_repair,
        "evaluator": eval_caption_repair,
    },
    "bench_neg_loudness_violation": {
        "id": "bench_neg_loudness_violation",
        "recipe": "neg-loudness-violation",
        "category": "negative_control",
        "description": "Negative control: seeded audio clipping (+1.8 dBTP) caught by QC; delivery blocked without false pass.",
        "executor": exec_neg_loudness_violation,
        "evaluator": eval_neg_loudness_violation,
    },
    "bench_neg_caption_sync_drift": {
        "id": "bench_neg_caption_sync_drift",
        "recipe": "neg-caption-sync-drift",
        "category": "negative_control",
        "description": "Negative control: severe caption sync drift (+1250ms) caught by QC; delivery blocked.",
        "executor": exec_neg_caption_sync_drift,
        "evaluator": eval_neg_caption_sync_drift,
    },
    "bench_neg_untrusted_origin_rejection": {
        "id": "bench_neg_untrusted_origin_rejection",
        "recipe": "neg-untrusted-origin-rejection",
        "category": "negative_control",
        "description": "Negative control: directory traversal and untrusted origin injection refused with PermissionDenied.",
        "executor": exec_neg_untrusted_origin_rejection,
        "evaluator": eval_neg_untrusted_origin_rejection,
    },
    "bench_neg_missing_topic_escalation": {
        "id": "bench_neg_missing_topic_escalation",
        "recipe": "neg-missing-topic-escalation",
        "category": "negative_control",
        "description": "Negative control: missing mandatory agenda item escalates to needs_input; agent cannot self-answer.",
        "executor": exec_neg_missing_topic_escalation,
        "evaluator": eval_neg_missing_topic_escalation,
    },
    "bench_cet_untrusted_isolation": {
        "id": "bench_cet_untrusted_isolation",
        "recipe": "cet-untrusted-isolation",
        "category": "security_gate",
        "description": "Negative AF5 security gate: redacted CET privacy defaults, traversal denial, and untrusted text provenance containment.",
        "executor": run_cet_safety_isolation_task,
        "evaluator": eval_cet_safety,
    },
}


def list_bench_tasks() -> list[dict[str, str]]:
    return [
        {
            "id": spec["id"],
            "recipe": spec["recipe"],
            "category": spec.get("category", "general"),
            "description": spec["description"],
        }
        for spec in BENCH_TASKS.values()
    ]
