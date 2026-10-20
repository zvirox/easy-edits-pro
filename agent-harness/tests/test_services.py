"""Service lifecycle tests against the real saturn binaries.

These are integration tests: they spawn the Rust control server and MCP server
when the workspace binaries are built, and skip otherwise.
"""

import json
import socket

import pytest
from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.core.runtime import find_binary


def _free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def _invoke(args):
    return CliRunner().invoke(cli, ["--json", *args])


def _payload(result):
    payload = json.loads(result.output)
    # Success payloads wrap their result under "data".
    return payload.get("data", payload)


def test_daemon_status_is_stopped_without_a_record():
    result = _invoke(["daemon", "status"])

    assert result.exit_code == 0
    assert json.loads(result.output)["state"] == "stopped"


@pytest.mark.skipif(find_binary("saturn-control") is None, reason="saturn-control not built")
def test_daemon_start_status_and_stop_round_trip():
    port = _free_port()
    try:
        started = _invoke(["daemon", "start", "--port", str(port)])
        assert started.exit_code == 0, started.output

        start_payload = _payload(started)
        assert start_payload["state"] == "running"
        assert start_payload["pid"] > 0
        assert start_payload["health"]["status"] == "ok"
        assert start_payload["health"]["commands"] > 0

        status = _invoke(["daemon", "status"])
        assert status.exit_code == 0
        status_payload = _payload(status)
        assert status_payload["state"] == "running"
        assert status_payload["port"] == port
        assert status_payload["health"]["status"] == "ok"

        duplicate = _invoke(["daemon", "start", "--port", str(_free_port())])
        assert duplicate.exit_code == 1
        assert json.loads(duplicate.output)["error"] == "ALREADY_EXISTS"
    finally:
        stopped = _invoke(["daemon", "stop"])

    assert stopped.exit_code == 0
    assert _payload(stopped)["state"] == "stopped"
    assert json.loads(_invoke(["daemon", "status"]).output)["state"] == "stopped"


@pytest.mark.skipif(find_binary("saturn-mcp") is None, reason="saturn-mcp not built")
def test_mcp_tools_are_discovered_over_stdio():
    result = _invoke(["mcp", "tools"])

    assert result.exit_code == 0, result.output
    tools = json.loads(result.output)["tools"]
    names = {tool["name"] for tool in tools}
    assert "command_list" in names
    assert "command_run" in names
    assert all("description" in tool for tool in tools)


@pytest.mark.skipif(find_binary("saturn-mcp") is None, reason="saturn-mcp not built")
def test_mcp_reports_empty_resources_and_prompts_honestly():
    resources = _invoke(["mcp", "resources"])
    prompts = _invoke(["mcp", "prompts"])

    assert resources.exit_code == 0, resources.output
    assert prompts.exit_code == 0, prompts.output
    assert json.loads(resources.output)["resources"] == []
    assert json.loads(prompts.output)["prompts"] == []
