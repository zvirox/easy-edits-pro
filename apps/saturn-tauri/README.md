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

In Edit, the UI supports native multi-file import, GStreamer metadata discovery, a searchable media bin, source and program previews, timeline clip add/move/split/remove, video and audio transitions, title/lower-third graphic clips, real-time SVG audio waveform visualization, fullscreen and close, and Rust-backed undo/redo. Drag media from the Project panel onto a matching timeline track to place it at the drop time; drag the orange playhead or scrub the ruler to seek the program preview. Timeline clip movement, zoom, and frame snapping are supported. The program preview supports play, pause, seeking, and previous/next frame stepping. The UI features a dark obsidian aesthetic (`#080a0f`) tailored for Linux rices (Hyprland / Niri) with glowing borders and tactile button click micro-interactions. Pro keyboard shortcuts include `Space` (play/pause), `Ctrl+T` (insert title graphic), `Ctrl+D` (insert transition), and `Delete` / `Backspace` (remove clip or transition).

Video previews are decoded by GStreamer into cached WebM/VP8 proxies to avoid depending on WebKit's native MP4 codec support; original media remains the render source. Color controls, per-track gain, clip keyframes, transitions, and title clips persist in schema-versioned project files and affect both the webview program preview and GStreamer export. Native GStreamer export produces MP4/H.264 + AAC or WebM/VP8 + Vorbis, allows output dimensions and frame rate to differ from the source sequence, and applies basic clip color controls, track gain, transitions, and title clips; MP4 export prefers an installed VA-API, Quick Sync, NVENC, or V4L2 H.264 encoder, then uses GStreamer software encoders when hardware encoding is unavailable. Keyframe transforms are preview-only.

## Current limitations and upcoming features

Color grading applies to the whole clip; keyframes currently animate position, scale, rotation, and opacity in the preview only. Phase 1 added video and audio transitions (`Crossfade`, `Dip to Black/White`, `Wipe`, `Slide`), title & lower-third graphic clips with live program monitor overlays, and audio waveform peak displays. Planned Phase 2+ work includes an extensible effects stack with keyframeable parameters, Bezier curve editing, multi-track audio mixing console with parametric EQ, queued exports and advanced codec/bitrate/audio controls, more frame-accurate playback/rendering, additional asset providers, timeline-aware AI actions, and verified Linux packages.

## Interior components

Interior is a source-copy React micro-interaction library used for workspace selection and loading buttons on the welcome screen and in Openverse search. Components live in `ui/src/components/interior/`, use the app's CSS rather than Tailwind, and use the `motion` package for animation. For future additions, copy only suitable components into that directory and preserve the upstream MIT license and attribution in [ui/src/components/interior/LICENSE](ui/src/components/interior/LICENSE). See [interior.dev](https://www.interior.dev/), its [segmented control](https://www.interior.dev/docs/segmented-control), and its [loading button](https://www.interior.dev/docs/loading-button).

Linux package builds include GStreamer base/good/ugly, libav, and bad plugin sets for common media decoding. Hardware H.264 export also needs a supported GPU driver and encoder plugin such as VA-API, Intel Quick Sync, or NVIDIA NVENC. If none is available, MP4 export uses software encoding.

Run Rust core workflow tests and build the Tauri UI with:

```sh
cargo test -p saturn-core
pnpm build
```
