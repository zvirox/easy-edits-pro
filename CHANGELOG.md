# Changelog

All notable changes to **Easy Edits Pro** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
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
