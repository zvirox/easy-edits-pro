<div align="center">

<img src="assets/logo.png" alt="Easy Edits Pro Logo" width="160" />

# Easy Edits Pro

**力と美のために** &middot; *Precision, Performance & Power for Modern Linux Video Editing.*

[![Sponsor zvirox](https://img.shields.io/badge/Sponsor-zvirox-ea4aaa?style=for-the-badge&logo=github-sponsors&logoColor=white)](https://github.com/sponsors/zvirox)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-zvirox-FF5E5B?style=for-the-badge&logo=kofi&logoColor=white)](https://ko-fi.com/zvirox)

<br />

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-E2342A?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri 2](https://img.shields.io/badge/Tauri-2.0-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![GStreamer](https://img.shields.io/badge/GStreamer-GES_1.24+-E2342A?style=for-the-badge&logo=gstreamer&logoColor=white)](https://gstreamer.freedesktop.org/)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/zvirox/easy-edits-pro)

<br />

Easy Edits Pro is a high-performance, non-linear desktop video editor engineered in Rust and Tauri 2 for Linux. Designed to combine the workspace fluidity of DaVinci Resolve, the rich asset shelf ergonomics of Wondershare Filmora, and the cutting precision of Adobe Premiere Pro—all inside an obsidian-dark interface crafted specifically for modern Linux desktop rices (Hyprland, Niri, Sway, GNOME, and KDE).



[![GitHub](https://img.shields.io/badge/GitHub-zvirox-181717?style=for-the-badge&logo=github&logoColor=white)](https://github.com/zvirox)
[![YouTube: @zvirox_in](https://img.shields.io/badge/YouTube-@zvirox__in-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/@zvirox_in)
[![YouTube: @climin_in](https://img.shields.io/badge/YouTube-@climin__in-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/@climin_in)
[![Instagram](https://img.shields.io/badge/Instagram-@plg.sevala-E4405F?style=for-the-badge&logo=instagram&logoColor=white)](https://www.instagram.com/plg.sevala)
[![LinkedIn](https://img.shields.io/badge/LinkedIn-plgsevala-0A66C2?style=for-the-badge&logo=linkedin&logoColor=white)](https://www.linkedin.com/in/plgsevala/)
[![Facebook](https://img.shields.io/badge/Facebook-plg.sevala-1877F2?style=for-the-badge&logo=facebook&logoColor=white)](https://www.facebook.com/plg.sevala)

<br />

<kbd>[Overview](#about)</kbd> &middot; <kbd>[Workspaces](#workspaces-suite)</kbd> &middot; <kbd>[Key Capabilities](#key-capabilities)</kbd> &middot; <kbd>[Architecture](#core-architecture)</kbd> &middot; <kbd>[Quick Start](#quick-start)</kbd> &middot; <kbd>[Roadmap](#development-roadmap)</kbd> &middot; <kbd>[Connect](#connect--follow)</kbd>

</div>

---

<div align="center">

<img src="assets/screenshots/wcss.png" alt="Easy Edits Pro Main Workspace Suite" width="960" />

<sub><b>The Easy Edits Pro Desktop Suite.</b> Multi-track timeline, DaVinci Resolve workspace ribbon, Filmora left asset shelf, Premiere tool rail, multi-cam quad split, and Fairlight audio console.</sub>

</div>

---

## About

Easy Edits Pro (originally *Project Saturn*) is an open-source, framework-independent desktop video editor built from first principles for Linux creators. Traditional Linux editors often force a choice between unstable monolithic wrappers or clunky legacy interfaces. Easy Edits Pro bridges that gap with a modern, decoupled architecture:

1. **Rust Core Engine (`saturn-core`):** A rock-solid, zero-drift rational timebase (`254,016,000,000` integer ticks/second) that manages sequences, clips, transitions, keyframes, bus routing, and transactional command history with instantaneous atomic undo/redo.
2. **GStreamer Render Engine (`saturn-render`):** Direct hardware-accelerated video decoding, 3D LUT tetrahedral interpolation, optical flow motion displacement tracking, audio cross-correlation sync, and GStreamer Editing Services (GES) multi-job rendering.
3. **Fluid Tauri 2 Shell (`saturn-tauri`):** A responsive, hardware-accelerated UI styled in obsidian dark (`#080a0f`) with 1px glowing borders, neon cyan/violet accents, real-time SVG audio waveforms, and tactile feedback built to complement tiling compositors like Hyprland and Niri.

---

## Workspaces Suite

Easy Edits Pro mirrors the streamlined, task-oriented workflow of industry-standard post-production suites through its integrated workspace ribbon:

<table>
  <tr>
    <td width="50%">
      <h3>1. Media Studio <kbd>Shift + 1</kbd></h3>
      <p>High-density project media bin with multi-file drag-and-drop, asynchronous GStreamer metadata probing, live cached video/audio proxies, and integrated Openverse FOSS stock media search.</p>
    </td>
    <td width="50%">
      <h3>2. Edit Suite <kbd>Shift + 2</kbd></h3>
      <p>Multi-track timeline with Premiere Pro tool rail (<kbd>V</kbd>, <kbd>A</kbd>, <kbd>B</kbd>, <kbd>C</kbd>, <kbd>Y</kbd>, <kbd>P</kbd>, <kbd>H</kbd>, <kbd>Z</kbd>), dual-deck preview staging (Deck A / Deck B), frame snapping, and real-time SVG waveform display.</p>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <h3>3. Color Grading Studio <kbd>Shift + 3</kbd></h3>
      <p>4 interactive 3-way color balance wheels (Lift, Gamma, Gain, Offset) with 2D chromaticity pucks, master luma rings, 3D LUT (.cube) tetrahedral grading, and real-time GPU scopes (Waveform, Vectorscope, Histogram, Parade).</p>
    </td>
    <td width="50%">
      <h3>4. Fairlight Audio Studio <kbd>Shift + 4</kbd></h3>
      <p>Multi-track mixing console with long-throw vertical dB faders, 60fps LED stereo peak VU meters, track pan knobs, Submix/Aux bus routing, fader automation modes (Read/Touch/Latch/Write), and VST3 plugin inserts.</p>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <h3>5. Effects &amp; Keyframes <kbd>Shift + 5</kbd></h3>
      <p>Composable multi-effect stack (Color Correction, Gaussian Blur, Crop, Vignette, Chroma Key), cubic bezier keyframe spline editor with Ease-In / Ease-Out tangent handles, and Lucas-Kanade optical flow motion tracking.</p>
    </td>
    <td width="50%">
      <h3>6. Delivery &amp; Batch Queue <kbd>Shift + 6</kbd></h3>
      <p>Multi-job batch export queue with background worker daemon, atomic pause/resume controls, target delivery presets (YouTube, TikTok 9:16, Cinema 4K), and GPU hardware encoding (NVENC, VA-API, QSV).</p>
    </td>
  </tr>
</table>

---

## Key Capabilities

### 🎥 Multi-Cam Synchronized Editing
- **Audio Waveform Cross-Correlation:** Automatically synchronizes multi-angle footage by cross-correlating decoded audio waveforms, calculating lag offsets with sub-millisecond precision.
- **2x2 Quad-Split Monitor:** Displays Angles 1–4 simultaneously in the Program stage.
- **Live Tally Indicator:** Active on-air angle glows in brilliant broadcast red with an `ON AIR` badge, while hovered preview tiles display a green tally border.
- **One-Key Angle Switching:** Cut live between camera angles on the fly using number keys <kbd>1</kbd>, <kbd>2</kbd>, <kbd>3</kbd>, and <kbd>4</kbd> during playback.

### 🎨 Color Grading &amp; 3D LUTs
- **Primary Color Wheels:** Lift modulates shadows ($1 - v$), Gamma curves midtones ($v^{1 - 0.5\gamma}$), Gain expands highlights ($v \cdot (1 + \text{gain})$), and Offset translates the pedestal across RGBA.
- **3D LUT (.cube) Support:** Parses standard $17\times17\times17$ to $64\times64\times64$ 3D LUT files and interpolates RGB color tables in-place.
- **GPU Scopes:** Real-time visual monitoring via Vectorscope, Waveform Monitor, RGB Parade, and Luminance Histogram.

### 🎯 Motion Tracking &amp; Camera Stabilization
- **Lucas-Kanade Optical Flow Tracker:** Tracks sub-pixel patch displacements frame-by-frame across high-motion sequences.
- **Camera Shake Stabilization:** Smooths raw trajectory curves using a Gaussian low-pass filter and applies real-time inverse transformation matrices.
- **Sticker &amp; Overlay Pinning:** Locks titles, lower-thirds, or graphic overlays to tracked object coordinates.
- **On-Screen Tracking Target:** High-visibility dashed crosshair overlay with corner brackets on the Program monitor.

### 🎚️ Fairlight Multitrack Mixer &amp; Automation
- **Console Strips:** Motorized vertical faders with unity gain resets (<kbd>0 dB</kbd>), rotary stereo pan sliders, and dual stereo LED VU peak meters.
- **Bus Routing Matrix:** Assign each track dynamically to `Master`, `Submix 1 (DX Dialogue)`, `Submix 2 (MX Music)`, or `Aux FX (Reverb)`.
- **Fader Automation Modes:** `Read`, `Touch` (punch-in on mouse drag), `Latch`, and `Write` for volume and pan envelopes.
- **VST3 Host Architecture:** Insert native DSP effects including Parametric 4-band EQ, Dynamics Compressor, De-Esser Pro, and Stereo Chorus.

### ⚡ Batch Render Queue &amp; Background Worker
- **Thread-Safe Daemon:** Multi-threaded Rust worker daemon (`saturn-core/src/job.rs`) with prioritized execution (`Immediate`, `High`, `Normal`, `Background`) and cooperative cancellation tokens.
- **Pause &amp; Resume:** Pause active rendering without dropping frame buffers, allowing resource recovery for foreground editing.
- **Hardware Acceleration:** Auto-probes Linux hardware encoders: NVIDIA NVENC (`nvcodec`), Intel Quick Sync (`vaapi`/`qsv`), AMD AMF / VA-API (`vaapih264enc`), with seamless CPU fallback (`x264enc`).

### 📝 Dedicated Subtitle Track &amp; CC
- **Timeline Subtitle Lane:** Independent subtitle card track on the sequence with auto-wrapping text blocks and live preview rendering.
- **Standards Compliant:** One-click import and export for both `.srt` (SubRip) and `.vtt` (WebVTT) formats.

---

## Core Architecture

Easy Edits Pro enforces a strict separation between domain state, native rendering, and UI presentation:

```mermaid
graph LR
    subgraph Frontend ["Tauri 2 Desktop Shell (TypeScript + React 19)"]
        UI["Obsidian UI & Timeline"]
        Playback["Dual-Deck Playback Engine"]
        Scopes["Video Scopes & Meters"]
    end

    subgraph IPC ["Tauri IPC Bridge"]
        Commands["Transactional Commands"]
        Events["Channel Progress Events"]
        Protocol["Custom Asset Streamer"]
    end

    subgraph Core ["Rust Core (saturn-core)"]
        Doc["Project Document (Schema v9)"]
        Time["Exact Rational Tick Timebase"]
        Jobs["JobManager (Background Daemon)"]
        History["Undo / Redo Checkpoints"]
    end

    subgraph Render ["Native Engine (saturn-render)"]
        GES["GStreamer Editing Services"]
        Wheels["RGBA Color Wheels Engine"]
        LUT["3D LUT Tetrahedral Engine"]
        Tracker["Optical Flow Motion Tracker"]
        Audio["Waveform Cross-Correlation Sync"]
    end

    UI -->|Execute JSON Command| Commands
    Commands -->|Dispatch| Core
    Core -->|State Snapshot| UI
    Playback -->|Custom URI Streaming| Protocol
    Protocol -->|Proxy Decoder| Render
    Jobs -->|Batch Export Task| Render
    Render -->|Hardware Encoders: NVENC / VAAPI| Output["MP4 / WebM Video"]
```

### Rational Integer Timebase
To avoid accumulated frame drift over long sequences, Easy Edits Pro calculates all time values in exact integer ticks:

$$\text{Ticks Per Second} = 254{,}016{,}000{,}000$$

`TICKS_PER_SECOND = 254_016_000_000`

This tick base is evenly divisible by all broadcast and digital frame rates: **23.976**, **24**, **25**, **29.97**, **30**, **48**, **50**, **59.94**, and **60 fps**, as well as pro audio sample rates (**44.1 kHz**, **48 kHz**, **96 kHz**, and **192 kHz**).

---

## System Requirements

| Component | Minimum | Recommended |
|:---|:---|:---|
| **Operating System** | Linux (Kernel 5.15+) | Modern Linux distribution (Arch, Fedora, Debian 12+, Ubuntu 22.04+) |
| **Desktop Environment** | Any X11 or Wayland compositor | Hyprland, Niri, Sway, GNOME 44+, or KDE Plasma 6 |
| **CPU** | 64-bit x86_64, Dual-Core | 64-bit Quad-Core (Intel Core i5/i7/i9 or AMD Ryzen 5/7/9) |
| **RAM** | 4 GB | 16 GB+ for smooth 4K timeline scrub and multi-cam playback |
| **GPU** | OpenGL 3.3 / Vulkan compatible | Dedicated NVIDIA (GTX 1060+), AMD Radeon (RX 580+), or Intel Arc |
| **Hardware Encoders** | Software fallback (`x264enc`) | NVIDIA NVENC, Intel VA-API / QSV, or AMD VA-API |
| **Storage** | 500 MB for application | NVMe SSD for fast media cache proxies and scratch disks |

---

## Quick Start

### 1. Prerequisites (Ubuntu/Debian)
```bash
sudo apt update && sudo apt install -y \
  build-essential curl wget git pkg-config \
  libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
  gstreamer1.0-libav gstreamer1.0-tools libges-1.0-dev
```

### 2. Prerequisites (Arch Linux)
```bash
sudo pacman -S --needed \
  base-devel curl git gstreamer gst-plugins-base gst-plugins-good \
  gst-plugins-bad gst-plugins-ugly gst-libav gst-editing-services \
  webkit2gtk-4.1 rustup nodejs pnpm
```

### 3. Clone and Run in Development
```bash
git clone https://github.com/zvirox/easy-edits-pro.git
cd easy-edits-pro/apps/saturn-tauri
pnpm install
pnpm tauri dev
```

### 4. Build Native Linux Packages (.deb / .rpm)
```bash
cd apps/saturn-tauri
pnpm tauri build
```
The compiled binaries, `.deb`, and `.rpm` packages will be placed in `apps/saturn-tauri/target/release/bundle/`.

### 5. Headless CLI Control
```bash
# Query available engine commands
cargo run -p saturn-cli -- commands

# Create and inspect project files
cargo run -p saturn-cli -- new ~/Videos/edit.saturn "My Video"
cargo run -p saturn-cli -- inspect ~/Videos/edit.saturn

# Batch automated command runner
cargo run -p saturn-cli -- control
```

---

## Repository Structure

| Directory | Purpose |
|:---|:---|
| [`apps/saturn-tauri/`](apps/saturn-tauri/) | Tauri 2 desktop shell, IPC command bridge, hardware encoder detection |
| [`apps/saturn-tauri/ui/`](apps/saturn-tauri/ui/) | React 19 UI, dual-deck playback engine, Fairlight console, VideoScopes |
| [`crates/saturn-core/`](crates/saturn-core/) | Core schema, integer timebase, undo/redo transactions, job manager |
| [`crates/saturn-render/`](crates/saturn-render/) | Color wheels, optical flow tracker, 3D LUT parser, GStreamer exporter |
| [`crates/saturn-cli/`](crates/saturn-cli/) | Headless automation CLI, project inspector, and script runner |
| [`assets/`](assets/) | Brand marks, icons, fonts, and screenshot showcases |
| [`docs/`](docs/) | Control protocol specifications and architectural documentation |

---

## Development Roadmap

- [x] **Phase 1:** Core Timeline, Trimming, Razor, Rolling Edit, Slide &amp; Slip Edits.
- [x] **Phase 2:** Seamless Multi-Track Playback Engine, Audio-Clock Master, JKL Shuttle.
- [x] **Phase 3:** Composable Effect Stack (Blur, Crop, Vignette, Chroma Key).
- [x] **Phase 4:** Interactive Color Wheels (Lift, Gamma, Gain, Offset) &amp; Real-time Video Scopes.
- [x] **Phase 5:** Multi-Cam Synchronized Editing via Audio Waveform Cross-Correlation &amp; Quad Split.
- [x] **Phase 6:** Hardware Accelerated Rendering (NVENC/VA-API/QSV) &amp; Batch Export Queue Daemon.
- [x] **Phase 7:** Keyframe Cubic Bezier Curve Editor, Optical Flow Motion Tracking &amp; Stabilization.
- [x] **Phase 8:** 3D LUT (.cube) tetrahedral interpolation across preview &amp; export.
- [x] **Phase 9:** Dedicated Subtitle Track with one-click `.srt` and `.vtt` Import/Export.
- [x] **Phase 10:** Fairlight Multitrack Console, dB Fader Automation (Read/Touch/Latch/Write), Submix Routing.
- [ ] **Phase 11:** Node-Based Compositing Workspace (Fusion-style node graph &amp; roto-splines).
- [ ] **Phase 12:** Real-Time LAN &amp; Cloud Multi-Editor Collaborative Project Locking.

---

## Connect &amp; Follow

Follow development, report issues, and watch tutorials:

<div align="center">

<p>
  <a href="https://github.com/zvirox">
    <img src="https://img.shields.io/badge/GitHub-zvirox-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub" />
  </a> &nbsp;
  <a href="https://www.youtube.com/@zvirox_in">
    <img src="https://img.shields.io/badge/YouTube-@zvirox__in-FF0000?style=for-the-badge&logo=youtube&logoColor=white" alt="YouTube @zvirox_in" />
  </a> &nbsp;
  <a href="https://www.youtube.com/@climin_in">
    <img src="https://img.shields.io/badge/YouTube-@climin__in-FF0000?style=for-the-badge&logo=youtube&logoColor=white" alt="YouTube @climin_in" />
  </a>
</p>

<p>
  <a href="https://www.instagram.com/plg.sevala">
    <img src="https://img.shields.io/badge/Instagram-@plg.sevala-E4405F?style=for-the-badge&logo=instagram&logoColor=white" alt="Instagram" />
  </a> &nbsp;
  <a href="https://www.linkedin.com/in/plgsevala/">
    <img src="https://img.shields.io/badge/LinkedIn-plgsevala-0A66C2?style=for-the-badge&logo=linkedin&logoColor=white" alt="LinkedIn" />
  </a> &nbsp;
  <a href="https://www.facebook.com/plg.sevala">
    <img src="https://img.shields.io/badge/Facebook-plg.sevala-1877F2?style=for-the-badge&logo=facebook&logoColor=white" alt="Facebook" />
  </a>
</p>

</div>

---

## License

Easy Edits Pro is released under the **[GNU General Public License v3.0](LICENSE)**.

```
Easy Edits Pro — Open Source Linux Video Editor
Copyright (C) 2026 zvirox & contributors.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

---

<div align="center">

### 💖 Support Easy Edits Pro

If you find Easy Edits Pro valuable, consider sponsoring **[@zvirox](https://github.com/sponsors/zvirox)** on GitHub Sponsors or supporting on Ko-fi to fuel ongoing open-source Linux video editing development!

<br />

[![Sponsor zvirox](https://img.shields.io/badge/Sponsor_on_GitHub-zvirox-ea4aaa?style=for-the-badge&logo=github-sponsors&logoColor=white)](https://github.com/sponsors/zvirox)
&nbsp;
[![Support on Ko-fi](https://img.shields.io/badge/Support_on_Ko--fi-zvirox-FF5E5B?style=for-the-badge&logo=kofi&logoColor=white)](https://ko-fi.com/zvirox)

</div>
