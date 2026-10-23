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

The smoke test launches the Vite UI with a small mocked Tauri IPC layer and a synthetic video fixture. It checks welcome-screen creation, media-bin pointer drop, playhead scrubbing, preview play/pause, timeline zoom, and clip movement. It does not replace a native `pnpm tauri dev` check of the Rust command bridge, GStreamer decoding/transcoding, file dialogs, or export.

The app starts at a project welcome screen with options to create a project, open an existing `.saturn` file, or reopen a recent project. New projects support a name, resolution preset or custom dimensions, and frame rate. Recent-project recovery can restore an interrupted editing session. The editor's primary navigation separates Import, Edit, and Export so users can find media intake, editing tools, and delivery settings directly. Import includes search, project media cards, and browse actions. Edit keeps the source/program monitors, timeline, and labeled timeline tools, alongside the Color, Audio, Keyframes, Asset Library, and AI Assistant workspaces. The Export workspace includes output name/location, MP4/H.264 or WebM/VP8 format, frame-size and frame-rate presets, hardware encoder preference, marked In/Out range, and a file summary.

In Edit, the UI supports local-media import, a searchable media bin, source and
program preview controls, timeline placement/move/split/remove, transitions,
graphic clips, timeline zoom, scrubbing, and Rust-backed undo/redo. Drag media
from the Project panel onto a matching timeline track to place it at the drop
time; drag the playhead or scrub the ruler to seek. The UI uses the shared dark
burgundy/deep-purple token palette and the Interior-style button primitives.

GStreamer renders MP4/H.264 + AAC or WebM/VP8 + Vorbis. MP4 export can prefer
an installed hardware encoder and falls back to a software encoder. On the
development laptop, software x264 output is the verified path; hardware output
is detected but remains unverified. Some transforms and creative controls are
preview-only or incomplete; see the root README and delivery plan before
describing them as export-capable.

## Current limitations and upcoming features

The editor is alpha software. The real export queue, advanced effect and Color
parameter editing, keyframe handles, full Fusion actions, VST3 hosting, and
hardware-output verification remain unfinished. Packages have been built but
not yet recorded as installed and launched. The current priorities and evidence
requirements are in [../../docs/planning.md](../../docs/planning.md).

## Interior components

Interior is a source-copy React micro-interaction library used for workspace selection and loading buttons on the welcome screen and in Openverse search. Components live in `ui/src/components/interior/`, use the app's CSS rather than Tailwind, and use the `motion` package for animation. For future additions, copy only suitable components into that directory and preserve the upstream MIT license and attribution in [ui/src/components/interior/LICENSE](ui/src/components/interior/LICENSE). See [interior.dev](https://www.interior.dev/), its [segmented control](https://www.interior.dev/docs/segmented-control), and its [loading button](https://www.interior.dev/docs/loading-button).

Linux package builds include GStreamer base/good/ugly, libav, and bad plugin sets for common media decoding. Hardware H.264 export also needs a supported GPU driver and encoder plugin such as VA-API, Intel Quick Sync, or NVIDIA NVENC. If none is available, MP4 export uses software encoding.

Run Rust core workflow tests and build the Tauri UI with:

```sh
cargo test -p saturn-core
pnpm build
```
