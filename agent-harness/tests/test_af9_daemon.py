# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Integration tests for AF9 Authenticated Long-Lived Daemon."""

import json
import os
import shutil
import subprocess
import time
from pathlib import Path

import pytest
from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.core import errors
from cli_anything_eep.core.backend import DaemonClient
from cli_anything_eep.core.runtime import ManagedService, ServiceRegistry, find_binary


def _resolve_control_binary() -> str:
    path = find_binary("saturn-control", "SATURN_CONTROL")
    if not path:
        pytest.skip("saturn-control binary not built; run `cargo build --bin saturn-control`")
    return path


@pytest.fixture
def test_daemon(tmp_path: Path):
    """Fixture that spins up an authenticated saturn-control daemon on a UDS."""
    binary = _resolve_control_binary()
    sock_path = tmp_path / "test_daemon.sock"
    token = "test-token-secret-xyz"
    audit_file = tmp_path / "audit.jsonl"
    log_file = tmp_path / "daemon.log"

    cmd = [
        binary,
        "--socket",
        str(sock_path),
        "--auth-token",
        token,
        "--rate-limit",
        "100",
        "--audit-file",
        str(audit_file),
        "--project-root",
        str(tmp_path),
    ]

    log = log_file.open("w", encoding="utf-8")
    proc = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT)

    # Wait for UDS socket to appear
    deadline = time.time() + 10.0
    while time.time() < deadline:
        if sock_path.exists():
            break
        time.sleep(0.05)
    else:
        proc.kill()
        log.close()
        raise TimeoutError(f"Daemon socket did not appear at {sock_path}")

    yield {
        "socket": str(sock_path),
        "token": token,
        "audit_file": audit_file,
        "work_dir": tmp_path,
        "proc": proc,
    }

    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            proc.kill()
    log.close()


def test_daemon_auth_handshake_missing_and_invalid_tokens(test_daemon):
    sock_path = test_daemon["socket"]
    valid_token = test_daemon["token"]

    # 1. Missing token -> Unauthorized
    with DaemonClient(sock_path) as client:
        with pytest.raises(errors.EepError) as exc_info:
            client.call("health")
        assert exc_info.value.code == errors.ErrorCode.UNAUTHORIZED
        assert "MissingAuthToken" in exc_info.value.message

    # 2. Invalid token -> Unauthorized
    with pytest.raises(errors.EepError) as exc_info:
        with DaemonClient(sock_path, token="wrong-invalid-token"):
            pass
    assert exc_info.value.code == errors.ErrorCode.UNAUTHORIZED
    assert "InvalidAuthToken" in exc_info.value.message

    # 3. Valid token -> Authenticated
    with DaemonClient(sock_path, token=valid_token) as client:
        res = client.call("health")
        assert res.get("status") == "ok"


def test_daemon_headless_project_lifecycle(test_daemon):
    sock_path = test_daemon["socket"]
    token = test_daemon["token"]
    proj_path = str(test_daemon["work_dir"] / "test_proj.eep")

    with DaemonClient(sock_path, token=token) as client:
        # Initial status: not open
        st0 = client.project_status()
        assert st0["is_open"] is False

        # Open project
        op = client.project_open(proj_path)
        assert op["opened"] is True

        # Status: open, clean
        st1 = client.project_status()
        assert st1["is_open"] is True
        assert st1["is_dirty"] is False
        initial_tracks = st1["tracks_count"]

        # Mutate project: add track
        add_res = client.execute_command("timeline.add_track", {"kind": "video"})
        assert add_res is not None

        # Status: dirty, track count incremented
        st2 = client.project_status()
        assert st2["is_dirty"] is True
        assert st2["tracks_count"] == initial_tracks + 1

        # Save project
        sv = client.project_save(proj_path)
        assert sv["saved"] is True
        assert Path(proj_path).exists()

        # Close project
        cl = client.project_close()
        assert cl["closed"] is True

        # Status: closed
        st3 = client.project_status()
        assert st3["is_open"] is False


def test_daemon_af2_session_stop_and_resume(test_daemon):
    sock_path = test_daemon["socket"]
    token = test_daemon["token"]

    with DaemonClient(sock_path, token=token, session_id="stop-test-session") as client:
        # Pre-stop call works
        assert client.call("health")["status"] == "ok"

        # Stop session
        stop_res = client.session_stop()
        assert stop_res["stopped"] is True

        # Subsequent call fenced with SessionStopped
        with pytest.raises(errors.EepError) as exc_info:
            client.call("health")
        assert exc_info.value.code == errors.ErrorCode.SESSION_STOPPED
        assert "SessionStopped" in exc_info.value.message

        # Resume under human authority
        resumed = client.session_resume(caller="human-editor")
        assert resumed["resumed"] is True

        # Subsequent call succeeds again
        assert client.call("health")["status"] == "ok"


def test_daemon_audit_logging_and_privacy_invariant(test_daemon):
    sock_path = test_daemon["socket"]
    token = test_daemon["token"]
    audit_file = test_daemon["audit_file"]

    with DaemonClient(sock_path, token=token, session_id="audit-sess") as client:
        client.call("health")
        client.call("tools.list")

    assert audit_file.exists()
    content = audit_file.read_text(encoding="utf-8")
    assert content.strip()

    for line in content.splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        assert "timestamp" in record
        assert "session_id" in record
        assert "argument_hash" in record
        assert len(record["argument_hash"]) == 64
        # Privacy invariant: no plaintext secret tokens in audit log
        assert token not in line


def test_daemon_cli_start_stop_status(tmp_path: Path):
    _resolve_control_binary()
    registry_file = tmp_path / "services.json"
    registry = ServiceRegistry(registry_file)
    sock_path = str(tmp_path / "cli_daemon.sock")
    token = "cli-token-123456789"

    runner = CliRunner()
    env = {
        "XDG_STATE_HOME": str(tmp_path),
        "SATURN_CONTROL": _resolve_control_binary(),
    }

    # Test status when not running
    res_st0 = runner.invoke(cli, ["--json", "daemon", "status"], env=env)
    assert res_st0.exit_code == 0
    data_st0 = json.loads(res_st0.output)
    assert data_st0["state"] == "stopped"

    # Start daemon via CLI
    res_start = runner.invoke(
        cli,
        [
            "--json",
            "daemon",
            "start",
            "--socket",
            sock_path,
            "--auth-token",
            token,
            "--audit-file",
            str(tmp_path / "cli_audit.jsonl"),
        ],
        env=env,
    )
    assert res_start.exit_code == 0
    data_start = json.loads(res_start.output)
    assert data_start["success"] is True
    assert data_start["data"]["state"] == "running"

    # Status shows running and healthy
    res_st1 = runner.invoke(cli, ["--json", "daemon", "status"], env=env)
    assert res_st1.exit_code == 0
    data_st1 = json.loads(res_st1.output)
    assert data_st1["state"] == "running"
    assert "health" in data_st1

    # Stop daemon via CLI
    res_stop = runner.invoke(cli, ["--json", "daemon", "stop"], env=env)
    assert res_stop.exit_code == 0
    data_stop = json.loads(res_stop.output)
    assert data_stop["success"] is True
    assert data_stop["data"]["state"] == "stopped"
