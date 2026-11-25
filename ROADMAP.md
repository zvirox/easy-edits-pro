# Roadmap

This page is a concise summary of current project status. The granular checklist lives in [docs/AI_NATIVE_PLAN_CHECKLIST.md](docs/AI_NATIVE_PLAN_CHECKLIST.md); installation and usage are in [README.md](README.md). Nothing here carries a promised release date.

## Where the project is

Easy Edits Pro (`v0.1.3-beta`) is a **beta** native Linux non-destructive video editor powered by a Rust core (`saturn-core`, `saturn-render`, `saturn-perception`, `saturn-audio-dsp`, `saturn-control`, `saturn-mcp`), a Tauri 2 + React 19 UI in the **locked Linear solid design system**, and GStreamer/GES rendering. Every shipped catalogue command (`498`) is exposed to both humans and AI agents over MCP / Starship Control with actor isolation, branches, durable locks, untrusted-media provenance tracking, and measured self-verification.

## Completed milestones

- **Core, Catalogue & Security (`Phase 0`, `Packet 0.0–0.2`):** Rational-tick Rust project core, schema migrations (up to v18), CoW timeline branches, BLAKE3 media fingerprints, tick overflow hardening, write-root allowlists, and `498` synced commands across menus and MCP.
- **Perceptual Feedback & Neural Perception (`Phase 1` & `Phase 2`):** GStreamer contact-sheet & raw-frame extraction, CPU video scopes telemetry, K-weighted EBU R128 + 10-band FFT audio telemetry, real `whisper-rs` transcription + script sync, shot detection, SigLIP semantic search, MODNet background matting, Depth-Anything-V2 relative depth, and ZNCC + RANSAC motion tracking.
- **GPU Colour & Audio DSP (`Phase 3` & `Phase 4` packets `4.1–4.2`):** `Rgba32Float` wgpu pipeline, ACEScg transforms, node graph + automated shot matching; out-of-process CLAP plugin host (`saturn-plugin-worker`) with render parity, GCC-PHAT phase alignment, room tone fill, RMS ducking, and master loudness normalization.
- **Narrative Editing & Co-Editing UI (`Phase 5.1` slices + `Packet 5.2` + `UI-P0..UI-P7` + Locked Linear UI):** J/L split edits, match-cut alignment, beat-grid quantization; branch diff ribbon, Video Scopes, Script Sync, and Agent Telemetry panels; custom 40 px `TitleBar`, collapsible `Sidebar`, `Breadcrumb`, dark/light Inter tokens, shared Linear components, and zero-blur/zero-glow solid matte surfaces (**UI style is final and locked**).
- **Agent-First Layer, Task Sheets, Daemon, Web Desk, Runner & SSO (`AF0–AF9`, `TS0–TS10`, `Phase 7`):** Tool profiles & tiers, actor isolation, durable locks, Stop fencing (`<= 1s`), `verify.run` measured checks, AF5 provenance, structured `tasksheet/1` work orders & review rounds, delivery packaging, CET v0.1, `cli-anything-eep` + 5 companion recipes, 16-task `EEP-Bench` (`--agent` mode) + live socket `ai_closed_loop_workflow.py`, authenticated UDS/TCP daemon (`saturn-control`), browser Web Desk (`?webdesk=1`), concurrent Runner + BYO endpoints, OIDC/SAML 2.0 SSO + RBAC, and `starship-plugin` SDK orchestration.
- **VFX & Kinetic Typography (`Phase 6`, all packets):** End-to-end `.sgt` `template apply`, planar corner pin (homography warp), depth-map compositor (z-threshold mask + depth-weighted bokeh), optical-flow clean-plate object removal, and kinetic typography (damped-spring word arrival with face-avoidant lanes). All Phase 6 gate checks are green.
- **Contributor Infrastructure (`C1`) & Packaging:** 6-step `CONTRIBUTING.md`, CLA/DCO/SPDX gates, `scripts/doctor.sh`, locked `nfpm` Linux packaging (`zvirox/release-workflows`) shipping exclusively `.deb`, `.rpm`, and `.pkg.tar.zst` + `SHA256SUMS` with CI container smoke tests (legacy Flatpak/AppImage/AUR/VM package tests permanently retired).

## Remaining / Open work

The authoritative, checkbox-level list is [docs/AI_NATIVE_PLAN_CHECKLIST.md](docs/AI_NATIVE_PLAN_CHECKLIST.md). Phases 0–7, UI-P0..UI-P7, the locked Linear UI, AF0–AF9 and TS0–TS10 are complete. What remains, in queue order:

1. **XP2 — `saturn-platform` abstraction crate (next actionable).** Move OS-specific seams out of the engine: GPU backend selection (macOS fails today on `wgpu::hal::vulkan` in `crates/saturn-render/src/gpu_context.rs:17`), IPC (UDS → named pipes/loopback), shared-memory plugin sandbox, XDG/CLAP directories, preview cache, and GPU/encoder detection. Add the CI grep gate that forbids new `cfg(unix)` or raw Linux paths outside the crate.
2. **XP3 — headless CLI/MCP port.** Port `saturn-cli` and the `saturn-control` daemon to macOS/Windows using `saturn-platform`, fix Windows ONNX Runtime provisioning (`ort-sys` DirectML staging currently fails), and publish Tier 2 headless binaries.
3. **XP4 — desktop shells.** Verify Tauri 2 on macOS (WKWebView) then Windows (WebView2) with the locked Linear UI, raw media surfaces and signed installers.
4. **XP1 gate close.** The macOS/Windows `cargo check` jobs are live in `.github/workflows/cross-platform-check.yml` and the breakage catalog is [docs/audit/xp1-breakage-inventory.md](docs/audit/xp1-breakage-inventory.md); the gate turns green only when XP2–XP4 resolve the recorded failures. No Tier 2 support is claimed before then.
5. **TS-X — shared agent-contract crate.** Extract the product-agnostic `tasksheet/1` contract only when Audio Labs Pro or Screen Studio Pro becomes a second active consumer.
6. **C2–C3 — community launch.** Starter issues with acceptance tests, 60–90s demo video, clean Ubuntu/Fedora container `doctor.sh` verification, then submissions to Good First Issue / Up For Grabs / CodeTriage and announcements.
7. **AF0 maintainer sign-offs.** Installed-worker CLAP host smoke inside the packaged layout, native-archive notices (ONNX Runtime/GES/SQLite), and formal maintainer approval of the default agent policy choices.
8. **Phase 4.2 stems (deferred).** `audio.separate_stems` waits for a verified, license-cleared Dialogue/Music/Ambience/SFX model; the named music model's stems must not be relabelled.

## Where help is welcome

Testing on additional Linux distributions, Tier 2 porting (`XP2–XP4`) and platform champions, regression tests, and documentation. See [CONTRIBUTING.md](CONTRIBUTING.md) and the issues labelled `good first issue` and `help wanted`.
