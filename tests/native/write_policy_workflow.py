#!/usr/bin/env python3

# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Native write-policy workflow (planning.md §6f; closes security audit
priority item 5 with acceptance-level evidence).

Drives the built desktop binary's local MCP bridge and proves the write
policy at the shared IPC/bridge choke point:

  phase 1 (no harness opt-in)
    -> file.save / interop.export_edl / shortcuts.export / render_start to a
       target OUTSIDE the allowed roots are ALL rejected with the named
       policy error - no silent write-anywhere primitive for a bridge agent;
    -> a file.save INSIDE the app data root succeeds and lands on disk;
    -> the bridge cannot mint session grants (no register_write_grant method
       and no engine command of that name);
  phase 2 (SATURN_ALLOW_ANY_WRITE_TARGETS=1, the documented harness opt-in)
    -> the same outside-roots file.save succeeds.

Exit code 0 = the write policy verified on this machine.
Set EEP_NATIVE_KEEP=1 to keep the work directory for inspection.
"""

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
KEEP = "--keep" in sys.argv[1:] or os.environ.get("EEP_NATIVE_KEEP") == "1"
APP_IDENTIFIER = "com.zvirox.easyeditspro"
LAUNCH_TIMEOUT_SECONDS = 60


def step(name):
    print(f"[policy] {name}", flush=True)


def fail(step_name, message):
    print(f"FAIL [{step_name}] {message}", file=sys.stderr)
    sys.exit(1)


def bridge_call(sock_path, method, params=None, timeout=30):
    """One JSONL request over the app's local MCP bridge."""
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(sock_path))
        request = {"method": method}
        if params is not None:
            request["params"] = params
        client.sendall(json.dumps(request).encode() + b"\n")
        buffer = bytearray()
        while b"\n" not in buffer:
            chunk = client.recv(65536)
            if not chunk:
                break
            buffer.extend(chunk)
        return json.loads(bytes(buffer).decode())
    finally:
        client.close()


def command_run(sock_path, command_id, params=None, timeout=60):
    return bridge_call(sock_path, "command_run", {"id": command_id, "params": params or {}})


def launch(work, binary, phase_env=None):
    """Launch the desktop app under Xvfb with an isolated HOME; wait for the
    bridge socket. Returns the app process (already verified listening)."""
    def existing_display_locks():
        import re
        numbers = []
        for lock in Path("/tmp").glob(".X*-lock"):
            match = re.fullmatch(r"\.X(\d+)-lock", lock.name)
            if match:
                numbers.append(int(match.group(1)))
        return sorted(numbers)

    display = f":{max(existing_display_locks(), default=0) + 1}" if shutil.which("Xvfb") else None
    xvfb = None
    if display:
        xvfb = subprocess.Popen(
            ["Xvfb", display, "-screen", "0", "1600x900x24", "-nolisten", "tcp"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.0)

    vite = None
    if os.environ.get("EEP_POLICY_WITH_UI", "0") == "1":
        vite = subprocess.Popen(
            ["pnpm", "dev", "--host", "127.0.0.1", "--strictPort"],
            cwd=REPO_ROOT / "apps/saturn-tauri",
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            preexec_fn=os.setsid)
        for _ in range(150):
            try:
                with urllib.request.urlopen("http://127.0.0.1:1420/", timeout=1):
                    break
            except OSError:
                time.sleep(0.4)

    env = dict(os.environ)
    env.update({
        "HOME": str(work / f"home-{env_suffix(phase_env)}"),
        "XDG_CONFIG_HOME": str(work / "config"),
        "XDG_DATA_HOME": str(work / "data"),
        "XDG_CACHE_HOME": str(work / "cache"),
        "LIBGL_ALWAYS_SOFTWARE": "1",
        "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
    })
    if display:
        env["DISPLAY"] = display
        env["XDG_RUNTIME_DIR"] = str(work / "xdg-runtime")
    elif os.environ.get("DISPLAY") and os.environ.get("XDG_RUNTIME_DIR"):
        # Reuse the live desktop session (no Xvfb on this machine). WebKit
        # renders on X11; Wayland-only variables are dropped so the app does
        # not end up on a compositor we did not prepare.
        env["DISPLAY"] = os.environ["DISPLAY"]
        env["XDG_RUNTIME_DIR"] = os.environ["XDG_RUNTIME_DIR"]
        env.pop("WAYLAND_DISPLAY", None)
        env.pop("WAYLAND_SOCKET", None)
    else:
        fail("launch", "no Xvfb and no desktop session to reuse")
    sock_path = Path(env["XDG_RUNTIME_DIR"]) / "easy-edits-pro-mcp.sock"
    sock_path.parent.mkdir(parents=True, exist_ok=True)
    if sock_path.exists() and socket.socket(
        socket.AF_UNIX, socket.SOCK_STREAM
    ).connect_ex(str(sock_path)) == 0:
        fail("launch", f"a live editor owns {sock_path}; close the running Easy Edits Pro first")
    sock_path.unlink(missing_ok=True)
    env.update(phase_env or {})
    for key in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR"):
        os.makedirs(env[key], mode=0o700, exist_ok=True)

    stderr_log = (work / f"app-{env_suffix(phase_env)}-stderr.log").open("wb")
    app = subprocess.Popen([str(binary)], env=env,
                           stdout=subprocess.DEVNULL, stderr=stderr_log)
    deadline = time.time() + LAUNCH_TIMEOUT_SECONDS
    while time.time() < deadline:
        if app.poll() is not None:
            xvfb.terminate()
            if vite:
                os.killpg(vite.pid, signal.SIGTERM)
            fail("launch", f"app exited early rc={app.returncode}: "
                           f"{stderr_log.name} tail "
                           f"{Path(stderr_log.name).read_text(errors='replace')[-800:]}")
        if sock_path.exists():
            try:
                bridge_call(sock_path, "command_list")
                print(f"[policy] bridge listening at {sock_path}")
                return {"app": app, "xvfb": xvfb, "vite": vite, "sock": sock_path}
            except OSError:
                pass
        time.sleep(0.4)
    for proc in (app, xvfb, vite):
        if proc:
            proc.terminate()
    fail("launch", f"MCP socket never appeared at {sock_path}")


def env_suffix(extra_env):
    return "bypass" if extra_env else "policy"


def teardown(handles, work, phase_env):
    app, xvfb, vite = handles["app"], handles["xvfb"], handles["vite"]
    if app.poll() is None:
        app.terminate()
        try:
            app.wait(timeout=15)
        except subprocess.Popen.TimeoutExpired:
            app.kill()
    if xvfb and xvfb.poll() is None:
        xvfb.terminate()
    if vite and vite.poll() is None:
        os.killpg(vite.pid, signal.SIGTERM)
    print(f"[policy] phase {env_suffix(phase_env)} app exit code: {app.returncode}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary",
                        default=str(REPO_ROOT / "apps" / "saturn-tauri" / "target" / "debug" / "saturn-tauri"),
                        help="desktop binary to drive (built with cargo build in apps/saturn-tauri)")
    args = parser.parse_args()
    binary = Path(args.binary)
    if not binary.is_file():
        fail("build", f"{binary} missing; run cargo build -p saturn-tauri first")
    in_session = bool(os.environ.get("DISPLAY") and os.environ.get("XDG_RUNTIME_DIR"))
    if not shutil.which("Xvfb") and not in_session:
        fail("environment", "Xvfb must be on PATH (desktop-session-independent launch)")

    work = Path(tempfile.mkdtemp(prefix="eep-policy-"))
    if KEEP:
        print(f"[policy] keeping work dir: {work}")
    failures = []
    expected_outside = str(work / "outside" / "noperm.eep")

    # ---------------------------------------------------------------- phase 1
    step("phase 1: policy enforcement without the harness opt-in")
    handles = launch(work, binary)
    sock1 = handles["sock"]
    try:
        started = command_run(sock1, "project.new",
                              {"name": "Write Policy", "sequence_name": "Main",
                               "width": 1920, "height": 1080,
                               "frame_rate": {"numerator": 30, "denominator": 1}})
        if "error" in str(started):
            fail("setup", f"project.new failed: {started}")

        # Every bridge-reachable write command against an outside target must
        # be rejected with the named policy error BEFORE any disk effect.
        for command_id, params in [
            ("file.save", {"path": expected_outside}),
            ("interop.export_edl", {"path": expected_outside}),
            ("shortcuts.export", {"path": expected_outside}),
            ("export.presets.export", {"path": expected_outside}),
        ]:
            step(f"negative: {command_id} outside the roots")
            response = command_run(sock1, command_id, params)
            error = response.get("error") or str(response)
            if response.get("result") is not None:
                failures.append(f"{command_id} wrote outside the roots: {response}")
            elif "outside the allowed roots" not in error:
                failures.append(f"{command_id} rejected without the policy wording: {error}")
        if Path(expected_outside).exists():
            failures.append(f"the rejected file.save still landed: {expected_outside}")
        if not failures:
            print("[policy] PASS - bridge write commands reject outside-roots targets")

        step("negative: render_start outside the roots")
        started = bridge_call(sock1, "render_start",
                              {"output_path": str(work / "outside" / "no.mp4"),
                               "range": {"start_ticks": 0, "end_ticks": 254_016_000_000}})
        if "error" not in started and started.get("result") is not None:
            failures.append(f"bridge render_start bypassed the policy: {started}")
        else:
            error = started.get("error") or json.dumps(started)
            if "outside the allowed roots" not in error:
                failures.append(f"render_start rejected without the policy wording: {error}")
            else:
                print("[policy] PASS - bridge render rejects outside-roots targets")

        step("negative: the bridge cannot mint session grants")
        response = command_run(sock1, "register_write_grant", {"path": expected_outside})
        grant_error = response.get("error") or ""
        if response.get("result") is not None:
            failures.append(f"the bridge granted a write: {response}")
        else:
            print("[policy] PASS - the bridge cannot create grants "
                  f"({(grant_error or 'rejected')[:80]})")

        step("positive: file.save inside the app data root")
        app_data = work / "data" / APP_IDENTIFIER
        app_data.mkdir(parents=True, exist_ok=True)
        in_roots = app_data / "in-roots.eep"
        response = command_run(sock1, "file.save", {"path": str(in_roots)})
        if response.get("error"):
            failures.append(f"in-roots file.save rejected: {response}")
        elif not in_roots.is_file():
            failures.append(f"in-roots file.save reported ok but no file: {in_roots}")
        else:
            print("[policy] PASS - in-roots file.save succeeds")

        # A second outside attempt stays rejected after the in-roots write
        # (session answers are not one-softener; grants only come from the UI).
        response = command_run(sock1, "file.save", {"path": expected_outside})
        if response.get("result") is not None:
            failures.append("a later outside-roots write succeeded without a grant")
    finally:
        teardown(handles, work, None)

    # ---------------------------------------------------------------- phase 2
    step("phase 2: SATURN_ALLOW_ANY_WRITE_TARGETS=1 re-arms outside-root writes")
    handles = launch(work, binary, {"SATURN_ALLOW_ANY_WRITE_TARGETS": "1"})
    sock2 = handles["sock"]
    try:
        response = command_run(sock2, "file.save", {"path": expected_outside})
        if response.get("error"):
            failures.append(f"bypassed harness still rejected: {response}")
        elif not Path(expected_outside).exists():
            failures.append("bypassed harness wrote nothing")
        else:
            print("[policy] PASS - harness bypass lets outside-roots writes through")
    finally:
        teardown(handles, work, {"SATURN_ALLOW_ANY_WRITE_TARGETS": "1"})

    if failures:
        fail("policy", "; ".join(failures))

    print(f"[policy] PASS - write policy workflow: bridge rejections named and "
          f"physical, in-roots writes land, grants untestable from the bridge, "
          f"harness opt-in documented. work dir {work}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
