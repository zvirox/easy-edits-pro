# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Flagship Task Sheet recipes for Easy Edits Pro (`podcast-to-clips`, `talking-head-tutorial`, `webinar-recap`)."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Callable

from .podcast_to_clips import run_podcast_to_clips
from .talking_head_tutorial import run_talking_head_tutorial
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
