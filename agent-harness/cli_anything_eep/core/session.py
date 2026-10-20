"""Append-only journal of commands executed through this CLI.

The Rust engine owns undo/redo. This journal exists so an agent can see which
commands the harness ran, in order, without pretending to be the engine's undo
stack.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

MAX_ENTRIES = 500


def _journal_file() -> Path:
    root = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local/state")
    return Path(root) / "easy-edits-pro" / "session.jsonl"


@dataclass(frozen=True)
class JournalEntry:
    """One command this CLI executed."""

    command: str
    params: dict[str, Any] = field(default_factory=dict)
    project: Optional[str] = None
    at: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "command": self.command,
            "params": self.params,
            "project": self.project,
            "at": self.at,
        }

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> "JournalEntry":
        return cls(
            command=str(value.get("command", "")),
            params=dict(value.get("params") or {}),
            project=value.get("project"),
            at=str(value.get("at", "")),
        )


class SessionLog:
    """JSON-lines journal shared by every invocation of the CLI."""

    def __init__(self, path: Optional[Path] = None):
        self.path = path or _journal_file()

    def record(
        self,
        command: str,
        params: Optional[dict[str, Any]] = None,
        *,
        project: Optional[str] = None,
    ) -> JournalEntry:
        entry = JournalEntry(
            command=command,
            params=dict(params or {}),
            project=project,
            at=datetime.now(timezone.utc).isoformat(timespec="seconds"),
        )
        entries = self.entries()
        entries.append(entry)
        self._write(entries[-MAX_ENTRIES:])
        return entry

    def entries(self, limit: Optional[int] = None) -> list[JournalEntry]:
        if not self.path.exists():
            return []
        entries: list[JournalEntry] = []
        try:
            with self.path.open("r", encoding="utf-8") as stream:
                for line in stream:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        entries.append(JournalEntry.from_dict(json.loads(line)))
                    except (json.JSONDecodeError, TypeError, ValueError):
                        # A truncated final line (crash during write) must not
                        # hide the rest of the journal.
                        continue
        except OSError:
            return []
        return entries[-limit:] if limit else entries

    def clear(self) -> None:
        try:
            self.path.unlink()
        except FileNotFoundError:
            pass

    def _write(self, entries: list[JournalEntry]) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        with temporary.open("w", encoding="utf-8") as stream:
            for entry in entries:
                stream.write(json.dumps(entry.to_dict(), separators=(",", ":")) + "\n")
        os.replace(temporary, self.path)
