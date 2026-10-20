"""Persistent adapter for the Rust editor engine and active desktop render jobs."""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import threading
from pathlib import Path
from typing import Any, Optional

from .errors import BackendNotFoundError, EepError, ErrorCode


def _find_saturn_cli() -> Optional[str]:
    configured = os.environ.get("SATURN_CLI")
    if configured:
        return configured if Path(configured).is_file() else None
    installed = shutil.which("saturn-cli")
    if installed:
        return installed
    repository = Path(__file__).resolve().parents[3]
    for candidate in (repository / "target/release/saturn-cli", repository / "target/debug/saturn-cli"):
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate)
    return None


def detect_backends() -> dict[str, Any]:
    """Report executable backends available to the harness."""
    result: dict[str, Any] = {}
    binary = _find_saturn_cli()
    if binary:
        result["saturn-cli"] = {"path": binary, "type": "rust-control"}
    for name, kind in (("saturn-mcp", "mcp"), ("saturn-control", "control")):
        path = shutil.which(name)
        if path:
            result[name] = {"path": path, "type": kind}
    cargo = shutil.which("cargo")
    if cargo:
        result["cargo"] = {"path": cargo, "type": "build"}
    return result


class SaturnCLI:
    """Keep one Rust ``control`` process alive so a REPL shares real editor state."""

    def __init__(self, cli_path: Optional[str] = None):
        self.cli_path = cli_path or _find_saturn_cli()
        if not self.cli_path:
            raise BackendNotFoundError()
        self.current_path: Optional[str] = None
        self._process: Optional[subprocess.Popen[str]] = None
        self._lock = threading.Lock()

    def _ensure_process(self) -> subprocess.Popen[str]:
        if self._process is None or self._process.poll() is not None:
            self._process = subprocess.Popen(
                [self.cli_path, "control"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=None,
                text=True,
                bufsize=1,
            )
        return self._process

    def execute(self, command_id: str, params: Optional[dict[str, Any]] = None, *, record_undo: bool = True) -> Any:
        """Send one real ``CommandRequest`` over saturn-cli's JSON-lines control mode."""
        with self._lock:
            process = self._ensure_process()
            assert process.stdin is not None and process.stdout is not None
            request = {"id": command_id, "params": params or {}}
            try:
                process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
                process.stdin.flush()
                line = process.stdout.readline()
            except (BrokenPipeError, OSError) as error:
                self.close()
                raise EepError(ErrorCode.BACKEND_ERROR, f"Could not communicate with saturn-cli: {error}") from error
            if not line:
                self.close()
                raise EepError(ErrorCode.BACKEND_ERROR, "saturn-cli control process exited without a response")
            try:
                response = json.loads(line)
            except json.JSONDecodeError as error:
                raise EepError(ErrorCode.BACKEND_ERROR, "saturn-cli returned invalid JSON", details={"response": line[:500]}) from error

        if not response.get("ok"):
            message = str(response.get("error", "Rust editor command failed"))
            code = ErrorCode.NOTHING_TO_UNDO if "nothing to undo" in message.lower() else ErrorCode.NOTHING_TO_REDO if "nothing to redo" in message.lower() else ErrorCode.COMMAND_EXECUTION_FAILED
            raise EepError(code, message)

        if command_id == "project.new":
            self.current_path = None
        elif command_id == "file.open":
            self.current_path = str((params or {}).get("path", "")) or None
            from .undo import UndoManager
            UndoManager.clear()
        elif command_id == "file.save":
            self.current_path = str((params or {}).get("path", self.current_path or "")) or None
        elif record_undo and command_id not in {"project.inspect", "project.status", "media.list", "edit.undo", "edit.redo", "file.save"}:
            from .undo import UndoManager
            UndoManager.record(command_id, params or {}, command_id.replace(".", " "))
        if self.current_path and command_id not in {"project.inspect", "project.status", "media.list", "file.open", "file.save"}:
            self.execute("file.save", {"path": self.current_path}, record_undo=False)
        return response.get("result")

    def close(self) -> None:
        process, self._process = self._process, None
        if process is None:
            return
        if process.stdin and not process.stdin.closed:
            process.stdin.close()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()

    def project_create(self, name: str, template: Optional[str] = None, path: Optional[str] = None, *, fps: float = 30, width: int = 1920, height: int = 1080) -> dict[str, Any]:
        if template == "cinema-4k":
            fps, width, height = 24, 4096, 2160
        elif template == "vertical-9x16":
            fps, width, height = 30, 1080, 1920
        elif template == "broadcast":
            fps, width, height = 29.97, 1920, 1080
        numerator, denominator = _frame_rate(fps)
        result = self.execute("project.new", {
            "name": name,
            "sequence_name": "Sequence 1",
            "width": width,
            "height": height,
            "frame_rate": {"numerator": numerator, "denominator": denominator},
        })
        if path:
            self.project_save(path)
        return result

    def project_open(self, path: str) -> Any:
        return self.execute("file.open", {"path": path})

    def project_save(self, path: Optional[str] = None) -> Any:
        target = path or self.current_path
        if not target:
            raise EepError(ErrorCode.PROJECT_NOT_OPEN, "Save requires a project path. Use --path or open a project first.")
        return self.execute("file.save", {"path": target}, record_undo=False)

    def project_inspect(self, path: Optional[str] = None) -> dict[str, Any]:
        if path and path != self.current_path:
            self.project_open(path)
        document = self.execute("project.inspect", {}, record_undo=False)
        return {"project": document, "path": self.current_path}

    def timeline_create(self, name: str, fps: float = 30, width: int = 1920, height: int = 1080) -> Any:
        raise _unsupported("timeline.create", "The Rust engine currently creates its initial sequence with the project.")

    def timeline_list(self) -> dict[str, Any]:
        document = self.execute("project.inspect", {}, record_undo=False)
        project = document.get("project", {})
        sequences = project.get("sequences") or [project.get("sequence", {})]
        return {"timelines": sequences}

    def timeline_inspect(self, timeline_id: str) -> Any:
        document = self.execute("project.inspect", {}, record_undo=False)
        project = document.get("project", {})
        sequences = project.get("sequences") or [project.get("sequence", {})]
        sequence = next((item for item in sequences if str(item.get("id")) == timeline_id), None)
        if sequence is None:
            raise EepError(ErrorCode.TIMELINE_NOT_FOUND, f"Timeline not found: {timeline_id}", recoverable=False)
        return sequence

    def timeline_open(self, timeline_id: str) -> Any:
        return self.execute("project.switch_sequence", {"sequence_id": int(timeline_id)})

    def track_add(self, kind: str, name: Optional[str] = None, index: Optional[int] = None) -> Any:
        if kind.lower() not in {"video", "audio"}:
            raise EepError(ErrorCode.INVALID_ARGUMENT, "Track kind must be video or audio")
        params: dict[str, Any] = {"kind": kind.lower()}
        if name:
            params["name"] = name
        return self.execute("timeline.add_track", params)

    def track_remove(self, track_id: str) -> Any:
        resolved = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        return self.execute("timeline.remove_track", {"track_id": resolved})

    def track_list(self) -> dict[str, Any]:
        document = self.execute("project.inspect", {}, record_undo=False)
        return {"tracks": document.get("project", {}).get("sequence", {}).get("tracks", [])}

    def track_settings(self, track_id: str, *, muted: Optional[bool] = None, solo: Optional[bool] = None, locked: Optional[bool] = None, volume: Optional[float] = None, pan: Optional[float] = None) -> dict[str, Any]:
        track = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        result = {}
        for key, value, command, field in (
            ("muted", muted, "track.set_muted", "muted"),
            ("solo", solo, "track.set_solo", "solo"),
            ("locked", locked, "track.set_locked", "locked"),
            ("volume", volume, "track.set_gain_db", "gain_db"),
            ("pan", pan, "track.set_pan", "pan"),
        ):
            if value is not None:
                payload = _linear_to_db(value) if key == "volume" else value
                result[key] = self.execute(command, {"track_id": track, field: payload})
        return result

    def media_bins(self) -> Any:
        document = self.execute("project.inspect", {}, record_undo=False)
        return {"bins": document.get("project", {}).get("bins", [])}

    def media_info(self, media_id: str) -> Any:
        document = self.execute("project.inspect", {}, record_undo=False)
        media = document.get("project", {}).get("media", [])
        match = next((item for item in media if str(item.get("id")) == media_id), None)
        if match is None:
            raise EepError(ErrorCode.MEDIA_NOT_FOUND, f"Media item not found: {media_id}", recoverable=False)
        return match

    def color_reset(self, clip_id: str) -> Any:
        track_id, index = _clip_reference(clip_id)
        return self.execute("clip.set_color", {"track_id": track_id, "clip_index": index, "exposure": 0, "contrast": 1, "saturation": 1, "hue": 0})

    def color_wheels(self, clip_id: str) -> Any:
        track_id, index = _clip_reference(clip_id)
        document = self.execute("project.inspect", {}, record_undo=False)
        tracks = document.get("project", {}).get("sequence", {}).get("tracks", [])
        track = next((item for item in tracks if item.get("id") == track_id), None)
        if track is None or index >= len(track.get("clips", [])):
            raise EepError(ErrorCode.CLIP_NOT_FOUND, f"Clip reference not found: {clip_id}", recoverable=False)
        return track["clips"][index].get("color", {}).get("wheels", {})

    def track_automation(self, track_id: str, mode: str, param: str) -> Any:
        track = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        current = next((item for item in self.track_list()["tracks"] if item.get("id") == track), None)
        if current is None:
            raise EepError(ErrorCode.TRACK_NOT_FOUND, f"Track not found: {track_id}", recoverable=False)
        automation = current.get("automation", {})
        if param == "volume":
            automation["volume_mode"] = mode.lower()
        elif param == "pan":
            automation["pan_mode"] = mode.lower()
        else:
            raise EepError(ErrorCode.INVALID_ARGUMENT, "Automation is supported for volume and pan only")
        automation["mode"] = mode.lower()
        return self.execute("track.set_automation", {"track_id": track, "automation": automation})

    def clip_list(self, track: Optional[str] = None) -> dict[str, Any]:
        document = self.execute("project.inspect", {}, record_undo=False)
        tracks = document.get("project", {}).get("sequence", {}).get("tracks", [])
        selected = [item for item in tracks if track is None or str(item.get("id")) == track or item.get("name") == track]
        clips = [
            {**clip, "track_id": item.get("id"), "track_name": item.get("name"), "clip_index": index}
            for item in selected
            for index, clip in enumerate(item.get("clips", []))
        ]
        return {"clips": clips}

    def clip_add(self, source: str, track_id: str, timeline_start: int, timeline_duration: Optional[int] = None) -> Any:
        path = Path(source)
        if not path.exists():
            raise EepError(ErrorCode.MEDIA_NOT_FOUND, f"Media file does not exist: {source}", recoverable=False)
        kind = _media_kind(path)
        self.execute("project.add_media", {"name": path.name, "path": str(path), "kind": kind})
        document = self.execute("project.inspect", {}, record_undo=False)
        media = next((item for item in document.get("project", {}).get("media", []) if item.get("path") == str(path)), None)
        if media is None:
            raise EepError(ErrorCode.BACKEND_ERROR, "Imported media is missing from the project snapshot")
        params: dict[str, Any] = {"media_id": media["id"], "timeline_start": timeline_start}
        if track_id:
            params["track_id"] = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        if timeline_duration:
            params["duration"] = timeline_duration
        return self.execute("timeline.add_clip", params)

    def _track_id_by_name(self, name: str) -> int:
        for item in self.track_list()["tracks"]:
            if item.get("name") == name:
                return int(item["id"])
        raise EepError(ErrorCode.TRACK_NOT_FOUND, f"Track not found: {name}", recoverable=False)

    def clip_move(self, clip_id: str, track_id: str, timeline_start: int) -> Any:
        source_track, index = _clip_reference(clip_id)
        target = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        return self.execute("timeline.move_clip", {"track_id": source_track, "clip_index": index, "target_track_id": target, "timeline_start": timeline_start})

    def clip_trim(self, clip_id: str, source_in: int, source_out: int) -> Any:
        raise _unsupported("clip.trim", "The Rust command catalogue does not expose a trim operation yet.")

    def clip_remove(self, clip_id: str) -> Any:
        track_id, index = _clip_reference(clip_id)
        return self.execute("timeline.remove_clip", {"track_id": track_id, "clip_index": index})

    def render_queue(self, preset: str, output: str, timeline_id: Optional[str] = None) -> Any:
        return call_desktop_bridge("render_start", {"output_path": output, "preset": preset})

    def render_status(self, job_id: int) -> Any:
        return call_desktop_bridge("render_status", {"job_id": job_id})

    def render_cancel(self, job_id: int) -> Any:
        return call_desktop_bridge("render_cancel", {"job_id": job_id})

    def audio_mix(self, track_id: str, volume: Optional[float] = None, pan: Optional[float] = None, *, mute: Optional[bool] = None, solo: Optional[bool] = None) -> Any:
        track = int(track_id) if track_id.isdigit() else self._track_id_by_name(track_id)
        result = {}
        if volume is not None:
            result["gain"] = self.execute("track.set_gain_db", {"track_id": track, "gain_db": _linear_to_db(volume)})
        if pan is not None:
            result["pan"] = self.execute("track.set_pan", {"track_id": track, "pan": pan})
        if mute is not None:
            result["mute"] = self.execute("track.set_muted", {"track_id": track, "muted": mute})
        if solo is not None:
            result["solo"] = self.execute("track.set_solo", {"track_id": track, "solo": solo})
        return result

    def media_import(self, path: str, bin_name: Optional[str] = None) -> Any:
        source = Path(path)
        if not source.exists():
            raise EepError(ErrorCode.MEDIA_NOT_FOUND, f"Media file does not exist: {path}", recoverable=False)
        return self.execute("project.add_media", {"name": source.name, "path": str(source), "kind": _media_kind(source)})

    def media_search(self, query: str) -> Any:
        media = self.execute("media.list", {}, record_undo=False)
        entries = media if isinstance(media, list) else media.get("media", [])
        needle = query.casefold()
        return {"media": [item for item in entries if needle in str(item.get("name", "")).casefold() or needle in str(item.get("path", "")).casefold()]}

    def media_list(self, bin_name: Optional[str] = None) -> Any:
        return self.execute("media.list", {}, record_undo=False)

    def commands_list(self) -> list[dict[str, str]]:
        result = subprocess.run([self.cli_path, "commands"], capture_output=True, text=True, timeout=10, check=False)
        if result.returncode:
            raise EepError(ErrorCode.BACKEND_ERROR, result.stderr.strip() or "Could not list Rust engine commands")
        return [dict(zip(("id", "label", "shortcut"), line.split("\t", 2))) for line in result.stdout.splitlines() if line.strip()]

    def command_execute(self, command_id: str, params: dict[str, Any]) -> Any:
        return self.execute(command_id, params)

def call_desktop_bridge(method: str, params: dict[str, Any]) -> Any:
    """Call render-job APIs exposed by the active desktop app's same-user socket."""
    socket_path = _mcp_socket_path()
    request = json.dumps({"method": method, "params": params}, separators=(",", ":")).encode() + b"\n"
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(10)
            connection.connect(str(socket_path))
            connection.sendall(request)
            response = bytearray()
            while len(response) <= 1_048_576:
                part = connection.recv(4096)
                if not part or b"\n" in part:
                    response.extend(part)
                    break
                response.extend(part)
    except FileNotFoundError as error:
        raise BackendNotFoundError("Easy Edits Pro desktop bridge") from error
    except OSError as error:
        raise EepError(ErrorCode.BACKEND_ERROR, f"Desktop render bridge failed: {error}") from error
    if len(response) > 1_048_576:
        raise EepError(ErrorCode.BACKEND_ERROR, "Desktop bridge response exceeded 1 MiB")
    try:
        payload = json.loads(bytes(response).split(b"\n", 1)[0])
    except json.JSONDecodeError as error:
        raise EepError(ErrorCode.BACKEND_ERROR, "Desktop bridge returned invalid JSON") from error
    if "error" in payload:
        message = str(payload["error"])
        code = ErrorCode.NOT_FOUND if "not found" in message.casefold() else ErrorCode.BACKEND_ERROR
        raise EepError(code, message, recoverable=code != ErrorCode.NOT_FOUND)
    return payload.get("result")


def _mcp_socket_path() -> Path:
    if os.environ.get("XDG_RUNTIME_DIR"):
        return Path(os.environ["XDG_RUNTIME_DIR"]) / "easy-edits-pro-mcp.sock"
    if os.environ.get("HOME"):
        return Path(os.environ["HOME"]) / ".cache/easy-edits-pro/runtime/mcp.sock"
    return Path("/tmp") / f"easy-edits-pro-mcp-{os.environ.get('USER', 'default')}.sock"


def _frame_rate(fps: float) -> tuple[int, int]:
    from fractions import Fraction
    if fps <= 0:
        raise EepError(ErrorCode.INVALID_ARGUMENT, "Frame rate must be positive")
    value = Fraction(str(fps)).limit_denominator(100_000)
    return value.numerator, value.denominator


def _media_kind(path: Path) -> str:
    extension = path.suffix.lower()
    if extension in {".mp4", ".mov", ".mkv", ".webm", ".avi", ".m4v"}:
        return "video"
    if extension in {".wav", ".mp3", ".flac", ".ogg", ".m4a", ".aac"}:
        return "audio"
    if extension in {".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tif", ".tiff"}:
        return "image"
    return "unknown"


def _clip_reference(reference: str) -> tuple[int, int]:
    try:
        track_id, clip_index = reference.split(":", 1)
        return int(track_id), int(clip_index)
    except (ValueError, TypeError) as error:
        raise EepError(ErrorCode.INVALID_ARGUMENT, "Clip references use TRACK_ID:CLIP_INDEX from `clip list`.") from error


def _linear_to_db(value: float) -> float:
    import math
    if value < 0:
        raise EepError(ErrorCode.INVALID_ARGUMENT, "Volume must be non-negative")
    return -60.0 if value == 0 else max(-60.0, min(12.0, 20 * math.log10(value)))


def _unsupported(command: str, reason: str) -> EepError:
    return EepError(ErrorCode.COMMAND_NOT_FOUND, f"{command} is unavailable: {reason}", details={"command": command}, recoverable=False)


_saturn_cli: Optional[SaturnCLI] = None


def get_saturn_cli() -> SaturnCLI:
    global _saturn_cli
    if _saturn_cli is None:
        _saturn_cli = SaturnCLI()
    return _saturn_cli


def close_saturn_cli() -> None:
    global _saturn_cli
    if _saturn_cli is not None:
        _saturn_cli.close()
        _saturn_cli = None
