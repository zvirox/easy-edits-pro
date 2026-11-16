# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
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
from .runtime import find_binary
from .session import SessionLog

# Commands that never change the project document, so they stay out of the journal.
_UNJOURNALLED_COMMANDS = {
    "clip.list",
    "command.list",
    "edit.undo",
    "edit.redo",
    "file.open",
    "file.save",
    "media.list",
    "project.branch_diff",
    "project.branch_list",
    "project.inspect",
    "project.status",
    "timeline.list",
    "verify.run",
}

# Commands that must not trigger the implicit save that follows a mutation.
_NO_AUTOSAVE_COMMANDS = {
    "file.open",
    "file.save",
    "media.list",
    "project.branch_diff",
    "project.branch_list",
    "project.inspect",
    "project.status",
    "verify.run",
}


def _find_saturn_cli() -> Optional[str]:
    return find_binary("saturn-cli", "SATURN_CLI")


def detect_backends() -> dict[str, Any]:
    """Report executable backends available to the harness."""
    result: dict[str, Any] = {}
    binary = _find_saturn_cli()
    if binary:
        result["saturn-cli"] = {"path": binary, "type": "rust-control"}
    for name, kind, variable in (
        ("saturn-mcp", "mcp", "SATURN_MCP"),
        ("saturn-control", "control", "SATURN_CONTROL"),
    ):
        path = find_binary(name, variable)
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

    def execute(self, command_id: str, params: Optional[dict[str, Any]] = None, *, record_history: bool = True) -> Any:
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
        elif command_id == "file.save":
            self.current_path = str((params or {}).get("path", self.current_path or "")) or None

        if record_history and command_id not in _UNJOURNALLED_COMMANDS:
            SessionLog().record(command_id, params or {}, project=self.current_path)

        if self.current_path and command_id not in _NO_AUTOSAVE_COMMANDS:
            self.execute("file.save", {"path": self.current_path}, record_history=False)
        return response.get("result")

    def run_cli_json(self, args: list[str]) -> Any:
        """Run a one-shot `saturn-cli` subcommand and parse its JSON output."""
        with self._lock:
            process = subprocess.run(
                [self.cli_path, *args],
                capture_output=True,
                text=True,
                timeout=60,
            )
        if process.returncode != 0:
            message = (process.stderr or process.stdout).strip() or f"saturn-cli {args[0]} failed"
            code = (
                ErrorCode.COMMAND_NOT_FOUND
                if "not supported" in message.lower()
                else ErrorCode.COMMAND_EXECUTION_FAILED
            )
            raise EepError(code, message, details={"args": args}, recoverable=False)
        try:
            return json.loads(process.stdout)
        except json.JSONDecodeError as error:
            raise EepError(
                ErrorCode.BACKEND_ERROR,
                f"saturn-cli {args[0]} returned invalid JSON",
                details={"stdout": process.stdout[:500]},
            ) from error

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
        return self.execute("file.save", {"path": target}, record_history=False)

    def project_inspect(self, path: Optional[str] = None) -> dict[str, Any]:
        if path and path != self.current_path:
            self.project_open(path)
        document = self.execute("project.inspect", {}, record_history=False)
        return {"project": document, "path": self.current_path}

    def timeline_create(self, name: str, fps: float = 30, width: int = 1920, height: int = 1080) -> Any:
        raise _unsupported("timeline.create", "The Rust engine currently creates its initial sequence with the project.")

    def timeline_list(self) -> dict[str, Any]:
        document = self.execute("project.inspect", {}, record_history=False)
        project = document.get("project", {})
        sequences = project.get("sequences") or [project.get("sequence", {})]
        return {"timelines": sequences}

    def timeline_inspect(self, timeline_id: str) -> Any:
        document = self.execute("project.inspect", {}, record_history=False)
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
        document = self.execute("project.inspect", {}, record_history=False)
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
        document = self.execute("project.inspect", {}, record_history=False)
        return {"bins": document.get("project", {}).get("bins", [])}

    def media_info(self, media_id: str) -> Any:
        document = self.execute("project.inspect", {}, record_history=False)
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
        document = self.execute("project.inspect", {}, record_history=False)
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
        document = self.execute("project.inspect", {}, record_history=False)
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
        document = self.execute("project.inspect", {}, record_history=False)
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
        media = self.execute("media.list", {}, record_history=False)
        entries = media if isinstance(media, list) else media.get("media", [])
        needle = query.casefold()
        return {"media": [item for item in entries if needle in str(item.get("name", "")).casefold() or needle in str(item.get("path", "")).casefold()]}

    def media_list(self, bin_name: Optional[str] = None) -> Any:
        return self.execute("media.list", {}, record_history=False)

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


class DaemonClient:
    """Connect to an authenticated headless saturn-control daemon over UDS (AF9)."""

    def __init__(
        self,
        socket_path: str,
        token: Optional[str] = None,
        *,
        session_id: Optional[str] = None,
        actor: str = "agent-harness",
        timeout: float = 30.0,
    ):
        self.socket_path = socket_path
        self.token = token
        self.session_id = session_id or f"harness-{os.getpid()}"
        self.actor = actor
        self.timeout = timeout
        self._socket: Optional[socket.socket] = None
        self._lock = threading.Lock()
        self.authenticated = False

    def connect(self) -> dict[str, Any]:
        """Establish UDS connection and authenticate if token is supplied."""
        with self._lock:
            if self._socket is not None:
                return {"authenticated": self.authenticated}
            try:
                sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                sock.settimeout(self.timeout)
                sock.connect(str(self.socket_path))
                self._socket = sock
            except OSError as error:
                raise EepError(
                    ErrorCode.BACKEND_ERROR,
                    f"Could not connect to daemon at {self.socket_path}: {error}",
                ) from error

        if self.token:
            return self.auth(self.token)
        return {"connected": True}

    def auth(self, token: str) -> dict[str, Any]:
        res = self.call("auth", {"token": token, "session_id": self.session_id, "actor": self.actor})
        self.authenticated = True
        return res

    def call(self, method: str, params: Optional[dict[str, Any]] = None) -> Any:
        with self._lock:
            if self._socket is None:
                self.connect()
            assert self._socket is not None
            payload: dict[str, Any] = {"method": method, "params": params or {}}
            if self.token and not self.authenticated:
                payload["token"] = self.token
            try:
                line = json.dumps(payload, separators=(",", ":")) + "\n"
                self._socket.sendall(line.encode("utf-8"))
                buffer = bytearray()
                while not buffer.endswith(b"\n"):
                    chunk = self._socket.recv(4096)
                    if not chunk:
                        self.close()
                        raise EepError(ErrorCode.BACKEND_ERROR, "Daemon closed connection")
                    buffer.extend(chunk)
                response = json.loads(bytes(buffer).decode("utf-8"))
            except (BrokenPipeError, OSError) as error:
                self.close()
                raise EepError(ErrorCode.BACKEND_ERROR, f"Daemon communication error: {error}") from error

        if not response.get("ok"):
            err_type = str(response.get("error", "DaemonError"))
            msg = str(response.get("message", "Daemon command failed"))
            if err_type in ("MissingAuthToken", "InvalidAuthToken"):
                raise EepError(ErrorCode.UNAUTHORIZED, f"{err_type}: {msg}")
            elif err_type == "RateLimitExceeded":
                raise EepError(ErrorCode.RATE_LIMIT_EXCEEDED, f"{err_type}: {msg}")
            elif err_type == "BudgetExceeded":
                raise EepError(ErrorCode.BUDGET_EXCEEDED, f"{err_type}: {msg}")
            elif err_type == "SessionStopped":
                raise EepError(ErrorCode.SESSION_STOPPED, f"{err_type}: {msg}")
            elif err_type in ("PathOutsideProjectRoot", "UntrustedMediaOrigin"):
                raise EepError(ErrorCode.PERMISSION_DENIED, f"{err_type}: {msg}")
            else:
                raise EepError(ErrorCode.BACKEND_ERROR, f"{err_type}: {msg}")

        return response.get("result")

    def project_open(self, path: Optional[str] = None) -> dict[str, Any]:
        params = {"path": path} if path else {}
        return self.call("project.open", params)

    def project_close(self) -> dict[str, Any]:
        return self.call("project.close")

    def project_status(self) -> dict[str, Any]:
        return self.call("project.status")

    def project_save(self, path: Optional[str] = None) -> dict[str, Any]:
        params = {"path": path} if path else {}
        return self.call("project.save", params)

    def session_stop(self) -> dict[str, Any]:
        return self.call("session.stop", {"session_id": self.session_id})

    def session_resume(self, caller: str = "human") -> dict[str, Any]:
        return self.call("session.resume", {"session_id": self.session_id, "caller": caller})

    def session_status(self) -> dict[str, Any]:
        return self.call("session.status", {"session_id": self.session_id})

    def execute_command(self, command_id: str, params: Optional[dict[str, Any]] = None) -> Any:
        return self.call(command_id, params)

    def close(self) -> None:
        with self._lock:
            sock, self._socket = self._socket, None
            self.authenticated = False
            if sock is not None:
                try:
                    sock.close()
                except OSError:
                    pass

    def __enter__(self) -> "DaemonClient":
        self.connect()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        self.close()
