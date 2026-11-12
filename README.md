<p align="center">
  <img src="docs/assets/easy-edits-pro-logo.png" alt="Easy Edits Pro logo" width="220" />
</p>

<h1 align="center">Easy Edits Pro</h1>

<p align="center">
  <strong>An alpha Linux desktop video editor built with Rust, Tauri, React, and GStreamer.</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/easy-edits-pro/releases"><img src="https://img.shields.io/github/v/release/zvirox/easy-edits-pro?include_prereleases&amp;label=latest%20release" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/zvirox/easy-edits-pro" alt="License" /></a>
  <a href="https://github.com/zvirox/easy-edits-pro/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/zvirox/easy-edits-pro/ci.yml?label=CI" alt="CI status" /></a>
</p>

<p align="center">
  <a href="https://github.com/zvirox/easy-edits-pro/releases">Releases</a> ·
  <a href="docs/planning.md">Project plan</a> ·
  <a href="docs/AI_NATIVE_PLAN_CHECKLIST.md">Live progress checklist</a> ·
  <a href="docs/PARITY.md">Command parity</a> ·
  <a href="#install-and-try-it">Install</a> ·
  <a href="#development-setup">Development</a>
</p>

## Project status

Easy Edits Pro is an alpha Linux desktop video editor. The Rust core owns
project, timeline, task sheet, and review state; the desktop UI, CLI, MCP
server, and control server send commands to that core.

The current checkout's generated catalogue contains **492 shipped commands
across 13 profiles**. The latest tagged release, [v0.1.2-alpha.7](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.2-alpha.7),
records 489 commands; the current checkout includes later task sheet work.
The generated [command reference](docs/COMMANDS.md) and [parity report](docs/PARITY.md)
are the sources for current command counts and audited coverage. All three
audited parity tiers report 100% effective coverage, excluding documented
`na` divergences; this does not mean every feature is complete or that full
reference-editor parity is a beta goal.

The latest tagged alpha is experimental and Linux-focused. The current
checkout contains work beyond that release tag, so use the [releases page](https://github.com/zvirox/easy-edits-pro/releases)
for downloadable builds and the [changelog](CHANGELOG.md) for tagged-release
contents.

## What works in the current checkout

- Create, open, save, and recover versioned editor projects; import local
  media, inspect it, organise it, and place clips on the timeline.
- Edit with timeline selection, movement, split, trim, scrubbing, zoom,
  transitions, titles, captions, markers, and Rust-backed undo/redo for shipped
  commands.
- Preview source and program media with playback, frame stepping, and timeline
  transport controls. Render MP4 or WebM through GStreamer; codec availability
  depends on the installed GStreamer plugins.
- Analyze media with contact sheets, shot detection, semantic search,
  audio/video telemetry, word-level transcription and script sync, plus
  measured matting, depth, and motion tracking.
- Work across media, editing, color, keyframe, node-graph, audio, and delivery
  areas. The command catalogue and parity report describe the shipped surface.
- Use `saturn-mcp` to inspect projects and sequences, search media, inspect
  placements, run editor commands, and manage asynchronous render jobs. Its
  desktop bridge uses a Unix socket.
- Build task sheets with validation, human plan approval, questions, review
  notes and rounds, delivery packages, checksums, and provenance.
- Run three deterministic task sheet recipes for podcast clips, talking-head
  tutorials, and webinar recaps. The synthetic Capture Event Track generator
  supports repeatable tutorial-recipe checks.
- Supervise agent sessions with Stop, branch review, conflict-aware reverts,
  and an explicit demo mode. The AF6.1 end-to-end gate is recorded as closed in
  the [progress checklist](docs/AI_NATIVE_PLAN_CHECKLIST.md).

The current task sheet implementation is TS0–TS6. The progress checklist also
tracks agent-first phases, verification work, and the remaining roadmap.

## Known limitations and work in progress

- This is alpha software for Linux. Windows and macOS packages are not built,
  and package compatibility still needs reports from non-development machines.
- Hardware encoding has not been verified. Export format and codec support
  depend on the GStreamer plugins available on the machine.
- The desktop MCP bridge uses a Unix socket. There is no Windows bridge.
- Text-to-speech and speaker diarization are not implemented. Transcription is
  available; speech separation for dialogue, music, ambience, and effects is
  deferred until a verified, license-cleared model is available.
- Proxy attachments exist, but proxy playback and cache regeneration are
  unfinished.
- EEP-Bench currently runs a scripted deterministic conformance suite. Its
  real-agent `--agent` evaluation mode is planned; scripted results are not
  model-evaluation results.
- Task Sheet web API, Web Desk, runner, SSO, and the shared agent-contract
  crate remain planned. See the live [AI-native progress checklist](docs/AI_NATIVE_PLAN_CHECKLIST.md)
  for current gates and remaining work.

See [distribution and releases](docs/distribution.md) for package policy and
recorded installation evidence. The [delivery plan](docs/planning.md) records
milestones and the evidence required before work is called complete.

## Architecture

~~~mermaid
flowchart LR
  UI[React editor and review UI] --> IPC[Tauri invoke commands]
  IPC --> Core[Rust core: projects, tasks, and review]
  Core --> Render[saturn-render / GStreamer]
  Core --> Automation[CLI · MCP · control server]
  Render --> Output[Preview and media output]
~~~

The Rust core owns project state and editing history. UI and automation clients
submit commands and render returned state. See the [architecture guide](docs/architecture.md)
for module-level data flow.

## Install and try it

Download a package and its checksums from the [GitHub releases page](https://github.com/zvirox/easy-edits-pro/releases),
then install the package for your Linux distribution and launch **Easy Edits
Pro** from your application menu (binary: `saturn-tauri`):

~~~bash
# Debian/Ubuntu and derivatives
sudo apt install ./Easy.Edits.Pro_<version>_amd64.deb
# Fedora/RPM-based distributions
sudo dnf install ./Easy.Edits.Pro_<version>_amd64.rpm
~~~

See [distribution and releases](docs/distribution.md) for Flatpak details and
the package acceptance workflow.

To generate the maintained sample projects (split, titles, captions, color
grade, multicam), then open the `.saturn` files in the app:

~~~bash
python3 tests/native/sample_projects.py --build samples-out
~~~

## Development setup

### Requirements

- Linux with a current Rust toolchain, Node.js 20+, and pnpm.
- Tauri 2 and GStreamer development dependencies. See
  [CONTRIBUTING.md](CONTRIBUTING.md) for Debian, Arch, and Fedora package lists.
- Playwright Chromium for the mocked browser smoke suite.

### Run the desktop app

~~~bash
git clone https://github.com/zvirox/easy-edits-pro.git
cd easy-edits-pro/apps/saturn-tauri
pnpm install
pnpm tauri dev
~~~

### Verify a change

~~~bash
cargo test --workspace
pnpm --dir apps/saturn-tauri build
pnpm --dir apps/saturn-tauri test
pnpm --dir apps/saturn-tauri check:glass
pnpm --dir apps/saturn-tauri check:commands
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
pnpm --dir apps/saturn-tauri test:e2e
~~~

The browser smoke suite uses mocked Tauri IPC and synthetic video. It does not
verify the native desktop bridge, GStreamer, file dialogs, rendering, or an
installed package. For those areas, follow the native workflows in
[docs/native-workflow.md](docs/native-workflow.md) and
[docs/distribution.md](docs/distribution.md).

## Automation

### CLI

~~~bash
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- inspect /path/to/project.saturn
~~~

The agent-oriented CLI harness is documented in
[agent-harness/README.md](agent-harness/README.md).

### MCP

~~~bash
cargo run -p saturn-mcp -- --project /path/to/project.saturn
cargo run -p saturn-mcp -- --bridge
~~~

See [crates/saturn-mcp/README.md](crates/saturn-mcp/README.md) for available
tools and the connection model.

## Releases and feedback

The [release workflow](docs/RELEASES.md) documents release gates and publishing.
For current alpha packages, checksums, and tagged changes, use the [releases page](https://github.com/zvirox/easy-edits-pro/releases)
and [changelog](CHANGELOG.md).

For an alpha report, include the release version, distribution and desktop
session, imported media codec, reproduction steps, and **Help → Copy System
Diagnostics** output. Report it through the GitHub issue tracker. This feedback
guides alpha and beta work and captures results that cannot be tested on the
development laptop.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. Keep project
state in Rust, keep source files at or below 800 lines, preserve public Tauri
invoke names and project-file compatibility, and describe only verified
behaviour in docs and release notes.

### Where help is most useful

| Area | Examples | Difficulty |
|---|---|---|
| Testing and bug reports | Install the alpha on your distro and report what breaks | Easy |
| Documentation | Clarify setup and workflows | Easy |
| UI polish | Missing icons, truncated labels, keyboard shortcuts | Easy to medium |
| Test coverage | Regression tests for commands and importers | Medium |
| Packaging | Verify packages on more Linux distributions | Medium |
| Rendering and audio | GStreamer/GES performance and DSP | Hard |
| Agent tooling | MCP tools, task sheet recipes, and conformance tests | Medium to hard |

Start with [CONTRIBUTING.md](CONTRIBUTING.md), the [roadmap](ROADMAP.md), and
issues labelled `good first issue` or `help wanted`. Questions are welcome in
[Discussions](https://github.com/zvirox/easy-edits-pro/discussions); see also
[SUPPORT.md](SUPPORT.md) and [GOVERNANCE.md](GOVERNANCE.md).
