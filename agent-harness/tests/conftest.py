"""Pytest configuration and shared fixtures."""

import pytest
import tempfile
import shutil
from pathlib import Path


@pytest.fixture
def temp_dir():
    """Create a temporary directory for tests."""
    tmp = tempfile.mkdtemp()
    yield Path(tmp)
    shutil.rmtree(tmp, ignore_errors=True)


@pytest.fixture
def sample_project_data():
    """Sample project data for testing."""
    return {
        "name": "Test Project",
        "template": "youtube-1080p",
        "timelines": [],
        "settings": {
            "frame_rate": 30,
            "resolution": [1920, 1080],
            "color_space": "rec709"
        }
    }


@pytest.fixture
def sample_timeline_data():
    """Sample timeline data for testing."""
    return {
        "id": "tl_test",
        "name": "Test Timeline",
        "frame_rate": 30,
        "resolution": [1920, 1080],
        "duration": 0,
        "tracks": []
    }


@pytest.fixture
def sample_clip_data():
    """Sample clip data for testing."""
    return {
        "id": "clip_test",
        "name": "Test Clip",
        "source": "test.mp4",
        "track_id": "V1",
        "timeline_start": 0,
        "timeline_duration": 3000000000000,  # 10 seconds at 300M ticks/sec
        "source_in": 0,
        "source_out": 3000000000000,
        "speed": 1.0,
        "reverse": False
    }


# Pytest configuration
def pytest_configure(config):
    """Configure pytest."""
    config.addinivalue_line(
        "markers", "unit: mark test as unit test"
    )
    config.addinivalue_line(
        "markers", "e2e: mark test as end-to-end test"
    )
    config.addinivalue_line(
        "markers", "slow: mark test as slow"
    )