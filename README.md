<p align="center">
  <img src="docs/assets/easy-edits-pro-logo.png" alt="Easy Edits Pro logo" width="180" />
  <h1 align="center">Easy Edits Pro</h1>
</p>

<p align="center">
  <strong>Native Linux video editing built for speed, precision, and privacy</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/easy-edits-pro/releases"><img src="https://img.shields.io/badge/Release-Latest_Packages-blue.svg" alt="Releases" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/Stack-Rust_·_Tauri_2_·_React_19-indigo.svg" alt="Stack" />
</p>

<p align="center">
  <a href="#what-is-easy-edits-pro">What is it?</a> ·
  <a href="#key-features">Key Features</a> ·
  <a href="#starship-control-integration">Starship Control Integration</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#frequently-asked-questions">FAQ</a>
</p>

---

## What is Easy Edits Pro?

Easy Edits Pro is a native Linux desktop video editor designed for creators, developers, and educators. Instead of dealing with clunky legacy interfaces or cloud-based subscription editors that upload your private raw footage to remote servers, Easy Edits Pro provides a fast, local-first editing studio right on your machine.

The application pairs a high-performance Rust core and GStreamer rendering engine with a responsive, modern Linear-style interface. Whether you are assembling screencasts, cutting YouTube videos, or polishing presentation recordings, Easy Edits Pro keeps editing responsive, accurate, and completely under your control.

---

## Key Features

- **Responsive Multitrack Timeline**: Arrange multiple video and audio tracks with smooth scrubbing, frame stepping, split, ripple trim, and zoom.
- **Non-Destructive Project Architecture**: The Rust core protects your source footage. All cuts, transitions, volume changes, and overlays are stored as non-destructive edits with instant undo and redo.
- **Smart Speech & Caption Sync**: Transcribe dialogue automatically, search for specific spoken words in your footage, and generate stylized on-screen captions with precise word timing.
- **Visual Color & Keyframe Control**: Fine-tune contrast, saturation, and exposure, and animate clip positions with customizable easing curves.
- **High-Performance GStreamer Pipeline**: Preview projects smoothly and render standard MP4 and WebM video files using your system's hardware codecs.
- **Modern Linear-Style Design**: Dark matte graphite surfaces, 1px precision borders, and smooth keyboard-driven navigation.

---

## Starship Control Integration

Easy Edits Pro is a completely self-contained video editor. You can create projects, import media, trim clips, and render final videos without installing any other tools.

When you want intelligent, hands-free timeline assistance, Easy Edits Pro connects directly with **Starship Control**, the unified Linux desktop AI assistant.

### Why It Exists

Rough-cutting video footage, trimming dead pauses, and syncing separate audio takes can consume hours of manual clicking. Connecting Easy Edits Pro with Starship Control lets you delegate repetitive editing chores using simple voice or text instructions, while you retain full final editorial review.

### Real-World Use Cases

- **Automated Silence Removal**: Tell Starship Control to *"Scan the timeline for silences longer than one second and trim them out cleanly."*
- **Multitrack Studio Assembly**: Direct a multi-app workflow: *"Import the screen take from Screen Studio Pro and the voiceover from Audio Labs Pro, align them to the first clap, and set up a 1080p timeline."*
- **Chapter Marker Creation**: Ask the assistant to *"Analyze the transcript and insert chapter markers at every major topic transition."*

### How Connection Works

Getting connected is fully automated:

1. **Automatic Discovery**: When Easy Edits Pro launches, it creates its discovery manifest in `$XDG_DATA_HOME/starship/plugins/eep.json`.
2. **Private Local Socket**: Communication occurs through a private Unix domain socket at `$XDG_RUNTIME_DIR/starship/bridges/eep.sock`.
3. **Linux Kernel Security**: Connections are validated using operating system peer credentials (`SO_PEERCRED`). Only programs running under your exact Linux user account can send commands. No web ports are opened, and your video footage never leaves your machine.
4. **Instant Control**: Open Starship Control or press `Super+Shift+Space`. It immediately detects Easy Edits Pro and stands ready to execute timeline adjustments and batch edits.

---

## Installation

Prebuilt Linux packages are available on the [Releases page](https://github.com/zvirox/easy-edits-pro/releases).

### Debian, Ubuntu, and Pop!_OS (`.deb`)

```bash
sudo apt install ./easy-edits-pro_*_amd64.deb
```

### Fedora, RHEL, and openSUSE (`.rpm`)

```bash
sudo dnf install ./easy-edits-pro-*.x86_64.rpm
```

### Arch Linux and Manjaro (`.pkg.tar.zst`)

```bash
sudo pacman -U ./easy-edits-pro-*-x86_64.pkg.tar.zst
```

### Building From Source

Requirements: Rust stable, Node.js 22+, pnpm, and development libraries for GStreamer (`libgstreamer1.0-dev`, `libgstreamer-plugins-base1.0-dev`), WebKitGTK (`libwebkit2gtk-4.1-dev`), and GTK 3.

```bash
git clone https://github.com/zvirox/easy-edits-pro.git
cd easy-edits-pro
pnpm install
pnpm build
cargo build --release -p saturn-tauri
```

---

## Frequently Asked Questions

#### Does Easy Edits Pro require an internet connection?
No. Easy Edits Pro is a 100% offline desktop application. Video importing, timeline editing, audio processing, and rendering happen locally on your computer.

#### Is Starship Control required to use the editor?
No. Easy Edits Pro has a complete visual user interface with full mouse and keyboard editing controls. Starship Control is an optional companion for automated workflows.

#### Can I undo edits made by Starship Control?
Yes. Every action performed through Starship Control is recorded in the editor's Rust undo stack. You can step backward or revert any automated change with standard `Ctrl+Z`.

#### What video formats can I export?
Easy Edits Pro supports exporting standard MP4 (H.264 / AAC) and WebM formats with customizable resolutions, frame rates, and bitrates.

---

## License

Easy Edits Pro is open-source software licensed under the [Apache License, Version 2.0](LICENSE).
