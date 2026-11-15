# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Unit tests for the 5 Packet 7.1 CLI subcommand groups (branch, ocio, telemetry, perception, vfx)."""

import json
from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.core import backend, errors


class MockSaturnCLI:
    """Mock backend capturing commands and parameters for CLI unit verification."""

    def __init__(self):
        self.calls = []

    def execute(self, command_id: str, params=None, record_history: bool = True):
        self.calls.append((command_id, params or {}))
        if command_id == "project.branch_create":
            return {"created": True, "branch": params.get("name"), "created_at_unix_ms": 1700000000000}
        elif command_id == "project.branch_list":
            return {"active": "main", "branches": [{"name": "alt-cut", "note": "Alt", "parent": "main", "created_at_unix_ms": 1700000000000}]}
        elif command_id == "project.branch_switch":
            return {"switched": True, "active_branch": params.get("name")}
        elif command_id == "project.branch_diff":
            return {"branch": params.get("branch"), "base_revision": 1, "target_revision": 3, "changes": []}
        elif command_id == "project.branch_merge":
            return {"merged": params.get("action", "accept"), "branch": params.get("branch"), "active_branch": "main"}
        elif command_id == "ocio.set_pipeline":
            return {"workingSpace": params.get("working_space"), "displayOdt": params.get("display_odt")}
        elif command_id == "ocio.set_media_idt":
            return {"mediaId": params.get("media_id"), "idt": params.get("idt")}
        elif command_id == "verify.run":
            return {
                "snapshot_id": "snap123",
                "sequence_id": 1,
                "preset": params.get("preset", "web-1080p"),
                "analyzed_range": {"start_tick": 0, "end_tick": 1000, "duration_ticks": 1000},
                "checks": [
                    {
                        "check_id": "audio.loudness",
                        "name": "Integrated Loudness",
                        "verdict": "pass",
                        "status": "measured",
                        "message": "Passed",
                    },
                    {
                        "check_id": "video.clipped_highlights",
                        "name": "Highlight Clipping",
                        "verdict": "pass",
                        "status": "measured",
                        "message": "Passed",
                    },
                ],
                "summary": {"total_checks": 2, "passed": 2, "warned": 0, "failed": 0, "unavailable": 0},
            }
        elif command_id == "project.inspect":
            return {
                "project": {
                    "sequence": {
                        "caption_tracks": [{"cues": [{"id": 1, "start_ticks": 0, "end_ticks": 1000, "text": "Test cue"}]}],
                        "tracks": [
                            {
                                "id": 1,
                                "name": "V1",
                                "kind": "video",
                                "clips": [{"id": 10, "place": {"timeline_in": 0, "duration": 5000}}],
                            }
                        ],
                    }
                }
            }
        return {"ok": True}


def test_command_registration_in_cli():
    commands = cli.commands
    for group in ("branch", "ocio", "telemetry", "perception", "vfx", "recipe"):
        assert group in commands, f"Command group {group} must be registered in cli.py"


def test_branch_create_and_list(monkeypatch):
    mock = MockSaturnCLI()
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: mock)

    res = CliRunner().invoke(cli, ["--json", "branch", "create", "--name", "task-cut", "--note", "Cut testing"])
    assert res.exit_code == 0
    payload = json.loads(res.output)
    assert payload["success"] is True
    assert payload["data"]["branch"] == "task-cut"

    res_list = CliRunner().invoke(cli, ["--json", "branch", "list"])
    assert res_list.exit_code == 0
    list_payload = json.loads(res_list.output)
    assert list_payload["active"] == "main"
    assert len(list_payload["branches"]) == 1


def test_branch_switch_diff_merge(monkeypatch):
    mock = MockSaturnCLI()
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: mock)

    res_sw = CliRunner().invoke(cli, ["--json", "branch", "switch", "--name", "task-cut"])
    assert res_sw.exit_code == 0
    assert json.loads(res_sw.output)["data"]["active_branch"] == "task-cut"

    res_diff = CliRunner().invoke(cli, ["--json", "branch", "diff", "--branch", "task-cut"])
    assert res_diff.exit_code == 0
    assert json.loads(res_diff.output)["branch"] == "task-cut"

    res_merge = CliRunner().invoke(cli, ["--json", "branch", "merge", "--branch", "task-cut", "--action", "accept", "--force"])
    assert res_merge.exit_code == 0
    assert json.loads(res_merge.output)["data"]["merged"] == "accept"


def test_ocio_set_pipeline_and_media_idt(monkeypatch):
    mock = MockSaturnCLI()
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: mock)

    res_pipe = CliRunner().invoke(
        cli,
        ["--json", "ocio", "set-pipeline", "-w", "aces_cg", "-d", "rec709", "-i", "arri_logc4"],
    )
    assert res_pipe.exit_code == 0
    assert json.loads(res_pipe.output)["data"]["workingSpace"] == "aces_cg"

    res_idt = CliRunner().invoke(cli, ["--json", "ocio", "set-media-idt", "-m", "42", "-i", "sony_slog3"])
    assert res_idt.exit_code == 0
    assert json.loads(res_idt.output)["data"]["idt"] == "sony_slog3"


def test_telemetry_audio_video_inspect(monkeypatch):
    mock = MockSaturnCLI()
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: mock)

    res_aud = CliRunner().invoke(cli, ["--json", "telemetry", "audio", "--preset", "broadcast-r128"])
    assert res_aud.exit_code == 0
    aud_payload = json.loads(res_aud.output)
    assert "audio_checks" in aud_payload
    assert aud_payload["summary"]["total"] >= 1

    res_vid = CliRunner().invoke(cli, ["--json", "telemetry", "video", "--preset", "web-1080p"])
    assert res_vid.exit_code == 0
    vid_payload = json.loads(res_vid.output)
    assert "video_checks" in vid_payload

    res_insp = CliRunner().invoke(cli, ["--json", "telemetry", "inspect", "--preset", "web-1080p", "--check", "audio.loudness"])
    assert res_insp.exit_code == 0
    insp_payload = json.loads(res_insp.output)
    assert insp_payload["snapshot_id"] == "snap123"


def test_perception_transcript_scenes(monkeypatch):
    mock = MockSaturnCLI()
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: mock)

    res_tr = CliRunner().invoke(cli, ["--json", "perception", "transcript"])
    assert res_tr.exit_code == 0
    tr_payload = json.loads(res_tr.output)
    assert tr_payload["total"] == 1
    assert tr_payload["cues"][0]["text"] == "Test cue"

    res_sc = CliRunner().invoke(cli, ["--json", "perception", "scenes"])
    assert res_sc.exit_code == 0
    sc_payload = json.loads(res_sc.output)
    assert sc_payload["total_cuts"] == 1


def test_perception_unbacked_actions_return_structured_errors():
    for subcmd in ("realtime", "voice-activity", "face-mesh"):
        res = CliRunner().invoke(cli, ["--json", "perception", subcmd])
        assert res.exit_code != 0
        payload = json.loads(res.output)
        assert payload["error"] == "COMMAND_NOT_FOUND"
        assert payload["recoverable"] is False
        assert "unavailable" in payload["message"].lower() or "unbacked" in payload["message"].lower()


def test_vfx_commands_return_deferred_structured_errors_per_a4():
    for subcmd in ("composite", "matte", "depth", "track", "effects"):
        res = CliRunner().invoke(cli, ["--json", "vfx", subcmd])
        assert res.exit_code != 0
        payload = json.loads(res.output)
        assert payload["error"] == "COMMAND_NOT_FOUND"
        assert payload["recoverable"] is False
        assert "phase 6" in payload["message"].lower() or "deferred" in payload["message"].lower()
