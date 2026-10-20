"""Unit tests for undo/redo manager."""

import pytest
from cli_anything_eep.core.undo import UndoManager, UndoEntry
from cli_anything_eep.core import backend
from datetime import datetime


def setup_function():
    """Reset undo manager before each test."""
    UndoManager.clear()


@pytest.fixture(autouse=True)
def fake_editor_backend(monkeypatch):
    class FakeBackend:
        def execute(self, command_id, params, record_undo=True):
            return {"undone": True} if command_id == "edit.undo" else {"redone": True}

    monkeypatch.setattr(backend, "get_saturn_cli", lambda: FakeBackend())


def test_record_single():
    """Test recording a single operation."""
    entry = UndoManager.record(
        command="clip_add",
        params={"source": "test.mp4", "track": "V1"},
        description="Added clip test.mp4"
    )
    assert isinstance(entry, UndoEntry)
    assert entry.command == "clip_add"
    assert len(UndoManager._history) == 1


def test_undo_single():
    """Test undo of single operation."""
    UndoManager.record("clip_add", {"id": "1"}, "Add clip")
    assert UndoManager.can_undo()
    
    result = UndoManager.undo()
    assert result["undone"] == 1
    assert "Add clip" in result["description"]
    assert not UndoManager.can_undo()
    assert UndoManager.can_redo()


def test_redo_single():
    """Test redo of single operation."""
    UndoManager.record("clip_add", {"id": "1"}, "Add clip")
    UndoManager.undo()
    assert UndoManager.can_redo()
    
    result = UndoManager.redo()
    assert result["redone"] == 1
    assert UndoManager.can_undo()
    assert not UndoManager.can_redo()


def test_transaction_group():
    """Test transaction grouping."""
    group_id = UndoManager.begin_transaction("Batch edit")
    UndoManager.record("clip_add", {"id": "1"}, "Add clip 1", group_id=group_id)
    UndoManager.record("clip_add", {"id": "2"}, "Add clip 2", group_id=group_id)
    UndoManager.commit_transaction()
    result = UndoManager.undo()
    assert result["undone"] == 2


def test_rollback_transaction():
    """Test transaction rollback."""
    group_id = UndoManager.begin_transaction("Failed edit")
    UndoManager.record("clip_add", {"id": "1"}, "Add clip 1", group_id=group_id)
    UndoManager.record("clip_move", {"id": "1"}, "Move clip", group_id=group_id)
    UndoManager.rollback_transaction()
    
    # History should be empty (rolled back)
    assert len(UndoManager._history) == 0


def test_history():
    """Test history output."""
    UndoManager.record("clip_add", {"id": "1"}, "Add clip 1")
    UndoManager.record("clip_trim", {"id": "1"}, "Trim clip")
    
    history = UndoManager.history()
    assert len(history["history"]) == 2
    assert history["can_undo"] is True
    assert history["can_redo"] is False
    assert history["history"][0]["command"] == "clip_add"
    assert history["history"][1]["command"] == "clip_trim"


def test_max_history_limit():
    """Test history size limit."""
    UndoManager._max_history = 3
    for i in range(5):
        UndoManager.record(f"cmd_{i}", {}, f"Command {i}")
    
    assert len(UndoManager._history) == 3
    assert UndoManager._history[0].command == "cmd_2"  # Oldest kept
    assert UndoManager._history[-1].command == "cmd_4"  # Newest


def test_clear():
    """Test clearing history."""
    UndoManager.record("clip_add", {}, "Add clip")
    UndoManager.undo()  # Adds to redo stack
    UndoManager.clear()
    
    assert len(UndoManager._history) == 0
    assert len(UndoManager._redo_stack) == 0
    assert not UndoManager.can_undo()
    assert not UndoManager.can_redo()
