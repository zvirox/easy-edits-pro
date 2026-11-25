# Roadmap

This page is a concise summary of current project status. The granular checklist lives in [docs/AI_NATIVE_PLAN_CHECKLIST.md](docs/AI_NATIVE_PLAN_CHECKLIST.md); installation and usage are in [README.md](README.md). Nothing here carries a promised release date.

## Where the project is

Easy Edits Pro (`v0.1.3-beta`) is a **beta** native Linux non-destructive video editor powered by a Rust core (`saturn-core`, `saturn-render`, `saturn-perception`, `saturn-audio-dsp`, `saturn-control`, `saturn-mcp`), a Tauri 2 + React 19 UI in the **locked Linear solid design system**, and GStreamer/GES rendering. Every shipped catalogue command (`492`) is exposed to both humans and AI agents over MCP / Starship Control with actor isolation, branches, durable locks, untrusted-media provenance tracking, and measured self-verification.

## Completed milestones

- **Core, Catalogue & Security (`Phase 0`, `Packet 0.0–0.2`):** Rational-tick Rust project core, schema migrations (up to v18), CoW timeline branches, BLAKE3 media fingerprints, tick overflow hardening, write-root allowlists, and `492` synced commands across menus and MCP.
- **Perceptual Feedback & Neural Perception (`Phase 1` & `Phase 2`):** GStreamer contact-sheet & raw-frame extraction, CPU video scopes telemetry, K-weighted EBU R128 + 10-band FFT audio telemetry, real `whisper-rs` transcription + script sync, shot detection, SigLIP semantic search, MODNet background matting, Depth-Anything-V2 relative depth, and ZNCC + RANSAC motion tracking.
- **GPU Colour & Audio DSP (`Phase 3` & `Phase 4` packets `4.1–4.2`):** `Rgba32Float` wgpu pipeline, ACEScg transforms, node graph + automated shot matching; out-of-process CLAP plugin host (`saturn-plugin-worker`) with render parity, GCC-PHAT phase alignment, room tone fill, RMS ducking, and master loudness normalization.
- **Narrative Editing & Co-Editing UI (`Phase 5.1` slices + `Packet 5.2` + `UI-P0..UI-P7` + Locked Linear UI):** J/L split edits, match-cut alignment, beat-grid quantization; branch diff ribbon, Video Scopes, Script Sync, and Agent Telemetry panels; custom 40 px `TitleBar`, collapsible `Sidebar`, `Breadcrumb`, dark/light Inter tokens, shared Linear components, and zero-blur/zero-glow solid matte surfaces (**UI style is final and locked**).
- **Agent-First Layer, Task Sheets, Daemon, Web Desk, Runner & SSO (`AF0–AF9`, `TS0–TS10`, `Phase 7`):** Tool profiles & tiers, actor isolation, durable locks, Stop fencing (`<= 1s`), `verify.run` measured checks, AF5 provenance, structured `tasksheet/1` work orders & review rounds, delivery packaging, CET v0.1, `cli-anything-eep` + 5 companion recipes, 16-task `EEP-Bench` (`--agent` mode) + live socket `ai_closed_loop_workflow.py`, authenticated UDS/TCP daemon (`saturn-control`), browser Web Desk (`?webdesk=1`), concurrent Runner + BYO endpoints, OIDC/SAML 2.0 SSO + RBAC, and `starship-plugin` SDK orchestration.
- **Contributor Infrastructure (`C1`) & Packaging:** 6-step `CONTRIBUTING.md`, CLA/DCO/SPDX gates, `scripts/doctor.sh`, `.deb`/`.rpm`/Flatpak/AppImage CI packaging pipelines with notice bundle generation.

## Remaining / Open work

- **Audio Stem Separation (`Phase 4.2` — `audio.separate_stems`):** Intentionally deferred until a verified, license-cleared commercial-safe Dialogue/Music/Ambience/SFX model is available.
- **Remaining Narrative Verbs (`Phase 5.1`):** `timeline.fluid_morph_cut` and `timeline.pin_clip_to_storyline`.
- **VFX & Kinetic Typography (`Phase 6`):** `vfx.depth_map_compositor`, `vfx.planar_corner_pin`, `vfx.inpaint_object_removal`, `graphic.kinetic_typography`, and end-to-end `template apply` fix.
- **Cross-Platform (`XP1–XP4`):** `windows-latest` / `macos-latest` CI check matrix (`XP1`), `saturn-platform` OS abstraction crate (`XP2`), headless CLI/MCP ports (`XP3`), and desktop GUI packaging (`XP4`).
- **Shared Agent Contract (`TS-X`):** Extract shared `tasksheet/1` contract crate when Audio Labs Pro or Screen Studio Pro becomes a second active workspace consumer.
- **Community Launch (`C2–C3`):** Starter issues (`good first issue`), 60–90s demo video, fresh container `doctor.sh` verification, and platform discovery submissions.
- **AF0 Maintainer Sign-offs & Packaging Follow-ups:** Installed-worker CLAP plugin host smoke inside packaged layout, native-archive notices (ONNX Runtime/GES/SQLite), and formal maintainer sign-off on default agent policy choices.

## Where help is welcome

Testing on additional Linux distributions, `Phase 5.1` / `Phase 6` engine features, cross-platform (`XP1–XP3`) porting, regression tests, and documentation. See [CONTRIBUTING.md](CONTRIBUTING.md) and the issues labelled `good first issue` and `help wanted`.
