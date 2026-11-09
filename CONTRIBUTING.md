# Contributing to Easy Edits Pro

Thank you for your interest in contributing to **Easy Edits Pro**! We welcome code contributions, bug reports, feature suggestions, documentation improvements, and translations from the open-source community.

---

## Your first contribution in 6 steps

1. Read the README sections "Project status" and "Known limitations", then pick
   an issue labelled `good first issue` (or see [ROADMAP.md](ROADMAP.md)).
2. Comment "I'd like to work on this". A maintainer will assign it, usually
   within a few days. Unclaimed issues return to the pool after 14 days.
3. Install the prerequisites below and run `scripts/doctor.sh` to check your
   setup.
4. Make a small change on a branch and run the checks in the
   [Pull Request Checklist](#pull-request-checklist).
5. Commit with `git commit -s` (DCO). Accept the [CLA](CLA.md) once, on your
   first non-trivial code PR.
6. Open a pull request using the template. We aim to reply within 7 days; this
   is a small team, so please be patient and ping once after 7 days.

Not sure where to start, or stuck? Ask in
[Discussions](https://github.com/zvirox/easy-edits-pro/discussions) or see
[SUPPORT.md](SUPPORT.md). No question is too small.

---

## Code of Conduct

All contributors and participants agree to uphold the standards outlined in our [Code of Conduct](CODE_OF_CONDUCT.md). Please treat fellow contributors with respect and empathy.

---

## Developer Certificate of Origin (DCO)

All code contributions to Easy Edits Pro must be accompanied by a Developer Certificate of Origin (DCO) sign-off in the git commit message. This certifies that you wrote the code or have the right to submit it under the project's AGPL-3.0-or-later license.

To sign off on your commits, use the `-s` flag with `git commit`:
```bash
git commit -s -m "feat(timeline): add ripple trim shortcut"
```
This appends a line like:
```text
Signed-off-by: Your Name <your.email@example.com>
```

---

## Contributor Licence Agreement (CLA)

Because Easy Edits Pro is dual licensed (open source under
[AGPL-3.0-or-later](LICENSE) plus a
[commercial licence](LICENSE-COMMERCIAL.md) — see
[LICENSING.md](LICENSING.md)), every code contributor must accept the
[CLA](CLA.md) so the project can keep offering both licences. You keep your
copyright; you grant the project a licence, not an assignment.

- **Sign once**: accepting the CLA is a one-time action per contributor. The
  CLA Assistant bot asks for acceptance on your first non-trivial pull
  request (CI enforces it while that is being configured via the
  statement-based flow below).
- **Until the bot is live**: add a comment on your pull request containing
  exactly: `I accept the terms of CLA.md` (with your Git identity). A
  maintainer records the acceptance.
- Documentation-only and typo-only changes do not need a signed CLA, but do
  need the DCO sign-off above.
- If you cannot or will not sign the CLA, please open an issue describing the
  change; maintainers will implement it themselves rather than merge your
  code. Licensing questions: **info@zvirox.in** / **zvirox@proton.me**.

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
the cascade. Keep `.rs`, `.ts`, `.tsx`, and `.css` source files at or below
800 lines. Run `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`
before submitting.

See [`docs/architecture.md`](docs/architecture.md) for the current module map
and data flow.

---

## Pull Request Checklist

Before submitting a Pull Request, ensure that:
1. `cargo test --workspace` passes cleanly.
2. `pnpm --dir apps/saturn-tauri build` completes with 0 type errors.
3. `cargo check --manifest-path apps/saturn-tauri/Cargo.toml` succeeds.
4. All commits include a DCO sign-off (`git commit -s`).
5. If this is your first non-trivial code PR, the CLA in [CLA.md](CLA.md) is
   accepted (bot signature or a PR comment stating `I accept the terms of
   CLA.md`).
6. New features include appropriate unit tests or regression tests.
7. `pnpm --dir apps/saturn-tauri test`, `check:glass`, `check:commands`, and
   `check:source-size --max-lines=800` pass when the change touches the UI or
   command catalogue.
8. A shipped command has a UI affordance, a test, and a catalogue entry marked
   `shipped: true`; incomplete handlers return an error rather than success.
9. Every new source file (`.rs`, `.ts`, `.tsx`, `.js`, `.mjs`, `.py`, `.sh`,
   `.css`) has the SPDX licence header. Run `node scripts/license-headers.mjs apply` from the
   repository root to add it, and `node scripts/license-headers.mjs check` to
   verify (CI runs the check).
10. New third-party dependencies or copied material are recorded in
    [NOTICE](NOTICE) and pass `cargo deny` (see "Provenance" below).

---

## Provenance and third-party code

Study other projects freely, but do not paste their code. Only
MIT / Apache-2.0 / BSD / ISC / Zlib material may be reused, and only with a
`NOTICE` entry in the same change. Never copy GPL, AGPL or unlicensed code, and
never add a model or asset without a row in
[docs/model-licences.md](docs/model-licences.md). The full rules are in
[AGENTS.md](AGENTS.md).

---

## AI-assisted contributions

Much of this codebase is written with AI coding agents, so AI assistance is
welcome under the same rules as any other contribution:

- You are responsible for everything you submit. Read it, run it, and be able
  to explain it in review.
- Say in the PR which tool you used and what you checked yourself.
- No pasted third-party code, and no code you do not have the right to license
  under the CLA. The DCO sign-off and the CLA apply to AI-assisted work in the
  same way.
- Agents working on this repository must follow [AGENTS.md](AGENTS.md).
- Large, unreviewed bulk or machine-generated PRs may be closed without review.
  Small, tested, focused changes are always preferred.
