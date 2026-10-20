# FilmCraft parity roadmap

This roadmap uses the supplied FilmCraft source archive as a reference for
feature boundaries and workflow design. FilmCraft is a source-code reference;
there is no FilmCraft application installed on this machine for live runtime
comparison. Easy Edits Pro remains its own implementation and project format.

FilmCraft's automation documentation describes a useful end-to-end pattern:
discover engine commands, inspect project/sequence state, run single or batched
commands, and connect automation to the running application. Its own parity
notes also identify unfinished areas such as docking, timeline editing details,
codec/hardware support, and real-media validation. Those notes are reference
material, not proof that each feature works in EEP.

## Current foundation

- `saturn-core` already owns project state and command dispatch for project,
  media, timeline, effects, audio, color, keyframes, subtitles, and interop.
- `saturn-mcp` is enabled in the Cargo workspace and now wraps that engine.
- MCP exposes command discovery/execution/batches, project and sequence
  inspection, media search/asset placement inspection, and asynchronous render
  start/status/cancel tools.
- The MCP process can open a project file with `--project`, or connect to the
  project open in the desktop app with `--bridge` over a private per-user Unix
  socket. The desktop bridge shares the app command engine and refreshes the UI.
- MCP can render the active sequence to MP4/WebM through the existing GStreamer
  render pipeline.
- Native Tauri runtime workflow completed through the app bridge using a real
  recording from `~/Videos/Recordings`: created/saved/reopened an `.eep` project,
  imported video/audio, split and transitioned clips, applied Color, Effects,
  Keyframes, Fusion node, and Fairlight edits, searched/inspected media, cancelled
  one render, and completed another. The 320x180 MP4 was verified as H.264/AAC
  with `ffprobe`.
- The Fairlight workspace now uses real project commands for track DSP,
  mute/solo, route, automation mode, plugin slot metadata, and bus gain. Browser
  playback meters and loudness values are connected to the Web Audio meter.
- Read-mode Fairlight volume/pan automation curves now interpolate against the
  playhead during browser playback. Plugin slots remain metadata only (no VST3
  host), denoise remains a high-pass filter, and GStreamer render DSP has not
  been validated against every browser playback effect.
- The native MP4 test requested hardware encoding, but the produced stream is
  tagged `x264`; this host exposes only the OpenH264 GStreamer encoder, so
  hardware output remains unverified and the software fallback was used.
- Fusion node commands were exercised, but the Fusion workspace still has
  unconnected actions. Advanced Color controls, effect reorder/parameter UI,
  keyframe handle editing, and the export queue UI remain incomplete.
- Existing Tauri panels/workspaces and the split playback/timeline modules are
  retained. This increment did not rewrite those UI systems.

## Planned increments

| Stage | Work | Completion criteria |
|---|---|---|
| 1. Engine-backed MCP | Completed foundation: replace the disconnected registry with `EditorEngine`, expose all catalogued commands through `command_list` and `command_run`, support batches and project inspection. | MCP crate compiles as part of the workspace; tool results reflect the real EEP engine. |
| 2. Desktop bridge | Implemented for Unix desktop builds and smoke-tested against the running Tauri app: user-owned socket, bounded requests, shared command engine, UI refresh, and a client reconnect test across a server restart. | Windows bridge support and live Tauri app restart coverage remain open. |
| 3. Timeline workflow | Add higher-level timeline inspect/analysis helpers and map common trim, split, move, ripple, transition, marker, and keyframe flows to existing commands. | Tools discover valid ids/params and report clip/track/time units; operations preserve undo and project invariants. |
| 4. Media and preview | Search project media and inspect an asset with its timeline placements/source path. | Still-frame extraction and client-displayable frame payload remain outstanding. |
| 5. Render and export | MCP starts asynchronous jobs, reports queued/running/terminal state and increasing render percentages, and cancels active or queued jobs; cancellation is checked during rendering and partial output is removed. Native render completion, progress, and cancellation were exercised. | Add presets and broaden codec/hardware tests. |
| 6. UI clarity pass | Audit the main editor with the existing UI component and workspace structure. Reduce duplicate actions, surface tool names and state, group timeline operations by task, and clarify export configuration. | A short list of high-friction changes is prioritized from screenshots and interactions; each change is localized and keyboard/accessibility checked. |
| 7. Parity hardening | Work through source-supported workflows and known gaps: drag/drop, playhead preview, transforms/keyframes, audio routing/mixing, titles, transitions, codec corpus, and import/export interchange. | Each workflow works on representative real media and has focused regression coverage before calling it complete. |

## Immediate next step

Extend Stage 4 with direct media import and still-frame extraction. Add UI
automation and extend the codec/hardware test matrix. The opt-in desktop bridge
currently uses a private Unix socket; Windows support remains a platform task.

## Limits and sequencing

“Full FilmCraft functionality” spans the UI, engine, media stack, codec/render
pipeline, automation, and file formats. It is not one safe patch. Each stage
should preserve existing EEP project files and public Tauri command names. Do
not claim parity for a workflow until it works end to end with real media.
