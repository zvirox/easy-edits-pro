# Glass UI — Phase 0 Feasibility Spike

**Date:** 2026-10-20
**Branch:** `feat/glass-spike`
**Plan:** [`GLASS_UI_PHASED_PLAN.md`](./GLASS_UI_PHASED_PLAN.md)
**Raw reports:** [`docs/spikes/glass/`](./spikes/glass/)

---

## Verdict

**Provisional GO for Full glass**, with one blocking condition:

> Glass costs almost nothing on this machine once WebKitGTK's DMA-BUF renderer is
> enabled. With the app's current default (DMA-BUF disabled), the same window
> collapses to **1–3.5 fps even with all blur removed**. The bottleneck is the
> video compositing path, not `backdrop-filter`.

The ≥30 fps gate is met with roughly 2× headroom on integrated graphics:

| Scenario | Result |
|---|---|
| Drag one full-glass panel over playing video | **58.9 fps** (p95 21 ms) |
| Drag two full-glass panels (stress) | **54.6 fps** (p95 26 ms) |
| Drag one panel, Lite (no blur) | 60.0 fps |
| Drag one panel, Off (flat) | 60.0 fps |
| Baseline, no video / no panels | 60.0 fps |

Do not begin Phase 2 styling until the DMA-BUF default is fixed and the real
editor preview is re-measured. Phase 1 (tokens/contrast) is not blocked.

---

## Environment

| Item | Value |
|---|---|
| OS / session | Arch Linux, Wayland (niri 26.04) |
| Kernel | 7.2.11-arch1-1 |
| CPU | Intel Core i5-1155G7 (11th gen, 4C/8T) |
| RAM | 11.7 GB |
| GPU | Intel Iris Xe (TigerLake-LP GT2, `8086:9a49`, `i915`) |
| GPU class | Integrated |
| WebKitGTK | 2.54.1 |
| Window | 1600 × 900 logical, `devicePixelRatio` 2, 60.02 Hz output |
| Build | Tauri dev build (same `webkit2gtk-4.1` runtime as a release build) |

This is the plan's "low-end / integrated graphics" class, not a high-end
discrete GPU. A discrete-GPU run is still outstanding.

---

## Method

The benchmark runs inside a real Tauri window (WebKitGTK), fullscreen and
focused so the compositor cannot throttle animation frames. No Chrome, no
mocks, no synthetic timers.

Stage content:

- a 1280×720, 30 fps H.264 clip, looping and playing;
- three static glass panels (L1 8 px, L2 14 px, L3 20 px blur);
- one draggable glass panel moved on every animation frame with `transform`;
- a stress scene with a second moving blurred panel.

Each run:

1. Warm up for 800 ms (discarded).
2. Record `requestAnimationFrame` deltas for 3.5 s (2.5 s for static scenes).
3. Compute fps, p50/p95/p99 frame time and long frames (>25 ms).
4. Count actually presented video frames with `requestVideoFrameCallback`.
5. Repeat drag scenarios 3× in rotated order and take the median.

Scenes isolate the variables:

- `empty`: solid background only → environment/rAF ceiling.
- `video`: video only, no panels → video pipeline.
- `panels`: glass panels + video.
- `stress`: two moving glass panels + video.
- Modes: `full` (blur), `lite` (gradient + shadow, zero `backdrop-filter`), `off`
  (opaque flat panel).

Reports: [`final-dmabuf-off.json`](./spikes/glass/final-dmabuf-off.json) and
[`final-dmabuf-on.json`](./spikes/glass/final-dmabuf-on.json).

---

## Results

Medians of three drag runs. `video fps` is the compositor-presented frame rate,
not the requested 30 fps.

### WebKit DMA-BUF renderer disabled (current app default)

| Scenario | UI fps | p50 | p95 | Video fps |
|---|---:|---:|---:|---:|
| Baseline empty | 60.0 | 17 ms | 17 ms | — |
| Video only | **1.1** | 1015 ms | 1024 ms | **0.0** |
| Drag — Full glass | **2.4** | 392 ms | 527 ms | 0.0 |
| Drag — Lite | **3.3** | 313 ms | 340 ms | 0.0 |
| Drag — Off (no blur) | **3.5** | 279 ms | 356 ms | 0.0 |
| Drag — 2 panels, Full | **1.4** | 682 ms | 1010 ms | 0.0 |
| Static — Full | 60.1 | 17 ms | 17 ms | 0.0 |
| Static — Lite | 60.0 | 17 ms | 17 ms | 0.0 |
| Static — Off | 60.0 | 17 ms | 17 ms | 0.0 |

Two things stand out:

1. **The video never presents a frame** (`0.0` video fps) although `play()`
   resolves and `readyState` is sufficient. The webview is playing a frozen
   surface.
2. **Blur is not the problem.** Removing it entirely (`off`) still gives 3.5 fps
   while anything moves over the video. The static 60 fps rows are cheap
   precisely because the video is frozen.

### WebKit DMA-BUF renderer enabled (`EEP_KEEP_DMABUF=1`)

| Scenario | UI fps | p50 | p95 | Video fps |
|---|---:|---:|---:|---:|
| Baseline empty | 60.0 | 17 ms | 17 ms | — |
| Video only | 60.0 | 17 ms | 17 ms | 30.0 |
| Drag — Full glass | **58.9** | 17 ms | 21 ms | 30.1 |
| Drag — Lite | 60.0 | 17 ms | 17 ms | 30.2 |
| Drag — Off (no blur) | 60.0 | 17 ms | 17 ms | 30.1 |
| Drag — 2 panels, Full | **54.6** | 17 ms | 26 ms | 30.2 |
| Static — Full | 60.0 | 17 ms | 17 ms | 30.5 |
| Static — Lite | 60.0 | 17 ms | 17 ms | 30.3 |
| Static — Off | 60.0 | 17 ms | 18 ms | 30.3 |

Full glass costs about **1.1 fps** against the flat baseline when dragging one
panel, and about **5.4 fps** in the two-panel stress case. p99 remains ≤30 ms.
For an editor whose main content is a 30 fps video, this is comfortably above
the 30 fps gate. A repeat run measured 58.5 fps (full) and 58.0 fps (stress),
so the result is stable rather than a one-off.

### Real editor re-measurement (production build)

The spike above used a synthetic stage. After the DMA-BUF default was fixed, the
same benchmark ran inside the real editor with a 10 s 1080p H.264 clip, at
1600×900 @ DPR 2, against the production asset bundle:

| Real editor scenario | DMA-BUF on (default) | `EEP_DISABLE_DMABUF=1` |
|---|---:|---:|
| Video only | **54.0 fps**, 30 fps video, 0 dropped | 13.1 fps, 0 presented frames |
| Drag full glass, run 1 | **43.9 fps** | 8.5 fps |
| Drag full glass, run 2 | **47.9 fps** | 7.2 fps |

After G1d tokenized the style sheet, the same build held **51.6 fps**
video-only and **47.2 / 47.8 fps** while dragging, still with 0 dropped video
frames. The full-glass path keeps roughly 1.5× headroom over the 30 fps gate.

The runtime probe was also verified live: the default reports
`dmabufPolicy=enabled-default` and `data-webkit-gtk-slow="false"`; with
`EEP_DISABLE_DMABUF=1` it reports `dmabuf=disabled`, `data-webkit-gtk-slow="true"`,
and the UI paints with no `backdrop-filter` at all.

---

## Interpretation

### The blocker was `WEBKIT_DISABLE_DMABUF_RENDERER=1`

`apps/saturn-tauri/src/main.rs` used to set `WEBKIT_DISABLE_DMABUF_RENDERER=1`
unconditionally on Linux (introduced in `c6b5ee5`, alongside the local media
server). On this Intel/Wayland stack that workaround does the opposite of what
the preview needs: the video sink never presents and the UI stalls whenever the
video layer is visible and something moves.

This was **not** a glass-specific issue. It was a preview regression first,
because it made the editor unusable with video regardless of the visual design.

**Resolved:** `src/gpu_detect.rs` now applies an opt-out policy before Tauri or
WebKit initialises:

- DMA-BUF renderer is **enabled by default**.
- `EEP_DISABLE_DMABUF=1` disables it for every driver.
- `EEP_DISABLE_DMABUF=i915,nvidia` (driver or vendor names) disables it only for
  matching GPUs, leaving other machines on the fast path.
- An externally set `WEBKIT_DISABLE_DMABUF_RENDERER` is respected.
- `EEP_KEEP_DMABUF=1` remains a deprecated force-enable shim.
- The frontend tags `[data-webkit-gtk-slow="true"]` whenever DMA-BUF is disabled
  or the GPU probe is unknown, which switches the UI to the Lite token set.

Eleven Rust policy tests cover the matrix.

### Glass is affordable on integrated graphics

With a healthy compositor path, one moving 20 px blur panel over 30 fps video
holds 58.9 fps; two hold 54.6 fps. The static panels are effectively free.
`backdrop-filter` is therefore viable at the plan's proposed elevations
(8/14/20 px).

### Lite and Off remain necessary

Not because of raw GPU cost here, but as:

- a kill switch for driver-specific compositor bugs;
- a fallback for older integrated GPUs and software-rendered sessions;
- the reduced-transparency accessibility path.

The spike ships both as `[data-glass="lite"]` (gradient + shadow, zero blur) and
`[data-glass="off"]` (flat), plus the `VITE_GLASS_ENABLED` build flag and a
`[data-webkit-gtk-slow]` class derived from Rust-side GPU detection.

---

## Plan vs. repository reality

The plan was written against a slightly different tree. Findings:

| Plan assumption | Reality |
|---|---|
| `src-tauri/src/gpu_detect.rs` | The Tauri app is `apps/saturn-tauri`, an independent Cargo workspace. |
| `styles/NN-editor.css` | Correct: 18 numbered files plus `index.css`, 8,553 lines total. |
| "0 ad-hoc backdrop-filter" | 11 ad-hoc rules (15 declarations including `-webkit-`) already exist in `styles/`, across 5 files. |
| `--text-muted: #7d6377` (139 rules to lift) | The token is already `#a890a0` (4.5:1 on surface). Recount before doing that batch. |
| `motion/react` to be added | `motion ^14.2.0` is installed and already used by `components/interior/*`. |
| `pnpm check:contrast` in CI | No contrast or stylelint scripts exist yet; CI is `.github/workflows/ci.yml`. |
| `transition: all` cleanup | 41 occurrences in `styles/`. |
| Raw literal cleanup | 762 raw hex + 436 `rgb()/rgba()` occurrences in `styles/`. |
| `styles/120-glass.css` | Does not exist; no `.glass` selectors outside the spike. Greenfield. |

The plan's Phase 1 "139 rules" figure should be recalculated from the current
tree rather than trusted, and Phase 2's rename map should be checked against the
actual import order before renaming (the numbering differs from the plan's
`10-base.css`/`20-layout.css` proposal).

---

## Reproduction

```bash
cd apps/saturn-tauri

# One-time: generate the moving test clip (ignored by git)
scripts/make-glass-spike-video.sh

# Current default: DMA-BUF enabled
VITE_GLASS_SPIKE=1 pnpm tauri dev

# Comparison: force the lite path (no DMA-BUF)
EEP_DISABLE_DMABUF=1 VITE_GLASS_SPIKE=1 pnpm tauri dev
```

The benchmark starts automatically, runs for about 70 seconds and writes
`$XDG_STATE_HOME/easy-edits-pro/glass-spike/<label>.json`. Set
`VITE_SPIKE_LABEL=name` to choose the file name. Manual re-runs are available
from the in-window **Run benchmark** button.

The spike is development-only: `main.tsx` mounts it only when
`VITE_GLASS_SPIKE=1`, and normal builds never render it.

---

## Limitations

- **One machine, one GPU.** Integrated Iris Xe only. No discrete GPU, older
  Intel UHD, AMD, NVIDIA or software-rendered session was measured.
- **Linux/Wayland only.** Windows (WebView2/Chromium) and macOS (WKWebView)
  are untested, though `backdrop-filter` support differs by engine.
- **Dev build.** Same WebKitGTK library as release; release packaging does not
  change compositing.
- **720p30 single video in the spike; 1080p30 single video in the real editor.**
  4K previews, multiple simultaneous videos, HDR and 10-bit sources are
  untested and will cost more.
- **Panel geometry is representative, not exhaustive.** Four panels plus one
  or two moving panels cover the common editor layout; full-screen blur or
  dozens of layers would be heavier.
- **DMA-BUF safety is validated only on Intel i915 here.** AMD, NVIDIA, older
  Intel and software-rendered sessions still need the driver-matrix pass. The
  opt-out policy and Lite fallback exist precisely so a bad driver can be
  quarantined without shipping a new UI.

---

## Follow-up status

1. **Done — preview path fixed.** DMA-BUF is opt-out (`EEP_DISABLE_DMABUF=1`)
   with a per-driver list; the real timeline preview was measured on Intel
   i915. AMD and NVIDIA validation is still outstanding.
2. **Done — real editor re-measured** with a 1080p30 source (see above).
3. **Done — three safety layers kept:** `VITE_GLASS_ENABLED`,
   `data-glass="lite|off"`, and `[data-webkit-gtk-slow]` (GPU class + DMA-BUF
   state).
4. **Done — Phase 2** (`120-glass.css`, `.glass` elevation system). The real
   editor held **60.1 fps video-only**, **60.2 fps with the L4 command palette
   open**, and **58.6 fps while dragging a clip**, with **0 dropped video
   frames** in all three runs. The 36-state dark/light × Full/Lite/Off visual
   baseline was captured at 1600×900.

---

## C1 addendum — hardware matrix (started 2026-10-23)

This is the packet-C1 results table. A row is only recorded from a real
benchmark run on that configuration (spike harness on `feat/glass-spike`,
later the C10 in-editor `perf:glass` script). Thresholds to hold: video ≥ 30
fps, drag p95 ≤ 25 ms, DMA-BUF on unless the row records a failure.

| # | GPU / driver | Session | WebKitGTK | Video fps | Drag fps | Drag p95 | DMA-BUF | Result |
|---|---|---|---|---|---|---|---|---|
| 1 | Intel Iris Xe (`8086:9a49`, i915) | Wayland (niri 26.04) | 2.54.1 | 58.9 (panel-over-video, spike) | 58.9 / 54.6 stress | 21-26 ms | on | PASS (phase-0 spike + real-editor re-checks) |
| 2 | NVIDIA proprietary | Wayland | current | - | - | - | - | open — needs machine |
| 3 | NVIDIA proprietary | X11 | current | - | - | - | - | open — needs machine |
| 4 | AMD (RADV) | Wayland | current | - | - | - | - | open — needs machine |
| 5 | AMD (RADV) | X11 | current | - | - | - | - | open — needs machine |
| 6 | Intel / software (llvmpipe) | X11, 2.4x WebKitGTK (oldest supported distro) | - | - | - | - | - | open — needs machine |

Record the contributing machine's details (distro, kernel, driver version,
WebKitGTK from `python3 -c "import gi; gi.require_version('WebKit','4.1')"`
or `webkit2gtk-4.1` pkg-config, window size, DPR) alongside each row, mirroring
the spike's Environment section. Rows 2-5 gate the C2 DMA-BUF defaults: any
configuration that fails skips DMA-BUF by default via the GPU-class list, and
the C2 first-paint watchdog covers the un-enumerated tail.
