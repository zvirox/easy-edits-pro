"""Behavioral regression tests for the previously stubbed CLI workflows."""

import json

from click.testing import CliRunner

from cli_anything_eep.cli import cli
from cli_anything_eep.commands import clip, project, render
from cli_anything_eep.core import backend, state, undo


def test_repl_dispatches_click_commands_instead_of_echoing_them(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path))
    result = CliRunner().invoke(cli, ["repl"], input="version\nexit\n")

    assert result.exit_code == 0
    assert "cli: 0.1.0" in result.output
    assert "Would execute" not in result.output
    assert "version" in (tmp_path / "easy-edits-pro" / "repl_history").read_text().splitlines()


def test_project_create_reports_backend_failure_as_structured_error(monkeypatch):
    monkeypatch.setattr(backend, "get_saturn_cli", lambda: (_ for _ in ()).throw(RuntimeError("not installed")))

    result = CliRunner().invoke(cli, ["--json", "project", "create", "--name", "Smoke"])

    assert result.exit_code != 0
    payload = json.loads(result.output)
    assert payload["error"] == "BACKEND_NOT_FOUND"


def test_clip_list_uses_the_active_project_snapshot(monkeypatch):
    class FakeBackend:
        def clip_list(self, track=None):
            return {"clips": [{"id": 9, "track_id": 7, "track_name": "V1", "clip_index": 0}]}

    monkeypatch.setattr(backend, "get_saturn_cli", lambda: FakeBackend())
    result = CliRunner().invoke(cli, ["--json", "clip", "list"])

    assert result.exit_code == 0
    payload = json.loads(result.output)
    assert payload["clips"] == [{"id": 9, "track_id": 7, "track_name": "V1", "clip_index": 0}]


def test_render_cancel_does_not_claim_success_when_job_is_missing(monkeypatch):
    def missing_job(*args, **kwargs):
        raise RuntimeError("render job 999 was not found")

    monkeypatch.setattr(backend, "call_desktop_bridge", missing_job)
    result = CliRunner().invoke(cli, ["--json", "render", "cancel", "--job-id", "999"])

    assert result.exit_code != 0
    assert json.loads(result.output)["error"] == "NOT_FOUND"


def test_undo_and_redo_dispatch_to_rust_engine(monkeypatch):
    calls = []

    class FakeBackend:
        def execute(self, command_id, params, **kwargs):
            calls.append(command_id)
            return {"undone": True} if command_id == "edit.undo" else {"redone": True}

    monkeypatch.setattr(backend, "get_saturn_cli", lambda: FakeBackend())
    undo.UndoManager.clear()
    undo.UndoManager.record("timeline.add_track", {}, "Add track")

    assert undo.UndoManager.undo()["undone"] == 1
    assert undo.UndoManager.redo()["redone"] == 1
    assert calls == ["edit.undo", "edit.redo"]


def test_grouped_undo_and_redo_apply_each_rust_checkpoint(monkeypatch):
    calls = []

    class FakeBackend:
        def execute(self, command_id, params, **kwargs):
            calls.append(command_id)
            return {"ok": True}

    monkeypatch.setattr(backend, "get_saturn_cli", lambda: FakeBackend())
    undo.UndoManager.clear()
    group_id = undo.UndoManager.begin_transaction("Batch edit")
    undo.UndoManager.record("timeline.add_track", {}, "Add V1")
    undo.UndoManager.record("timeline.add_track", {}, "Add V2")
    undo.UndoManager.commit_transaction()

    assert undo.UndoManager.undo()["undone"] == 2
    assert calls == ["edit.undo", "edit.undo"]
    assert undo.UndoManager.redo()["redone"] == 2
    assert calls == ["edit.undo", "edit.undo", "edit.redo", "edit.redo"]
    assert all(item.group_id == group_id for item in undo.UndoManager._history)


def test_unsupported_daemon_command_returns_error_instead_of_fake_running_status():
    result = CliRunner().invoke(cli, ["--json", "daemon", "status"])

    assert result.exit_code != 0
    payload = json.loads(result.output)
    assert payload["error"] == "COMMAND_NOT_FOUND"
    assert "running" not in payload


def test_project_state_loads_metadata_and_persists_current_project(tmp_path, monkeypatch):
    project_file = tmp_path / "project.eep"
    project_file.write_text(json.dumps({
        "project": {"name": "Review", "settings": {"width": 1280, "height": 720, "frame_rate": {"numerator": 24, "denominator": 1}}, "sequences": [{"name": "Main"}]}
    }))
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path / "state"))

    loaded = state.ProjectState.load(str(project_file))
    loaded.save()
    restored = state.ProjectState.load_saved()

    assert loaded.current.name == "Review"
    assert restored.current.path == str(project_file)
    assert restored.current.settings["width"] == 1280
