# Easy Edits Pro

Easy Edits Pro is an open-source Linux desktop video editor in early development. The app was previously called Project Saturn; its Rust crates, repository URL, and existing `.saturn` project format retain those internal identifiers for compatibility.

The Tauri desktop UI uses React, TypeScript, and Vite in `apps/saturn-tauri`. The Rust workspace contains the framework-independent project core, CLI, and GStreamer renderer; the core provides project commands, exact rational frame timing, versioned `.saturn` project files, atomic saves, and undo/redo.

## Build and run

### Tauri desktop UI

Install the Tauri 2 [Linux prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, Node.js, pnpm, and GStreamer Editing Services development/runtime packages with the H.264/AAC and VP8/Vorbis plugins. From the repository root, run:

```sh
cd apps/saturn-tauri
pnpm install
pnpm tauri dev
```

On launch, the Tauri interface opens a project welcome screen where you can create a project with a name, resolution, and frame rate, reopen a recent project, or browse for an existing `.saturn` project. The editor provides native multi-file media import, GStreamer metadata discovery, a searchable media bin, source and program previews, and shared Rust undo/redo. Drag media from the Project panel onto a matching timeline track to place it at that time; the orange playhead can be dragged to scrub the timeline and preview the active clip. Timeline editing includes clip move, split, remove, zoom, frame snapping, and keyframe tools. The program preview supports play/pause, seeking, and frame stepping. The native renderer uses GStreamer Editing Services to export MP4 (H.264/AAC) or WebM (VP8/Vorbis) using the project resolution and frame rate. It applies clip color controls and track gain; animated keyframes are currently preview-only. Export progress appears in the Export workspace.

The Asset Library searches Openverse for openly licensed images and audio, downloads selected media into Easy Edits Pro's application data folder, and saves a neighboring attribution JSON file. Review each source and license before distributing a project. The AI Assistant can send prompts to a local Ollama instance at `http://127.0.0.1:11434`; install Ollama and pull a model such as `llama3.2:3b` first. Prompts and responses stay on the local machine.

The interface bundles the Inter Variable font for consistent typography. Inter is licensed under the SIL Open Font License; see [`assets/fonts/OFL.txt`](assets/fonts/OFL.txt).

### Linux packages

The Tauri bundler is configured for Debian (`.deb`) and RPM (`.rpm`) packages. On Linux, build them with:

```sh
cd apps/saturn-tauri
pnpm install
pnpm tauri build
```

The package build needs the Tauri/WebKitGTK build prerequisites, GStreamer Editing Services development files, GStreamer runtime plugins, and the Debian/RPM packaging tools. The `Linux packages` GitHub Actions workflow builds both package formats on manual dispatch or a `v*` tag. Package dependencies declare GStreamer and Tauri runtime libraries; the encoder/decoder plugin packages are included as dependencies as well.

Headless CLI examples:

```sh
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- new ~/Videos/first-edit.saturn "First edit"
cargo run -p saturn-cli -- inspect ~/Videos/first-edit.saturn
cargo run -p saturn-cli -- add-media ~/Videos/first-edit.saturn ~/Videos/shot.mp4
```

The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current progress

- Easy Edits Pro Tauri 2 React/TypeScript/Vite desktop shell with responsive editing workspaces and bundled Inter Variable typography
- Rust Tauri command bridge backed by `saturn-core`, including project status and timeline editing commands
- Native project and media dialogs, asynchronous GStreamer metadata discovery, and asset-protocol media access
- Project welcome screen with new/open/recent project workflows, resolution and frame-rate settings, and interrupted-project recovery
- Tauri source and program monitors, media-bin preview, timeline clip operations, and undo/redo
- Draggable orange playhead for timeline scrubbing, with the program monitor following the active timeline clip
- Pointer drag from the Project media panel to a matching video/audio track, placing clips at the drop time with frame snapping
- Program preview transport with play/pause, seeking, and frame stepping; timeline clip movement, split, remove, and zoom
- Framework-independent core with stable command IDs, enabled-state reasons, and undo/redo
- Exact integer timeline ticks with rational frame and sample-rate conversion
- Versioned project files with migration, external media references, and atomic saves
- JSON Lines control mode for agent and automation clients: `cargo run -p saturn-cli -- control`
- Edit, Color, Audio, Keyframes, and Export workspaces; Color, Audio, and Keyframes controls persist and affect Tauri preview
- Native GStreamer Editing Services rendering from the Export workspace to MP4 or WebM, with progress and error reporting
- Openverse image/audio search and download with license/creator/source attribution sidecar files
- Local Ollama prompt integration, opt-in through the AI Assistant workspace
- Debian and RPM package configuration plus a GitHub Actions build workflow

Color grading currently adjusts exposure, contrast, and saturation for a whole clip. Audio controls save per-track gain and apply attenuation during playback. Keyframes can animate position, scale, rotation, and opacity in the preview, but those transforms are not rendered in exports. Track gain and basic clip color grading are rendered. See [the control protocol](docs/control-protocol.md) for the shared command interface.

## Upcoming features

The editor is still in active development. The next major editing features are:

- Text/title clips with editable content, typography, placement, and duration
- Clip transitions, starting with common fades and crossfades, with matching preview and export behavior
- A broader effect stack, including effect parameters that can be animated at the playhead and rendered
- More reliable frame-accurate playback and preview/render consistency
- Full audio mixing, additional asset providers, and timeline-aware AI editing actions
- Verified Linux release packages and broader platform/runtime testing
