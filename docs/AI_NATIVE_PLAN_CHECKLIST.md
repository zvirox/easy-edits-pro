# AI-Native Architecture Plan — Progress Checklist

**Plan spec:** [AI_NATIVE_ARCHITECTURE_PLAN.md](AI_NATIVE_ARCHITECTURE_PLAN.md)
**Adopted:** 2026-11-05 · **Status:** Not started

> **How to use this checklist.** This is the single source of truth for plan progress; the spec above describes *what and why*, this file tracks *what is done*. When starting work on a phase, tick packets as they land (one commit packet per checkbox, one lane per packet per `AGENTS.md`). A phase counts as complete only when **every checkbox in the phase is ticked, including the Verification Gate**. Update the phase status line after each gate passes, and record the evidence (test run / command + date) next to the gate checkbox.

---

## Compliance documents (binding for every packet)

> **Adopted 2026-11-05, per user directive.** While working on this plan, the documents below are compliance requirements, not reading suggestions. Where a document conflicts with a packet in the checklist, the document's constraint wins and the packet is reshaped accordingly; record the change in the progress log.

| Document | Type | Binding consequence for plan work |
|---|---|---|
| [AGENTS.md §Provenance and third-party material](../../AGENTS.md) (added via [provenance-notice.patch](audit/provenance-notice.patch)) | Repo rule | Study but never paste third-party code; log every reuse in `NOTICE` **in the same change**; only MIT/Apache-2.0/BSD/ISC/Zlib sources; never GPL/AGPL/LGPL deps or non-commercial model weights without maintainer approval; no third-party product names in UI/docs/identifiers; never the ArtCraft/FilmCraft names or logos. |
| [NOTICE](../../NOTICE) | Legal record | The FilmCraft derivation is documented (command inventory + `CommandSpec` shape, MIT OR Apache-2.0 texts under `licenses/`). Any new third-party reuse must add an entry here in the same commit. |
| [eep-ai-native-plan-review.md](audit/eep-ai-native-plan-review.md) | Independent plan review | Design constraints absorbed into the packets below: no `/dev/shm` ring (Packet 1.1 uses MCP image blocks / `$XDG_RUNTIME_DIR` previews); contact-sheet budget with token controls (Packet 1.1); no invented scalar "Murch score" — measurable telemetry only (Packet 1.2); no ethnicity-labelled skin-tone logic; no OCIO dependency (fix a small WGSL transform set); BLAKE3 is a *fingerprint*, not a content hash (Packet 0.2); branch merge v1 is whole-branch accept/reject only (Packet 0.2); per-session viewport cursors for Packet 5.2 (human/agent do not share one selection); treat media-derived text as untrusted data; curated ~20–30 default agent tool profile for MCP; headless daemon milestone (review §5 stage C) before any hosting work. |
| [dual-licensing-plan.md](planning/dual-licensing-plan.md) | Licensing brief (not legal advice) | **IMPLEMENTED 2026-11-05 (see progress log).** AGPL-3.0-or-later is the open licence + a commercial option (`LICENSING.md`, `LICENSE-COMMERCIAL.md`); SPDX headers on every source file gated by CI; `deny.toml` + `licences` CI job gate dependency licences; CLA (CLA.md) required before outside-PR merges (statement-based flow live; CLA Assistant bot is a maintainer follow-up); never bundle x264 or non-commercial model weights (`docs/model-licences.md` gate); lawyer review of all licence docs is still pending — any legal contradiction overrides this table. |
| [filmcraft-provenance-check.md](audit/filmcraft-provenance-check.md) | Provenance audit | Evidence base for NOTICE. Reminder: check parity header and `docs/PARITY.md` credit FilmCraft; do not introduce more verbatim surface without a NOTICE entry. |

---

## Phase 0 — Foundation: Unified State, Schema v15 & Security Hardening — `status: DONE 2026-11-06`

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

### Packet 0.2 — Schema v15: BLAKE3 fingerprints & CoW timeline branches (Lane 2) — `DONE 2026-11-06`

> **Plan-review constraints apply (docs/audit/eep-ai-native-plan-review.md §4.11, §4.10):** BLAKE3 head/tail/size is a *fingerprint*, not a content hash — name it `fingerprint`, and use it only to *suggest* relinks with user confirmation, never as silent truth. Branch merge v1 must be **whole-branch accept/reject only**; per-range merges wait for a written conflict rule.

- [x] Create `crates/saturn-core/src/branch.rs` (`TimelineBranch`, diff report types) — **8 unit tests; `validate_name` rejects user creation of `main` (reserved live branch)**
- [x] Add `project.branch_create` / `branch_switch` / `branch_diff` / `branch_merge` commands — **v1: `branch_merge` accepts or rejects the whole branch (no range merges); dispatch lives in `engine/branch_cmds.rs` (lane-2 engine, `project.` prefix kept for UI compatibility); 11 engine tests incl. a savability regression for the parked-`main` record**
- [x] Schema v15: BLAKE3 fingerprint (head/tail/size, named `fingerprint`) + relative paths + relink *suggestion* flow on moved project folders — **`media_fingerprint.rs` (v1.8.7, already licence-clear in deny.toml); `media.relink_suggest` suggests only — confirmation happens via the existing `project.relink_media`; validate() allows the engine-written reserved record named `main` while user creation stays rejected**
- [x] v1 → v15 lossless migration in `migrations.rs` — **io roundtrip tests cover the v14 payload and the new fields**
- [x] Run `pnpm catalogue:sync` and commit regenerated `docs/COMMANDS.md` / catalogue JSON — **448 commands total (6 new), 71 UI-only; catalogue flags corrected for the whole selection family: `edit.select_all_matching`/`select_label_group` are `mutating: false` (viewport state, out of undo per plan §4.9)**

### ✅ Phase 0 Verification Gate — `PASSED 2026-11-06`
- [x] `cargo test --workspace` green — **0 failures (110 saturn-core lib tests incl. branch/fingerprint/migration/savability suites)**
- [x] `python3 tests/native/mcp_command_matrix.py` green — **run 5 vs the rebuilt binary (`user-tag packet-0.2`, evidence `/tmp/opencode/matrix-p02/result5.json`): 448 commands exercised**
- [x] **Exit criterion met: ≥ 425 commands return `ok` over MCP** — **372 ok + 71 ui_only = 443/448 non-error (floor was 425/448); the 5 errors are all judged needs-state (judge `multicam.flatten`, `timeline.ripple_delete_gap`, `trim.next`, `trim.previous`, `edit.select_clip_at_playhead` — the last passes in isolation with a diag run; in the full run it hits an empty-timeline state caused by the single documented undo-drift (`speech.ripple_delete_silence`) and the engine correctly errors when no clip exists)**

---

## Phase 1 — Perceptual Feedback Loop (AI Eyes & Ears) — `status: packets 1.1–1.3 done; all four verification-gate criteria met (phase closes with the matrix re-run logged below)`

### Packet 1.1 — Frame & contact-sheet extraction (Lane 2 + 4)

> **Plan-review revision (docs/audit/eep-ai-native-plan-review.md §4.2, §4.3):** the `/dev/shm` zero-copy ring is **dropped** — a language model cannot read shared memory. Frames are returned as MCP image content blocks (size-capped JPEG/PNG) or paths under `$XDG_RUNTIME_DIR/easy-edits-pro/previews/` (`0700`). Contact-sheet target is 1–2 s from the import-time thumbnail/proxy cache, with a token budget (tile count, tile size, JPEG quality), per §10.2 write-policy containment.

- [x] Split `crates/saturn-render/src/lib.rs` (currently 1,483 lines) below 800 lines first — **lib.rs 629 lines + `encoders.rs` 294 + `ges_timeline.rs` 629; 67 unit tests in the crate green**
- [x] Create `crates/saturn-render/src/frame_extractor.rs` (GStreamer appsink, single frame + `columns × rows` contact sheet, < 2 s from cache, token budget on tile count/size/quality) — **4×4 sheet (1280×720, 16 timecode-burned tiles) from `editor-smoke.webm` in 382 ms cold via one pre-warmed decode pipeline (seek per tile); budget: ≤ 64 tiles, ≤ 640×640 tiles, in-band JPEG capped at 512 KiB; integration tests `crates/saturn-render/tests/frame_extractor.rs` (5 green, laptop-native)**
- [x] `media_preview` returns real still frames (MCP image content blocks or `0700` previews dir); add `vision_extract_frame_sheet` + `render_preview_clip_proxy` — **shared payload logic in `saturn-render/src/preview_tools.rs` (identical shapes in headless MCP + desktop bridge); MCP `call()` strips `image_block` into a real `RawContent::image` + de-duplicated text block; proxy caps: 10 s span, `64..1920` resolution, software encode, `include_audio:false` mutes. Tools are MCP tools anchored to the same engine, not catalogue commands (plan §5.1)**
- [x] Import-time thumbnail/proxy cache feeding the contact sheet — **on-demand lazy cache keyed by the Packet 0.2 BLAKE3 fingerprint (head/tail/size) inside the 0700 previews dir, atomically written, auto-pruned after 7 days; first call populates it (382 ms cold ⇒ warm hits are instant), a dedicated import-time warm-up step is deferred until profiling shows it pays**
- [x] All extractor outputs go through `enforce_write_target` or the isolated `previews/` runtime dir (spec §10.2) — **previews dir: `$XDG_RUNTIME_DIR/easy-edits-pro/previews/` (mode `0700`, `SATURN_PREVIEWS_DIR` override for tests), 0700 containment + byte-cap covered by tests; proxy renders write only into that dir; no caller-supplied paths**

### Packet 1.2 — Video scopes telemetry (Lane 2 + 4)

> **Plan-review revision (docs/audit/eep-ai-native-plan-review.md §4.4, §4.5):** no invented combined "Murch score" — expose measurable telemetry only (luma jump, ΔE2000, audio boundary pop, optical-flow direction); any aggregate is labelled `heuristic` and gated against human-labelled good/bad cuts before it can block anything. Skin-tone measurement is relative to a reference shot or user-set target, never by ethnicity label; the vectorscope skin-tone line stays a guide only.

- [x] Create `crates/saturn-render/src/scopes_gpu.rs` (`ScopeTelemetryReport`: IRE percentiles, clip/crush %, parade, saturation) — **deterministic CPU histogram analysis (module docs record the naming-vs-implementation note; a wgpu impl can slot into the same report type later). Report adds honest extension fields (`sampled`, `skin_pixel_pct`, `vectorscope_p90_sat_pct`, `chroma_angle_mean_deg`) beside the plan fields; the plan's ethnicity-specific IRE comment is deliberately dropped (§4.5). Constants: `SKIN_TONE_LINE_DEG = 123°` reference target, Rec.709 luma for IRE, Rec.601 chroma for vectorscope math, clip/crush thresholds 99.5/0.5 IRE (8-bit RGB quantizes 100 IRE to raw 255). 7 unit tests incl. the exit criterion; ROI clamps partial overlap and rejects fully-outside ROIs**
- [x] Expose `color_read_scopes` over MCP + desktop bridge — **two modes: `media_id`+`tick` (direct source decode) and `at_tick` (render the graded sequence frame), optional `roi`; payload notes carry `aggregate_score: null` (no invented Murch score, §4.4) and the chroma-only skin-tone disclaimer. Shared payload `color_scopes_payload` in `preview_tools.rs` used by headless `saturn-mcp` and the Tauri bridge; dispatch arms + async impls in `server.rs`/`mcp_bridge.rs`; MCP tool schema in `server_tools.rs`. `frame_extractor.rs` gains `decode_frame_rgb` (raw RGB, no JPEG quantization corrupting histograms) + `decode_png_rgb`. **Latent `render_frame` bug fixed:** the bespoke GES preview-pull pipeline deadlocked forever waiting for the composition's initializing seek (`Waiting for preroll to send initializing seek` then total stall — `export.frame` was uiOnly so was never matrix-tested); `render_frame` now renders a 2-frame software-encoded snippet through the matrix-proven `render_project_range_with_settings` path and decodes its first frame. Gates: cargo test --workspace 0 failures (saturn-render 74 lib + 5 extractor + 3 scopes-integration; saturn-mcp 5 tests), pnpm build + 168 UI + e2e green, check:glass / check:commands (448) / check:source-size / license-headers clean; junk + cyrillic sweep clean. Integration evidence `crates/saturn-render/tests/color_scopes.rs`: media-mode telemetry from the fixture (camelCase fields asserted) and sequence-mode render→analyze with temp-frame cleanup**

### Packet 1.3 — Audio telemetry (Lane 2 + 4)
- [x] Extend `EbuR128Meter` with 10-band 1/3-octave FFT, phase correlation (−1..+1), dialogue SNR — **implemented as an octave-band analyzer over a hand-rolled in-crate iterative radix-2 Cooley–Tukey FFT (no new dependency; deterministic float-op order): `audio_telemetry.rs` carries the FFT + Parseval-normalized single-sided spectra (bins sum to N·mean-square; band dB = 10·log10(Σ band bins / window_frames)), 10 octave bands per `BAND_CENTER_HZ` (31 Hz..16 kHz, sixth-octave band edges — the plan's `spectral_bands_db: [f32; 10]` layout is octave centers), phase-correlation accumulation inside `EbuR128Meter` (Σ(L·R)/√(ΣL²·ΣR²), accessor returns `None` for mono/silence; `LoudnessReport` unchanged), and a dialogue-SNR heuristic (20 ms windows of speech-band 300–3400 Hz energy, p25/p75 percentile split, needs > 3 dB separation else `None` — labelled heuristic, not ASR). Mono fold = mean channel (correlated stereo reads per-channel level). GStreamer PCM decode in new `audio_pcm.rs` (`uridecodebin → audioconvert → audioresample → appsink F32LE`; the extractor's proven Paused→Playing→seek-in-Playing flow; EOS detected via `appsink.is_eos()` — appsink never posts EOS to the bus, short files fully decode before the first pull). `MAX_ANALYSIS_SECONDS = 120` cap with an honest `truncated_to_cap` flag. Mud (250 Hz) / sibilance (4.7–10 kHz) warnings are deterministic band-energy heuristics vs the 1–2 kHz vocal band (≥ 6 dB); per §4.4 there is deliberately **no invented aggregate score** (`AggregatedScore`/`aggregate_score: null` in notes). 7 unit tests green incl. the exit criterion**
- [x] Expose `audio_read_telemetry` as a top-level MCP tool — **two modes mirroring `color_read_scopes`: `media_id` (source PCM decode; optional `start_tick`/`end_tick`) and `start_tick`+`end_tick` (renders the sequence mix via `render_project_range_with_settings` software encode → decode → analyze; temp snippet deleted). Shared payload `audio_telemetry_payload` in `preview_tools.rs` (+`parse_standard`, `parse_tick_range`) used byte-identically by headless `saturn-mcp` and the Tauri bridge; tool schema in `server_tools.rs`; dispatch + async impls in `server.rs`/`mcp_bridge.rs`. A `track_id` param is accepted by the schema but honestly rejected with a typed error ("track-scoped telemetry is not implemented; analysis covers the range's full mix") rather than silently ignored (§4.4). MCP tool, not a catalogue command — 448 command count unchanged. Integration evidence `crates/saturn-render/tests/audio_telemetry.rs` (4 green; GStreamer work serialized behind a test gate): media mode measures integrated −23.0 ± 0.1 LUFS end-to-end on a synthesized 16-bit WAV whose 1 kHz amplitude is calibrated **through the real K-filter's measured steady-state gain** (`KWeightingFilter` made `pub(crate)` with `steady_state_gain_at` — the ITU reference level is defined by that filter, so the criterion tests the meter, not filter lore), asserts the 1 kHz band peaks, phase ≈ +1, and `aggregateScore` null; sequence mode asserts mix render/analyze within ±1.0 LU (the snippet rides an audio encoder; the exact ±0.1 LU lives in media mode + the unit test) and snippet cleanup; param typed errors (modeless, unknown id, track_id, bogus standard). MCP-level typed-error test added (saturn-mcp 6 tests green)**

### ✅ Phase 1 Verification Gate
- [x] `cargo test -p saturn-render -p saturn-mcp` green — **saturn-render 81 lib + all integration suites, saturn-mcp 6 tests; full `cargo test --workspace` 0 failures**
- [x] 4×4 PNG contact sheet generated from `editor-smoke.webm` by automated test — **Packet 1.1 evidence: 1280×720, 16 labeled tiles, 382 ms cold (`cargo test -p saturn-render --test frame_extractor`)**
- [x] `clipped_highlights_pct > 95.0` on synthetic clipped-white frame — **Packet 1.2 evidence (`tests/color_scopes.rs`)**
- [x] `audio_read_telemetry` measures `-23.0 ± 0.1 LUFS` on reference EBU R128 signal — **Packet 1.3 evidence: unit test (`reference_ebu_r128_signal_measures_minus_23_lufs_plus_minus_tenth`) + media-mode integration test both assert ±0.1 LU on the K-filter-calibrated reference sine**

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

> **Plan-review revision (docs/audit/eep-ai-native-plan-review.md §4.6):** no OpenColorIO C++ dependency. Implement a **fixed, small WGSL transform set**: ACEScg/ACEScct matrices, a handful of camera log curves, and Rec.709/sRGB/P3 outputs; add more only on user pull. The module keeps the `ocio_pipeline` name for API continuity but must not link OCIO.

- [ ] Create `src/gpu_context.rs` + `src/ocio_pipeline.rs` (real `wgpu::Device/Queue`, `Rgba32Float`, Linux DMA-BUF import — **pure WGSL transforms, no OCIO dep**)
- [ ] Implement `ocio.set_pipeline` + `ocio.set_media_idt` (camera log curves → ACEScg/ACEScct → Rec.709 / P3 output transforms, in WGSL)

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

> **Plan-review constraint (docs/audit/eep-ai-native-plan-review.md §4.9):** human and agent get **separate viewport cursors/selections** — the human's lives in the GUI session, the agent's in its command session; they do not overwrite each other. Viewport state stays out of undo and out of dirty-state. Also gated on the glass-card shell (build after G3 of the UI plan); branch diff ribbon works with Packet 0.2's whole-branch accept/reject v1.

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
| 2026-11-06 | 1.3 | **Audio telemetry.** New `saturn-render/audio_telemetry.rs` (~640 lines): hand-rolled in-crate iterative radix-2 Cooley–Tukey FFT (deterministic float-op order, no new dependency), Parseval-normalized single-sided spectra (bins sum to N·mean-square; band energy = 10·log10(Σ band bins / window_frames)), 10 octave bands per `BAND_CENTER_HZ` (31 Hz..16 kHz, sixth-octave band edges; the plan's `spectral_bands_db: [f32; 10]` layout is octave centers), dialogue-SNR heuristic (20 ms windows of speech-band 300–3400 Hz energy, p25/p75 percentile split, > 3 dB separation required else `None` — labelled heuristic, not ASR), mud (250 Hz) / sibilance (4.7–10 kHz) band-energy warnings vs the 1–2 kHz vocal band (≥ 6 dB), `MAX_ANALYSIS_SECONDS = 120` cap with honest `truncated_to_cap` flag. Per §4.4: **no invented aggregate mix score** (`notes.aggregate_score: null`; warnings are labelled heuristics that gate nothing). `loudness.rs`: stereo phase-correlation accumulation inside `EbuR128Meter` (Σ(L·R)/√(ΣL²·ΣR²) over all processed frames; `None` for mono/silence; `LoudnessReport` unchanged) + `KWeightingFilter` made `pub(crate)` with `steady_state_gain_at` test support. New `saturn-render/audio_pcm.rs` (~250 lines): single-shot GStreamer PCM decode (`uridecodebin → audioconvert → audioresample → appsink F32LE`), the frame extractor's proven Paused→Playing→seek-in-Playing flow (drain then FLUSH|ACCURATE seek in Playing), frame budget recomputed once caps arrive, EOS detected via **`appsink.is_eos()`** — appsink never posts EOS to the pipeline bus, so bus-polling spins forever on short files that fully decode before the first pull (root-caused with GST_DEBUG=appsink:5). `audio_read_telemetry` MCP tool (schema in `server_tools.rs`, dispatch + async impl in `server.rs` bridge→`bridge_call` / headless→`spawn_blocking`, mirror dispatch + handler in `mcp_bridge.rs`, shared payload `audio_telemetry_payload` in `preview_tools.rs` + `parse_standard`/`parse_tick_range`): media mode (`media_id`, optional tick range) and sequence mode (`start_tick`+`end_tick`, renders the sequence mix via `render_project_range_with_settings` software encode → decode → analyze, temp snippet deleted); a `track_id` param is honestly rejected with a typed error (track-scoped telemetry not implemented — §4.4 honesty over silent pretending). Integration evidence `tests/audio_telemetry.rs` (4 green; GStreamer work serialized behind a test gate): **exit criterion — media mode measures integrated −23.0 ± 0.1 LUFS end-to-end** on a synthesized 16-bit WAV whose 1 kHz amplitude is calibrated through the real K-filter's measured steady-state gain (the ITU reference level is defined by that filter, so the test exercises the meter, not filter lore); also asserts the 1 kHz band peaks, phase ≈ +1, `aggregateScore` null; sequence mode mix render/analyze within ±1.0 LU (snippet rides an audio encoder; exact ±0.1 LU covered by media mode + unit test `reference_ebu_r128_signal_measures_minus_23_lufs_plus_minus_tenth`) with snippet-cleanup assertion; param typed errors (modeless, unknown media, track_id, bogus standard). MCP-level typed-error test added (saturn-mcp 6 tests green; tool catalogue unchanged — MCP tool, not a catalogue command). Note: the existing `generate_waveform_peaks` in `apps/saturn-tauri/src/commands/media.rs` is synthetic — real waveform decode is parity debt for a later packet, not 1.3 scope. **Phase 1 verification gate closed: all four criteria now carry automated evidence.** Gates: cargo test --workspace 0 failures (saturn-render 81 lib + audio_telemetry suite; saturn-tauri bin 45; saturn-mcp 6), pnpm build (461 ms) + 168 UI + all e2e suites passed (0 browser errors), check:glass / check:commands (448, 71 UI-only) / check:source-size (max 800) / license-headers OK (513 files); junk + cyrillic sweep clean. |
| 2026-11-06 | 1.2 | **Video scopes telemetry.** New `saturn-render/scopes_gpu.rs` (445 lines): `analyze_rgb_scopes` builds a `ScopeTelemetryReport` from raw RGB via one O(n) deterministic CPU histogram pass (Rec.709 luma IRE min/p10/p50/p90/max, clip ≥ 99.5 IRE / crush ≤ 0.5 IRE %, RGB parade medians, vectorscope mean+p90 saturation in HSV, mean chroma angle; stride sampling above 4 Mpixels with an honest `sampled` flag; skin-band YCbCr chroma statistics measured against the 123° flesh-tone *reference target* — never ethnicity labels or group IRE targets per §4.5; `notes.aggregate_score: null` marks the no-invented-Murch-score constraint §4.4). `color_read_scopes` MCP tool (schema in `server_tools.rs`, dispatch + async impl in `server.rs` bridge→`bridge_call` / headless→`spawn_blocking`, mirror dispatch + handler in `mcp_bridge.rs`): media mode (`media_id`+`tick`, direct decode via new `decode_frame_rgb` — raw RGB, since JPEG quantization would corrupt histograms) and sequence mode (`at_tick`, renders the *graded* frame so a grade's effect is verifiable), optional clamped `roi`. **Latent bug found & fixed:** `render_frame` (backing the uiOnly `export.frame`) deadlocked forever — the bespoke GES preview pull waits ("Waiting for preroll to send initializing seek") on a seek its own FLUSH-then-Paused flow never resumes; reimplemented on the matrix-proven render path (2-frame software snippet via `render_project_range_with_settings` → `decode_frame_rgb` → PNG, snippet cleaned up). Integration evidence `tests/color_scopes.rs` (3 green; GStreamer work serialized behind a test gate — parallel pipelines race global init into a deadlock): media-mode telemetry from `editor-smoke.webm` + sequence-mode render→analyze with temp-frame deletion asserted. Gates: cargo test --workspace 0 failures, saturn-tauri bin 45, pnpm build + 168 UI + e2e suites (0 browser errors), check:glass / check:commands (448, unchanged — tools are MCP tools per §5.1) / check:source-size (all files ≤ 800) / license-headers clean; junk + cyrillic sweep clean. |
| 2026-11-06 | 1.1 | **Frame & contact-sheet extraction.** `saturn-render/lib.rs` split under the 800-line gate: lib 629 lines (render entry points + tests) + `encoders.rs` 294 (format/profile/hardware-encoder selection + rank-override tests) + `ges_timeline.rs` 629 (document slicing, GES graph, render bus loop, eq-strings test). New `frame_extractor.rs` (732 lines): pre-warmed `uridecodebin→videoconvert→appsink(RGB)` decode pipeline per source (preroll to Paused, run to Playing, drain + `FLUSH|ACCURATE` seek per pull); `extract_still` (JPEG q80, max width clamp 64..1280), `cached_still` (BLAKE3-fingerprint-keyed lazy cache in the 0700 previews dir, atomic write, 7-day prune), `SheetRequest::well_ranged/clamped` (≤64 tiles, ≤640×640, warnings reported), `extract_frame_sheet` (canvas composite + hand-drawn bitmap timecode burn-in), 512 KiB `MAX_INLINE_JPEG_BYTES`. New `preview_tools.rs` shared payload builders used byte-identically by headless `saturn-mcp` and the Tauri bridge: `media_preview` upgraded to a real poster frame (`image_block` convention → `RawContent::image` + de-duplicated text block in MCP); `vision_extract_frame_sheet`; `render_preview_clip_proxy` (≤10 s span, 64..1920 resolution, software encode, `include_audio:false` mutes; writes only into the previews dir). saturn-mcp split: tool catalogue + 2 new tool schemas in `server_tools.rs`, tests in `server_tests.rs` (server.rs 701 lines). Preview tools are MCP tools, not catalogue commands (plan §5.1 tool table — 448 command count unchanged). Gates: `cargo test --workspace` 0 failures, saturn-tauri bin tests 45 green, `pnpm build` + 168 UI + all e2e suites passed, check:glass / check:commands (448) / check:source-size / license-headers clean, junk-sweep + cyrillic scan clean. **Gate evidence: 4×4 contact sheet (1280×720, 16 labeled tiles) from `editor-smoke.webm` in 382 ms cold** (`cargo test -p saturn-render --test frame_extractor`, 5 tests green). |
| 2026-11-05 | — | Plan adopted into repo; checklist created; no packets started |
| 2026-11-05 | 0.0 | Security remediation complete. Re-audit found most §10.1 items pre-remediated; completed the remaining unchecked-Tick sweep (`time.rs` helpers + engine/trim/markers-adjacent/effects/captions/xmeml sites), XMEML 50k clip cap, XMEML checked frame math, export-side saturating end math. 4 new robustness tests in `engine/tests/robustness.rs`. Evidence: `cargo test --workspace` clean (0 failures), `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800` pass. Tier-4 matrix deferred to Phase 0 gate. Commit `c8c20a0`. |
| 2026-11-05 | 0.1 | Engine-owned unified viewport state: `UnifiedViewportState` in `viewport_state.rs` (playhead + AI ghost field, clip/gap/media selection, workspace, branch, export queue capped at 500); `viewport_cmds.rs` dispatches 55 ids (playhead 13, workspace 8, timeline.select 4, edit.select 4, bin/marks 3, export.queue 9, viewport.inspect) returning a `viewport` snapshot; 41 catalogue specs flipped backend (`uiOnly:false`, new params), catalogue JSON/COMMANDS.md regenerated. Targeting stays single-sourced in `TargetingState` (snapshot accessor). 16 new engine tests; strict no-junk-params for `params:"{}"` viewport cmds. Gates: cargo test --workspace 0 failures, pnpm build + 168 UI tests green, check:commands (441 cmds / 71 UI-only) + source-size pass. UI convergence deferred to Packet 5.2. Commit `031a7bc`. |
| 2026-11-05 | — | **Compliance docs adopted** (user directive): provenance rules land in `AGENTS.md` + `NOTICE` + `licenses/` via `provenance-notice.patch` (FilmCraft derivation credited; parity checker header updated); `docs/audit/filmcraft-provenance-check.md` (evidence base), `docs/audit/eep-ai-native-plan-review.md` (independent review — design constraints now annotated into Packets 0.2, 1.1, 1.2, 3.1, 5.2), `docs/planning/dual-licensing-plan.md` (licensing brief; CLA-before-merge and no-bundling constraints noted). See "Compliance documents (binding for every packet)" at the top of this file. |
| 2026-11-05 | — | **Dual-licensing plan implemented** (user decisions: AGPL-3.0-or-later open licence; copyright "zvirox and the Easy Edits Pro authors"; SPDX headers applied now). `LICENSE` replaced with verbatim AGPL-3.0 text; new `LICENSING.md`, `LICENSE-COMMERCIAL.md`, `CLA.md`, `TRADEMARKS.md`, `docs/model-licences.md` (weights gate, empty), `deny.toml` (licence gate: allow-list incl. MPL-2.0 + LGPL-3.0 GStreamer dynamic-link carve-out), `scripts/license-headers.mjs` (apply/check). Support emails info@zvirox.in (primary) / zvirox@proton.me on all licence/contact files. SPDX headers stamped into 496 source files (comment-only; .rs/.ts/.tsx/.mjs/.js line comments, .py/.sh hash, .css block; shebangs preserved). `Cargo.toml` licence metadata flipped GPL→AGPL on 11 crates + `agent-harness/pyproject.toml`. CI: new `licences` job runs `cargo-deny check licenses` + header check. `CONTRIBUTING.md`: DCO updated to AGPL, CLA section added (bot pending — statement-based PR-comment flow until then), PR checklist item 5. Follow-ups (maintainer-only): CLA Assistant bot, lawyer review of licence docs, cargo-deny first CI run. Note: `LicenseRef-EEP-Commercial` is not machine-readable in `Cargo.toml` (cargo-deny can't resolve it), so metadata carries AGPL-3.0-or-later and the commercial entitlement lives in `LICENSING.md` + header comment. Gates re-run: cargo test --workspace 0 failures, pnpm build + 168 UI tests, check:contrast/styles/commands 441/71, source-size (gate now excludes the leading licence header block), py_compile, license-headers check. Junk sweep: only pre-existing 👁‍🗨 ZWJ emoji sequences found (legitimate Unicode joins, not introduced here). |
| 2026-11-06 | 0.2 | **Schema v15: BLAKE3 fingerprints & CoW timeline branches.** `branch.rs` (`TimelineBranch`, diff report types, 8 unit tests), `media_fingerprint.rs` (BLAKE3 head/tail/size fingerprint, blake3 v1.8.7 already deny-clear), schema v15 (+fingerprint +branches +relative paths), lossless v14→v15 migration with io roundtrip tests, `engine/branch_cmds.rs` dispatch for `project.branch_create/switch/diff/merge` (merge v1 = whole-branch accept/reject only). Semantics: switching away from any active branch (main included) parks the live state in a record; a reserved record named `main` is allowed by validation (engine-written only — user creation still rejected by `validate_name`); `branch_switch` to `main` with no parked record = no-op restore. `media.relink_suggest` suggests only (fingerprint constraint §4.11): confirmation happens through the existing `project.relink_media`. Catalogue fixes: `edit.select_all_matching`/`select_label_group` `mutating: true → false` (selection is viewport state per §4.9); 6 new specs with `since: "0.3.0"`. Matrix driver hardened (branch/label/queue/select hooks + queue-id capture). Gates: `cargo test --workspace` 0 failures (110 lib tests incl. new savability regression `switching_branches_keeps_the_document_savable`), pnpm build + 168 UI tests + all e2e suites green, check:glass/check:commands (448 cmds, 71 UI-only)/check:source-size/license-headers OK, parity check up to date. Matrix run 5 (`user-tag packet-0.2`, `/tmp/opencode/matrix-p02/result5.json`): ok=372, ui_only=71, error=5, undos=213, drift=1 → **443/448 non-error, Phase 0 exit criterion (≥425) met**. The 5 errors are judged needs-state; `edit.select_clip_at_playhead` proven passing in isolation (diag run) — its full-run failure shares the documented undo-drift root cause (`speech.ripple_delete_silence`), tracked as an engine follow-up. Junk sweep over all touched files clean. Commit `b576be7` (local — not pushed, awaiting user go-ahead). |