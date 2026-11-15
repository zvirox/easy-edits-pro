# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Integration tests for the Five Companion §2.6 Recipes (AF7 / Packet 7.1)."""

from pathlib import Path

import pytest
from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.recipes import (
    RECIPES,
    list_recipes,
    run_captioned_vertical_excerpt,
    run_music_bed_ducking_master,
    run_podcast_cleanup,
    run_sequence_colour_matching,
    run_transcript_interview_rough_cut,
)


def test_recipe_registry_includes_all_companion_recipes():
    recipes = list_recipes()
    ids = {r["id"] for r in recipes}
    expected = {
        "podcast-to-clips",
        "talking-head-tutorial",
        "webinar-recap",
        "podcast-cleanup",
        "transcript-interview-rough-cut",
        "captioned-vertical-excerpt",
        "sequence-colour-matching",
        "music-bed-ducking-master",
    }
    assert expected.issubset(ids), f"Missing recipes: {expected - ids}"


def test_recipe1_podcast_cleanup_end_to_end(tmp_path: Path):
    out = run_podcast_cleanup(
        tmp_path / "r_cleanup",
        approve_review=True,
        package_delivery=True,
        auto_merge=True,
    )
    assert out["recipe"] == "podcast-cleanup"
    assert Path(out["project_path"]).is_file()
    assert out["branch"] == "agent/podcast-cleanup"
    assert out["silence_deleted_ranges"] >= 1
    assert out["filler_deleted_ranges"] >= 1
    assert out["gate_state"]["agent_gate_denied"] is True
    assert out["review"]["human_approved"] is True
    assert out["merged"] is not None
    assert out["merged"]["merged"] == "accept"
    assert out["verify_report"] is not None
    assert out["tool_calls"] >= 5


def test_recipe2_transcript_interview_rough_cut_end_to_end(tmp_path: Path):
    out = run_transcript_interview_rough_cut(
        tmp_path / "r_interview",
        approve_review=True,
        package_delivery=True,
        auto_merge=True,
    )
    assert out["recipe"] == "transcript-interview-rough-cut"
    assert Path(out["project_path"]).is_file()
    assert out["selected_segments_count"] >= 1
    assert out["assembled_duration_ticks"] > 0
    assert out["review"]["human_approved"] is True
    assert out["merged"]["merged"] == "accept"


def test_recipe3_captioned_vertical_excerpt_end_to_end(tmp_path: Path):
    out = run_captioned_vertical_excerpt(
        tmp_path / "r_vertical",
        approve_review=True,
        package_delivery=True,
    )
    assert out["recipe"] == "captioned-vertical-excerpt"
    assert Path(out["project_path"]).is_file()
    assert out["clip_duration_ticks"] > 0
    assert out["verify_report"] is not None
    assert out["review"]["human_approved"] is True


def test_recipe4_sequence_colour_matching_end_to_end(tmp_path: Path):
    out = run_sequence_colour_matching(
        tmp_path / "r_color",
        approve_review=True,
        package_delivery=False,
    )
    assert out["recipe"] == "sequence-colour-matching"
    assert Path(out["project_path"]).is_file()
    assert out["pipeline"]["workingSpace"] == "aces_cg"
    assert out["idt_a"]["idt"] == "arri_logc4"
    assert out["idt_b"]["idt"] == "sony_slog3"
    assert out["verify_report"] is not None


def test_recipe5_music_bed_ducking_master_end_to_end(tmp_path: Path):
    out = run_music_bed_ducking_master(
        tmp_path / "r_ducking",
        approve_review=True,
        package_delivery=False,
    )
    assert out["recipe"] == "music-bed-ducking-master"
    assert Path(out["project_path"]).is_file()
    assert out["music_track_id"] >= 1
    assert out["verify_report"] is not None
    assert out["review"]["human_approved"] is True


def test_recipe_cli_list_command():
    runner = CliRunner()
    result = runner.invoke(cli, ["--json", "recipe", "list"])
    assert result.exit_code == 0
    recipes = result.output
    assert "podcast-cleanup" in recipes
    assert "sequence-colour-matching" in recipes
    assert "music-bed-ducking-master" in recipes
