"""Unit tests for error handling."""

import pytest
from cli_anything_eep.core.errors import (
    EepError, ErrorCode, ProjectNotOpenError, TimelineNotFoundError,
    TrackNotFoundError, ClipNotFoundError, BackendNotFoundError
)


def test_eep_error_to_dict():
    """Test EepError serialization."""
    err = EepError(
        ErrorCode.INVALID_ARGUMENT,
        "Test message",
        details={"field": "test"},
        recoverable=True
    )
    d = err.to_dict()
    assert d["error"] == "INVALID_ARGUMENT"
    assert d["message"] == "Test message"
    assert d["details"] == {"field": "test"}
    assert d["recoverable"] is True


def test_error_codes():
    """Test all error codes are valid."""
    for code in ErrorCode:
        assert isinstance(code.value, str)
        assert len(code.value) > 0


def test_specific_errors():
    """Test specific error types."""
    assert isinstance(ProjectNotOpenError(), EepError)
    assert ProjectNotOpenError().code == ErrorCode.PROJECT_NOT_OPEN
    
    assert isinstance(TimelineNotFoundError("tl_123"), EepError)
    assert TimelineNotFoundError("tl_123").code == ErrorCode.TIMELINE_NOT_FOUND
    
    assert isinstance(TrackNotFoundError("tr_456"), EepError)
    assert TrackNotFoundError("tr_456").code == ErrorCode.TRACK_NOT_FOUND
    
    assert isinstance(ClipNotFoundError("cl_789"), EepError)
    assert ClipNotFoundError("cl_789").code == ErrorCode.CLIP_NOT_FOUND
    
    assert isinstance(BackendNotFoundError(), EepError)
    assert BackendNotFoundError().code == ErrorCode.BACKEND_NOT_FOUND


def test_validation_error():
    """Test validation error factory."""
    err = EepError.validation_error("fps", "must be positive")
    assert err.code == ErrorCode.INVALID_ARGUMENT
    assert "fps" in err.message
    assert err.details["field"] == "fps"


def test_not_found_error():
    """Test not found error factory."""
    err = EepError.not_found("Timeline", "tl_123")
    assert err.code == ErrorCode.NOT_FOUND
    assert "tl_123" in err.message
    assert err.details["resource"] == "Timeline"
    assert err.details["identifier"] == "tl_123"