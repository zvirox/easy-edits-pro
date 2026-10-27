# Architecture

## Runtime data flow

```text
+----------------------------------------------------------------------------------------------------+
|                                         React 19 Frontend Shell                                    |
|                                                                                                    |
|  [App.tsx] (Thin Shell: < 300 lines)                                                               |
|     |                                                                                              |
|     +---> [app/] Providers & Hooks (ProjectProvider, usePlayback, useJobs, useProjectOperations)  |
|     |                                                                                              |
|     +---> [features/] Modular UI Components                                                        |
|     |        |-- menubar/     (AppHeader, 2-tier NLE top bar, window controls)                     |
|     |        |-- media-pool/  (MediaPanel, asset bins, Openverse stock, AI)                        |
|     |        |-- preview/     (PreviewSection, Program/Source monitors, multicam)                  |
|     |        |-- timeline/    (TimelineSection, tracks, tools, playhead, scrubbing)                |
|     |        |-- inspector/   (InspectorPanel, effects, keyframes, subtitles, audio mixer)         |
|     |        `-- dialogs/     (EditorModals, new project, interop, AI transcribe)                 |
|     |                                                                                              |
|     `---> [engine/playback/] Frontend playback scheduling, audio clock, visual effects            |
+----------------------------------------------------------------------------------------------------+
                                      |
                                      |  Tauri invoke("...")
                                      v
+----------------------------------------------------------------------------------------------------+
|                                         Tauri Backend Bridge                                       |
|                                                                                                    |
|  [apps/saturn-tauri/src/main.rs] + [commands/*]                                                    |
|     |-- project.rs    (project document CRUD, mutations, autosave)                                |
|     |-- timeline.rs   (track management, clip operations, ripple edits)                            |
|     |-- media.rs      (import validation, binning, local streaming)                                |
|     |-- playback.rs   (render preview control, transport sync)                                     |
|     |-- export.rs     (render job dispatch, hardware encoder detection)                            |
|     `-- interop.rs    (EDL, FCPXML, Premiere XML import/export)                                    |
|                                                                                                    |
|  [media_server.rs]    Token-authenticated local HTTP streaming server                              |
|  [net.rs]             DNS pinning, SSRF prevention, IP validation                                  |
|  [recovery.rs]        Crash recovery and periodic autosave storage                                 |
+----------------------------------------------------------------------------------------------------+
                                      |
                                      |  Internal Rust API
                                      v
+----------------------------------------------------------------------------------------------------+
|                                         saturn-core Engine                                         |
|                                                                                                    |
|  - Rational tick timebase (TICKS_PER_SECOND = 254,016,000,000)                                     |
|  - Transactional project mutations & bidirectional undo/redo history stacks                        |
|  - Schema migrations (v1 -> v14) & atomic disk persistence                                         |
|  - Subtitles, silence detection, speech analysis, and timeline math                                |
+----------------------------------------------------------------------------------------------------+
                                      |
                                      |  Pipeline graph & render jobs
                                      v
+----------------------------------------------------------------------------------------------------+
|                                         saturn-render Engine                                       |
|                                                                                                    |
|  - GStreamer pipeline construction & hardware-accelerated video decoding/encoding                  |
|  - wgpu preview presentation, color wheels, 3D LUT sampling, RGB curve splines                     |
|  - EBU R128 loudness metering, optical flow speed ramping, and motion tracking                     |
+----------------------------------------------------------------------------------------------------+
```

A project mutation flows from a UI feature through the stable Tauri invoke
contract into the domain command dispatcher in `saturn-core`. The core updates
the versioned project model and history; the UI refreshes project state through
`ProjectProvider`. Preview playback resolves allow-listed media through the
local media server and coordinates video decks and audio clocks in the frontend
playback engine. Export work is sent to `saturn-render` through Tauri commands.

## Module map

- `apps/saturn-tauri/src/main.rs` owns the Tauri builder, invoke registration,
  setup, and shared app-level helpers.
- `apps/saturn-tauri/src/commands/` groups Tauri commands by domain (`project.rs`,
  `timeline.rs`, `media.rs`, `playback.rs`, `export.rs`, `interop.rs`, `ai.rs`).
  `mod.rs` re-exports the commands registered with `generate_handler!`.
- `apps/saturn-tauri/src/state.rs` owns shared application state (`AppState`).
- `apps/saturn-tauri/src/net.rs` resolves public network targets, rejects
  non-public addresses, pins DNS results, and re-validates manual redirects.
- `apps/saturn-tauri/src/media_server.rs` streams only canonical, allow-listed
  media paths and authenticates requests with a per-run token.
- `apps/saturn-tauri/src/recovery.rs` stores and restores project autosaves.
  `register_project_media` is the shared allow-list registration path used by
  project changes, startup, and recovery.
- `apps/saturn-tauri/src/mcp_bridge.rs` serves the same-user local Unix-socket
  bridge (`doc_inspect`, `command_run`, `render_start`) used by agents and the
  packaged-build acceptance runbook to drive the live desktop editor.
- `apps/saturn-tauri/src/gpu_detect.rs` probes the GPU and enables WebKit
  DMA-BUF rendering by default (`EEP_DISABLE_DMABUF` is the escape hatch).
- `crates/saturn-core/src/project/` separates schema domains, migrations, and
  project I/O. `crates/saturn-core/src/engine/` separates command domains,
  helpers, parameter types, history, and tests. Keep project serialization and
  public command names stable when extending these modules.
- `crates/saturn-render/` owns media processing and rendering.
- `apps/saturn-tauri/ui/src/app/` owns project context and focused hooks:
  `ProjectProvider.tsx`, `useProject.ts`, `usePlayback.ts`, `useJobs.ts`,
  `useRecovery.ts`, `useShortcuts.ts`, `useTimelineOperations.ts`,
  `useMediaOperations.ts`, `useInspectorOperations.ts`, and `useProjectOperations.ts`.
- `apps/saturn-tauri/ui/src/features/` provides modular UI components:
  - `menubar/`: 2-tier NLE top bar (`AppHeader.tsx`), menus, and workspace switcher.
  - `media-pool/`: Asset management bin (`MediaPanel.tsx`), stock search, and AI assistant.
  - `preview/`: Program/Source monitors (`PreviewSection.tsx`), multicam switcher, audio visualizer.
  - `timeline/`: Multi-track timeline (`TimelineSection.tsx`, `TimelineToolbar.tsx`, `TimelineTrackRow.tsx`).
  - `inspector/`: Property inspection (`InspectorPanel.tsx`, `ClipInspector.tsx`, `EffectsInspector.tsx`, `KeyframesInspector.tsx`, `SubtitleInspector.tsx`) and docked `AudioMonitorMixer`.
  - `dialogs/`: Modal dialogs (`EditorModals.tsx`, `NewProjectModal.tsx`, `TranscriptionModal.tsx`, `SilenceRemovalModal.tsx`, etc.).
- `apps/saturn-tauri/ui/src/workspaces/` routes stage/workspace views (`WorkspaceRouter.tsx`, `EditWorkspace.tsx`, `MediaWorkspace.tsx`, `ColorWorkspace.tsx`, `FairlightWorkspace.tsx`, etc.).
- `apps/saturn-tauri/ui/src/engine/playback/` owns the audio clock, media URL
  resolution, scheduling, interpolation, effects, and playback types. The
  legacy `playbackEngine.ts` path re-exports this public API.
- `apps/saturn-tauri/ui/src/style.css` imports the ordered stylesheet sections
  through `styles/index.css`. Preserve that order to keep the existing cascade.

## App extraction status

The former App.tsx monolith has been extracted into a thin shell, focused app
hooks, feature components, workspaces, and playback modules. Keep the shell
focused on providers and routing; put new behaviour in the owning feature or
Rust command domain. File-size enforcement, rather than this historical line
count, is the continuing guardrail.

## Source size guardrails

All `.rs`, `.ts`, `.tsx`, and `.css` source files must stay at or below **800
lines**. Run `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`
before submitting.

## Adding a Tauri feature

Put the command in the relevant `commands/<domain>.rs` module, re-export it from
`commands/mod.rs`, and register it in `generate_handler!` in `main.rs`. Keep
project mutations in `saturn-core`; the Tauri command should adapt inputs and
outputs and handle app-specific services. Preserve existing `invoke("...")`
command strings and project file compatibility. Add pure helper tests next to
the module that owns the helper, then run Rust tests, the UI build and tests, and
the source-size check.
