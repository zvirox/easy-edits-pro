# Easy Edits Pro — Delivery Plan

**Updated:** 2026-10-23

**Status source:** the command catalogue generates [PARITY.md](PARITY.md);
GitHub Actions and the release page are the source for remote build and package
status.

This is the single forward-looking plan for Easy Edits Pro. It replaces the former roadmap, Glass phase plan, and FilmCraft-parity roadmap. Architecture, spike evidence, generated command data, and release notes remain in their own documents.

## 1. Scope and decisions

Easy Edits Pro is an alpha Linux desktop video editor. The work now focuses on making a reliable import → edit → save → render workflow on the development laptop, then using alpha and beta releases to collect real-world feedback.

### Decisions

| Decision | Rationale |
|---|---|
| Validate on this laptop only | No other hardware is available. A compatibility matrix would create unverified work and claims. |
| Release early alpha and beta builds | Releases provide a controlled way to receive reports from systems we cannot test locally. |
| Treat hardware encoding as optional | The verified host path is software H.264. Encoder detection must retain a software fallback and never be described as verified without an output check. |
| Keep the Rust core authoritative | UI, CLI, MCP, and control-server actions execute the same project commands; client code renders snapshots rather than owning a competing project model. |
| Keep the command catalogue truthful | A command is shipped only with a UI affordance, a focused test, and `shipped: true`. Unfinished actions return a real error. |

### Explicit exclusions

- No new cross-GPU, cross-compositor, or cross-distribution testing plan.
- No hardware-encoding certification work.
- No claim that a package or workflow works beyond the evidence recorded for it.

Feedback from future alpha and beta users is evidence for prioritisation. It is not a substitute for a local regression test when the reported issue can be reproduced on this laptop.

## 2. Current baseline

### Delivered foundations

- The Tauri shell, Rust command bridge, project persistence, recovery path, media server, and split frontend feature/workspace structure are present.
- The editor has project creation/opening, media import, timeline placement and editing interactions, source/program preview controls, project save/reopen, MP4/WebM export controls, and the main Media, Edit, Color, Fairlight, Keyframes, Fusion, and Deliver surfaces.
- `saturn-mcp` can inspect and mutate the engine, search project media, inspect placements, and start, observe, or cancel render jobs. The running-desktop bridge is a same-user Unix socket.
- The first alpha, [`v0.1.0-alpha.1`](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.0-alpha.1), published `.deb`, `.rpm`, and SHA-256 assets successfully.
- The generated catalogue currently records **230 commands**, **119 shipped**, and **111 planned**. FilmCraft coverage is **64/443 Tier 1**, **15/207 Tier 2**, and **0/17 Tier 3** commands. See [PARITY.md](PARITY.md).

### Evidence and limits

- A native workflow on this laptop exercised a saved project, real media, editing commands, MCP desktop-bridge operations, cancellation, and a small H.264/AAC render. That output used the x264 software encoder.
- Local UI build, unit, command, contrast, and source-size checks have passed in the latest recorded verification. The local browser E2E runner is blocked by this environment's Chromium sandbox, so it is not native-desktop proof.
- At the time this plan was updated, [GitHub Actions run #21](https://github.com/zvirox/easy-edits-pro/actions/runs/39909028312) had passed its Rust, frontend build/unit, style, catalogue, parity, size, and Tauri-backend steps. Its Python harness job had failed in `test_media_bins_and_subtitles`; its Playwright step was still running. Main is therefore **not green**. Treat the Actions page as newer evidence than this dated snapshot.
- `.deb` and `.rpm` assets have been built and published, but no package install and launch result is yet recorded. Flatpak is the intended supported delivery route, not a completed artifact.

## 3. Delivery sequence

Work is ordered to make each release more trustworthy before expanding the feature surface. Each item should land as a small, independently reviewable change with its own regression evidence.

### M0 — Restore a reproducible baseline

**Goal:** make `main` green and make documentation reflect only current facts.

1. Reproduce and fix the Python harness media-bin/subtitle `IndexError`. Verify the command returns data or a structured backend error; it must never turn missing state into a success or an internal error. — **done** (`24e876e`)
2. Stabilise the Playwright smoke selector and make its waiting conditions explicit. Keep browser smoke tests mocked; do not represent them as a native-engine or codec check. — **done** (`30e2b5b`)
3. Add a native workflow test script for this laptop: create/open a project, import the maintained local fixture, make a timeline edit, save/reopen, render, and inspect the output with `ffprobe`. — **done** (`af2089c`; evidence in [native-workflow.md](native-workflow.md))
4. Clear the existing source-size warning backlog: split the files above the 800-line hard limit by domain, beginning with `interop.rs`, Fairlight, and Keyframes. Do not raise the limit. — **done** (`25f725f`, `cbce9db`)
5. Keep generated command and parity material in sync after catalogue changes. — **standing rule; current docs in sync** (`39917362806` green)

**Exit evidence:** CI is green; the laptop workflow script passes; generated documents have no drift; the source-size gate passes without warnings.

### M1 — Make the editor workflow dependable

**Goal:** a creator can complete a small edit on the development laptop without guessing which controls work.

1. **Project and media reliability:** missing-media relink behaviour, recovery restore, error states for unsupported codecs, proxy lifecycle, and clear import status.
2. **Timeline and preview:** preserve pointer drag/drop, playhead scrubbing, zoom, trimming, split, move, ripple, transitions, selection, and undo/redo through save/reopen tests.
3. **Export truthfulness:** connect the visible export queue to real job state, expose honest preset/range/codec errors, and verify cancellation removes partial output. Retain software H.264 as the tested path.
4. **MCP lifecycle:** finish reconnect and desktop-lifecycle coverage for the Unix bridge; add still-frame preview only when the tool returns a real image payload rather than a source-path placeholder.
5. **Safety:** cover recovery, media allow-listing, SSRF protection, and bridge permission/error cases with focused tests.

**Exit evidence:** the laptop workflow passes from a clean project directory; MCP performs the same workflow against the running app; expected errors are visible in the UI and structured for automation.

### M2 — Alpha feedback loop and release readiness

**Goal:** turn user reports into well-scoped fixes without inventing hardware compatibility claims.

1. Ship a new alpha only after M0 gates are green and a release note lists tested workflow, package limitations, and known gaps.
2. Ask testers to attach the app's diagnostics, codec details, reproduction steps, and the release version to GitHub issues. Categorise reports by import, preview, timeline, audio, export, MCP, or package installation.
3. Triage reports on a fixed cadence. Reproduce what is possible on this laptop, add a regression test, then publish the fix in the next alpha.
4. Record package **install + launch** results only when a real result is supplied. Do not infer compatibility from a successful package build.
5. Prepare the Flatpak manifest and CI artifact only after the app's normal laptop workflow is reliable. Its first install and launch result is required before it becomes the supported release artifact.

**Exit evidence:** a release has a green CI run, attached checksums, a verified laptop workflow, actionable feedback instructions, and an honest known-issues section. Beta candidacy requires several alphas without a data-loss or unrecoverable-edit regression.

### M3 — Tier 1 editing parity for Beta 1

**Goal:** finish the practical core before pursuing breadth.

| Family | Work still needed | Completion evidence |
|---|---|---|
| Project, file, sequence | project manager, relink, preferences, sequence lifecycle | save/open/migrate and recovery fixtures |
| Media | bins, labels/metadata, proxy states, search, missing-media handling | import/relink/bin regression set |
| Timeline, clip, edit, trim, playhead | complete the remaining catalogue gaps and robust pointer workflows | real-media edit/save/reopen/render scenario |
| Markers and captions | marker navigation/CSV paths and caption editing/import/export gaps | round-trip fixture and UI interaction test |
| History and shortcuts | command-to-UI coverage, grouped undo/redo, preference persistence | engine + shortcut/UI tests |
| Export and interchange | queue/presets, EDL/FCPXML/OTIO project fidelity | verified output and round-trip fixtures |

**Exit evidence:** Tier 1 commands reported as shipped have UI affordances, focused tests, and an end-to-end real-media test where applicable. The exact percentage is generated, never hand-written.

### M4 — Quality and advanced editing after Beta 1

**Goal:** improve creative controls once core editing is dependable.

1. **Fairlight:** validate Read-mode volume/pan playback end to end; complete other automation modes plus clip gain/fades, essential-sound/ducking workflow, and insert DSP. Keep plugin slots explicitly metadata-only until a tested native host exists.
2. **Color and effects:** finish advanced curves, HSL, LUT/preset integration, scopes, effect parameter editing/reordering, and keyframe handle editing.
3. **Fusion and graphics:** replace stub actions with real node graph edits, text/title workflow, and export-aware rendering.
4. **Multicam and speech:** source monitor workflow, sync/switch/flatten, transcript editing, and proxies, each backed by fixtures.
5. **Glass/UI quality:** finish panel layout tests, responsive behaviour, keyboard accessibility, contrast, and visual baselines. Media surfaces remain unblurred and styling uses design tokens.

**Exit evidence:** each advanced control changes the Rust project model, persists, previews or renders where promised, and fails visibly when unsupported.

### M5 — Later capabilities

TTS, media browsing, collaboration, scripting, richer interchange, VST3 hosting, and non-Unix desktop bridge support remain later work. They need a separate proposal with scope, file-format compatibility, security constraints, and test evidence before they enter an alpha milestone.

## 4. Engineering rules for every work item

1. Preserve public Tauri `invoke` names and the project-file format unless a migration plus fixture proves compatibility.
2. Keep project state in Rust. UI, CLI, MCP, and control-server layers submit commands and render returned state.
3. Keep source files at or below 800 lines. Split by domain, not by arbitrary helper extraction.
4. Use design tokens, named transition properties, and the shared `.glass` surfaces. Never blur video or audio-render surfaces.
5. Do not turn an unfinished handler into a pretend success. Return a typed, actionable error.
6. Update documentation in the same change as the verified behaviour. Generated files are updated by their scripts, never edited by hand.

## 5. Required verification

Run the relevant checks before a feature is described as complete:

```bash
cargo test --workspace
pnpm --dir apps/saturn-tauri build
pnpm --dir apps/saturn-tauri test
pnpm --dir apps/saturn-tauri check:glass
pnpm --dir apps/saturn-tauri check:commands
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
pnpm --dir apps/saturn-tauri test:e2e
```

For media, MCP, packaging, or native-window changes, add the focused laptop workflow evidence. A mocked browser test does not prove the desktop bridge, GStreamer decoding, file dialogs, rendering, or installed package behaviour.

## 6. Documentation ownership

| Document | Purpose |
|---|---|
| This file | priorities, scope boundaries, milestones, and completion rules |
| [architecture.md](architecture.md) | current module and data-flow map |
| [PARITY.md](PARITY.md) | generated command coverage |
| [COMMANDS.md](COMMANDS.md) | generated command reference |
| [distribution.md](distribution.md) | package and release evidence |
| [native-workflow.md](native-workflow.md) | real-engine laptop workflow evidence (M0.3) |
| [glass-spike.md](glass-spike.md) | measured single-laptop rendering evidence |
| [design-qa.md](design-qa.md) | UI quality evidence and open UI work |

The former `ROADMAP.md`, `GLASS_UI_PHASED_PLAN.md`, and `filmcraft-parity-roadmap.md` were removed because their timelines and hardware plans conflicted with this plan. `phase5-text-stack.md` remains a technical decision record, not a roadmap.
