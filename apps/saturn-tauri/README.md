# Easy Edits Pro · Tauri app

This is the app's desktop frontend: Tauri 2 with React, TypeScript, and Vite. Its Rust command bridge reuses the framework-independent `saturn-core`.

## Development

Install the Tauri 2 Linux system dependencies, including WebKitGTK 4.1, Rust, Node.js, and pnpm. See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

From this directory:

```sh
pnpm install
pnpm tauri dev
```

Build the frontend assets and type-check them with:

```sh
pnpm build
```

Run the Playwright editor smoke test with:

```sh
pnpm exec playwright install chromium # once, if the Playwright browser is not installed
pnpm test:e2e
```

The smoke test launches the Vite UI with a small mocked Tauri IPC layer and a synthetic video fixture. It checks welcome-screen creation, media-bin pointer drop, playhead scrubbing, preview play/pause, timeline zoom, and clip movement. It does not replace a native `pnpm tauri dev` check of the Rust command bridge, GStreamer, file dialogs, or export.

The app starts at a project welcome screen with options to create a project, open an existing `.saturn` file, or reopen a recent project. New projects support a name, resolution preset or custom dimensions, and frame rate. Recent-project recovery can restore an interrupted editing session. In the editor, the UI supports native multi-file import, GStreamer metadata discovery, a searchable media bin, source and program previews, timeline clip add/move/split/remove, fullscreen and close, and Rust-backed undo/redo. Drag media from the Project panel onto a matching timeline track to place it at the drop time; drag the orange playhead or scrub the ruler to seek the program preview. Timeline clip movement, zoom, and frame snapping are supported. The program preview supports play, pause, seeking, and previous/next frame stepping. Color controls, per-track gain, and clip keyframes persist in schema-versioned project files and affect the webview program preview. Native GStreamer export from the Export workspace produces MP4 or WebM and applies basic clip color controls and track gain; keyframe transforms are preview-only.

## Current limitations and upcoming features

Color grading applies to the whole clip; keyframes currently animate position, scale, rotation, and opacity in the preview only. The project has no editable text/title clips or clip transition system yet. Planned work includes editable title clips, common transitions with preview and export support, an extensible effects stack with keyframeable parameters, more frame-accurate playback/rendering, full audio mixing, additional asset providers, timeline-aware AI actions, and verified Linux packages.

## Interior components

Interior is a source-copy React micro-interaction library used for workspace selection and loading buttons on the welcome screen and in Openverse search. Components live in `ui/src/components/interior/`, use the app's CSS rather than Tailwind, and use the `motion` package for animation. For future additions, copy only suitable components into that directory and preserve the upstream MIT license and attribution in [ui/src/components/interior/LICENSE](ui/src/components/interior/LICENSE). See [interior.dev](https://www.interior.dev/), its [segmented control](https://www.interior.dev/docs/segmented-control), and its [loading button](https://www.interior.dev/docs/loading-button).

Run Rust core workflow tests and build the Tauri UI with:

```sh
cargo test -p saturn-core
pnpm build
```
