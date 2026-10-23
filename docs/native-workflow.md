# Native laptop workflow (M0.3 evidence)

`tests/native/laptop_workflow.py` proves the real, unmocked editing pipeline
on this laptop: the Rust engine (`saturn-cli`, which embeds `saturn-core`)
and the real GStreamer render pipeline (`saturn-render` via GES). It is the
evidence standard for media, MCP, packaging, and native-window changes; a
green mocked browser e2e is explicitly **not** this proof.

## What it does

1. Builds `saturn-cli`.
2. Creates a project through the real persistence path.
3. Probes the maintained fixture (`apps/saturn-tauri/tests/fixtures/editor-smoke.webm`)
   with `ffprobe` and imports it with the true metadata (the desktop app
   probes via GStreamer; the script feeds real values, not estimates).
4. Second `saturn-cli control --project` session re-opens the saved project
   (save/reopen proof), places the clip, splits it at the mid-point, saves.
5. Verifies persistence with a fresh `inspect`: two clips, split retime intact.
6. Renders MP4 through GES (software H.264 on this host).
7. Verifies the output with `ffprobe`: codec `h264`, plausible duration.

## Usage

```bash
python3 tests/native/laptop_workflow.py        # run; temp files cleaned
python3 tests/native/laptop_workflow.py --keep # keep the work dir for inspection
```

Requires: `cargo`, `ffprobe`, and the system GStreamer/GES runtime (present on
the dev laptop). Not wired into CI because GitHub runners do not ship the
GStreamer `ges`/`libav` runtime plugins; CI covers the mocked-browser
contract, the python CLI harness covers engine commands.

## Latest recorded run (2026-10-23, dev laptop)

```
[native] build saturn-cli (real engine)
[native] create project
[native] probe and import fixture
[native] reopen and edit timeline
[native] verify saved timeline
[native] render MP4 (real GStreamer)
[native] verify render output with ffprobe
[native] PASS - h264 1920x1080, 6.06s, 1615005 bytes
```

Notes and limits, stated honestly:

- Render is **software x264**; hardware encoding remains unverified here.
- The `render` output was checked for codec and duration, not frame-by-frame
  against the preview.
- Single-fixture (VP8 640x360 -> 1080p render), single laptop; no
  cross-hardware claim is made or implied.
