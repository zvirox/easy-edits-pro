# Desktop architecture

## Decision

Easy Edits Pro uses Tauri 2 with React, TypeScript, and Vite as its only desktop application frontend. The standalone GTK4 frontend and egui prototype have been removed. Rust remains responsible for shared project state, commands, media discovery, timeline time, serialization, and rendering.

Tauri uses the operating system's webview instead of shipping a separate browser runtime. On Linux, the Tauri webview depends on WebKitGTK and its GTK runtime libraries; those are platform dependencies of Tauri, not a second application UI.

## Architecture

- `apps/saturn-tauri` owns the desktop window, React UI, platform dialogs, and command bridge.
- `crates/saturn-core` owns project data, command dispatch, timeline editing, and project-file persistence.
- `crates/saturn-render` owns the native GStreamer Editing Services renderer.
- `crates/saturn-cli` exposes headless project and command workflows.

The Tauri bridge calls the shared core for project status, creation, open/save, media registration, timeline clip add/move/remove, and undo/redo. GStreamer discovers media metadata on a blocking worker. Tauri's scoped asset protocol exposes selected files to source and program previews. Timeline moves snap to project frame boundaries and reject incompatible audio/video track placements.

Project schema v3 includes per-clip exposure/contrast/saturation and keyframe data, plus per-track gain. Tauri routes these through undoable Rust commands. The webview preview applies color filters, track attenuation, and linear keyframe interpolation for position, scale, rotation, and opacity.

## Remaining work

Playback and effects are still webview-based rather than a frame-accurate Rust playback/render engine. Full audio mixing, broader asset-provider APIs, timeline-aware AI actions, and Linux release packaging need further work and verification.
