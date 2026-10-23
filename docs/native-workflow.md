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

Two runs recorded for the v0.1.0-alpha.2 / v0.1.0-alpha.3 releases:

- Commit `6d5f48a` (alpha.3/G7 tag target, current main):

```
[native] build saturn-cli (real engine)
[native] create project
[native] probe and import fixture
[native] reopen and edit timeline
[native] verify saved timeline
[native] render MP4 (real GStreamer)
[native] verify render output with ffprobe
[native] PASS - h264 1920x1080, 6.06s, 1612986 bytes
```

- Commit `c9ecc2f` (alpha.2 tag target), in a clean worktree at that
  commit: see the alpha.2 section below.

### alpha.2 commit run (`c9ecc2f`)

In a clean worktree at that commit:

```
[native] build saturn-cli (real engine)
[native] create project
[native] probe and import fixture
[native] reopen and edit timeline
[native] verify saved timeline
[native] render MP4 (real GStreamer)
[native] verify render output with ffprobe
[native] PASS - h264 1920x1080, 6.06s, 1630599 bytes
```

Notes and limits, stated honestly:

- Render is **software x264**; hardware encoding remains unverified here.
- The `render` output was checked for codec and duration, not frame-by-frame
  against the preview.
- Single-fixture (VP8 640x360 -> 1080p render), single laptop; no
  cross-hardware claim is made or implied.

## Render-cancel evidence (2026-10-23, dev laptop)

`cargo test -p saturn-render --lib -- --ignored render_cancel_midway`
(`tests::render_cancel_midway_removes_partial_output`, marked
`#[ignore]` because CI runners lack the GES runtime plugins):

```
test tests::render_cancel_midway_removes_partial_output ... ok
test result: ok. 1 passed; finished in 3.26s
```

What it proves on the real pipeline: a 12-clip timeline built from the
maintained fixture is rendered through GES with software H.264; the render
is cancelled two seconds in from the render crate's cancellation callback;
the result is `Err("Render cancelled")` and **the partial `.mp4` is gone**.
The CI-level unit (`render_cancellation_is_checked_before_pipeline_setup`)
pins the complementary boundary: a cancel before the pipeline starts never
touches a pre-existing file at the output path. Failed (non-cancel) renders
also remove their partial output, enforced inside
`render_project_range_with_control`.
