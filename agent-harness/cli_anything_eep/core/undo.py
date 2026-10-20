"""
Undo/Redo manager for Easy Edits Pro CLI.
Integrates with saturn-commands undo system.
"""

from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
import json


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
    _max_history: int = 1000
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
            id=f"undo_{len(cls._history)}",
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
        
        # Clear redo stack on new operation
        if not group_id:  # Don't clear during transaction
            cls._redo_stack.clear()
        
        return entry
    
    @classmethod
    def undo(cls) -> Dict[str, Any]:
        """Undo last operation or group."""
        if not cls._history:
            raise RuntimeError("Nothing to undo")
        
        # Pop entries until group boundary or single entry
        entries = []
        while cls._history:
            entry = cls._history.pop()
            entries.append(entry)
            if entry.group_id is None or entry.group_id != cls._current_group:
                break
        
        # Execute inverse operations
        results = []
        for entry in reversed(entries):
            # Would call backend to execute inverse
            result = cls._execute_inverse(entry)
            results.append(result)
            cls._redo_stack.append(entry)
        
        return {
            "undone": len(entries),
            "description": entries[-1].description if entries else "Unknown",
            "results": results
        }
    
    @classmethod
    def redo(cls) -> Dict[str, Any]:
        """Redo last undone operation or group."""
        if not cls._redo_stack:
            raise RuntimeError("Nothing to redo")
        
        entries = []
        while cls._redo_stack:
            entry = cls._redo_stack.pop()
            entries.append(entry)
            if entry.group_id is None or entry.group_id != cls._current_group:
                break
        
        results = []
        for entry in entries:
            # Would call backend to re-execute
            result = cls._execute_forward(entry)
            results.append(result)
            cls._history.append(entry)
        
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
        import uuid
        group_id = f"txn_{uuid.uuid4().hex[:8]}"
        cls._current_group = group_id
        cls.record("transaction_begin", {}, description, group_id=group_id)
        return group_id
    
    @classmethod
    def commit_transaction(cls) -> Dict[str, Any]:
        """Commit current transaction."""
        group_id = cls._current_group
        cls._current_group = None
        cls.record("transaction_commit", {}, "Transaction committed", group_id=group_id)
        return {"transaction_id": group_id, "committed": True}
    
    @classmethod
    def rollback_transaction(cls) -> Dict[str, Any]:
        """Rollback current transaction."""
        group_id = cls._current_group
        cls._current_group = None
        
        # Undo all entries in this transaction
        undone = []
        while cls._history and cls._history[-1].group_id == group_id:
            entry = cls._history.pop()
            result = cls._execute_inverse(entry)
            undone.append({"command": entry.command, "result": result})
        
        return {"transaction_id": group_id, "rolled_back": True, "undone": undone}
    
    @classmethod
    def _execute_inverse(cls, entry: UndoEntry) -> Dict[str, Any]:
        """Execute inverse of an operation."""
        # This would call the actual backend inverse
        # For now, return placeholder
        return {
            "command": entry.command,
            "inverse_executed": True,
            "description": f"Undid: {entry.description}"
        }
    
    @classmethod
    def _execute_forward(cls, entry: UndoEntry) -> Dict[str, Any]:
        """Re-execute an operation for redo."""
        return {
            "command": entry.command,
            "redo_executed": True,
            "description": f"Redid: {entry.description}"
        }
    
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