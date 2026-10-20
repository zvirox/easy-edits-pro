"""CLI subprocess verification tests.

These tests verify the installed CLI works correctly.
"""

import subprocess
import sys
import pytest


class TestCLISubprocess:
    """Test installed CLI via subprocess."""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Ensure CLI is installed."""
        # In CI, this would be: pip install -e .
        # For local testing, use the installed CLI
        self.cli_cmd = ["cli-anything-eep"]
    
    def run_cli(self, args, expect_json=False):
        """Run CLI command and return parsed output."""
        cmd = self.cli_cmd + args
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        return result
    
    def test_help(self):
        """Test --help works."""
        result = self.run_cli(["--help"])
        assert result.returncode == 0
        assert "Easy Edits Pro" in result.stdout
        assert "project" in result.stdout
        assert "timeline" in result.stdout
    
    def test_version(self):
        """Test --version works."""
        result = self.run_cli(["--version"])
        assert result.returncode == 0
        assert "cli-anything-eep" in result.stdout
    
    def test_json_output(self):
        """Test --json flag produces valid JSON."""
        result = self.run_cli(["--json", "project", "templates"])
        assert result.returncode == 0
        
        import json
        try:
            data = json.loads(result.stdout)
            assert isinstance(data, list)
            assert len(data) > 0
            assert all("name" in t for t in data)
        except json.JSONDecodeError:
            pytest.fail("Output is not valid JSON")
    
    def test_project_create_help(self):
        """Test project create --help."""
        result = self.run_cli(["project", "create", "--help"])
        assert result.returncode == 0
        assert "--name" in result.stdout
        assert "--template" in result.stdout
    
    def test_timeline_create_help(self):
        """Test timeline create --help."""
        result = self.run_cli(["timeline", "create", "--help"])
        assert result.returncode == 0
        assert "--fps" in result.stdout
    
    def test_track_add_help(self):
        """Test track add --help."""
        result = self.run_cli(["track", "add", "--help"])
        assert result.returncode == 0
        assert "--kind" in result.stdout
    
    def test_clip_add_help(self):
        """Test clip add --help."""
        result = self.run_cli(["clip", "add", "--help"])
        assert result.returncode == 0
        assert "--source" in result.stdout
        assert "--track" in result.stdout
        assert "--at" in result.stdout
    
    def test_render_queue_help(self):
        """Test render queue --help."""
        result = self.run_cli(["render", "queue", "--help"])
        assert result.returncode == 0
        assert "--preset" in result.stdout
        assert "--output" in result.stdout
    
    def test_color_grade_help(self):
        """Test color grade --help."""
        result = self.run_cli(["color", "grade", "--help"])
        assert result.returncode == 0
        assert "--lift" in result.stdout
    
    def test_audio_mix_help(self):
        """Test audio mix --help."""
        result = self.run_cli(["audio", "mix", "--help"])
        assert result.returncode == 0
        assert "--volume" in result.stdout
    
    def test_media_import_help(self):
        """Test media import --help."""
        result = self.run_cli(["media", "import", "--help"])
        assert result.returncode == 0
        assert "--path" in result.stdout
    
    def test_error_handling(self):
        """Test error output format."""
        # Run command that will fail (no project open)
        result = self.run_cli(["--json", "timeline", "list"])
        # Should return error but valid JSON on stderr
        assert result.returncode != 0
        
        import json
        try:
            data = json.loads(result.stderr)
            assert "error" in data
            assert "message" in data
        except json.JSONDecodeError:
            pytest.fail("Error output is not valid JSON")
    
    def test_repl_help(self):
        """Test repl command exists."""
        result = self.run_cli(["repl", "--help"])
        assert result.returncode == 0
    
    def test_undo_redo_commands(self):
        """Test undo/redo/history commands exist."""
        for cmd in ["undo", "redo", "history"]:
            result = self.run_cli([cmd, "--help"])
            assert result.returncode == 0, f"{cmd} --help failed"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])