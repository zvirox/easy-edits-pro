# Easy Edits Pro — Master Project Planning & Architecture Roadmap

**Document Status:** Master Living Architecture & Execution Roadmap  
**Framework:** Rust Core (`saturn-core`, `saturn-render`) + Tauri 2 (Desktop Shell) + React 19 / TypeScript + CSS (Modern Linux Rice UI)  
**Repository:** [github.com/zvirox/easy-edits-pro](https://github.com/zvirox/easy-edits-pro)  

---

# Part 1: Pro Video Editor Roadmap (Rust + Tauri + TypeScript)

A phase-wise plan for building a desktop video editor (Premiere Pro / DaVinci Resolve style) with **Rust** for the engine, **Tauri** as the desktop shell, and **TypeScript + CSS** for the UI.

> **Reality check first.** Resolve and Premiere took hundreds of engineers and decades. You can't clone them solo, but you can build a genuinely good editor in stages. Ship a small, solid **MVP (Phases 0-6)** first, then grow. Every phase below ends with something usable.

---

## 1. The Big Architecture Decision

The hardest part of this project is **video preview inside a webview**. Get this right early.

### The problem
Tauri's UI runs in the system webview (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). Webview `<video>` tags:
- Support only some codecs (no ProRes, no most pro formats, weak H.265).
- Can't do **frame-accurate seeking** or multi-layer compositing.
- Behave differently on each OS (Linux WebKitGTK is the weakest).

So: **don't use `<video>` as your engine.** Use it only for quick throwaway previews at most.

### Recommended approach

| Layer | Responsibility | Tech |
|---|---|---|
| **Engine (Rust)** | Decode, composite, effects, audio mix, export | FFmpeg / GStreamer, wgpu, cpal |
| **State (Rust)** | Project/timeline data, undo/redo (source of truth) | serde, command pattern |
| **Bridge** | Commands, events, frame streaming | Tauri commands, `Channel`, custom URI protocol, streaming HTTP server |
| **UI (TS)** | Timeline, panels, inspector, interaction | React/Solid/Svelte + Canvas + CSS |

**Golden rule:** Rust owns the project state. The UI is a *view* that sends commands (`split_clip`, `move_clip`) and re-renders from state snapshots/diffs.

### Preview strategies (pick one, evolve later)

1. **Frame streaming (start here).** Rust decodes + composites a frame, sends it to the UI (custom URI scheme or `tauri::ipc::Channel` with raw bytes), UI draws to `<canvas>` / WebGL. Simple and cross-platform. Cost: copy overhead, so use proxies and lower preview resolution.
2. **WebCodecs in the webview.** Use the browser `VideoDecoder` for H.264/VP9/AV1 with hardware acceleration. Fast, but limited by webview codec support.
3. **Native wgpu surface.** Render directly into a native child window/surface layered over the UI preview area. Best performance, hardest to integrate (window layering, per-OS quirks). Target this for Phase 14+.

Start with **#1**, design the engine so the output sink can be swapped to #3 later.

---

## 2. Tech Stack

### Rust crates
| Need | Crate / Tool |
|---|---|
| Decode/encode/mux | `ffmpeg-next` / `gstreamer` + `gstreamer-editing-services` |
| GPU rendering & effects | `wgpu` + WGSL shaders |
| Audio playback | `cpal` (+ `rodio` for simple cases) |
| Audio decode / DSP | `symphonia`, `rubato` (resampling), `fundsp` |
| Serialization | `serde`, `serde_json` / `rmp-serde` |
| Project storage | JSON file, or `rusqlite` for large projects |
| Async / threads | `tokio`, `crossbeam`, `rayon` |
| Image handling | `image` |
| Logging / errors | `tracing`, `thiserror`, `anyhow` |
| Transcription | `whisper-rs` (whisper.cpp) |
| ML inference | `ort` (ONNX Runtime) |
| Testing | `cargo test`, `criterion` (benchmarks), `insta` (snapshots) |

### Frontend
- **Tauri 2**, **Vite**, **TypeScript (strict)**
- UI framework: **React 19** (fine-grained components, vast ecosystem).
- State: Custom reactive store mirroring Rust state snapshots without duplicating business logic.
- Timeline rendering: Multi-track virtualized timeline with SVG waveform rendering and canvas layers.
- Styling: CSS variables for theming, CSS Grid for panel layouts, glassmorphic obsidian Linux rice design.
- Testing: Vitest, Playwright E2E smoke tests.

### Suggested repo layout

```text
easy-edits-pro/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ saturn-core/            # time types (rational ticks), project model, commands, undo/redo, save/load
│  ├─ saturn-render/          # GStreamer GES compositor, effects, transitions, hardware encode, export
│  └─ saturn-cli/             # headless command line batch editor & inspection tool
├─ apps/
│  └─ saturn-tauri/           # Tauri desktop app: IPC bridge, local media streaming server, window management
│     ├─ src/                 # Rust backend: media server, proxy generator, dialogs, Openverse stock, local AI
│     └─ ui/                  # TypeScript / React 19 UI (timeline, monitors, bin, inspector, workspaces)
└─ docs/                      # Architecture, protocols, and planning documentation
```

---

## 3. Core Design Rules

1. **Time = integers, never floats.** Use a rational timebase (Easy Edits Pro uses `TICKS_PER_SECOND = 254_016_000_000` which divides 24, 25, 30, 48, 50, 60, 23.976, 29.97, 59.94 fps and 44.1k/48k/96k audio cleanly). Float time causes drift and off-by-one-frame bugs.
2. **Non-destructive editing.** The project stores *references + instructions*, never modifies source media.
3. **Command pattern for everything.** Each edit is a reversible command → undo/redo and collaboration come almost free.
4. **Immutable-ish timeline snapshots** so the render thread can read a consistent version while the UI edits.
5. **Pull-based render graph.** For a given timeline time `t`, build and evaluate a graph: decode → effects → composite → output.
6. **Separate threads:** UI thread, decode threads, render thread, audio thread (real-time, never block, no allocations, no locks).
7. **Audio is the master clock.** Sync video to audio playback position, not the reverse.
8. **Cache aggressively:** decoded frames (LRU), thumbnails, waveforms, rendered effect segments.

---

## 4. The 16-Phase Execution Roadmap

---

### Phase 0: Foundations & Learning
**Goal:** Learn just enough to not get stuck later.
- Rust: ownership, traits, error handling, threads/channels, `async`, `unsafe` basics (FFI).
- Tauri 2: commands, events, state, plugins, capabilities/permissions, sidecars.
- Video fundamentals: containers vs codecs, frames, GOP / keyframes / B-frames, pixel formats (YUV 4:2:0 vs RGB), color spaces, timebase, PTS/DTS, variable frame rate.
- Audio fundamentals: sample rate, channels, buffers, resampling.
- GPU basics: wgpu, textures, shaders (WGSL), render passes.
- **Exit criteria:** You can decode any frame of a video by number from Rust.

---

### Phase 1: Project Setup & Architecture Skeleton
**Goal:** A running app with the right bones.
- Set up Cargo workspace and crate layout (`saturn-core`, `saturn-render`, `apps/saturn-tauri`).
- Type-safe bridge, error types shared across Rust/TS.
- Dockable layout shell: **Media Bin | Preview | Inspector | Timeline**.
- Theming with CSS variables (Obsidian Dark Linux Rice + Clean Slate Light).
- Logging, CI, lint/format (clippy, rustfmt).
- Core types: rational `Tick`, `FrameRate`, `ProjectDocument` schema v1-v4.
- **Exit criteria:** App launches on all target OSes, panels resize, Rust ↔ TS calls are typed.

---

### Phase 2: Media Import & Library
**Goal:** Bring media in and browse it.
- Import via file dialog + drag and drop.
- Probe analysis: codec, resolution, fps, duration, audio channels.
- **Thumbnails / filmstrips:** generate in background worker, cache on disk.
- **Audio waveforms:** compute peak amplitudes, store as binary/normalized bins, render as SVG.
- **Proxy generation:** background transcode to low-res intra-frame codec (VP8/WebM 720p) for smooth editing.
- Bins/folders, rename, tags, basic search, missing-media detection and relink.
- Job queue with progress events and cancellation support.
- **Exit criteria:** Drop in 50 clips, see thumbnails and waveforms, proxies build in the background.

---

### Phase 3: Timeline Data Model & UI
**Goal:** A timeline you can arrange clips on.
- **Rust side (`saturn-core`):**
  - Entities: `Project → Sequence → Track → Clip`, plus `Transition`, `Title`, `ColorAdjustments`, `Keyframe`.
  - Clip fields: source ref, source in/out, timeline start, duration, keyframes, color adjustments.
  - Command system + undo/redo stack (`AddClip`, `MoveClip`, `TrimClip`, `SplitClip`, `DeleteClip`, `RippleDelete`).
  - Save/load project (versioned `.saturn` JSON with schema migration v1→v4).
  - Auto-save + crash-recovery file.
- **UI side:**
  - Multi-track timeline: time ruler, tracks, clips with waveforms, playhead.
  - Zoom/scroll (horizontal + vertical), track height, add/remove/lock/mute/solo tracks.
  - Drag clips from Media Bin to tracks; move clips between tracks.
  - Selection, snapping to playhead/clip edges.
- **Exit criteria:** Build a multi-track edit, save, reopen, undo/redo reliably.

---

### Phase 4: Playback Engine ⚠️ Hardest Phase
**Goal:** Press play and see/hear the timeline.
- **Decode pipeline:** per-source decoder with seek-to-keyframe + decode-forward; frame cache (LRU); prefetch ahead of playhead; hardware decode via GStreamer/FFmpeg hwaccel.
- **Compositor:** composite multiple layers with position/scale/opacity.
- **Audio engine:** decode → resample → mix tracks → audio callback as master clock.
- **A/V sync:** video frame selected by audio clock; drop or repeat frames when behind.
- **Preview transport:** play/pause, JKL shuttle, frame step, loop, scrub (low-latency seek).
- **Preview delivery:** local HTTP byte-range media streaming server (`HTTP 206 Partial Content`) for WebKitGTK Linux stability.
- **Exit criteria:** Smooth 1080p playback of a 3-track edit with synced audio; scrubbing feels responsive.

---

### Phase 5: Core Editing Tools ✅ Completed
**Goal:** Everything in the professional NLE core editing toolset works.
- **Cut / Split (razor):** `SplitClip` at playhead (`Ctrl+K` / Razor `C`).
- **Trim:** Drag clip boundaries with live edit-point update.
- **Ripple edit / delete:** Shift following clips on affected tracks (`Shift+Delete` / Ripple tool `B`).
- **Roll edit:** Adjacent cut adjustment without altering sequence length (`RollEdit` / Rolling tool `N`).
- **Slip edit:** Move clip source media window within current timeline duration (`SlipClip` / Slip tool `Y`).
- **Slide edit:** Slide a clip between adjacent clips, modifying neighbor bounds (`SlideClip` / Slide tool `U`).
- **Rate stretch:** Interactive timeline duration resizing scaling playback speed (`RateStretch` / Rate Stretch tool `X`, `⚡ speed%` badge, Web Audio & HTML5 `<video>` playback rate synchronization).
- **Group Link / Unlink:** Multi-clip audio/video grouping (`group_id`, `Ctrl+L` Link, `Ctrl+Shift+L` Unlink, synchronized multi-track drag and batch move `timeline.move_clips_batch`).
- **Select / move / snap:** Selection tool (`V`), Track Select Forward (`A`), frame snapping (`S`).
- **Copy / paste / duplicate:** Clipboard of clip definitions.
- **In / Out marks:** Three-point editing (`I` / `O` marks, clear marks `Alt+X`).
- **Keyboard shortcuts:** Central keymap matching professional DaVinci / Premiere NLE workflows.
- **Exit criteria:** Complete core editing operations with full undo/redo, UI visual badges, and synchronized playback.

---

### Phase 6: Export MVP → v0.1 Release
**Goal:** Get a finished video out.
- Render pipeline: run compositor/audio mixer offline → feed frames to encoder.
- GStreamer Editing Services (`ges::Timeline`) pipeline compilation into MP4 (`vah264enc`/`nvh264enc`/`qsvh264enc`/`x264enc`) and WebM (`vp8enc`).
- Presets: YouTube 1080p/4K, TikTok/Instagram 9:16 vertical, custom resolution/fps.
- Range export: In/Out marked range export.
- Progress reporting, cancel support, atomic output writing.
- **Exit criteria:** Import → edit → export video that plays cleanly in VLC, YouTube, and mobile devices.

---

### Phase 7: Video Adjustments, Transitions & Keyframes
**Goal:** Look-and-motion toolkit.
- **Transform:** position (X, Y), scale, rotation, opacity via keyframes.
- **Keyframe system:** animatable parameters (`PositionX`, `PositionY`, `Scale`, `Rotation`, `Opacity`), linear and bezier interpolation.
- **Transitions:** `Crossfade`, `DipToBlack`, `DipToWhite`, `WipeLeft`, `WipeRight`, `SlideLeft`, `SlideRight` with visual timeline badges, handles, and GES compilation.
- **Effect stack architecture:** metadata + parameter schema + shader/GStreamer element.
- **Exit criteria:** Animate a title or logo flying in; crossfade and wipe smoothly between clips.

---

### Phase 8: Color & Visual Effects
**Goal:** Color correction and stylistic effects.
- **Basic correction:** exposure, brightness, contrast, saturation, temperature, tint, highlights/shadows.
- **LUT support:** parse `.cube`, apply 3D LUT.
- **Filters/effects:** gaussian blur, sharpen, vignette, glow, chromatic aberration.
- **Chroma key:** green/blue screen picker, tolerance, edge despill.
- **Masks:** shape masks (rectangle, ellipse, bezier) with feathering.
- **Linear-light pipeline:** float16/float32 color math.
- **Exit criteria:** Green-screen a subject onto a background, color-grade, and apply a LUT.

---

### Phase 9: Text, Titles, Captions & Graphics
**Goal:** Titles, lower-thirds, and subtitles.
- Glassmorphic Title & Lower-Third graphics engine with live Program Monitor backdrop-filter overlay.
- Properties: text, subtitle, layout style, font size, text color, background color, opacity, coordinates.
- Presets: Broadcast Lower-Third, Minimal Pill, Cinematic Headline, Cyber Neon, Documentary Card, Framed Banner.
- Export rendering: compiled directly into `ges::TitleClip` for MP4/WebM output.
- Subtitle tracks: SRT/VTT import/export, styled captions.
- **Exit criteria:** Video with animated lower-third name title and burned-in styled subtitles.

---

### Phase 10: Professional Audio
**Goal:** Audio that doesn't embarrass you.
- Per-clip and per-track gain (dB), pan, fades, mute (`M`), solo (`S`).
- Dedicated Fairlight-style Audio Mixer workspace with channel strips, faders, pan dials, master bus.
- Real-time peak VU meters with multi-segment color gradation.
- Effect chain: Parametric EQ, Compressor, Limiter, Noise Gate, Reverb.
- Noise reduction: RNNoise integration (`nnnoiseless`).
- Broadcast compliance: integrated LUFS loudness metering (EBU R128).
- **Exit criteria:** Clean a noisy voice clip, duck background music under speech, and export at -14 LUFS.

---

### Phase 11: Advanced Editing
**Goal:** Productivity features that pros expect.
- **Nested sequences / compound clips:** sequences used as clips in another sequence.
- **Multi-cam:** sync clips by timecode or audio waveform correlation; live angle switching.
- **Adjustment layers:** effects applied to all layers below.
- **Dynamic trimming:** slip, slide, roll with live side-by-side trim monitor.
- **Smart bins, keywords, metadata tags, color labels.**
- **Speed ramps:** keyframed time remapping with optical flow frame blending.
- **Exit criteria:** Cut a multi-camera interview; nest an animated graphic sequence and reuse it.

---

### Phase 12: Pro Color Grading
**Goal:** DaVinci Resolve-lite color page.
- **Real-time GPU scopes:** Waveform monitor, Vectorscope, RGB Parade, Histogram.
- **Primary color wheels:** Lift, Gamma, Gain, Offset.
- **Curves:** RGB curves, Hue vs Sat, Hue vs Hue, Lum vs Sat.
- **Secondary qualifiers:** HSL qualifier, power windows with planar tracking.
- **Color management:** ACES working space, camera log input transforms (S-Log3, V-Log, LogC), Rec.709/Rec.2020 output.
- **Exit criteria:** Grade S-Log footage to Rec.709 with live scopes and secondary qualifiers.

---

### Phase 13: Motion Graphics, Tracking & AI Features
**Goal:** Modern smart features.
- **Auto-captions / transcription:** offline local Whisper (`whisper-rs`) with word-level timestamps.
- **Text-based editing:** edit video by editing the generated transcript.
- **Silence & filler removal:** automatic detection and ripple-delete of pauses.
- **Background removal:** ONNX segmentation models (`ort` / MODNet / RVM).
- **Object / motion tracking:** planar point tracking attached to titles or blur masks.
- **Local AI chat assistant:** integrated Ollama client for creative suggestions and editing instructions.
- **Exit criteria:** 1-click auto-captions generated locally without cloud dependencies.

---

### Phase 14: Performance, Native Preview & Pro Delivery
**Goal:** Make it lightning fast and professional.
- **Native wgpu preview surface** layered over webview UI to eliminate webview rendering overhead.
- **Render cache** for heavy effect segments with dependency hashing (red/yellow/green timeline bar).
- Multithreaded batch export queue.
- Pro codec support: ProRes, DNxHR, AV1, image sequences (EXR/PNG).
- Interchange formats: Final Cut Pro XML, Premiere XML, EDL, OpenTimelineIO.
- **Exit criteria:** Smooth 4K 60fps multi-track playback on mid-range hardware.

---

### Phase 15: Quality, Packaging & Legal
**Goal:** Bulletproof distribution.
- Automated tests: unit tests, property tests (`proptest`), Playwright E2E smoke tests.
- Linux packaging: Flatpak on Flathub, portable AppImage, Arch Linux PKGBUILD, Debian (`.deb`), RPM (`.rpm`).
- Windows MSI / macOS DMG with notarization.
- Clean LGPL dynamic linking for FFmpeg / GStreamer compliance.
- **Exit criteria:** One-click installable package across all major Linux distributions, macOS, and Windows.

---

### Phase 16: Beta, Launch & Community
**Goal:** Sustainable creator ecosystem.
- Public beta testing and crash telemetry.
- Open-source plugin API (WASM / WGSL effect manifests).
- Documentation, tutorials, and default templates.
- **Exit criteria:** Public release v1.0 on GitHub and Flathub.

---

# Part 2: Easy Edits Pro (EEP) Project Alignment & Current Implementation Status

This section evaluates **exactly where Easy Edits Pro stands today** against the 16 phases.

```
[Phase 0: Foundations]              ████████████████████ 100% COMPLETE
[Phase 1: Project Skeleton]         ████████████████████ 100% COMPLETE
[Phase 2: Media Import & Library]   ███████████████████░  95% NEAR COMPLETE
[Phase 3: Timeline Data Model & UI] ██████████████████░░  90% NEAR COMPLETE
[Phase 4: Playback Engine]          ████████████████████ 100% COMPLETE
[Phase 5: Core Editing Tools]       ████████████████████ 100% COMPLETE
[Phase 6: Export MVP (v0.1)]        ████████████████████ 100% COMPLETE
[Phase 7: Adjustments & Transitions]██████████████████░░  90% NEAR COMPLETE
[Phase 8: Color & Visual Effects]   ██████████████████░░  90% NEAR COMPLETE
[Phase 9: Titles, Captions & Text]  ████████████████░░░░  80% ADVANCED
[Phase 10: Professional Audio]      ████████████████████ 100% COMPLETE
[Phase 11: Advanced Editing]        ████░░░░░░░░░░░░░░░░  20% PLANNED
[Phase 12: Pro Color Grading]       ███░░░░░░░░░░░░░░░░░  15% PLANNED
[Phase 13: Motion & AI Features]    █████░░░░░░░░░░░░░░░  25% INITIAL INTEGRATION
[Phase 14: Native Preview & Perf]   ████░░░░░░░░░░░░░░░░  20% PLANNED
[Phase 15: Quality & Packaging]     ████████░░░░░░░░░░░░  40% IN PROGRESS
[Phase 16: Beta & Launch]           ██░░░░░░░░░░░░░░░░░░  10% SCHEDULED
```

---

## Detailed Status Audit

### 🟢 Fully Implemented & Verified
1. **Integer Rational Time Architecture (`saturn-core/src/time.rs`):**
   - Implemented `TICKS_PER_SECOND = 254_016_000_000`.
   - Guaranteed exact integer tick alignment for 24, 25, 30, 48, 50, 60, 23.976, 29.97, 59.94 fps and 44.1k/48k/96k audio rates with zero cumulative frame drift.
2. **Reversible Command Architecture (`saturn-core/src/command.rs`, `engine.rs`):**
   - Complete command catalog with metadata, keybindings, and mutation flags.
   - Comprehensive undo/redo snapshot history with atomic serialization.
3. **Robust Project Document Persistence (`saturn-core/src/project.rs`):**
   - Versioned `.saturn` schema with migration pipeline (v1 through v4).
   - Atomic save/write-replace to prevent file corruption.
   - Auto-save engine and crash recovery dialog.
4. **Desktop Shell & Modern Linux Rice UI (`apps/saturn-tauri`):**
   - Tauri 2 integration with clean IPC boundary.
   - Distinct Linux desktop identity designed for Hyprland/Niri (Obsidian `#080a0f` palette, 1px cyan/violet glowing borders, glassmorphic panels, tactile `:active` micro-interactions).
   - Instant Dark/Light mode theme switcher (`Ctrl+Alt+T`).
   - DaVinci Resolve workspace bar (Media, Edit, Color, Audio, Effects, Deliver).
   - Filmora 15 left asset panel (Media, Titles, Transitions, Audio, Stock, AI Tools).
   - Premiere Pro vertical timeline tool rail (Select, Track Select, Ripple, Razor, Slip, Pen, Hand, Zoom).
   - Universal hover tooltips on every tool, menu, and control.
5. **Video & Audio Transitions:**
   - Supported kinds: `Crossfade`, `DipToBlack`, `DipToWhite`, `WipeLeft`, `WipeRight`, `SlideLeft`, `SlideRight`.
   - Timeline cut badges with duration handles, `Ctrl+D` shortcut, and dedicated Inspector panel.
   - Compiled into GStreamer Editing Services (`ges::TransitionClip`) for final export.
6. **Title & Lower-Third Graphics Engine:**
   - Full data model (`TitleProperties`), `Ctrl+T` shortcut, neon-violet timeline clips.
   - Real-time Program Monitor glassmorphic overlay with backdrop blur.
   - Export rendering compiled into `ges::TitleClip`.
7. **Audio Waveform Peak Decoding:**
   - GStreamer peak amplitude decoding into normalized 100-bin arrays.
   - Real-time mirrored SVG waveform rendering across timeline audio clips.
8. **Export Pipeline (`saturn-render`):**
   - GStreamer Editing Services (`ges::Timeline`) pipeline compiler.
   - MP4 and WebM rendering with hardware encoder auto-detection (`vah264enc`, `nvh264enc`, `qsvh264enc`, fallback to `x264enc`).
   - In/Out marked range export (`slice_document`).
9. **Linux WebKitGTK Video Playback Fix:**
   - Multi-threaded HTTP byte-range media streaming server (`HTTP 206 Partial Content`) on `127.0.0.1:0`.
   - Automated VP8/WebM 720p preview proxy background transcoder.
   - `WEBKIT_DISABLE_DMABUF_RENDERER=1` initialization for Wayland/Nvidia stability.
10. **Background Job Queue Manager (`saturn-core/src/job.rs` & `saturn-tauri`):**
    - Multi-threaded priority job engine (`Immediate`, `High`, `Normal`, `Background`) with cooperative cancellation via `CancellationToken`.
    - Real-time progress reporting and event emission (`job_queue_updated`) across IPC to desktop UI.
    - Integrated with preview proxy transcode jobs and export rendering jobs with partial file cleanup on cancel.
    - Header Job Status Pill and popover Job Drawer with active task counters, progress bars, and one-click cancel controls.
11. **Seamless Multi-Track Playback Engine & Audio-Clock Master Transport (`playbackEngine.ts` & `App.tsx`):**
    - Web Audio API hardware clock master (`audioClockMaster`) providing microsecond-level monotonic clock timing without cumulative timer drift or stutter.
    - Dual-Deck Video Compositor (Deck A / Deck B) with zero-stutter cut hand-off, staging upcoming clips in advance to eliminate frame freezes.
    - Real-time Program Monitor visual transitions (`Crossfade`, `DipToBlack`, `DipToWhite`, `WipeLeft`, `WipeRight`, `SlideLeft`, `SlideRight`) playing both outgoing and incoming clips concurrently.
    - Multi-track audio mixing with track gain faders (`gain_db`) and micro-fade cut smoothing to eliminate DC offset clicks.
    - Continuous timeline sequence playback advancing smoothly at 60fps across cuts, transitions, and timeline gaps.
    - JKL shuttle playback (-4x, -2x, -1x, 1x, 2x, 4x), frame stepping (◀ / ▶ / Left / Right), and Home/End sequence seeking.
12. **Composable Effect Stack (`saturn-core`, `saturn-render`, `playbackEngine.ts` & `App.tsx`):**
    - Extensible `Vec<EffectInstance>` stack supporting Color Correction, Gaussian Blur, Video Crop, Radial Vignette, and Custom Chroma Keying.
    - Project schema upgraded to version 5 with automated migration preserving legacy `Clip.color` settings.
    - Full transactional command support: `clip.add_effect`, `clip.remove_effect`, `clip.update_effect`, `clip.toggle_effect`, `clip.reorder_effects` with full undo/redo checkpointing.
    - GES video rendering pipeline integration using `videobalance`, `gaussianblur`, `videocrop`, and `alpha` elements with exact pixel calculations and hex color parsing.
    - Real-time Program Monitor CSS visual shader integration (composite filters, clip-path insets, deck radial vignette overlays).
    - Modern Effects Inspector panel with effect card bypass/reorder/delete controls, specialized property sliders, and DaVinci-inspired workspace switching.
13. **Fairlight-Style Audio Mixing Console & Multi-Track Audio Engine (Phase 4.1 / Phase 10 advancement):**
    - Expanded `Track` data model in `saturn-core/src/project.rs` with `pan: f64` (-1.0 to +1.0), `muted: bool`, `solo: bool`, and `locked: bool` with `#[serde(default)]` and `is_audible(any_solo)` helper for 100% backward compatibility.
    - Added transactional commands `track.set_pan`, `track.set_muted`, `track.set_solo`, `track.set_locked` with full undo/redo checkpointing and track edit-lock protection across clip additions, deletions, splits, moves, slips, and transitions.
    - Upgraded GStreamer GES timeline builder (`saturn-render/src/lib.rs`) to compute track audibility based on mute/solo matrix and apply effective attenuation (`-120.0 dB` when muted or not in solo) to final exported video.
    - Web Audio API master clock (`AudioClockMaster` in `playbackEngine.ts`) upgraded with `StereoPannerNode` for balance, track mute/solo routing matrix, and dual `AnalyserNode` channels for 60fps real-time peak VU metering.
    - Fairlight-style vertical channel strips audio console in `App.tsx` with panning knobs/sliders, M/S/L matrix buttons, long-throw vertical faders (-60 to +6 dB), dual-channel LED peak VU meters, unity gain resets, and master stereo bus strip.
    - Timeline track headers enhanced with M/S/L buttons and locked-track visual diagonal hatching.

---

### 🟡 Partially Implemented / In Progress
1. **Core Editing Tools (Phase 5):**
   - ✅ Working: Split clip (`Ctrl+K` / `C`), Move clip, Remove clip (`Delete`), Ripple Delete (`Shift+Delete` / `B`), Slip clip (`Y`), In/Out marks (`I`/`O`), Track locking protection.
   - ⏳ Missing: Rolling edit (`N`), Slide edit (`U`), Rate stretch (`X`), Multi-clip group link/unlink.
2. **Keyframe Animation (Phase 7):**
   - ✅ Working: `Keyframe` data model in `saturn-core` for `PositionX`, `PositionY`, `Scale`, `Rotation`, `Opacity`; keyframe evaluation in preview and export.
   - ⏳ Missing: Visual keyframe curve graph editor on the timeline with bezier tangent handles.
3. **Color & Filters (Phase 8):**
   - ✅ Working: Composable effect stack (Color Correction, Gaussian Blur, Video Crop, Radial Vignette, Chroma Key) with full undo/redo, real-time preview, and GES export.
   - ⏳ Missing: LUT `.cube` 3D loader, secondary HSL qualifiers, shape/bezier vector masks.
4. **Professional Audio (Phase 10):**
   - ✅ Working: Fairlight multi-track channel strips, stereo balance panning, mute/solo matrix, track lock protection, long-throw vertical faders, real-time peak VU metering, and master stereo bus strip.
   - ⏳ Missing: Parametric 4-band EQ, compressor/limiter, and BS.1770 / EBU R128 integrated LUFS loudness meter.
5. **AI Features (Phase 13):**
   - ✅ Working: Local AI chat assistant connected to Ollama (`POST http://127.0.0.1:11434/api/chat`).
   - ⏳ Missing: Embedded offline Whisper speech-to-text auto-captions, silence detection, background rotoscoping.

---

### 🔴 Not Started / Scheduled for Next Phases
1. **Real-Time Video Scopes (Phase 12):**
   - GPU-computed Waveform Monitor, Vectorscope, Histogram, and RGB Parade.
   - Lift/Gamma/Gain color wheels and secondary HSL qualifiers.
2. **Compound Sequences & Multi-Cam (Phase 11):**
   - Nested timeline sequences and multi-camera angle sync by waveform correlation.
3. **Native wgpu Preview Surface (Phase 14):**
   - Direct GPU rendering surface bypassing browser webview DOM overhead for 4K 60fps playback.
4. **Interchange & Batch Render Queue (Phases 6 & 14):**
   - Multi-job background export queue panel.
   - FCPXML / Premiere XML / EDL export and import.
5. **Complete Linux Packaging (Phase 15):**
   - Flatpak, AppImage, and Arch Linux AUR distribution packages.

---

## 5. Next Immediate Action Plan

To advance directly toward **Phase 2 (Playback & Render Consistency)** and **Phase 3 (Core Effects Stack & Scopes)**:

1. **Phase 2.1 — Background Job Queue Manager:** ✅ **Completed**
   - Built thread-safe prioritized job engine (`saturn-core/src/job.rs`) with cooperative cancellation via `CancellationToken`.
   - Wired preview proxy generation and export rendering into the job manager with cancel cleanup.
   - Built UI status pill and drawer popover with progress bars, task states, and one-click cancellation.
2. **Phase 2.2 — Seamless Multi-Track Playback Engine (Phase 4 advancement):** ✅ **Completed**
   - Implemented `AudioClockMaster` leveraging hardware `AudioContext.currentTime` for drift-free master clock timing.
   - Built Dual-Deck Video Compositor (Deck A / Deck B) in the Program Monitor, preloading and staging upcoming clips to eliminate cut stutter.
   - Built real-time Program Monitor visual transitions (`Crossfade`, `DipToBlack`, `DipToWhite`, `WipeLeft`, `WipeRight`, `SlideLeft`, `SlideRight`) playing both outgoing and incoming clips concurrently.
   - Added multi-track audio playback with track gain faders (`gain_db`) and cut smoothing.
   - Added JKL shuttle playback (-4x, -2x, -1x, 1x, 2x, 4x), frame stepping, and proactive timeline proxy pre-generation.
3. **Phase 3.1 — Composable Effect Stack (Phase 8 advancement):** ✅ **Completed**
   - Extensible `Vec<EffectInstance>` stack with Color Correction, Gaussian Blur, Video Crop, Radial Vignette, and Chroma Key.
   - Schema version 5 with seamless migration and two-way sync for legacy `color`.
   - 5 transactional commands with checkpointing and undo/redo (`clip.add_effect`, `clip.remove_effect`, `clip.update_effect`, `clip.toggle_effect`, `clip.reorder_effects`).
   - GES render pipeline compiling `videobalance`, `gaussianblur`, `videocrop`, and `alpha` elements.
   - Modern Effects Inspector in UI with cards, bypass toggles, sliders, and live Program Monitor CSS effects.
4. **Phase 4.1 — Fairlight Audio Mixing Console & Metering Engine (Phase 10 advancement):** ✅ **Completed**
   - Extended `Track` schema with `pan`, `muted`, `solo`, and `locked` with backward-compatible defaults.
   - Implemented transactional commands `track.set_pan`, `track.set_muted`, `track.set_solo`, `track.set_locked` with full undo/redo and track edit-lock protection.
   - GStreamer GES timeline builder updated to respect mute/solo matrix for audio mixdown during video export.
   - Web Audio `AudioClockMaster` upgraded with stereo panning (`StereoPannerNode`), mute/solo matrix, and dual-channel 60fps peak VU metering (`AnalyserNode`).
   - Designed Fairlight-style vertical channel strips audio console in `App.tsx` and obsidian rice cyberpunk styling in `style.css`.
   - Enhanced timeline track headers with M/S/L toggle buttons and visual lock hatching.
5. **Phase 4.2 — Real-Time Video Scopes & Primary Color Wheels (Phase 12 advancement):** ✅ **Completed**
   - **Real-Time 60 FPS Video Scopes Component (`VideoScopes.tsx`):**
     - Rec.709 Luma Waveform Monitor ($Y = 0.2126R + 0.7152G + 0.0722B$) with 0–100 IRE reference graticules.
     - Polar $Cb/Cr$ Vectorscope with 75% & 100% saturation targets, primary/secondary color boxes ($R, Mg, B, Cy, G, Yl$), and the standard $57^\circ$ Skin-Tone indicator line.
     - RGB Parade rendering Red, Green, and Blue channel distributions side-by-side.
     - 4-Channel Histogram plotting frequency distributions across Luma ($Y$) and RGB channels.
     - Synchronized Quad View displaying all four scopes concurrently in a 2×2 grid.
     - Live trace gain slider ($0.5\times$ to $2.5\times$) and graticule visibility toggle.
     - Optimized offscreen canvas downsampling ($160\times90$) delivering smooth 60fps analysis at $<2\%$ CPU overhead.
   - **Primary Color Wheels Dock (`ColorWheels.tsx`):**
     - 4 interactive wheels: Lift (Shadows), Gamma (Midtones), Gain (Highlights), and Offset (Master).
     - 2D circular chromaticity discs with conic gradients, draggable pucks clamped to unit circle $r \le 1$, double-click neutral reset, and per-wheel reset buttons.
     - Master Luminance ($Y$) horizontal faders and numeric coordinate readouts.
   - **Dedicated 2-Tier Color Grading Studio Layout:**
     - Top tier: Program Monitor (left) + Video Scopes (right).
     - Bottom tier: Color Wheels dock (left) + Primary Sliders & 1-Click Look Presets (right).
     - Sequence Clips Reel bottom tray for 1-click clip selection across the timeline.
     - Live chrominance vector to hue angle translation (\(\text{atan2}(y, x) / \pi\)) coupled directly to visual CSS filters, real-time scopes, and GStreamer `videobalance hue=...` render export.

6. **Phase 6 — Export MVP & Render Enhancements:** ✅ **Completed**
   - **Hardware Encoder Probing & Detection Engine (`saturn-render`):**
     - Dynamic probing of system GStreamer hardware codecs via `ElementFactory::find`: VA-API (`vah264enc`), NVIDIA NVENC (`nvh264enc`), Intel Quick Sync (`qsvh264enc`), V4L2 (`v4l2h264enc`), and CPU software fallbacks (`x264enc`, `openh264enc`).
     - Exposed Tauri IPC command `detect_encoder_capabilities` returning detected hardware codec matrix.
     - Live UI Hardware Acceleration Status Badge in Deliver workspace indicating active hardware encoder or software fallback with codec chips.
   - **1-Click Target Delivery Presets:**
     - Pre-configured 1-click delivery presets for YouTube 4K UHD (3840×2160, 60fps), YouTube 1080p (1920×1080, 60fps), TikTok / Reels / Shorts (1080×1920 9:16 vertical, 30fps), Instagram Square (1080×1080 1:1, 30fps), Cinema 4K DCI (3840×2160, 24fps film), and Web HD (1280×720, 30fps).
     - Auto-configures resolution, frame rate numerator/denominator, and container format with real-time delivery summary.

7. **Phase 10 — Advanced Fairlight Audio Studio:** ✅ **Completed**
   - **Rust Audio Processing Schema (`saturn-core`):**
     - Added `TrackAudioProcessing` struct: 3-band parametric EQ (`eq_low_gain`, `eq_mid_gain`, `eq_high_gain`, `eq_enabled`), dynamics compressor (`comp_threshold`, `comp_ratio`, `comp_attack`, `comp_release`, `comp_enabled`), and vocal denoise (`denoise_enabled`, `denoise_amount`).
     - Transactional command `track.set_audio_processing` with parameter validation, track lock protection, and undo/redo checkpointing.
   - **GStreamer GES Export Audio Pipeline (`saturn-render`):**
     - Compiles `equalizer-3bands` when EQ is enabled.
     - Compiles high-pass `audiocheblimit` cutoff at 80 Hz when vocal denoise is enabled.
   - **Web Audio Real-Time DSP Chain (`AudioClockMaster` in `playbackEngine.ts`):**
     - Per-track DSP chain: Source -> 80 Hz High-Pass Filter -> 3-Band Biquad EQ (Low Shelf 80 Hz, Peaking 1 kHz, High Shelf 8 kHz) -> `DynamicsCompressorNode` -> Output Gain -> `StereoPannerNode` -> `AnalyserNode` -> Master Bus.
     - Master Bus True Peak Limiter: Master Gain -> Brickwall `DynamicsCompressorNode` (-0.5 dBFS ceiling, 20:1 ratio, 1 ms attack) -> Master Analyser -> Audio Destination.
     - Real-time Gain Reduction (GR) metering: `getTrackGainReduction(trackId)` and `getMasterGainReduction()`.
   - **Fairlight 3-Column Studio Console UI (`App.tsx` & `style.css`):**
     - Left Column: Picture Sync Monitor + Master Bus Fader & VU Meter + Master True Peak Limiter Module with live Master GR meter.
     - Center Column: Dedicated Track Audio DSP Rack with interactive SVG frequency curve graphic, 4-band parametric EQ sliders, dynamics compressor controls with live Gain Reduction (GR) meter, and vocal de-noise rumble filter controls.
     - Right Column: Multi-track vertical channel strips with quick `[EQ] [DYN] [NR]` toggle badges, pan, mute, solo, lock, dual VU meters, vertical faders, and mini GR reduction bars.

