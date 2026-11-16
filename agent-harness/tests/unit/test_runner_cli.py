# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Unit tests for the runner CLI subcommand group (TS9)."""

import json
from click.testing import CliRunner
import pytest

from cli_anything_eep.cli import cli
from cli_anything_eep.commands import runner as runner_mod


def test_runner_status_command(monkeypatch):
    """Test `runner status` returns runner status JSON."""
    def mock_request(method, path, data=None):
        assert method == "GET"
        assert path == "/runner/status"
        return {
            "running": True,
            "max_concurrent_workers": 2,
            "active_jobs": 1,
            "pending_jobs": 0,
            "total_jobs": 1,
        }

    monkeypatch.setattr(runner_mod, "_runner_request", mock_request)
    result = CliRunner().invoke(cli, ["--json", "runner", "status"])
    assert result.exit_code == 0
    parsed = json.loads(result.output)
    assert parsed["running"] is True
    assert parsed["active_jobs"] == 1


def test_runner_start_command(monkeypatch):
    """Test `runner start` sends POST /runner/start."""
    def mock_request(method, path, data=None):
        assert method == "POST"
        assert path == "/runner/start"
        assert data.get("max_concurrent_workers") == 4
        return {"running": True, "max_concurrent_workers": 4}

    monkeypatch.setattr(runner_mod, "_runner_request", mock_request)
    result = CliRunner().invoke(cli, ["--json", "runner", "start", "--workers", "4"])
    assert result.exit_code == 0
    parsed = json.loads(result.output)
    assert parsed["success"] is True
    assert parsed["data"]["running"] is True


def test_runner_stop_command(monkeypatch):
    """Test `runner stop` sends POST /runner/stop."""
    def mock_request(method, path, data=None):
        assert method == "POST"
        assert path == "/runner/stop"
        return {"running": False}

    monkeypatch.setattr(runner_mod, "_runner_request", mock_request)
    result = CliRunner().invoke(cli, ["--json", "runner", "stop"])
    assert result.exit_code == 0
    parsed = json.loads(result.output)
    assert parsed["success"] is True
    assert parsed["data"]["running"] is False


def test_runner_list_command(monkeypatch):
    """Test `runner list` returns list of jobs."""
    def mock_request(method, path, data=None):
        assert method == "GET"
        assert path == "/runner/jobs"
        return {
            "jobs": [
                {
                    "id": "11111111-2222-3333-4444-555555555555",
                    "tasksheet_id": "ts_1",
                    "state": "running",
                    "session_id": "agent-worker-11111111-2222-3333-4444-555555555555",
                }
            ]
        }

    monkeypatch.setattr(runner_mod, "_runner_request", mock_request)
    result = CliRunner().invoke(cli, ["--json", "runner", "list"])
    assert result.exit_code == 0
    parsed = json.loads(result.output)
    assert len(parsed["jobs"]) == 1
    assert parsed["jobs"][0]["tasksheet_id"] == "ts_1"
