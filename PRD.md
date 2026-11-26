# Easy Edits Pro — Product Requirements Document (PRD)

**Status:** Beta (`v0.1.3-beta.5`) · **Primary Platform:** Linux (Wayland & X11 native; Tier 2 macOS/Windows cross-platform portability in progress)
**License:** Dual-licensed — GNU Affero General Public License v3.0 (`AGPL-3.0-or-later`) or Commercial (`LICENSE-COMMERCIAL.md`)

---

## 1. Product Vision & Target Users

**Easy Edits Pro (EEP)** is a fast, local-first, non-destructive multitrack video editor built natively for Linux creators, engineers, and studio teams. Source footage never leaves the user's machine: all timeline state, undo/redo history, speech transcription, semantic search, color grading, and GStreamer rendering execute locally on user hardware, with optional human-in-the-loop AI co-editing through MCP, Task Sheets, Web Desk, and Starship Control.

### Primary Personas
1. **Desktop Creators & Technical Educators:** Record screen walkthroughs, talking-head explainers, and tutorials needing fast cuts, ripple edits, silence removal, synchronized captions, color correction, and local H.264/WebM export without cloud subscriptions.
2. **Agent-Assisted Editors (Co-Editing):** Delegate rough cuts, chapter markers, beat-grid quantization, J/L split trims, and colour shot matching to local/BYO agents operating on isolated copy-on-write branches under strict Tier 2 approval and instant `Ctrl+Z` undo.
3. **Self-Hosted Studio Teams (Web Desk):** Submit `tasksheet/1` work orders from curated recipe templates, monitor worker runs, answer clarifying agent questions, inspect raw proxy renders with timecoded review notes, and approve deliverables.

---

## 2. Core Goals & Non-Goals

### Goals
- **G1. Rust-Owned Deterministic State:** Every timeline mutation, transaction, CoW branch (`TimelineBranch`), viewport selection, and undo/redo entry executes cleanly inside `saturn-core` with rational tick arithmetic (`TICKS_PER_SECOND = 254,016,000,000`) and checked overflow protection.
- **G2. Locked Linear Matte UI:** High-density solid dark/light matte editing workspace with custom 40 px frameless `TitleBar`, exclusive Dark/Light toggle, zero frosted glass blur, raw media/scope surfaces, and strict WCAG AA contrast (`DESIGN.md`).
- **G3. Hardware-Accelerated Local Media Pipeline:** Smooth timeline preview and local MP4 (H.264/AAC) & WebM export powered by GStreamer and `wgpu`, with cached async hardware encoder detection (`nvenc`, `vaapi`, `videotoolbox`, `qsv`) and software fallback.
- **G4. Offline Perception & Audio Processing:** On-device Whisper transcription (`whisper-rs`), script-to-take synchronization, SigLIP semantic visual indexing, MODNet portrait matting, Depth-Anything-v2-Small depth estimation, motion tracking, EBU R128 loudness metering, and out-of-process CLAP DSP plugins.
- **G5. Safe Agentic Automation & Auditability:** Shared 498-command catalogue across GUI, CLI, MCP, and HTTP/UDS daemon with 12 permission profiles, tier gates, durable region locks, AF2 Stop fencing (`<= 1s`), AF5 `<untrusted_media>` containment, and Starship Control (`eep.sock` `SO_PEERCRED`) integration.
- **G6. Strict Provenance & Packaging Discipline:** Dual AGPL/commercial compliance via `cargo-deny`, SPDX headers across sources, `NOTICE` attribution records, and native Linux packages (`.deb`, `.rpm`, `.pkg.tar.zst` + `SHA256SUMS.sig`).

### Non-Goals
- **N1. Cloud Lock-In & Telemetry:** Zero forced cloud accounts, remote rendering queues, or usage analytics. External asset/AI providers are strictly opt-in.
- **N2. Destructive Source Edits:** Raw imported video/audio assets on disk are never overwritten during editing.
- **N3. Frosted-Glass UI Churn:** The visual design system (`DESIGN.md`) is locked; experimental blur effects, floating MDI tool windows, and UI green/chart clutter are permanently out of scope.
- **N4. Non-Commercial / Copyleft AI Model Weights:** Models lacking verified permissive/commercial clearance (e.g., RVM GPL-3.0, DA-V2 Large/Base CC-BY-NC-4.0) are explicitly blocked by `docs/model-licences.md`.

---

## 3. Product Capabilities & Requirements

### 3.1 Multitrack NLE & Editing Workspaces
- **Nine Docked Suites:** `Welcome`, `Media`, `Edit`, `Fusion` (compositing nodes), `Color` (wheels, RGB curves, LUTs, Canvas scopes), `Fairlight` (mixer, EBU R128, DSP rack), `Effects`, `Keyframes`, and `Deliver`.
- **Editing Operations:** Frame-accurate trim/ripple/roll/razor/slip/slide, J/L asymmetric split edits (`asymmetric_trim.rs`), magnetic storyline pins, fluid morph cut, beat-grid quantization (`beat_grid.rs`), match-cut alignment (`match_cut.rs`), planar corner pin, depth compositing, clean-plate object removal, kinetic typography, markers, and styled caption tracks.
- **Interchange:** Import and export across CMX 3600 EDL, FCPXML, OpenTimelineIO (OTIO), and XML (`xmeml`), bounded at 50,000 items with path traversal sanitization (`sanitize_untrusted_media_path`).

### 3.2 Media, Render & Audio Engines
- **Local Streaming & Previews:** Token-authenticated local HTTP media server (`media_server.rs`) with strict allowlisted project media registration and age-pruned 720p VP9 WebM proxy generation (`preview_cache.rs`).
- **Audio & DSP:** 14 LUFS loudness targets (`audio_process`), phase alignment, room-tone fill, dialogue-keyed music ducking, multicam waveform sync, and isolated CLAP plugin execution via `saturn-plugin-worker`.
- **Perception Models (`crates/saturn-perception`):** Fetch-on-first-run HuggingFace downloads verified by pinned LFS SHA-256 checksums (`ggml-*` Whisper, `Xenova/siglip-base-patch16-224`, `Xenova/modnet`, `onnx-community/depth-anything-v2-small`).

### 3.3 Agent Control, Task Sheets & Web Desk
- **Command Parity:** `498` shipped commands tracked in `docs/PARITY.md` (`388/388` eff. Tier 1, `123/123` eff. Tier 2, `1/1` eff. Tier 3).
- **Task Sheets & CET (`tasksheet/1`, `cet-0.1`):** Structured work orders moving through `draft -> ready -> planning -> awaiting_plan_approval -> running <-> needs_input -> in_review -> revision_requested -> approved -> delivered`, paired with open Capture Event Track screen telemetry.
- **Web Desk (`?mode=desk`):** Browser interface for B2B self-hosted teams featuring Jobs Board, New Job modal, Plan Approval, Questions Inbox, and Web Review with raw proxy video playback, timecoded notes, and AGPL §13 source notice link.
- **Runner, Auth & Enterprise SSO:** Multi-worker lease pool (`RunnerService`) with wall-clock/cost/render budgets, BYO model endpoints with credential redaction, and OIDC / SAML 2.0 RBAC in `saturn-control`.

### 3.4 Security & Operational Safety
- **Write-Root Enforcement (`WriteGrants`):** Commands and raw exporters can only write inside app data dir, app cache dir, the active project directory, or session-scoped grants created exclusively by native OS save/export file dialogs.
- **Prompt Injection Defense:** Media-derived transcripts, subtitles, and filenames carry `UntrustedText` / `OriginTracker` provenance tags to block implicit escalation into destructive actions.

---

## 4. Implementation Status Matrix

Derived from `docs/AI_NATIVE_PLAN_CHECKLIST.md`, `docs/PARITY.md`, and current codebase gates:

| Workstream | Scope | Status |
| :--- | :--- | :--- |
| **Core NLE & Parity (P1–P19, Beta 1)** | 498 shipped commands, schema v19, EDL/FCPXML/OTIO/XMEML, GStreamer render & encoders | **Done** (`check:commands` & `check:parity` green) |
| **Locked Linear Solid UI** | Shared `linear-theme.css`, EEP Orange `app-accent.css`, 40 px `TitleBar`, Canvas `VideoScopes`, chart removal | **Done** (`check:linear`, `check:contrast` 304 pairs, `test:e2e` 13 areas green) |
| **AI-Native Phases 0–3, 5–6** | Rational ticks, CoW branches, BLAKE3 fingerprints, Whisper, SigLIP, MODNet, DA-V2, motion track, VFX/narrative | **Done** (`saturn-core` + `saturn-perception` + `saturn-render` suites green) |
| **AI-Native Phase 4 (Audio / Stems)** | Offline audio processing, EBU R128, isolated CLAP worker | **Partial** (DSP & audio process **Done**; cinema stem separation deferred awaiting licence-clear model) |
| **Agent-First & Task Sheets (AF1–AF9, TS0–TS10)** | Profiles, locks, Stop, provenance, `cli-anything-eep`, `EEP-Bench`, `saturn-control` UDS/REST/WS, Web Desk, Runner, SSO | **Done** (`saturn-control` + Python harness conformance green) |
| **Cross-Platform (XP1–XP4)** | Tier 2 macOS & Windows build/runtime compatibility | **Open** (`XP1` CI & breakage inventory complete; `XP2` `saturn-platform` abstraction next) |
| **Release & Community (AF0, C2–C3)** | Maintainer policy/CLA/legal sign-offs, installed worker package smoke, starter issues & demo video | **Open** (Blocked on maintainer/release tasks & ONNX/GES archive notice finalization) |

### Known Current Limits
- Preview-side `analyze_motion_tracking` reports bounded analysis telemetry (`MotionTrackResult`), while full motion-track baking applies during render.
- Proxy generation targets a single `1280×720` VP9 WebM profile (`preview_cache`), not user-configurable multi-tier proxy codecs.
- Text-to-speech (`tts.*`) is not implemented (speech subsystem provides offline transcription and script alignment only).

---

*Authored by Zvirox · Easy Edits Pro.*
