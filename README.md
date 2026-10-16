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

On launch, the Tauri interface opens a project welcome screen where you can create a project with a name, resolution, and frame rate, reopen a recent project, or browse for an existing `.saturn` project. Inside the editor, clearly labeled Import, Edit, and Export workspaces organize media intake, timeline editing, and delivery settings. Import provides searchable project media cards and a direct browse action; Edit retains the timeline, labeled tools, monitors, and existing editing workspaces. Export provides format, frame-size and frame-rate presets, an output location, marked-range export, and a summary of the resulting file. The editor provides native multi-file media import, GStreamer metadata discovery, a searchable media bin, source and program previews, and shared Rust undo/redo. Video previews are decoded by GStreamer into cached WebM/VP8 proxies, so playback does not rely on WebKit having native MP4 codec support; the original file remains the source for editing and export. Drag media from the Project panel onto a matching timeline track to place it at that time; the orange playhead can be dragged to scrub the timeline and preview the active clip. Timeline editing includes clip move, split, remove, zoom, frame snapping, transitions, title graphics, and keyframe tools. The program preview supports play/pause, seeking, and frame stepping. The native renderer uses GStreamer Editing Services to export MP4 (H.264/AAC) or WebM (VP8/Vorbis) with selectable output size and frame rate, supporting transitions, title clips, color controls, and track gain. MP4 export can prefer an installed VA-API, Intel Quick Sync, NVIDIA NVENC, or V4L2 H.264 encoder, with GStreamer software encoders used when hardware encoding is unavailable. Export progress appears in the Export workspace.

The UI is custom-styled to fit modern Linux desktop rices (Hyprland / Niri), featuring an obsidian dark canvas (`#080a0f`), 1px subtle glowing borders, electric cyan and neon violet accents, and tactile micro-interactions on button clicks. Audio clips on the timeline display real-time mirrored SVG waveforms generated from decoded peak samples. Video and audio transitions (`Crossfade`, `Dip to Black`, `Dip to White`, `Wipe`, `Slide`) can be inserted at any cut point (`Ctrl+D`) and configured via a dedicated Transition Inspector. Title and lower-third graphic clips (`Ctrl+T`) render live glassmorphic graphics on top of the program monitor and in final exports.

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
- Modern Linux desktop rice identity styled for Hyprland / Niri (obsidian dark canvas, subtle 1px border glows, neon cyan/violet accents, tactile button click animations)
- **DaVinci Resolve 2026 Workspace Flow**: Integrated sequential workspace bar (`Media` Shift+1, `Edit` Shift+2, `Color` Shift+3, `Audio` Shift+4, `Effects` Shift+5, `Deliver` Shift+6) unifying asset management, multi-track cutting, color grading, Fairlight audio mixing, keyframing, and delivery
- **Wondershare Filmora 15 Left Asset Panel**: Tabbed category navigation (`Media`, `Titles`, `Transitions`, `Audio`, `Stock`, `AI Tools`) featuring instant 1-click visual preset cards for Title templates (`Broadcast Lower-Third`, `Minimal Pill`, `Cinematic Headline`, `Cyber Neon`, `Documentary Card`, `Framed Banner`) and Transitions (`Crossfade`, `Dip to Black`, `Dip to White`, `Wipe Left/Right`, `Slide Left/Right`)
- **Adobe Premiere Pro Timeline Tool Rail**: Dedicated vertical tool rail beside the timeline with pro NLE tools and single-key shortcuts: Selection Tool (`V`), Track Select Forward (`A`), Ripple Edit (`B`), Razor (`C`), Slip (`Y`), Keyframe Pen (`P`), Hand Pan (`H`), and Zoom (`Z`)
- **Automatic Floating Tooltips**: Tooltips on hover across all tools, workspace buttons, menu items, timeline actions, and transport controls showing tool name and keyboard shortcut
- **Light & Dark Mode Theme Switcher**: Easily accessible header toggle button (`Ctrl+Alt+T`) switching between Obsidian Dark Rice (`#080a0f`) and Pro Studio Clean Slate (`#f1f5f9`), with local storage persistence
- Video and audio transitions (`Crossfade`, `Dip to Black`, `Dip to White`, `Wipe Left/Right`, `Slide Left/Right`) with interactive timeline badge handles, duration adjustment, GStreamer Editing Services (`ges::TransitionClip`) export rendering, and `Ctrl+D` shortcut
- Title and Lower-Third graphic clips (`Ctrl+T`), neon-violet timeline graphic clips, live Program Monitor overlay rendering with glassmorphism backdrop blur (`lower_third`, `center_title`, `minimal_pill`), GStreamer `ges::TitleClip` export rendering, and dedicated Title & Graphic Inspector
- Audio waveform peak visualization decoded via GStreamer audio pipelines and rendered as crisp mirrored SVG waveforms across timeline audio clips
- Pro keyboard shortcuts: `Space` (toggle play/pause), `Ctrl+T` (add title graphic), `Ctrl+D` (add default transition), `Shift+1`..`Shift+6` (workspaces), `V`/`A`/`B`/`C`/`Y`/`P`/`H`/`Z` (tools), `Delete` / `Backspace` (remove selected clip or transition)
- Rust Tauri command bridge backed by `saturn-core`, including project status and timeline editing commands with undo/redo
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
- GStreamer-decoded cached VP8 proxies for reliable video previews across common source codecs
- MP4 H.264 export that prefers supported hardware encoders and falls back to installed software encoders
- Openverse image/audio search and download with license/creator/source attribution sidecar files
- Local Ollama prompt integration, opt-in through the AI Assistant workspace
- Debian and RPM package configuration plus a GitHub Actions build workflow
- Playwright editor smoke test with mocked Tauri IPC for project creation, pointer-based media placement, program seek/playback, timeline zoom, and clip movement (`cd apps/saturn-tauri && pnpm test:e2e`)

Color grading currently adjusts exposure, contrast, and saturation for a whole clip. Audio controls save per-track gain and apply attenuation during playback. Keyframes can animate position, scale, rotation, and opacity in the preview, but those transforms are not rendered in exports. Track gain, basic clip color grading, transitions, and title clips are rendered in export. See [the control protocol](docs/control-protocol.md) for the shared command interface and [the planning roadmap](docs/planning.md) for the complete multi-phase development plan.

## Upcoming features

The editor is still in active development. The next major editing features are:

- A broader effect stack, including effect parameters that can be animated at the playhead and rendered
- Bezier keyframe curve editor for smooth speed ramps and position animations
- Full multi-track audio mixing console with parametric EQ, compressors, and volume meters
- More reliable frame-accurate playback and preview/render consistency
- Queued exports and more detailed codec, bitrate, and audio delivery controls
- Timeline-aware AI editing actions and smart cuts
- Verified Linux release packages and broader platform/runtime testing
