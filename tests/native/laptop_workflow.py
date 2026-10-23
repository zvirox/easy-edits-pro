#!/usr/bin/env python3
"""Native laptop workflow test (planning.md M0.3).

Drives the REAL Rust engine and GStreamer renderer -- no browser mocks:

  cargo build saturn-cli
  -> create project
  -> import the maintained fixture with ffprobe-derived metadata
  -> timeline edit (add + split) through `saturn-cli control --project`
  -> save / reopen / inspect persisted state
  -> render MP4 through the GES pipeline
  -> verify the output with ffprobe (codec, size, duration)

Usage: python3 tests/native/laptop_workflow.py [--keep]
Exit code 0 = workflow verified on this machine. Set EEP_NATIVE_KEEP=1 or
pass --keep to keep the working files for inspection.
"""

import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CLI = REPO_ROOT / "target" / "debug" / "saturn-cli"
FIXTURE = REPO_ROOT / "apps" / "saturn-tauri" / "tests" / "fixtures" / "editor-smoke.webm"
TICKS_PER_SECOND = 254_016_000_000
KEEP = "--keep" in sys.argv[1:] or os.environ.get("EEP_NATIVE_KEEP") == "1"


def fail(step, message):
    print(f"FAIL [{step}] {message}", file=sys.stderr)
    sys.exit(1)


def step(name):
    print(f"[native] {name}", flush=True)


def run(args, input_text=None, expect_zero=True):
    result = subprocess.run(
        args,
        input=input_text,
        capture_output=True,
        text=True,
        cwd=str(REPO_ROOT),
    )
    if expect_zero and result.returncode != 0:
        fail(args[0], f"exit {result.returncode}: {result.stderr[:2000]}")
    return result


def ffprobe_json(path):
    result = run([
        "ffprobe", "-v", "error", "-print_format", "json",
        "-show_streams", "-show_format", str(path),
    ])
    return json.loads(result.stdout)


def parse_ticks(seconds):
    return round(float(seconds) * TICKS_PER_SECOND)


def control_session(project_path, requests, session_name):
    """Run one `saturn-cli control --project` process: write one JSONL request
    per line, read one response per line, assert every response ok, return the
    parsed results in order."""
    process = subprocess.Popen(
        [str(CLI), "control", "--project", str(project_path)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True,
    )
    results = []
    for request in requests:
        process.stdin.write(json.dumps({"id": request["id"], "params": request["params"]}) + "\n")
        process.stdin.flush()
        line = process.stdout.readline()
        if not line:
            fail(session_name, f"no response for {request['id']}: {process.stderr.read()}")
        response = json.loads(line)
        if not response.get("ok"):
            fail(session_name, f"{request['id']} rejected: {response.get('error')}")
        results.append(response.get("result"))
    process.stdin.close()
    remaining = process.stdout.read()
    process.wait(timeout=30)
    if process.returncode != 0:
        fail(session_name, f"control exited {process.returncode}: {process.stderr.read()}")
    if remaining.strip():
        fail(session_name, f"unexpected extra output: {remaining[:500]}")
    return results


def main():
    if not FIXTURE.exists():
        fail("fixture", f"missing maintained fixture: {FIXTURE}")
    if not shutil.which("ffprobe") or not shutil.which("cargo"):
        fail("environment", "ffprobe and cargo must be on PATH (this is a laptop-native test)")

    step("build saturn-cli (real engine)")
    run(["cargo", "build", "-q", "--bin", "saturn-cli"])
    if not CLI.exists():
        fail("build", "saturn-cli binary not found after cargo build")

    work = Path(tempfile.mkdtemp(prefix="eep-native-"))

    try:
        project_path = work / "workflow.saturn"
        output_path = work / "workflow-out.mp4"

        # 1. Create a new project through the real persistence path.
        step("create project")
        created = run([str(CLI), "new", str(project_path), "Native Workflow"])
        if "Created" not in created.stdout:
            fail("create", f"unexpected output: {created.stdout!r}")

        # 2. Import the maintained fixture. The desktop app probes media with
        #    GStreamer; here the script probes with ffprobe and records the
        #    real metadata, so the render uses true source values.
        step("probe and import fixture")
        probe = ffprobe_json(FIXTURE)
        video = next(s for s in probe["streams"] if s.get("codec_type") == "video")
        seconds = float(probe["format"]["duration"])
        numerator, _, denominator = video["avg_frame_rate"].partition("/")
        control_session(project_path, [
            {"id": "project.add_media", "params": {
                "name": FIXTURE.name,
                "path": str(FIXTURE),
                "kind": "video",
                "duration": parse_ticks(seconds),
                "width": video["width"],
                "height": video["height"],
                "fps": round(
                    int(numerator) / float(denominator or 1), 6
                ),
                "codec": video.get("codec_name", "vp8"),
            }},
            {"id": "file.save", "params": {"path": str(project_path)}},
        ], "import")

        # 3. A SECOND control session re-opens the saved project (save/reopen
        #    proof), then edits the timeline: place the clip, split it at the
        #    mid-point, and save again.
        step("reopen and edit timeline")
        results = control_session(project_path, [
            {"id": "project.inspect", "params": {}},
        ], "inspect")
        status = results[0]
        # The engine serialises the ProjectDocument itself: {format, ..., project}.
        project = status.get("project", {})
        media_id = project["media"][0]["id"]
        video_track = next(t for t in project["sequence"]["tracks"] if t["kind"] == "video")
        results = control_session(project_path, [
            {"id": "timeline.add_clip", "params": {
                "track_id": video_track["id"], "media_id": media_id,
            }},
            {"id": "timeline.split_clip", "params": {
                "track_id": video_track["id"],
                "clip_index": 0,
                "at": parse_ticks(seconds / 2),
            }},
            {"id": "file.save", "params": {"path": str(project_path)}},
        ], "edit")

        # 4. Verify the edit survived persistence: fresh `inspect` process.
        step("verify saved timeline")
        inspected = json.loads(run([str(CLI), "inspect", str(project_path)]).stdout)
        saved_track = next(
            t for t in inspected["project"]["sequence"]["tracks"] if t["kind"] == "video"
        )
        clips = saved_track["clips"]
        if len(clips) != 2:
            fail("verify", f"expected 2 clips after split/reopen, got {len(clips)}")
        half = parse_ticks(seconds) // 2
        if clips[1]["source_in"] != half:
            fail("verify", f"split retime lost: source_in {clips[1]['source_in']} != {half}")

        # 5. Render through the real GES/GStreamer pipeline (software H.264).
        step("render MP4 (real GStreamer)")
        rendered = run([str(CLI), "render", str(project_path), str(output_path)])
        if "Rendered" not in rendered.stdout or not output_path.exists():
            fail("render", f"render did not produce {output_path.name}")
        if output_path.stat().st_size < 10_000:
            fail("render", "output suspiciously small")

        # 6. Prove what came out of the pipeline with ffprobe.
        step("verify render output with ffprobe")
        out = ffprobe_json(output_path)
        vstream = next(s for s in out["streams"] if s.get("codec_type") == "video")
        if vstream.get("codec_name") != "h264":
            fail("ffprobe", f"expected h264, got {vstream.get('codec_name')}")
        out_seconds = float(out["format"]["duration"])
        if not math.isclose(out_seconds, seconds, abs_tol=0.4):
            fail("ffprobe", f"duration drifted: {out_seconds:.2f}s vs source {seconds:.2f}s")
        if vstream["width"] != video["width"] and vstream["width"] == 0:
            fail("ffprobe", "render reports zero width")

        print(f"[native] PASS - h264 {vstream['width']}x{vstream['height']}, "
              f"{out_seconds:.2f}s, {output_path.stat().st_size} bytes, work dir {work}")
        sys.exit(0)
    finally:
        if KEEP:
            print(f"[native] kept work dir: {work}")
        else:
            shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
