"""
Structured error classes for Easy Edits Pro CLI.
"""

from typing import Dict, Any, Optional
from enum import Enum


class ErrorCode(str, Enum):
    """Standard error codes."""
    # Generic
    INTERNAL_ERROR = "INTERNAL_ERROR"
    INVALID_ARGUMENT = "INVALID_ARGUMENT"
    NOT_FOUND = "NOT_FOUND"
    ALREADY_EXISTS = "ALREADY_EXISTS"
    PERMISSION_DENIED = "PERMISSION_DENIED"
    
    # Backend
    BACKEND_NOT_FOUND = "BACKEND_NOT_FOUND"
    BACKEND_ERROR = "BACKEND_ERROR"
    BACKEND_TIMEOUT = "BACKEND_TIMEOUT"
    
    # Project
    PROJECT_NOT_OPEN = "PROJECT_NOT_OPEN"
    PROJECT_INVALID = "PROJECT_INVALID"
    PROJECT_CORRUPT = "PROJECT_CORRUPT"
    
    # Timeline
    TIMELINE_NOT_FOUND = "TIMELINE_NOT_FOUND"
    TIMELINE_INVALID = "TIMELINE_INVALID"
    
    # Track
    TRACK_NOT_FOUND = "TRACK_NOT_FOUND"
    TRACK_LOCKED = "TRACK_LOCKED"
    TRACK_KIND_MISMATCH = "TRACK_KIND_MISMATCH"
    
    # Clip
    CLIP_NOT_FOUND = "CLIP_NOT_FOUND"
    CLIP_INVALID_TIMING = "CLIP_INVALID_TIMING"
    CLIP_SOURCE_MISSING = "CLIP_SOURCE_MISSING"
    
    # Render
    RENDER_FAILED = "RENDER_FAILED"
    RENDER_PRESET_INVALID = "RENDER_PRESET_INVALID"
    RENDER_OUTPUT_INVALID = "RENDER_OUTPUT_INVALID"
    
    # Media
    MEDIA_NOT_FOUND = "MEDIA_NOT_FOUND"
    MEDIA_UNSUPPORTED = "MEDIA_UNSUPPORTED"
    MEDIA_CORRUPT = "MEDIA_CORRUPT"
    
    # Color/Audio
    COLOR_INVALID_PARAMS = "COLOR_INVALID_PARAMS"
    AUDIO_INVALID_PARAMS = "AUDIO_INVALID_PARAMS"
    
    # Undo/Redo
    NOTHING_TO_UNDO = "NOTHING_TO_UNDO"
    NOTHING_TO_REDO = "NOTHING_TO_REDO"
    
    # Command
    COMMAND_NOT_FOUND = "COMMAND_NOT_FOUND"
    COMMAND_VALIDATION_FAILED = "COMMAND_VALIDATION_FAILED"
    COMMAND_EXECUTION_FAILED = "COMMAND_EXECUTION_FAILED"


class EepError(Exception):
    """Base exception for Easy Edits Pro CLI errors."""
    
    def __init__(
        self,
        code: ErrorCode,
        message: str,
        details: Optional[Dict[str, Any]] = None,
        recoverable: bool = True
    ):
        self.code = code
        self.message = message
        self.details = details or {}
        self.recoverable = recoverable
        super().__init__(message)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to JSON-serializable dict."""
        return {
            "error": self.code.value,
            "message": self.message,
            "details": self.details,
            "recoverable": self.recoverable
        }
    
    @classmethod
    def from_backend_error(cls, error: Exception, context: str = "") -> 'EepError':
        """Create EepError from backend exception."""
        msg = str(error)
        if context:
            msg = f"{context}: {msg}"
        
        # Classify based on error message
        if "not found" in msg.lower():
            return cls(ErrorCode.NOT_FOUND, msg, recoverable=False)
        elif "permission" in msg.lower():
            return cls(ErrorCode.PERMISSION_DENIED, msg, recoverable=False)
        elif "timeout" in msg.lower():
            return cls(ErrorCode.BACKEND_TIMEOUT, msg)
        else:
            return cls(ErrorCode.BACKEND_ERROR, msg)
    
    @classmethod
    def validation_error(cls, field: str, message: str) -> 'EepError':
        return cls(
            ErrorCode.INVALID_ARGUMENT,
            f"Validation failed for {field}: {message}",
            details={"field": field},
            recoverable=True
        )
    
    @classmethod
    def not_found(cls, resource: str, identifier: str) -> 'EepError':
        return cls(
            ErrorCode.NOT_FOUND,
            f"{resource} not found: {identifier}",
            details={"resource": resource, "identifier": identifier},
            recoverable=False
        )


# Specific error types for common cases
class ProjectNotOpenError(EepError):
    def __init__(self):
        super().__init__(
            ErrorCode.PROJECT_NOT_OPEN,
            "No project is currently open. Use 'project open' or 'project create' first.",
            recoverable=True
        )


class TimelineNotFoundError(EepError):
    def __init__(self, timeline_id: str):
        super().__init__(
            ErrorCode.TIMELINE_NOT_FOUND,
            f"Timeline not found: {timeline_id}",
            details={"timeline_id": timeline_id},
            recoverable=False
        )


class TrackNotFoundError(EepError):
    def __init__(self, track_id: str):
        super().__init__(
            ErrorCode.TRACK_NOT_FOUND,
            f"Track not found: {track_id}",
            details={"track_id": track_id},
            recoverable=False
        )


class ClipNotFoundError(EepError):
    def __init__(self, clip_id: str):
        super().__init__(
            ErrorCode.CLIP_NOT_FOUND,
            f"Clip not found: {clip_id}",
            details={"clip_id": clip_id},
            recoverable=False
        )


class BackendNotFoundError(EepError):
    def __init__(self, backend: str = "saturn-cli"):
        super().__init__(
            ErrorCode.BACKEND_NOT_FOUND,
            f"Required backend not found: {backend}. Install Easy Edits Pro or build from source.",
            details={"backend": backend},
            recoverable=False
        )