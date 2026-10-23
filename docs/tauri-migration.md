# Desktop architecture

## Decision

Easy Edits Pro uses Tauri 2 with React, TypeScript, and Vite as its only desktop application frontend. The standalone GTK4 frontend and egui prototype have been removed. Rust remains responsible for shared project state, commands, media discovery, timeline time, serialization, and rendering.

Tauri uses the operating system's webview instead of shipping a separate browser runtime. On Linux, the Tauri webview depends on WebKitGTK and its GTK runtime libraries; those are platform dependencies of Tauri, not a second application UI.

## Architecture

- `apps/saturn-tauri` owns the desktop window, React UI, platform dialogs, and command bridge.
- `crates/saturn-core` owns project data, command dispatch, timeline editing, and project-file persistence.
- `crates/saturn-render` owns the native GStreamer Editing Services renderer.
- `crates/saturn-cli` exposes headless project and command workflows.

The Tauri bridge calls the shared core for project status, creation, open/save,
media registration, timeline edits, and undo/redo. GStreamer discovers media
metadata on a blocking worker. Tauri's scoped asset protocol exposes selected
files to source and program previews. Timeline moves snap to project frame
boundaries and reject incompatible audio/video track placements.

The current project format is `easy-edits-pro.project`, schema version 14, with
migrations from supported earlier versions and the legacy
`project-saturn.project` identifier. Tauri routes edits through undoable Rust
commands. The webview preview handles interactive playback and preview effects;
the GStreamer renderer is a separate export path, so preview behaviour does not
by itself prove export parity.

## Remaining work

Advanced Color and effect controls, full Fusion actions, Fairlight insert DSP,
the export queue UI, and hardware-encoded output verification remain open. The
Unix MCP bridge has no Windows implementation. Alpha packages have been
published, but install-and-launch evidence is not recorded. The active work
sequence and current build evidence are maintained in [planning.md](planning.md).
