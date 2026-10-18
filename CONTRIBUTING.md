# Contributing to Easy Edits Pro

Thank you for your interest in contributing to **Easy Edits Pro**! We welcome code contributions, bug reports, feature suggestions, documentation improvements, and translations from the open-source community.

---

## Code of Conduct

All contributors and participants agree to uphold the standards outlined in our [Code of Conduct](CODE_OF_CONDUCT.md). Please treat fellow contributors with respect and empathy.

---

## Developer Certificate of Origin (DCO)

All code contributions to Easy Edits Pro must be accompanied by a Developer Certificate of Origin (DCO) sign-off in the git commit message. This certifies that you wrote the code or have the right to submit it under the project's GPL-3.0-or-later license.

To sign off on your commits, use the `-s` flag with `git commit`:
```bash
git commit -s -m "feat(timeline): add ripple trim shortcut"
```
This appends a line like:
```text
Signed-off-by: Your Name <your.email@example.com>
```

---

## Development Prerequisites

Easy Edits Pro requires the following tools and system libraries:

1. **Rust toolchain**: 1.84+ (edition 2024 support)
   ```bash
   rustup update stable
   ```
2. **Node.js**: v20+ or v22+
3. **pnpm**: v9+ or v10+
   ```bash
   corepack enable
   corepack prepare pnpm@latest --activate
   ```
4. **GStreamer Development Libraries**:
   - Ubuntu / Debian:
     ```bash
     sudo apt-get install -y \
       libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev \
       gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
       gstreamer1.0-libav libges-1.0-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
     ```
   - Arch Linux:
     ```bash
     sudo pacman -S gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-editing-services webkit2gtk-4.1
     ```
   - Fedora:
     ```bash
     sudo dnf install -y gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-plugins-bad-free-devel gst-editing-services-devel webkit2gtk4.1-devel
     ```

---

## Building and Running

### 1. Rust Core Engine & CLI
```bash
# Run all unit and workflow tests across the workspace
cargo test --workspace

# Run CLI headless tests
cargo run -p saturn-cli -- --help
```

### 2. Frontend & Tauri Desktop Shell
```bash
cd apps/saturn-tauri

# Install dependencies
pnpm install

# Run the frontend dev server standalone
pnpm dev

# Run full desktop app in development mode
pnpm tauri dev

# Typecheck and build the UI bundle
pnpm build
```

---

## Repository Structure

- [`crates/saturn-core`](crates/saturn-core): Rational integer timebase (`TICKS_PER_SECOND = 254_016_000_000`), transactional project mutations, undo/redo undo stacks, and interchange exporters.
- [`crates/saturn-render`](crates/saturn-render): GStreamer pipeline builders, GES timeline export, color wheels math, 3D LUT parser, and motion tracking.
- [`crates/saturn-cli`](crates/saturn-cli): Headless command-line runner and project verification tool.
- [`apps/saturn-tauri`](apps/saturn-tauri): Tauri 2 backend bridge, secure media streaming server, panic crash logger, and React 19 UI suite.

## Architecture and file boundaries

The Tauri bridge lives in `apps/saturn-tauri/src`. Keep `main.rs` focused on
application setup and shared helpers; add new Tauri commands to the closest
module under `src/commands/` and re-export them from `commands/mod.rs` so the
invoke registration stays explicit. Keep network URL validation in `src/net.rs`
and project media allow-list registration in `register_project_media`.

The React UI lives in `apps/saturn-tauri/ui/src`. Put reusable UI in
`components/`, feature entry points in `ui/src/features/<domain>/`, established
screens in `workspaces/`, application context and hooks in `app/`, and pure
logic in `utils/` or the relevant engine module. Keep CSS imports in
`style.css` → `styles/index.css` in their listed order because order determines
the cascade. Keep new `.rs`, `.ts`, `.tsx`, and `.css` source files at or below 800 lines
(target under 500 where practical). Run `pnpm check:source-size` before
submitting. The check lists the small, explicit legacy extraction backlog; do
not add new exceptions without documenting the reason and follow-up.

See [`docs/architecture.md`](docs/architecture.md) for the current module map
and data flow.

---

## Pull Request Checklist

Before submitting a Pull Request, ensure that:
1. `cargo test --workspace` passes cleanly.
2. `pnpm --dir apps/saturn-tauri build` completes with 0 type errors.
3. `cargo check --manifest-path apps/saturn-tauri/Cargo.toml` succeeds.
4. All commits include a DCO sign-off (`git commit -s`).
5. New features include appropriate unit tests or regression tests.
