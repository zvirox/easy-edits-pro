"""Unit tests for output formatting."""

import pytest
from cli_anything_eep.output import format_output, format_success, format_progress, format_command_help


def test_format_output_json(capsys):
    """Test JSON output."""
    data = {"key": "value", "number": 42}
    format_output(data, 'json')
    captured = capsys.readouterr()
    assert '"key": "value"' in captured.out
    assert '"number": 42' in captured.out


def test_format_output_text(capsys):
    """Test text output."""
    data = {"key": "value"}
    format_output(data, 'text')
    captured = capsys.readouterr()
    assert "key: value" in captured.out


def test_format_output_table(capsys):
    """Test table output."""
    data = [{"name": "a", "value": 1}, {"name": "b", "value": 2}]
    format_output(data, 'table')
    captured = capsys.readouterr()
    assert "a" in captured.out
    assert "b" in captured.out


def test_format_success(capsys):
    """Test success formatting."""
    format_success("Operation complete", {"id": "123"}, 'json')
    captured = capsys.readouterr()
    assert '"success": true' in captured.out
    assert '"id": "123"' in captured.out


def test_format_error(capsys):
    """Test error formatting."""
    error_data = {"error": "TEST_ERROR", "message": "Something went wrong"}
    format_output(error_data, 'text')
    captured = capsys.readouterr()
    assert "Error:" in captured.err
    assert "Something went wrong" in captured.err


def test_format_progress(capsys):
    """Test progress indicator."""
    format_progress(5, 10, "Rendering")
    captured = capsys.readouterr()
    assert "50.0%" in captured.out


def test_format_command_help(capsys):
    """Test command help formatting."""
    commands = [
        {"name": "create", "description": "Create project", "params": [{"name": "name"}]}
    ]
    format_command_help(commands, 'text')
    captured = capsys.readouterr()
    assert "create" in captured.out
    assert "Create project" in captured.out