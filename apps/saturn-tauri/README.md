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

The UI supports New/Open/Save project workflows, native multi-file import, GStreamer metadata discovery, a searchable media bin, source and program previews, timeline clip add/move/remove, fullscreen and close, and Rust-backed undo/redo. Color controls, per-track gain, and clip keyframes persist in schema-versioned project files and affect the webview program preview. Native GStreamer export from the Export workspace produces MP4 or WebM and applies basic clip color controls and track gain; keyframe transforms are preview-only. Full audio mixing, additional asset providers, timeline-aware AI actions, and verified Linux release packages remain future work.

## Interior components

Interior is a source-copy React micro-interaction library used for the workspace segmented control and Openverse loading button. Components live in `ui/src/components/interior/`, use the app's CSS rather than Tailwind, and use the `motion` package for animation. For future additions, copy only suitable components into that directory and preserve the upstream MIT license and attribution in [ui/src/components/interior/LICENSE](ui/src/components/interior/LICENSE). See [interior.dev](https://www.interior.dev/), its [segmented control](https://www.interior.dev/docs/segmented-control), and its [loading button](https://www.interior.dev/docs/loading-button).

Run Rust core workflow tests and build the Tauri UI with:

```sh
cargo test -p saturn-core
pnpm build
```
