# AI-Native Architecture Plan — Progress Checklist

**Plan spec:** [AI_NATIVE_ARCHITECTURE_PLAN.md](AI_NATIVE_ARCHITECTURE_PLAN.md)
**Adopted:** 2026-11-05 · **Status:** Not started

> **How to use this checklist.** This is the single source of truth for plan progress; the spec above describes *what and why*, this file tracks *what is done*. When starting work on a phase, tick packets as they land (one commit packet per checkbox, one lane per packet per `AGENTS.md`). A phase counts as complete only when **every checkbox in the phase is ticked, including the Verification Gate**. Update the phase status line after each gate passes, and record the evidence (test run / command + date) next to the gate checkbox.

---

## Phase 0 — Foundation: Unified State, Schema v15 & Security Hardening — `status: todo`

### Packet 0.0 — Security remediation (spec §10.1, mandatory before/with Phase 0) — `DONE 2026-11-05`

> **Audit note (2026-11-05):** On adoption, a line-level re-audit found most §10.1 items had already been remediated in the codebase since the original security-audit report (verified by reading current code, not just the cited line numbers): `caption.cue_nudge` uses `checked_add` with pre-validation, the 7 history-blind commands all call `checkpoint()`, `catch_unwind` wraps `engine.execute` inside the lock scope in `execute_command_with_state` (`commands/project.rs`), `ProjectDocument::validate()` rejects timeline_bounds overflow, `overflow-checks = true` was present in `[profile.release]`, importers use `sanitize_untrusted_media_path`, XMEML parse is already single-pass linear (guarded by `xmeml_import_scales_linearly_over_many_clipitems`), `source_url` requires https, reads are capped via `validate_read_path`, and `reveal_path` canonicalizes. This packet therefore completed the **remaining gaps only** (sites marked ✅* below).

- [x] Replace all raw `+`/`-` on `Tick(i64)` with `Tick::checked_add` / `checked_sub` returning typed `Err(String)` ✅ — added `Tick::checked_add/checked_sub/saturating_add/saturating_sub` helpers in `time.rs`; hardened remaining unchecked sites: gap-close in `timeline_cmds.rs` (gap_duration + 4 guarded shifts), `trim.extendToPlayhead` delta, `trim.selectNearest` distance key, `edit_labels` head-removal, `effects_cmds` transition-distance `.abs()` + max-end sum, `captions_cmds` cue-split `chars*into/total` (i128 intermediates), XMEML frame→tick math
- [x] Validate `timeline_start >= 0` and `timeline_start.checked_add(duration)` in `ProjectDocument::validate()` ✅ — pre-existing (`project/mod.rs`)
- [x] Set `overflow-checks = true` in `[profile.release]` (root `Cargo.toml`) ✅ — pre-existing
- [x] Wrap `mcp_bridge.rs::dispatch` in `std::panic::catch_unwind` (no more mutex poisoning) ✅ — pre-existing, implemented at the stronger site: `catch_unwind` inside the lock scope in `execute_command_with_state` (`apps/saturn-tauri/src/commands/project.rs`)
- [x] Reject `..` path traversal in all interop importers (FCPXML/XMEML/OTIO/EDL); mark imported media paths `unconfirmed: true` ✅ — `sanitize_untrusted_media_path` pre-existing (guarded by `xmeml_import_never_stores_parent_escape_paths`)
- [x] Make XMEML/FCPXML parsing linear-time, parse before taking `state.0.lock()`, cap items per import (50,000) ✅ — linear scan pre-existing; **50,000-item cap added this pass** (`MAX_IMPORT_CLIPS`, errors instead of truncating)
- [x] Route the 7 history-blind mutating commands through the undo-push wrapper (`clip.remove_keyframe`, `project.update_media`, `media.check_missing`, `media.auto_relink`, `media.make_offline`, `media.refresh_changed`, `marker.split`) ✅ — all seven call `self.checkpoint()` (verified per-handler)
- [x] Enforce `https://` on `source_url`, cap project-file reads at 100 MiB, call `validate_read_path` in `reveal_path` ✅ — pre-existing (https gate in `commands/assets.rs`, read cap via `validate_read_path`, canonicalize in `reveal_path`)

### Packet 0.1 — Rust viewport & selection state (Lane 2) — `DONE 2026-11-05`
- [x] Create `crates/saturn-core/src/viewport_state.rs` (< 400 lines) with `UnifiedViewportState`
- [x] Convert `playhead.*`, `timeline.select*`, `edit.select_*`, `export.queue.*` commands to mutate `UnifiedViewportState` in `EditorEngine` — **plus `workspace.*` (8 ids), `project.select`/`selectAll`, `sequence.selection_follows_playhead` and the new `viewport.inspect` query; 41 ids flipped from `ui_only: false`, docs regenerated via `pnpm catalogue:sync` (441 commands, 71 stay UI-only)**

### Packet 0.2 — Schema v15: BLAKE3 fingerprints & CoW timeline branches (Lane 2)
- [ ] Create `crates/saturn-core/src/branch.rs` (`TimelineBranch`, diff report types)
- [ ] Add `project.branch_create` / `branch_switch` / `branch_diff` / `branch_merge` commands
- [ ] Schema v15: BLAKE3 head/tail hash + relative paths + auto-relink on moved project folders
- [ ] v1 → v15 lossless migration in `migrations.rs`
- [ ] Run `pnpm catalogue:sync` and commit regenerated `docs/COMMANDS.md` / catalogue JSON

### ✅ Phase 0 Verification Gate
- [ ] `cargo test -p saturn-core` green — **passing 2026-11-05 (Packet 0.0 run)**
- [ ] `python3 tests/native/mcp_command_matrix.py` green — pending Packet 0.1/0.2 (matrix must be re-run against ≥425/440 after those land)
- [ ] **Exit criterion met: ≥ 425 of 440 commands return `ok` over MCP** (only OS window controls like `view.fullscreen` remain `ui_only`)

---

## Phase 1 — Perceptual Feedback Loop (AI Eyes & Ears) — `status: todo`

### Packet 1.1 — Shared-memory ring & frame/contact-sheet extractor (Lane 2 + 4)
- [ ] Split `crates/saturn-render/src/lib.rs` (currently 1,483 lines) below 800 lines first
- [ ] Create `crates/saturn-render/src/shm_ring.rs` (`0600`, `O_CREAT|O_EXCL`, ≤ 64 MiB / 4 slots, Drop guard)
- [ ] Create `crates/saturn-render/src/frame_extractor.rs` (GStreamer appsink, single frame + `columns × rows` contact sheet, < 150 ms)
- [ ] `media_preview` returns real still frames; add `vision_extract_frame_sheet` + `render_preview_clip_proxy`
- [ ] All extractor outputs go through `enforce_write_target` or the isolated `previews/` runtime dir (spec §10.2)

### Packet 1.2 — Video scopes telemetry (Lane 2 + 4)
- [ ] Create `crates/saturn-render/src/scopes_gpu.rs` (`ScopeTelemetryReport`: IRE percentiles, clip/crush %, parade, saturation, skin-tone I-line)
- [ ] Expose `color_read_scopes` over MCP + desktop bridge

### Packet 1.3 — Audio telemetry (Lane 2 + 4)
- [ ] Extend `EbuR128Meter` with 10-band 1/3-octave FFT, phase correlation (−1..+1), dialogue SNR
- [ ] Expose `audio_read_telemetry` as a top-level MCP tool

### ✅ Phase 1 Verification Gate
- [ ] `cargo test -p saturn-render -p saturn-mcp` green
- [ ] 4×4 PNG contact sheet generated from `editor-smoke.webm` by automated test
- [ ] `clipped_highlights_pct > 95.0` on synthetic clipped-white frame
- [ ] `audio_read_telemetry` measures `-23.0 ± 0.1 LUFS` on reference EBU R128 signal

---

## Phase 2 — Local Neural Perception Engine (`saturn-perception`) — `status: todo`

### Packet 2.1 — Real Whisper transcription & script sync (Lane 2)
- [ ] Create `crates/saturn-perception` crate (ort + rusqlite deps)
- [ ] Implement `src/whisper.rs` (word-level tick alignment + diarization, models: base/small/medium/large-v3-turbo)
- [ ] Replace 4 hardcoded demo sentences in `apps/saturn-tauri/src/commands/audio.rs` with real inference
- [ ] Implement `speech.script_sync_align` (script vs recorded takes, take ranking)

### Packet 2.2 — Shot detection, SigLIP search & cut continuity (Lane 2)
- [ ] Implement `src/shot_detect.rs` + `src/embeddings.rs` + sidecar `.saturn-index.sqlite`
- [ ] Implement `CutContinuityScore` (eye-trace, luma jump, CIEDE2000, flow angle, 180° rule, audio pop)
- [ ] Expose `media_semantic_query` + `edit_evaluate_cut_continuity` over MCP

### Packet 2.3 — Neural matting, depth & motion tracking (Lane 2)
- [ ] Implement `src/matting_ort.rs` (RVM v2 / MODNet) replacing the Gaussian/skin-color heuristic
- [ ] Implement `src/depth_ort.rs` (Depth-Anything-V2)
- [ ] Replace synthetic sine/cosine motion data in `apps/saturn-tauri/src/commands/preview.rs` with real `track_patch_displacement` + 4-corner RANSAC homography

### ✅ Phase 2 Verification Gate
- [ ] `cargo test -p saturn-perception -p saturn-tauri` green
- [ ] **Zero fake/hardcoded outputs remain** in `commands/audio.rs`, `commands/preview.rs`, `matting.rs`
- [ ] `media_semantic_query` returns relevant clips in < 50 ms post-indexing

---

## Phase 3 — Scene-Referred 32-Bit Float GPU Color Engine — `status: todo`

### Packet 3.1 — Real wgpu `Rgba32Float` pipeline & OCIO v2 ACEScg (Lane 2)
- [ ] Create `src/gpu_context.rs` + `src/ocio_pipeline.rs` (real `wgpu::Device/Queue`, `Rgba32Float`, Linux DMA-BUF import)
- [ ] Implement `ocio.set_pipeline` + `ocio.set_media_idt` (ARRI LogC4, RED Log3G10, Sony S-Log3, BM Gen5, Apple Log → ACEScg/ACEScct → Rec.709 / P3 / Rec.2020 PQ ODTs)

### Packet 3.2 — WGSL color node graph, power windows & film emulation (Lane 2)
- [ ] Create `src/color_nodes_gpu.rs` + `src/film_emulation.rs` (wheels, HDR zones, curves, 3D LUTs, HSL, warper, power windows, halation + grain)
- [ ] `color.node_add` / `node_update` / `add_power_window` / `emulate_film_print` wired end-to-end
- [ ] Shader hardening per spec §10.2.3 (naga pre-validation, error scopes, no crash on faulty shader)

### Packet 3.3 — Automated shot matcher (Lane 2)
- [ ] Create `src/shot_match.rs` (`color.match_shot_to_reference` with CDF alignment + skin-tone lock)

### ✅ Phase 3 Verification Gate
- [ ] `cargo test -p saturn-render --test color_pipeline` green
- [ ] +3.0 stops (Node 1) then −3.0 stops (Node 2) in ACEScg recovers 100% highlights, zero clipping
- [ ] Shot match brings mismatched angles within CIEDE2000 ΔE < 1.5

---

## Phase 4 — Cinema Audio DSP (`saturn-audio-dsp`) — `status: todo`

### Packet 4.1 — Native plugin host & render parity (Lane 2)
- [ ] Create `crates/saturn-audio-dsp` crate + `src/clap_host.rs` (CLAP/LV2/VST3 via `clack-host`)
- [ ] `dsp.load_plugin` / `dsp.set_plugin_param` wired into preview AND export graph (bit-parity)
- [ ] Plugin out-of-process sandbox (`saturn-plugin-worker`) per spec §10.2.4

### Packet 4.2 — Stems, phase align, room tone & ducking (Lane 2)
- [ ] `src/stems_ort.rs` — `audio.separate_stems` (Dialogue/Music/Ambience/SFX via ONNX MDX-Net/Demucs)
- [ ] `src/phase_align.rs` — `audio.auto_phase_align` (GCC-PHAT sub-sample)
- [ ] `src/room_tone.rs` — `audio.fill_gaps_with_room_tone`
- [ ] `src/ducking.rs` — `audio.sidechain_duck_envelope` + `audio.loudness_normalize_master`

### ✅ Phase 4 Verification Gate
- [ ] `cargo test -p saturn-audio-dsp` green
- [ ] Preview vs rendered MP4 audio match within −90 dBFS
- [ ] Boom+lav comb filtering eliminated on 1.7 ms delayed fixture
- [ ] Normalization hits `-14.0 ± 0.1 LUFS` and `<= -1.0 dBTP`

---

## Phase 5 — Narrative Editing Verbs & Co-Editing UI — `status: todo`

### Packet 5.1 — Narrative editing verbs (Lane 2)
- [ ] Implement `timeline.asymmetric_trim` (J/L-cuts with constant-power crossfade)
- [ ] Implement `edit.match_cut_align` (eye_trace | motion | transient)
- [ ] Implement `timeline.beat_grid_quantize`
- [ ] Implement `timeline.fluid_morph_cut`
- [ ] Implement `timeline.pin_clip_to_storyline` (magnetic storyline pins in `saturn-timeline`)
- [ ] Run `pnpm catalogue:sync`, commit regenerated docs

### Packet 5.2 — Co-editing React 19 panels (Lane 1)
- [ ] `ui/src/features/timeline/AiBranchDiffRibbon.tsx` (ghost playhead, accept/reject via `project.branch_merge`)
- [ ] `ui/src/features/preview/VideoScopesPanel.tsx` (raw canvas — no glass over scope surfaces)
- [ ] `ui/src/features/captions/ScriptSyncPanel.tsx` (word-level ripple cutting, multi-take carousel)
- [ ] `ui/src/features/inspector/AgentTelemetryDrawer.tsx` (live MCP feed, deltas, skill saver)

### ✅ Phase 5 Verification Gate
- [ ] `pnpm --dir apps/saturn-tauri build` green
- [ ] `pnpm --dir apps/saturn-tauri test` green
- [ ] `pnpm --dir apps/saturn-tauri check:glass` green
- [ ] `pnpm --dir apps/saturn-tauri check:commands` green

---

## Phase 6 — VFX, Depth Compositing & Kinetic Typography — `status: todo`

### Packet 6.1 — Depth compositing, corner-pin & object removal (Lane 2)
- [ ] Implement `vfx.depth_map_compositor` (Depth-Anything-V2 z-threshold compositing / DOF bokeh)
- [ ] Implement `vfx.planar_corner_pin` (4-corner homography + motion blur)
- [ ] Implement `vfx.inpaint_object_removal` (optical-flow clean-plate inpainting)

### Packet 6.2 — Kinetic typography & `.sgt` template fix (Lane 2)
- [ ] Fix `template apply` end-to-end (`crates/saturn-graphics` + `agent-harness`; remove `COMMAND_NOT_FOUND`)
- [ ] Implement `graphic.kinetic_typography` (per-word spring physics on Whisper timestamps, face-avoidant layout)

### ✅ Phase 6 Verification Gate
- [ ] `cargo test -p saturn-render -p saturn-graphics -p saturn-text` green
- [ ] `cli-anything-eep template apply` passes end-to-end

---

## Phase 7 — Agent Harness Expansion & Closed-Loop Acceptance — `status: todo`

### Packet 7.1 — `cli-anything-eep` CLI & skill upgrade (Lane 4)
- [ ] Add `branch`, `perception`, `ocio`, `telemetry`, `vfx` subcommand groups under `agent-harness/cli_anything_eep/commands/`
- [ ] Update `agent-harness/cli_anything_eep/skills/cli-anything-eep.md` with closed-loop agent skill definitions

### Packet 7.2 — Closed-loop acceptance suite (Lane 3)
- [ ] Create `tests/native/ai_closed_loop_workflow.py` (multi-cam rough cut → continuity check → ACEScg grade → stem ducking + EBU R128 master → H.264/WebM export, over the live socket bridge)

### ✅ Phase 7 Final Release Gates (all must pass)
- [ ] `cargo test --workspace`
- [ ] `pnpm --dir apps/saturn-tauri build`
- [ ] `pnpm --dir apps/saturn-tauri test`
- [ ] `pnpm --dir apps/saturn-tauri check:glass`
- [ ] `pnpm --dir apps/saturn-tauri check:commands`
- [ ] `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`
- [ ] `pnpm --dir apps/saturn-tauri test:e2e`
- [ ] `python3 -m pytest agent-harness/tests -q`
- [ ] `python3 tests/native/mcp_command_matrix.py`
- [ ] `python3 tests/native/ai_closed_loop_workflow.py`

---

## Cross-cutting rules (apply to every packet above)

- [ ] One lane per packet; one `<menu>.rs` family per agent; sequential `main.rs` handler commits
- [ ] Query `prowl` before editing; never grep-and-read-whole-file
- [ ] Never hand-edit generated files — always `pnpm catalogue:sync`
- [ ] Every file ≤ 800 lines (`check:source-size`) — split legacy oversized files before touching them
- [ ] `shipped: true` only with real engine implementation + UI affordance + automated test; stubs return `Err`
- [ ] Design tokens only; `.glass` never over video/scope/meter canvases; no `transition: all`
- [ ] Clean tree: never commit `.prowl/`, `test-results/`, agent dirs
- [ ] Sweep new files for junk/control characters before committing

---

## Progress log

| Date | Packet | Note / Evidence |
|---|---|---|
| 2026-11-05 | — | Plan adopted into repo; checklist created; no packets started |
| 2026-11-05 | 0.0 | Security remediation complete. Re-audit found most §10.1 items pre-remediated; completed the remaining unchecked-Tick sweep (`time.rs` helpers + engine/trim/markers-adjacent/effects/captions/xmeml sites), XMEML 50k clip cap, XMEML checked frame math, export-side saturating end math. 4 new robustness tests in `engine/tests/robustness.rs`. Evidence: `cargo test --workspace` clean (0 failures), `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800` pass. Tier-4 matrix deferred to Phase 0 gate. Commit `c8c20a0`. |
| 2026-11-05 | 0.1 | Engine-owned unified viewport state: `UnifiedViewportState` in `viewport_state.rs` (playhead + AI ghost field, clip/gap/media selection, workspace, branch, export queue capped at 500); `viewport_cmds.rs` dispatches 55 ids (playhead 13, workspace 8, timeline.select 4, edit.select 4, bin/marks 3, export.queue 9, viewport.inspect) returning a `viewport` snapshot; 41 catalogue specs flipped backend (`uiOnly:false`, new params), catalogue JSON/COMMANDS.md regenerated. Targeting stays single-sourced in `TargetingState` (snapshot accessor). 16 new engine tests; strict no-junk-params for `params:"{}"` viewport cmds. Gates: cargo test --workspace 0 failures, pnpm build + 168 UI tests green, check:commands (441 cmds / 71 UI-only) + source-size pass. UI convergence deferred to Packet 5.2. Commit `031a7bc`. |
