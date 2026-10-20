"""
Core state management for Easy Edits Pro CLI.
"""

from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from pathlib import Path
import json
import os
import tempfile


@dataclass
class ProjectState:
    """Current project state for REPL and commands."""
    current: Optional['ProjectInfo'] = None
    recent: List['ProjectInfo'] = field(default_factory=list)
    
    @classmethod
    def load(cls, path: str) -> 'ProjectState':
        """Read project metadata from an Easy Edits Pro project document."""
        project_path = Path(path).expanduser().resolve()
        try:
            document = json.loads(project_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ValueError(f"Could not read project file {project_path}: {error}") from error
        project = document.get("project")
        if not isinstance(project, dict) or not isinstance(project.get("name"), str):
            raise ValueError(f"Invalid Easy Edits Pro project document: {project_path}")
        settings = project.get("settings", {})
        sequences = project.get("sequences") or [project.get("sequence", {})]
        info = ProjectInfo(
            name=project["name"],
            path=str(project_path),
            timelines=[item.get("name", "") for item in sequences if isinstance(item, dict)],
            settings=settings if isinstance(settings, dict) else {},
        )
        previous = cls.load_saved()
        recent = [info] + [item for item in previous.recent if item.path != info.path]
        return cls(current=info, recent=recent[:20])
    
    def save(self, path: Optional[str] = None):
        """Persist CLI session metadata (the editor engine saves the project file)."""
        target = Path(path).expanduser() if path else _state_file()
        target.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "current": _project_to_dict(self.current),
            "recent": [_project_to_dict(project) for project in self.recent[-20:]],
        }
        fd, temporary = tempfile.mkstemp(prefix=".eep-state-", dir=target.parent)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as stream:
                json.dump(payload, stream, indent=2)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, target)
        except Exception:
            try:
                os.unlink(temporary)
            except OSError:
                pass
            raise

    @classmethod
    def load_saved(cls, path: Optional[str] = None) -> 'ProjectState':
        """Restore the last-open and recent-project metadata saved by the CLI."""
        target = Path(path).expanduser() if path else _state_file()
        if not target.exists():
            return cls()
        try:
            payload = json.loads(target.read_text(encoding="utf-8"))
            current = _project_from_dict(payload.get("current"))
            recent = [_project_from_dict(value) for value in payload.get("recent", [])]
            return cls(current=current, recent=[item for item in recent if item is not None])
        except (OSError, json.JSONDecodeError, TypeError, ValueError) as error:
            raise ValueError(f"Could not read CLI project state {target}: {error}") from error


def _state_file() -> Path:
    root = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local/state")
    return Path(root) / "easy-edits-pro" / "project-state.json"


def _project_to_dict(project: Optional["ProjectInfo"]) -> Optional[dict[str, Any]]:
    if project is None:
        return None
    return {
        "name": project.name,
        "path": project.path,
        "template": project.template,
        "timelines": project.timelines,
        "settings": project.settings,
    }


def _project_from_dict(value: Any) -> Optional["ProjectInfo"]:
    if not isinstance(value, dict):
        return None
    return ProjectInfo(
        name=str(value["name"]),
        path=str(value["path"]),
        template=value.get("template"),
        timelines=list(value.get("timelines", [])),
        settings=dict(value.get("settings", {})),
    )


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
