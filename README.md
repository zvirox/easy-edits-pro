# Easy Edits Pro

**An alpha Linux desktop video editor built with Rust, Tauri, React, and GStreamer.**

[Documentation](docs/planning.md) · [Architecture](docs/architecture.md) · [Releases](https://github.com/zvirox/easy-edits-pro/releases)

## Project status

Easy Edits Pro is in active alpha development, moving toward a first beta. It
has a Rust-owned project model, a Tauri desktop shell, a multi-workspace
editor, and an automation stack shared by the desktop app, CLI, MCP server,
and control server.

The generated catalogue contains **440 commands, all shipped** (each with a UI
affordance and a test). The reference-editor parity audits for all three tiers report
**100% coverage** (documented `na` divergences excluded); see [the generated
parity report](docs/PARITY.md) for the current numbers - it is the source of
truth for coverage.

The most recent release is
[v0.1.0-alpha.26](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.0-alpha.26),
with .deb, .rpm, and SHA-256 checksum assets. Packages are experimental: a
package build is not an install or hardware-compatibility claim. The recorded
install-and-launch evidence so far is an automated extracted-prefix acceptance
run of the alpha.26 .deb on the development laptop (see *Known limitations*).

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

- Hardware encoder detection is implemented, but hardware-encoded output has
  not been verified on this laptop. Software encoding is the tested fallback.
- The desktop MCP bridge uses a Unix socket; it has no Windows implementation.
- media_preview reports asset metadata, source path, and placements. It does
  not extract a still frame.
- Motion tracking: the setup (track point, region) persists on the clip, but
  analysis runs during the render pass; the preview-side analyzer reports
  simulated points, not real per-frame tracking.
- Proxies are single-profile 1280x720 VP9 WebM preview attachments per asset;
  there is no preset system and no auto-regeneration over pruned attachments.
- Text-to-speech is not included: the speech surface is transcription-only
  (`speech.transcribe_to_subtitles`); captions are authored or imported.
- Fairlight plugin slots store metadata only; there is no native VST3 host.
  Denoise is currently a high-pass filter, and full render-DSP parity is open.
- Fusion actions, advanced Color controls, effect parameter/reordering controls,
  keyframe handle editing, and the export queue need further implementation.
- Install evidence is honest but narrow: the alpha.26 .deb passed the scripted
  extracted-prefix acceptance workflow (lint, launch, edit, render) on one
  machine, which is **not** a dependency-managed install proof - the host is
  Arch with no dpkg. The .rpm has passed lint but has never been installed
  anywhere; the container-based real-install mode of the acceptance runbook
  awaits a container daemon. The Flatpak spike passed on GNOME 49 (launch,
  edit, H.264 render) and the route is planned for Beta 1, but no Flatpak
  build has shipped in a release yet. Record your own result per
  [docs/distribution.md](docs/distribution.md).
- `.saturn` project files embed absolute media paths; moving a project or its
  media shows clips offline until they are relinked.

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

## Install and try it (alpha packages)

Grab the newest [release](https://github.com/zvirox/easy-edits-pro/releases)
assets, install the package for your distribution, and launch **Easy Edits Pro**
from your application menu (binary: `saturn-tauri`):

~~~bash
# Debian/Ubuntu and derivatives
sudo apt install ./Easy.Edits.Pro_<version>_amd64.deb
# Fedora/RPM-based distributions
sudo dnf install ./Easy.Edits.Pro_<version>_amd64.rpm
~~~

To see what the maintained sample projects demonstrate (split, titles,
captions, colour grade, multicam), generate them locally and open the `.saturn`
files in the app:

~~~bash
python3 tests/native/sample_projects.py --build samples-out
~~~

Maintainers verifying a packaged build should run the scripted acceptance
workflow instead of trusting a successful build - see the beta acceptance
runbook in [docs/distribution.md](docs/distribution.md).

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
an installed package. For changes to those areas, follow the native workflows
in [docs/native-workflow.md](docs/native-workflow.md) and
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
