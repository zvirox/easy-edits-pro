"""Tests for the CLI command journal."""

import json

from cli_anything_eep.core.session import MAX_ENTRIES, SessionLog


def test_record_and_read_roundtrip(tmp_path):
    log = SessionLog(tmp_path / "session.jsonl")

    log.record("timeline.add_track", {"kind": "video"}, project="/tmp/demo.saturn")
    entries = log.entries()

    assert len(entries) == 1
    assert entries[0].command == "timeline.add_track"
    assert entries[0].params == {"kind": "video"}
    assert entries[0].project == "/tmp/demo.saturn"
    assert entries[0].at.endswith("+00:00")


def test_entries_are_append_only_and_ordered(tmp_path):
    log = SessionLog(tmp_path / "session.jsonl")
    log.record("first")
    log.record("second")
    log.record("third")

    assert [entry.command for entry in log.entries()] == ["first", "second", "third"]


def test_limit_returns_the_most_recent_entries(tmp_path):
    log = SessionLog(tmp_path / "session.jsonl")
    for index in range(10):
        log.record(f"command.{index}")

    assert [entry.command for entry in log.entries(limit=3)] == [
        "command.7",
        "command.8",
        "command.9",
    ]


def test_journal_is_capped(tmp_path):
    log = SessionLog(tmp_path / "session.jsonl")
    for index in range(MAX_ENTRIES + 5):
        log.record(f"command.{index}")

    entries = log.entries()
    assert len(entries) == MAX_ENTRIES
    assert entries[-1].command == f"command.{MAX_ENTRIES + 4}"


def test_corrupt_line_does_not_hide_later_entries(tmp_path):
    path = tmp_path / "session.jsonl"
    log = SessionLog(path)
    log.record("good.one")
    with path.open("a", encoding="utf-8") as stream:
        stream.write("{not json\n")
    log.record("good.two")

    assert [entry.command for entry in log.entries()] == ["good.one", "good.two"]


def test_clear_removes_the_journal(tmp_path):
    path = tmp_path / "session.jsonl"
    log = SessionLog(path)
    log.record("command.one")

    log.clear()

    assert not path.exists()
    assert log.entries() == []


def test_missing_journal_reads_as_empty(tmp_path):
    assert SessionLog(tmp_path / "absent.jsonl").entries() == []


def test_params_are_copied_not_referenced(tmp_path):
    log = SessionLog(tmp_path / "session.jsonl")
    params = {"kind": "video"}
    log.record("timeline.add_track", params)
    params["kind"] = "audio"

    assert log.entries()[0].params == {"kind": "video"}


def test_written_file_is_json_lines(tmp_path):
    path = tmp_path / "session.jsonl"
    SessionLog(path).record("timeline.add_track", {"kind": "video"})

    line = path.read_text(encoding="utf-8").strip()
    assert json.loads(line)["command"] == "timeline.add_track"
