# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Flagship Task Sheet & Companion §2.6 Recipes for Easy Edits Pro."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Callable

from .captioned_vertical_excerpt import run_captioned_vertical_excerpt
from .music_bed_ducking_master import run_music_bed_ducking_master
from .podcast_cleanup import run_podcast_cleanup
from .podcast_to_clips import run_podcast_to_clips
from .sequence_colour_matching import run_sequence_colour_matching
from .talking_head_tutorial import run_talking_head_tutorial
from .transcript_interview_rough_cut import run_transcript_interview_rough_cut
from .webinar_recap import run_webinar_recap

RECIPES: dict[str, dict[str, Any]] = {
    "podcast-to-clips": {
        "id": "podcast-to-clips",
        "title": "Podcast / Interview to Social Clips",
        "runner": run_podcast_to_clips,
        "template": "docs/templates/tasksheets/podcast-social-clips.json",
        "preset": "vertical-9x16",
    },
    "talking-head-tutorial": {
        "id": "talking-head-tutorial",
        "title": "Talking-Head & Desktop Tutorial with CET",
        "runner": run_talking_head_tutorial,
        "template": "docs/templates/tasksheets/talking-head-tutorial.json",
        "preset": "web-1080p",
    },
    "webinar-recap": {
        "id": "webinar-recap",
        "title": "Webinar & Training Recap",
        "runner": run_webinar_recap,
        "template": "docs/templates/tasksheets/webinar-recap.json",
        "preset": "web-1080p",
    },
    # Five Companion §2.6 Recipes (AF7 / Packet 7.1)
    "podcast-cleanup": {
        "id": "podcast-cleanup",
        "title": "Podcast Dialogue Cleanup & Silence Reduction",
        "runner": run_podcast_cleanup,
        "template": "docs/templates/tasksheets/podcast-social-clips.json",
        "preset": "web-1080p",
    },
    "transcript-interview-rough-cut": {
        "id": "transcript-interview-rough-cut",
        "title": "Transcript Interview Rough Cut",
        "runner": run_transcript_interview_rough_cut,
        "template": "docs/templates/tasksheets/podcast-social-clips.json",
        "preset": "web-1080p",
    },
    "captioned-vertical-excerpt": {
        "id": "captioned-vertical-excerpt",
        "title": "Captioned Vertical Excerpt (9:16)",
        "runner": run_captioned_vertical_excerpt,
        "template": "docs/templates/tasksheets/podcast-social-clips.json",
        "preset": "vertical-9x16",
    },
    "sequence-colour-matching": {
        "id": "sequence-colour-matching",
        "title": "Sequence Colour Matching & ACEScg Setup",
        "runner": run_sequence_colour_matching,
        "template": "docs/templates/tasksheets/webinar-recap.json",
        "preset": "web-1080p",
    },
    "music-bed-ducking-master": {
        "id": "music-bed-ducking-master",
        "title": "Music Bed Ducking & Master Mix",
        "runner": run_music_bed_ducking_master,
        "template": "docs/templates/tasksheets/webinar-recap.json",
        "preset": "broadcast-r128",
    },
}


def list_recipes() -> list[dict[str, str]]:
    return [
        {
            "id": spec["id"],
            "title": spec["title"],
            "template": spec["template"],
            "preset": spec["preset"],
        }
        for spec in RECIPES.values()
    ]


def run_recipe(recipe_id: str, workdir: str | Path, **kwargs: Any) -> dict[str, Any]:
    spec = RECIPES.get(recipe_id)
    if spec is None:
        raise KeyError(f"Unknown recipe `{recipe_id}`; expected one of {list(RECIPES)}")
    runner: Callable[..., dict[str, Any]] = spec["runner"]
    return runner(workdir, **kwargs)
