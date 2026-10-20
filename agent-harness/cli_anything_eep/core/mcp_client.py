"""Minimal MCP stdio client for one-shot introspection.

MCP clients own the server process. The CLI only needs to answer "what does
this server expose?", so it starts `saturn-mcp`, performs the initialize
handshake, issues one request, and terminates the process.
"""

from __future__ import annotations

import json
import select
import subprocess
import time
from typing import Any, Optional

from .errors import EepError, ErrorCode

PROTOCOL_VERSION = "2024-11-05"


class McpSession:
    """One initialized MCP connection over stdin/stdout."""

    def __init__(self, binary: str, project: Optional[str] = None, timeout: float = 10.0):
        argv = [binary]
        if project:
            argv += ["--project", project]
        self.timeout = timeout
        self._next_id = 0
        self._process = subprocess.Popen(
            argv,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self._initialized = False

    def __enter__(self) -> "McpSession":
        try:
            self.request(
                "initialize",
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "cli-anything-eep", "version": "0.1.0"},
                },
            )
            self._notify("notifications/initialized")
            self._initialized = True
        except Exception:
            self.close()
            raise
        return self

    def __exit__(self, *_exc) -> None:
        self.close()

    def request(self, method: str, params: Optional[dict[str, Any]] = None) -> Any:
        self._next_id += 1
        request_id = self._next_id
        message: dict[str, Any] = {"jsonrpc": "2.0", "id": request_id, "method": method}
        if params is not None:
            message["params"] = params
        self._send(message)

        deadline = time.monotonic() + self.timeout
        while True:
            response = self._read(deadline)
            if response.get("id") != request_id:
                continue
            if "error" in response:
                error = response["error"]
                raise EepError(
                    ErrorCode.BACKEND_ERROR,
                    f"MCP error from {method}: {error.get('message', error)}",
                    details={"method": method, "error": error},
                )
            return response.get("result")

    def close(self) -> None:
        process = self._process
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
        for stream in (process.stdin, process.stdout, process.stderr):
            try:
                if stream:
                    stream.close()
            except OSError:
                pass

    def _send(self, message: dict[str, Any]) -> None:
        assert self._process.stdin is not None
        try:
            self._process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
            self._process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise EepError(
                ErrorCode.BACKEND_ERROR,
                f"The MCP server exited before accepting {message.get('method')}: {error}",
                details={"stderr": self._stderr()},
            ) from error

    def _notify(self, method: str) -> None:
        self._send({"jsonrpc": "2.0", "method": method})

    def _read(self, deadline: float) -> dict[str, Any]:
        assert self._process.stdout is not None
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise EepError(ErrorCode.BACKEND_TIMEOUT, "The MCP server did not answer in time.")

        ready, _, _ = select.select([self._process.stdout], [], [], remaining)
        if not ready:
            raise EepError(ErrorCode.BACKEND_TIMEOUT, "The MCP server did not answer in time.")

        line = self._process.stdout.readline()
        if not line:
            raise EepError(
                ErrorCode.BACKEND_ERROR,
                "The MCP server closed its output stream.",
                details={"stderr": self._stderr()},
            )
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise EepError(
                ErrorCode.BACKEND_ERROR,
                f"The MCP server returned invalid JSON: {line[:200]!r}",
            ) from error
        if not isinstance(message, dict):
            raise EepError(ErrorCode.BACKEND_ERROR, "The MCP server returned a non-object message.")
        return message

    def _stderr(self) -> str:
        assert self._process.stderr is not None
        if self._process.poll() is None:
            return ""
        try:
            return self._process.stderr.read()[-1000:]
        except OSError:
            return ""
