"""End-to-end tests for engine-backed editing verbs.

Every test here drives the real Rust engine through the CLI; the suite skips
when `saturn-cli` is not built instead of falling back to a fake backend.
"""

import json
import shutil
from pathlib import Path

import pytest
from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.core.runtime import find_binary

pytestmark = pytest.mark.skipif(find_binary("saturn-cli") is None, reason="saturn-cli not built")

TICK = 254_016_000_000  # one second


def invoke(*args):
    return CliRunner().invoke(cli, ["--json", *args])


def payload(result):
    assert result.exit_code == 0, result.output
    body = json.loads(result.output)
    return body.get("data", body) if isinstance(body, dict) else body


@pytest.fixture
def project(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    media = tmp_path / "clip.mp4"
    media.write_bytes(b"")
    created = invoke("project", "create", "--name", "Verbs", "--path", str(tmp_path / "verbs.eep"))
    assert created.exit_code == 0, created.output

    tracks = payload(invoke("track", "list"))["tracks"]
    track = next(int(item["id"]) for item in tracks if item.get("kind") == "video")

    clip = invoke("clip", "add", "--source", str(media), "--track", str(track), "--at", "0")
    assert clip.exit_code == 0, clip.output

    listed = payload(invoke("media", "list"))
    media_id = next(int(item["id"]) for item in listed if item["path"] == str(media))
    return {"root": tmp_path, "track": track, "media_id": media_id}


def test_timeline_edits_use_real_tick_math(project):
    track = project["track"]

    assert payload(invoke("timeline", "mark-in", "--at", "0"))["mark_in"] == 0
    assert payload(invoke("timeline", "mark-out", "--at", str(TICK)))["mark_out"] == TICK
    assert payload(invoke("timeline", "clear-marks"))["cleared"] is True

    split = payload(
        invoke("timeline", "split", "--track", str(track), "--clip-index", "0", "--at", str(TICK // 2))
    )
    assert split["split"] is True and split["right_index"] == 1

    transition = payload(
        invoke(
            "timeline", "add-transition",
            "--track", str(track), "--cut-at", str(TICK // 2), "--duration", str(TICK // 10),
        )
    )
    assert transition["kind"] == "crossfade"

    removed = payload(
        invoke("timeline", "remove-transition", "--track", str(track), "--transition-id", str(transition["transition_id"]))
    )
    assert removed["removed"] is True


def test_clip_effects_keyframes_and_motion(project):
    track = project["track"]
    reference = ["--track", str(track), "--clip-index", "0"]

    effect = payload(invoke("clip", "add-effect", *reference, "--kind", "gaussian_blur", "--name", "Blur"))
    effect_id = str(effect["effect_id"])
    assert payload(invoke("clip", "toggle-effect", *reference, "--effect-id", effect_id, "--disabled"))["enabled"] is False
    assert payload(invoke("clip", "remove-effect", *reference, "--effect-id", effect_id))["removed"] is True

    assert payload(
        invoke("clip", "keyframe-set", *reference, "--property", "opacity", "--time", "0", "--value", "0.5")
    )["updated"] is True
    assert payload(
        invoke("clip", "keyframe-remove", *reference, "--property", "opacity", "--time", "0")
    )["removed"] is True

    assert payload(invoke("clip", "motion-blur", *reference, "--enabled", "--samples", "4"))["updated"] is True


def test_media_bins_and_subtitles(project):
    bin_id = payload(invoke("media", "bin-create", "--name", "Footage"))["bin"]["id"]
    assert payload(invoke("media", "bin-rename", "--bin", str(bin_id), "--name", "Broll"))["renamed"] is True
    assert payload(
        invoke("media", "bin-move", "--media", str(project["media_id"]), "--bin", str(bin_id))
    )["moved"] is True

    added = payload(invoke("subtitle", "add", "--start", "0", "--end", str(TICK), "--text", "Hello"))
    assert payload(invoke("subtitle", "list"))["subtitles"][0]["text"] == "Hello"
    assert payload(invoke("subtitle", "remove", "--id", str(added["id"])))["removed"] is True
    assert payload(invoke("subtitle", "clear"))["cleared"] is True


def test_nodes_collab_and_titles(project):
    node = payload(invoke("nodegraph", "add-node", "--kind", "merge", "--title", "Merge"))
    assert node["added"] is True

    lock = payload(
        invoke("collab", "acquire-lock", "--user-id", "u1", "--user-name", "Alice", "--target-kind", "clip", "--target-id", "1")
    )
    assert lock["acquired"] is True
    assert payload(
        invoke("collab", "record-change", "--user-id", "u1", "--user-name", "Alice", "--action", "edit", "--summary", "moved clip")
    )["recorded"] is True

    title = payload(invoke("timeline", "add-title", "--text", "Intro", "--duration", str(TICK), "--track", str(project["track"]), "--at", "0"))
    updated = payload(invoke("title", "update", "--media-id", str(title["media_id"]), "--text", "Opening", "--font-size", "48"))
    assert updated["title"]["text"] == "Opening"


def test_template_group_round_trip(tmp_path):
    source = tmp_path / "lower.sgt"
    created = invoke(
        "template", "create", "--name", "Lower", "--type", "lower-third",
        "--output", str(source), "--author", "Tester", "--tags", "a,b",
    )
    assert created.exit_code == 0, created.output
    assert source.is_file()

    listed = invoke("template", "list", "--dir", str(tmp_path))
    templates = json.loads(listed.output)["templates"]
    assert [item["name"] for item in templates] == ["Lower"]

    inspected = json.loads(invoke("template", "inspect", str(source)).output)
    assert inspected["metadata"]["category"] == "LowerThird"

    exported = invoke("template", "export", str(source), "--output", str(tmp_path / "copy.sgt"))
    assert exported.exit_code == 0, exported.output
    assert (tmp_path / "copy.sgt").is_file()

    applied = invoke("template", "apply", str(source))
    assert applied.exit_code == 1
    assert json.loads(applied.output)["error"] == "COMMAND_NOT_FOUND"
