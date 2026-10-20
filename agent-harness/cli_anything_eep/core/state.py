"""
Core state management for Easy Edits Pro CLI.
"""

from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from pathlib import Path
import json


@dataclass
class ProjectState:
    """Current project state for REPL and commands."""
    current: Optional['ProjectInfo'] = None
    recent: List['ProjectInfo'] = field(default_factory=list)
    
    @classmethod
    def load(cls, path: str) -> 'ProjectState':
        """Load project from file."""
        # Would integrate with saturn-project
        return cls()
    
    def save(self, path: str):
        """Save project to file."""
        pass


@dataclass
class ProjectInfo:
    """Project metadata."""
    name: str
    path: str
    template: Optional[str] = None
    timelines: List[str] = field(default_factory=list)
    settings: Dict[str, Any] = field(default_factory=dict)


@dataclass
class TimelineInfo:
    """Timeline metadata."""
    id: str
    name: str
    frame_rate: float
    resolution: tuple
    duration: int  # ticks
    tracks: List[str] = field(default_factory=list)


@dataclass
class TrackInfo:
    """Track metadata."""
    id: str
    name: str
    kind: str  # video, audio, subtitle, submix
    clips: List[str] = field(default_factory=list)


@dataclass
class ClipInfo:
    """Clip metadata."""
    id: str
    name: str
    source: str
    track_id: str
    timeline_start: int
    timeline_duration: int
    source_in: int
    source_out: int


# Global state instance (for REPL)
_state: Optional[ProjectState] = None


def get_state() -> ProjectState:
    """Get or create global state."""
    global _state
    if _state is None:
        _state = ProjectState()
    return _state


def set_state(state: ProjectState):
    """Set global state."""
    global _state
    _state = state