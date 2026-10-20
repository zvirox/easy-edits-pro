"""Binary discovery and lifecycle management for companion services.

The harness drives two long-lived Rust binaries besides `saturn-cli`:

* `saturn-control` — HTTP/WebSocket control server, started as a daemon.
* `saturn-mcp` — MCP server over stdin/stdout, owned by an MCP client.

This module finds those binaries, records the daemons the CLI started, and
probes their health without pretending a dead process is alive.
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

from .errors import BackendNotFoundError, EepError, ErrorCode

# Seconds a freshly spawned service may take to answer its health probe.
START_TIMEOUT = 10.0
# Seconds a terminating service may take before it is killed.
STOP_TIMEOUT = 5.0


def state_dir() -> Path:
    root = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local/state")
    return Path(root) / "easy-edits-pro"


def find_binary(name: str, env_var: Optional[str] = None) -> Optional[str]:
    """Resolve a workspace binary from an override, PATH, or the cargo target dir."""
    if env_var:
        configured = os.environ.get(env_var)
        if configured:
            return configured if Path(configured).is_file() else None

    installed = shutil.which(name)
    if installed:
        return installed

    repository = Path(__file__).resolve().parents[3]
    built = [
        candidate
        for candidate in (
            repository / "target" / "release" / name,
            repository / "target" / "debug" / name,
        )
        if candidate.is_file() and os.access(candidate, os.X_OK)
    ]
    if not built:
        return None
    # A stale release build must not shadow a newer debug build during development.
    built.sort(key=lambda candidate: candidate.stat().st_mtime, reverse=True)
    return str(built[0])


def require_binary(name: str, env_var: Optional[str] = None) -> str:
    binary = find_binary(name, env_var)
    if binary is None:
        raise BackendNotFoundError(
            f"{name} was not found. Build it with `cargo build --bin {name}` "
            f"or set {env_var or 'PATH'}."
        )
    return binary


def http_json(url: str, timeout: float = 2.0) -> dict[str, Any]:
    """GET a JSON document from a local service."""
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, OSError, json.JSONDecodeError) as error:
        raise EepError(
            ErrorCode.BACKEND_ERROR,
            f"Could not reach {url}: {error}",
            details={"url": url},
        ) from error
    if not isinstance(payload, dict):
        raise EepError(ErrorCode.BACKEND_ERROR, f"{url} did not return a JSON object")
    return payload


@dataclass
class ServiceRecord:
    """One daemon started by this CLI."""

    name: str
    pid: int
    host: str
    port: int
    command: list[str]
    log: str
    started_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat(timespec="seconds")
    )

    def is_alive(self) -> bool:
        try:
            os.kill(self.pid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return True
        return True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> "ServiceRecord":
        return cls(
            name=str(value["name"]),
            pid=int(value["pid"]),
            host=str(value.get("host", "127.0.0.1")),
            port=int(value.get("port", 0)),
            command=[str(part) for part in value.get("command", [])],
            log=str(value.get("log", "")),
            started_at=str(value.get("started_at", "")),
        )


class ServiceRegistry:
    """JSON file listing daemons started by the harness."""

    def __init__(self, path: Optional[Path] = None):
        self.path = path or state_dir() / "services.json"

    def _load(self) -> dict[str, ServiceRecord]:
        if not self.path.exists():
            return {}
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return {}
        records = {}
        for name, value in payload.get("services", {}).items():
            try:
                records[name] = ServiceRecord.from_dict(value)
            except (KeyError, TypeError, ValueError):
                continue
        return records

    def _save(self, records: dict[str, ServiceRecord]) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        payload = {"services": {name: record.to_dict() for name, record in records.items()}}
        temporary.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        os.replace(temporary, self.path)

    def get(self, name: str) -> Optional[ServiceRecord]:
        return self._load().get(name)

    def set(self, record: ServiceRecord) -> None:
        records = self._load()
        records[record.name] = record
        self._save(records)

    def remove(self, name: str) -> Optional[ServiceRecord]:
        records = self._load()
        record = records.pop(name, None)
        self._save(records)
        return record


class ManagedService:
    """Start, probe, and stop one recorded daemon."""

    def __init__(self, name: str, registry: Optional[ServiceRegistry] = None):
        self.name = name
        self.registry = registry or ServiceRegistry()

    def start(
        self,
        argv: list[str],
        *,
        host: str,
        port: int,
        health_path: str = "/health",
        timeout: float = START_TIMEOUT,
    ) -> dict[str, Any]:
        existing = self.registry.get(self.name)
        if existing and existing.is_alive():
            raise EepError(
                ErrorCode.ALREADY_EXISTS,
                f"{self.name} is already running (pid {existing.pid}).",
                details={"pid": existing.pid, "port": existing.port},
                recoverable=False,
            )

        log_path = state_dir() / "logs" / f"{self.name}.log"
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log = log_path.open("w", encoding="utf-8")
        process = subprocess.Popen(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
            text=True,
        )

        record = ServiceRecord(
            name=self.name,
            pid=process.pid,
            host=host,
            port=port,
            command=argv,
            log=str(log_path),
        )

        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if process.poll() is not None:
                break
            try:
                health = http_json(f"http://{host}:{port}{health_path}", timeout=1.0)
            except EepError:
                time.sleep(0.1)
                continue
            self.registry.set(record)
            log.close()
            return {**record.to_dict(), "state": "running", "health": health}

        log.close()
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=STOP_TIMEOUT)
            except subprocess.TimeoutExpired:
                process.kill()

        raise EepError(
            ErrorCode.BACKEND_ERROR,
            f"{self.name} did not become healthy on http://{host}:{port}{health_path}.",
            details={"log": str(log_path), "tail": _tail(log_path), "command": argv},
            recoverable=False,
        )

    def status(self) -> dict[str, Any]:
        record = self.registry.get(self.name)
        if record is None:
            return {"name": self.name, "state": "stopped"}
        if not record.is_alive():
            self.registry.remove(self.name)
            return {"name": self.name, "state": "stopped", "stale_pid": record.pid}

        result: dict[str, Any] = {**record.to_dict(), "state": "running"}
        try:
            result["health"] = http_json(
                f"http://{record.host}:{record.port}/health", timeout=1.5
            )
        except EepError as error:
            result["health_error"] = error.message
        return result

    def stop(self) -> dict[str, Any]:
        record = self.registry.get(self.name)
        if record is None:
            return {"name": self.name, "state": "stopped"}

        if record.is_alive():
            try:
                os.kill(record.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass

            deadline = time.monotonic() + STOP_TIMEOUT
            while time.monotonic() < deadline and record.is_alive():
                time.sleep(0.05)

            if record.is_alive():
                try:
                    os.kill(record.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass

        self.registry.remove(self.name)
        return {"name": self.name, "state": "stopped", "pid": record.pid}


def _tail(path: Path, lines: int = 20) -> list[str]:
    try:
        content = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return []
    return content[-lines:]
