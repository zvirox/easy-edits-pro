# Easy Edits Pro

**An alpha Linux desktop video editor built with Rust, Tauri, React, and GStreamer.**

[Documentation](docs/planning.md) · [Architecture](docs/architecture.md) · [Latest alpha release](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.0-alpha.1)

## Project status

Easy Edits Pro is in active alpha development. It has a Rust-owned project model,
a Tauri desktop shell, a multi-workspace editor, and an automation stack shared
by the desktop app, CLI, MCP server, and control server.

The current generated catalogue contains **230 commands**: **119 shipped** and
**111 planned**. See [the generated parity report](docs/PARITY.md) for the
current FilmCraft comparison; it is the source of truth for coverage numbers.

The first alpha release,
[v0.1.0-alpha.1](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.0-alpha.1),
contains .deb, .rpm, and SHA-256 checksum assets. They are experimental
package builds. A package build is not an install or hardware-compatibility
claim.

## What currently works

- Create, open, save, and recover versioned editor projects.
- Import local media, inspect its metadata, organise project media, and place
  clips on the timeline.
- Use timeline selection, movement, split, trimming, playhead scrubbing, zoom,
  transitions, titles, captions, markers, and Rust-backed undo/redo where the
  corresponding command is marked shipped.
- Preview source and program media in the desktop editor, with playback,
  frame-stepping, and timeline transport controls.
- Render MP4 or WebM through GStreamer. The confirmed render path on the
  development laptop is software H.264/AAC through x264.
- Use the Media, Edit, Color, Fairlight, Keyframes, Fusion, and Deliver
  workspaces. Some advanced controls in these surfaces are intentionally still
  marked incomplete.
- Use saturn-mcp to inspect projects and sequences, search media, inspect
  placements, execute real editor commands, and manage asynchronous render jobs.
  On Unix desktops, --bridge works with the project open in the app.

## Known limitations

- Main is not yet green: the latest recorded GitHub CI run has a failing Python
  harness media-bin/subtitle test, and its browser smoke result must be checked
  in Actions before treating it as a release-quality build.
- The desktop MCP bridge uses a Unix socket; it has no Windows implementation.
- media_preview reports asset metadata, source path, and placements. It does
  not extract a still frame.
- Hardware encoder detection is implemented, but hardware-encoded output has not
  been verified on this laptop. Software encoding is the tested fallback.
- Fairlight plugin slots store metadata only; there is no native VST3 host.
  Denoise is currently a high-pass filter, and full render-DSP parity is open.
- Fusion actions, advanced Color controls, effect parameter/reordering controls,
  keyframe handle editing, and the export queue need further implementation.
- Packages have been built but no install-and-launch evidence is recorded yet.
  Flatpak is the intended supported distribution route, but its manifest and
  artifact are not complete.

See [the delivery plan](docs/planning.md) for milestones, release feedback, and
the evidence required before a feature is called complete.

## Architecture

~~~mermaid
flowchart LR
  UI[React workspaces] --> IPC[Tauri invoke commands]
  IPC --> Core[saturn-core project engine]
  Core --> Render[saturn-render / GStreamer]
  Core --> Automation[CLI · MCP · control server]
  Render --> Output[Preview and media output]
~~~

The Rust core owns project state and editing history. UI and automation clients
submit commands and render returned project state. The module-level data flow is
documented in [docs/architecture.md](docs/architecture.md).

## Development setup

### Requirements

- Linux with a current Rust toolchain, Node.js 20+, and pnpm.
- Tauri 2 and GStreamer development dependencies. See
  [CONTRIBUTING.md](CONTRIBUTING.md) for Debian, Arch, and Fedora package lists.
- Playwright Chromium only for the mocked browser smoke suite.

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

The browser smoke suite uses mocked Tauri IPC and a synthetic video. It does
not verify the native desktop bridge, GStreamer, file dialogs, rendering, or
an installed package. For changes to those areas, follow the laptop workflow
in [docs/planning.md](docs/planning.md).

## Automation

### CLI

~~~bash
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- inspect /path/to/project.eep
~~~

The agent-oriented CLI harness is documented in
[agent-harness/README.md](agent-harness/README.md).

### MCP

~~~bash
cargo run -p saturn-mcp -- --project /path/to/project.eep
cargo run -p saturn-mcp -- --bridge
~~~

See [crates/saturn-mcp/README.md](crates/saturn-mcp/README.md) for the available
tools and connection model.

## Releases and feedback

Push an alpha tag such as v0.1.0-alpha.2 to build and attach .deb, .rpm,
and checksum assets to a GitHub prerelease. The packaging workflow is documented
in [docs/distribution.md](docs/distribution.md).

For an alpha report, include the release version, distribution and desktop
session, imported media codec, reproduction steps, and **Help → Copy System
Diagnostics** output. Report it through the GitHub issue tracker. This feedback
guides future alpha and beta work; it is how the project gathers information
that cannot be tested on the development laptop.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. Keep project
state in Rust, keep source files at or below 800 lines, preserve public Tauri
invoke names and project-file compatibility, and describe only verified
behaviour in docs and release notes.
