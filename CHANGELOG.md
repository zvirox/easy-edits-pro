# Changelog

All notable changes to **Easy Edits Pro** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-alpha.4] - 2026-10-24

Fourth engineering alpha: the glass-cards overhaul plan v4 is complete —
functional workstream F1–F6 (reactive selection, media relinking, transport,
keyframed exports, async encoder probe, Fairlight 4-band EQ), G-workstream
debt closed (typed per-workspace shell column presets, 1100–1280 right-dock
collapse), and the placeholder plugin-slot system removed.

### Removed
- **Fairlight placeholder plugin slots:** Removed the fake "PLUGIN SLOTS ·
  BUILT-IN EFFECT HOST PENDING" rack from the Fairlight channel strips, and
  the dead `track.add_audio_plugin` / `track.toggle_audio_plugin` commands
  plus the `Track.plugins` schema field behind them (catalogue is now 213
  commands; the real track DSP rack — 4-band parametric EQ, dynamics
  compressor, denoise — is the binding).
- **Collaborative multi-user "Lead Editor" subsystem:** Removed the header
  collab pill and identity, the dead Phase 12.1 modal state, the collab CSS,
  the `collab.acquire_lock` / `collab.release_lock` / `collab.record_change`
  commands and their engine dispatch, the `CollaborationState` project field
  (older projects simply drop the key on next save), and the Tauri
  `*_collaboration_lock` handlers. Catalogue is now 213 commands (113 shipped).

### Added
- **Per-workspace shell column presets & right-dock collapse (G7 debt):** The
  workspace switcher now applies plan §5's fixed column-ratio table
  (Edit/media/effects/fusion/animation `28/48/24`, Color `34/42/24`,
  Fairlight `40/36/24`) through a typed table that drives the shell's dock
  columns; in the 1100–1280 viewport band the right dock card collapses so
  the persistent monitor, toolbox, and timeline keep their room. The
  geometry-invariance walk verifies the preset re-ratio, the collapse band
  boundaries, and that the status-bar and no-overlap invariants hold
  throughout.
- **True 4-band track EQ (F6):** The Fairlight track EQ is a real 4-band
  parametric end-to-end — low shelf (80 Hz), mid bell (1 kHz), a new
  high-mid bell (2.5 kHz), and high shelf (8 kHz) — with the fourth band in
  the schema (backward-compatible default), the preview WebAudio chain, the
  DSP rack UI, and MP4 exports. Export now actually applies track EQ at all
  (a silent invalid-property bug previously dropped it), with failures
  surfaced instead of swallowed.
- **Async hardware encoder probe (F5):** GStreamer hardware-encoder detection
  now runs once at startup on a background job and is cached process-wide;
  the Deliver workspace reads the cached result (or the follow-up event) and
  shows the real encoder readout instead of a permanent "Probing..." placeholder.
- **Keyframes baked into exports (F4):** Clip position, scale, rotation, and
  opacity keyframes now appear in exported MP4s, matching the preview. The
  preview's cubic-bezier evaluator is ported 1:1 to Rust and a new
  `saturnkfvid` GStreamer element warps each frame around centre with
  keyframed opacity (straight alpha), attaching only to clips with visual
  keyframes; verified by live-pipeline tests.
- **Transport & frame export (F3):** The Program monitor transport now has
  Add Marker, Mark In/Out, Lift/Extract (`sequence.lift`/`sequence.extract`
  operating on the marked range — Lift keeps gaps, Extract ripples them
  closed) and Export Frame (single-frame PNG render via GES), plus monitor
  resolution telemetry and a Fit/50–200% zoom dropdown; the playhead
  timecode is accented violet.
- **MCP editing workflow:** Re-enabled `saturn-mcp` in the Cargo workspace with command discovery/execution, project and sequence inspection, media search and asset-placement preview, and asynchronous render jobs with progress and cancellation.
- **Live desktop MCP bridge:** Added a same-user Unix socket bridge so MCP edits use the open desktop project's shared editor state and undo history. Requests are size-limited and the runtime socket directory and socket use restrictive permissions.
- **Fairlight automation playback:** Added interpolated Read-mode volume and pan automation playback, plus a dedicated audio workspace hook and regression tests.
- **Render workflow coverage:** Added render range handling, cancellation/progress coverage, and expanded project command workflow tests.
- **Development plan:** Updated the main README and MCP documentation with verified capabilities, limitations, and the active delivery plan.
- **Control server:** `saturn-control` now builds and exposes `GET /health`, `GET /commands`, `POST /commands/:id/execute`, and a `GET /ws` event stream backed by the shared command registry, with route-level tests.
- **Graphics and project templates:** `saturn-graphics` and `saturn-project` are back in the Cargo workspace, with each type defined once in its owning module and library roots reduced to module declarations and re-exports.
- **CLI command journal:** `cli-anything-eep history` lists the commands the harness executed, stored as JSON lines in the XDG state directory.
- **Full CLI command surface:** Every command id in the engine registry now has a named CLI verb, including timeline split/ripple/slip/slide/roll/rate-stretch, transitions, marks and titles, clip effects/keyframes/motion blur/motion tracking/speed ramps/color wheels/curves/HSL/shader, media bins, project relink, subtitles, node graph, multicam, EDL/FCPXML/OTIO/Premiere import, collaboration locks, speech edits, and bus routing.
- **Managed control daemon:** `daemon start` launches `saturn-control`, records pid/port/log in the XDG state directory, `daemon status` probes `/health`, and `daemon stop` terminates only the recorded process.
- **Real MCP introspection:** `mcp tools`, `mcp resources`, and `mcp prompts` perform an MCP initialize handshake against `saturn-mcp` over stdin/stdout; `mcp start` runs the server in the foreground for a client.
- **Graphics template CLI:** `saturn-cli template create|list|inspect|export` reads and writes the `.sgt` format through `saturn-graphics`, and the harness exposes the same operations as `cli-anything-eep template ...`.
- **Sequence marks:** `timeline.set_mark_in` and `timeline.set_mark_out` are implemented in the engine as undoable sequence state, alongside `timeline.clear_marks`.

### Changed
- **App icon:** Replaced the old camera icon with the EEP hummingbird mark
  (`assets/icons/eep-icon.png`); the full Tauri desktop icon set (32–512 px,
  `.icns`, `.ico`) is generated from it and the header/welcome logos now use
  the same mark.
- **Title bar:** The app-name text next to the header logo is gone — the
  header leads with the brand mark alone (the window/Native title is
  unchanged).
- **CLI undo/redo:** `cli-anything-eep undo` and `redo` now delegate straight to the engine's `edit.undo`/`edit.redo` commands; the duplicate in-process undo manager and its fake inverse replay are gone.
- **Workspace membership:** `saturn-control`, `saturn-graphics`, and `saturn-project` are compiled and tested by `cargo test --workspace` again.
- **Python packaging:** The CLI harness now uses one `pyproject.toml` with a `[project]` table and the correct GPL-3.0-or-later license; `setup.py` and `requirements.txt` are removed, and unused `httpx`/`pydantic` dependencies are dropped.
- **CI workflow:** Renamed `.github/workflows/rust.yml` to `ci.yml`; its Python job now builds `saturn-cli`, `saturn-mcp`, and `saturn-control` before running the harness suite.
- **Warning cleanup:** Removed unused imports and variables across the workspace, replaced deprecated `IndexMap::remove` calls with `shift_remove`, and fixed an unreachable modifier pattern in shortcut parsing. `cargo check --workspace --all-targets` is warning-free.
- Expanded media/project IPC and path validation support, and updated editor smoke coverage for audio and export workspace interactions.
- Formatted and aligned Rust command, timeline, render, and project schema modules with current workspace conventions.

## [0.1.0-alpha.1] - 2026-10-23

First public alpha pre-release of Easy Edits Pro.

### Added

- A Linux Tauri desktop editor with project creation, persistence, recovery,
  media import, multi-track editing surfaces, and Rust-backed undo/redo.
- Media, Edit, Color, Fairlight, Keyframes, Fusion, and Deliver workspaces.
- GStreamer MP4/WebM rendering controls with software-encoder fallback.
- saturn-mcp, saturn-cli, and saturn-control integration with the shared Rust
  command engine.
- GitHub prerelease automation that published .deb, .rpm, and SHA-256 assets.
- Local media-server, path, and network-import hardening.

### Known limitations

- Software x264 output is the verified render path on the development laptop.
  Encoder detection exists, but hardware-encoded output is not verified.
- The published packages have not yet been recorded as installed and launched.
- The Unix desktop MCP bridge has no Windows implementation.
- Advanced Color, Fusion, effects, keyframe, export-queue, Fairlight DSP, and
  VST3-hosting work remains incomplete. See docs/planning.md for the active
  priorities and evidence required before a feature is called complete.
