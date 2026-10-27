#!/usr/bin/env python3
"""Container-side driver for the installed-package workflow.

Runs INSIDE the debian:12 container (mounted by
installed_package_workflow.py --container). The package has been
dpkg-installed by the wrapper; this script launches the real binary under
Xvfb, waits for the app's local MCP bridge, and drives the same
edit-to-render workflow, then ffprobes the result inside the container.

Usage: python3 container_driver.py --binary /usr/bin/saturn-tauri \
         --fixture /fixture/editor-smoke.webm --work /root/work
"""

import argparse
import json
import math
import socket
import subprocess
import sys
import time
from pathlib import Path

TICKS_PER_SECOND = 254_016_000_000
LAUNCH_TIMEOUT_SECONDS = 90
RENDER_TIMEOUT_SECONDS = 600


def fail(step, message):
    print(f"FAIL [{step}] {message}", file=sys.stderr)
    sys.exit(1)


def step(name):
    print(f"[pkg] {name}", flush=True)


def bridge_call(sock_path, method, params=None, timeout=60):
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(sock_path))
        request = {"method": method}
        if params is not None:
            request["params"] = params
        client.sendall(json.dumps(request).encode() + b"\n")
        buffer = bytearray()
        while not buffer.endswith(b"\n"):
            chunk = client.recv(65536)
            if not chunk:
                fail("bridge", f"socket closed while waiting for {method}")
            buffer.extend(chunk)
        response = json.loads(bytes(buffer))
    finally:
        client.close()
    if "error" in response:
        fail("bridge", f"{method} error: {response['error']}")
    return response.get("result")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    work = Path(args.work)
    work.mkdir(parents=True, exist_ok=True)

    fixture = Path(args.fixture)
    if not fixture.is_file():
        fail("fixture", f"missing fixture: {fixture}")

    # HOME comes from the wrapper; the bridge binds $XDG_RUNTIME_DIR first and
    # falls back to $HOME/.cache/easy-edits-pro/runtime/mcp.sock. No
    # XDG_RUNTIME_DIR is set inside the container, so expect the fallback.
    candidates = []
    runtime_dir = Path("/tmp/runtime")
    runtime_dir.mkdir(parents=True, exist_ok=True)
    candidates.append(runtime_dir / "easy-edits-pro-mcp.sock")
    home = os.environ.get("HOME") or "/root"
    candidates.append(Path(home) / ".cache" / "easy-edits-pro" / "runtime" / "mcp.sock")

    env = dict(os.environ)
    env["XDG_RUNTIME_DIR"] = str(runtime_dir)
    env.setdefault("HOME", "/root")
    step("launch the dpkg-installed binary under Xvfb")
    app = subprocess.Popen(
        [args.binary],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=(work / "app-stderr.log").open("wb"),
    )
    try:
        sock_path = None
        deadline = time.time() + LAUNCH_TIMEOUT_SECONDS
        while time.time() < deadline:
            if app.poll() is not None:
                fail("launch", f"app exited early {app.returncode}; see {work / 'app-stderr.log'}")
            for candidate in candidates:
                if candidate.exists():
                    sock_path = candidate
                    break
            if sock_path:
                break
            time.sleep(1)
        else:
            fail("launch", f"bridge socket never appeared; looked in {[str(c) for c in candidates]}")
        print(f"[pkg] bridge listening at {sock_path}")

        step("import fixture through command_run")
        probe = json.loads(subprocess.run(
            ["ffprobe", "-v", "error", "-print_format", "json",
             "-show_streams", "-show_format", str(fixture)],
            capture_output=True, text=True, check=True).stdout)
        video = next(s for s in probe["streams"] if s.get("codec_type") == "video")
        seconds = float(probe["format"]["duration"])
        ticks = lambda s: round(s * TICKS_PER_SECOND)
        numerator, _, denominator = video["avg_frame_rate"].partition("/")

        bridge_call(sock_path, "command_run", {
            "id": "project.new", "params": {"name": "Package Acceptance"},
        })
        bridge_call(sock_path, "command_run", {
            "id": "project.add_media",
            "params": {
                "name": fixture.name, "path": str(fixture), "kind": "video",
                "duration": ticks(seconds), "width": video["width"],
                "height": video["height"],
                "fps": round(int(numerator) / float(denominator or 1), 6),
                "codec": video.get("codec_name", "vp8"),
            },
        })
        doc = bridge_call(sock_path, "doc_inspect")
        media_id = doc["project"]["project"]["media"][0]["id"]
        video_track = next(
            t for t in doc["project"]["project"]["sequence"]["tracks"]
            if t["kind"] == "video"
        )

        step("timeline edit: add + split + save")
        bridge_call(sock_path, "command_run", {
            "id": "timeline.add_clip",
            "params": {"track_id": video_track["id"], "media_id": media_id},
        })
        bridge_call(sock_path, "command_run", {
            "id": "timeline.split_clip",
            "params": {
                "track_id": video_track["id"], "clip_index": 0,
                "at": ticks(seconds / 2),
            },
        })
        save_path = work / "container-edit.saturn"
        bridge_call(sock_path, "command_run", {
            "id": "file.save", "params": {"path": str(save_path)},
        })
        if not save_path.is_file():
            fail("save", "file.save reported ok but the file is missing")
        sequence = bridge_call(sock_path, "sequence_inspect")
        saved_track = next(t for t in sequence["tracks"] if t["kind"] == "video")
        if len(saved_track["clips"]) != 2:
            fail("verify", f"expected 2 clips after split, got {len(saved_track['clips'])}")

        step("render MP4 through the GUI job queue")
        output_path = work / "container-out.mp4"
        started = bridge_call(sock_path, "render_start", {
            "output_path": str(output_path),
            "range": {"start_ticks": 0, "end_ticks": ticks(seconds)},
        })
        job_id = started["job_id"]
        deadline = time.time() + RENDER_TIMEOUT_SECONDS
        while time.time() < deadline:
            status = bridge_call(sock_path, "render_status", {"job_id": job_id})
            state = status.get("state")
            if isinstance(state, dict):
                # JobState serializes as {"status": "running"|"completed"|...
                # with "phase"/"progress" while running, "output" when done}.
                tag = state.get("status")
                if tag == "completed":
                    break
                if tag == "failed":
                    fail("render", f"render job failed: {state}")
            elif state == "failed":
                fail("render", f"render job failed: {status}")
            if app.poll() is not None:
                fail("render", f"app exited during render with {app.returncode}")
            time.sleep(2)
        else:
            fail("render", "render job did not finish in time")
        if not output_path.is_file():
            fail("render", f"render completed without output at {output_path}")

        step("verify render output with ffprobe")
        out = json.loads(subprocess.run(
            ["ffprobe", "-v", "error", "-print_format", "json",
             "-show_streams", "-show_format", str(output_path)],
            capture_output=True, text=True, check=True).stdout)
        vstream = next(s for s in out["streams"] if s.get("codec_type") == "video")
        if vstream.get("codec_name") != "h264":
            fail("ffprobe", f"expected h264, got {vstream.get('codec_name')}")
        out_seconds = float(out["format"]["duration"])
        if not math.isclose(out_seconds, seconds, abs_tol=0.6):
            fail("ffprobe", f"duration drifted: {out_seconds:.2f}s vs {seconds:.2f}s")

        print(f"[pkg] PASS - container install: h264 {vstream['width']}x{vstream['height']}, "
              f"{out_seconds:.2f}s, {output_path.stat().st_size} bytes, work dir {work}")
    finally:
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=15)
            except subprocess.Popen.TimeoutExpired:
                app.kill()
    return 0


if __name__ == "__main__":
    sys.exit(main())
