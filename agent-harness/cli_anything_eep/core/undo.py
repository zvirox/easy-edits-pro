"""
Undo/Redo manager for Easy Edits Pro CLI.
Integrates with saturn-commands undo system.
"""

from typing import Dict, Any, List, Optional
from dataclasses import dataclass
from datetime import datetime
import uuid
from .errors import EepError, ErrorCode


@dataclass
class UndoEntry:
    """Single undo/redo entry."""
    id: str
    timestamp: datetime
    command: str
    params: Dict[str, Any]
    description: str
    group_id: Optional[str] = None
    inverse: Optional[Dict[str, Any]] = None  # For redo


class UndoManager:
    """Manages undo/redo stack with transaction support."""
    
    _history: List[UndoEntry] = []
    _redo_stack: List[UndoEntry] = []
    # Keep the CLI's display stack aligned with EditorEngine's 100 snapshots.
    _max_history: int = 100
    _current_group: Optional[str] = None
    
    @classmethod
    def record(
        cls,
        command: str,
        params: Dict[str, Any],
        description: str,
        group_id: Optional[str] = None,
        inverse: Optional[Dict[str, Any]] = None
    ) -> UndoEntry:
        """Record an operation for undo/redo."""
        entry = UndoEntry(
            id=f"undo_{uuid.uuid4().hex[:12]}",
            timestamp=datetime.now(),
            command=command,
            params=params,
            description=description,
            group_id=group_id or cls._current_group,
            inverse=inverse
        )
        
        cls._history.append(entry)
        if len(cls._history) > cls._max_history:
            cls._history.pop(0)
        
        # Any new engine mutation invalidates the redo branch, including edits
        # recorded inside a transaction group.
        cls._redo_stack.clear()
        
        return entry
    
    @classmethod
    def undo(cls) -> Dict[str, Any]:
        """Undo last operation or group."""
        if not cls._history:
            raise EepError(ErrorCode.NOTHING_TO_UNDO, "Nothing to undo.", recoverable=True)
        
        # A transaction is undone by replaying each corresponding Rust engine undo,
        # newest first. Ungrouped commands still undo one checkpoint at a time.
        group_id = cls._history[-1].group_id
        count = 1
        if group_id is not None:
            while count < len(cls._history) and cls._history[-count - 1].group_id == group_id:
                count += 1
        entries = list(reversed(cls._history[-count:]))
        
        # Execute inverse operations
        results = []
        try:
            for entry in entries:
                results.append(cls._execute_inverse(entry))
        except Exception as error:
            # Restore any snapshots already undone before leaving the metadata intact.
            for completed in reversed(entries[:len(results)]):
                cls._execute_forward(completed)
            if isinstance(error, EepError):
                raise
            raise EepError.from_backend_error(error, "Could not undo operation") from error
        del cls._history[-count:]
        cls._redo_stack.extend(entries)
        
        return {
            "undone": len(entries),
            "description": entries[-1].description if entries else "Unknown",
            "results": results
        }
    
    @classmethod
    def redo(cls) -> Dict[str, Any]:
        """Redo last undone operation or group."""
        if not cls._redo_stack:
            raise EepError(ErrorCode.NOTHING_TO_REDO, "Nothing to redo.", recoverable=True)
        
        group_id = cls._redo_stack[-1].group_id
        count = 1
        if group_id is not None:
            while count < len(cls._redo_stack) and cls._redo_stack[-count - 1].group_id == group_id:
                count += 1
        entries = list(reversed(cls._redo_stack[-count:]))
        
        results = []
        try:
            for entry in entries:
                results.append(cls._execute_forward(entry))
        except Exception as error:
            # Rewind successful forward steps so the project and CLI stacks remain aligned.
            for completed in reversed(entries[:len(results)]):
                cls._execute_inverse(completed)
            if isinstance(error, EepError):
                raise
            raise EepError.from_backend_error(error, "Could not redo operation") from error
        del cls._redo_stack[-count:]
        cls._history.extend(entries)
        
        return {
            "redone": len(entries),
            "description": entries[-1].description if entries else "Unknown",
            "results": results
        }
    
    @classmethod
    def history(cls) -> Dict[str, Any]:
        """Get undo history."""
        return {
            "history": [
                {
                    "id": e.id,
                    "timestamp": e.timestamp.isoformat(),
                    "command": e.command,
                    "description": e.description,
                    "group_id": e.group_id
                }
                for e in cls._history
            ],
            "can_undo": len(cls._history) > 0,
            "can_redo": len(cls._redo_stack) > 0
        }
    
    @classmethod
    def begin_transaction(cls, description: str) -> str:
        """Begin a transaction group."""
        group_id = f"txn_{uuid.uuid4().hex[:8]}"
        cls._current_group = group_id
        return group_id
    
    @classmethod
    def commit_transaction(cls) -> Dict[str, Any]:
        """Commit current transaction."""
        group_id = cls._current_group
        cls._current_group = None
        return {"transaction_id": group_id, "committed": True}
    
    @classmethod
    def rollback_transaction(cls) -> Dict[str, Any]:
        """Rollback current transaction."""
        group_id = cls._current_group
        cls._current_group = None
        if group_id is None:
            return {"transaction_id": None, "rolled_back": False, "undone": []}
        
        # Undo the transaction atomically from the CLI's perspective. If the
        # engine rejects a step, replay already-undone snapshots and keep history.
        count = 0
        while count < len(cls._history) and cls._history[-count - 1].group_id == group_id:
            count += 1
        if count == 0:
            return {"transaction_id": group_id, "rolled_back": True, "undone": []}
        entries = list(reversed(cls._history[-count:]))
        undone = []
        try:
            for entry in entries:
                undone.append({"command": entry.command, "result": cls._execute_inverse(entry)})
        except Exception as error:
            for entry in reversed(entries[:len(undone)]):
                cls._execute_forward(entry)
            if isinstance(error, EepError):
                raise
            raise EepError.from_backend_error(error, "Could not roll back transaction") from error
        del cls._history[-count:]
        
        return {"transaction_id": group_id, "rolled_back": True, "undone": undone}
    
    @classmethod
    def _execute_inverse(cls, entry: UndoEntry) -> Dict[str, Any]:
        """Ask the Rust engine to restore its previous project snapshot."""
        from .backend import get_saturn_cli
        result = get_saturn_cli().execute("edit.undo", {}, record_undo=False)
        return {"command": entry.command, "backend": result, "description": f"Undid: {entry.description}"}
    
    @classmethod
    def _execute_forward(cls, entry: UndoEntry) -> Dict[str, Any]:
        """Ask the Rust engine to restore the snapshot popped by undo."""
        from .backend import get_saturn_cli
        result = get_saturn_cli().execute("edit.redo", {}, record_undo=False)
        return {"command": entry.command, "backend": result, "description": f"Redid: {entry.description}"}
    
    @classmethod
    def can_undo(cls) -> bool:
        return len(cls._history) > 0
    
    @classmethod
    def can_redo(cls) -> bool:
        return len(cls._redo_stack) > 0
    
    @classmethod
    def clear(cls):
        """Clear all history (use with caution)."""
        cls._history.clear()
        cls._redo_stack.clear()
        cls._current_group = None
