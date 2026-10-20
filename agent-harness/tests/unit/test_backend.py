"""Regression coverage for the Rust control-process adapter."""

import json
import pytest

from cli_anything_eep.core.backend import SaturnCLI


def test_control_backend_sends_command_requests_to_saturn_cli(tmp_path):
    executable = tmp_path / "saturn-cli"
    executable.write_text(
        "#!/usr/bin/env python3\n"
        "import json, sys\n"
        "for line in sys.stdin:\n"
        " request = json.loads(line)\n"
        " print(json.dumps({'ok': True, 'result': {'id': request['id'], 'params': request['params']}}), flush=True)\n"
    )
    executable.chmod(0o755)

    backend = SaturnCLI(str(executable))
    try:
        result = backend.execute("timeline.add_track", {"kind": "video"})
        assert result == {"id": "timeline.add_track", "params": {"kind": "video"}}
    finally:
        backend.close()


def test_control_backend_maps_engine_errors_to_structured_errors(tmp_path):
    executable = tmp_path / "saturn-cli"
    executable.write_text(
        "#!/usr/bin/env python3\n"
        "import json, sys\n"
        "for line in sys.stdin:\n"
        " request = json.loads(line)\n"
        " print(json.dumps({'ok': False, 'error': 'Nothing to undo'}), flush=True)\n"
    )
    executable.chmod(0o755)

    backend = SaturnCLI(str(executable))
    try:
        try:
            backend.execute("edit.undo", {})
        except Exception as error:
            assert getattr(error, "code", None).value == "NOTHING_TO_UNDO"
        else:
            raise AssertionError("engine error was not raised")
    finally:
        backend.close()


def test_built_rust_engine_project_track_undo_redo_roundtrip(tmp_path):
    from cli_anything_eep.core.backend import _find_saturn_cli

    binary = _find_saturn_cli()
    if not binary:
        pytest.skip("saturn-cli has not been built")
    project_path = tmp_path / "control-roundtrip.eep"
    backend = SaturnCLI(binary)
    try:
        backend.project_create("Control Roundtrip", path=str(project_path))
        backend.track_add("video", "V-Test")
        assert any(track["name"] == "V-Test" for track in backend.track_list()["tracks"])
        backend.execute("edit.undo", {}, record_undo=False)
        assert not any(track["name"] == "V-Test" for track in backend.track_list()["tracks"])
        backend.execute("edit.redo", {}, record_undo=False)
        assert any(track["name"] == "V-Test" for track in backend.track_list()["tracks"])
        assert project_path.is_file()
    finally:
        backend.close()
