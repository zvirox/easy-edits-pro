# AI-Native and Agent-First Plans — Progress Checklist

**Plan specs:** [AI-native architecture](AI_NATIVE_ARCHITECTURE_PLAN.md) ·
[Agent-first control/review](AGENT_FIRST_PLAN.md) · [Fixed glass cards](GLASS_CARDS_PLAN.md)
**Adopted:** AI-native 2026-11-05; agent-first companion 2026-11-08.
**Status:** In progress · **Reconciled baseline:** `7c20b98` (465 shipped catalogue commands).

> **How to use this checklist.** This is the single source of truth for plan progress; the spec above describes *what and why*, this file tracks *what is done*. When starting work on a phase, tick packets as they land (one commit packet per checkbox, one lane per packet per `AGENTS.md`). A phase counts as complete only when **every checkbox in the phase is ticked, including the Verification Gate**. Update the phase status line after each gate passes, and record the evidence (test run / command + date) next to the gate checkbox.

## Current checkpoint and next-work queue

- **Completed:** AI-native Phases 0–3; Phase 4 plugin/offline operations; Packet
  5.1 J/L, match-cut and beat-grid slices. The glass shell and geometry tests exist.
- **Open:** two remaining 5.1 verbs, all 5.2 panels, Phases 6–7 and the new
  agent-first implementation work below. Phase 4 stays incomplete while cinema
  Dialogue/Music/Ambience/SFX separation is deferred for a verified, license-cleared
  model. Keep the documented room-tone, diarization and plugin-packaging follow-ups.
- **Next actionable packet:** AF0 inventory/actor design, then AF1's proposed
  command/tool profile CSV. Obtain human review before tier/profile implementation.
- **Proposed execution queue, pending maintainer decision:** AF0 → AF1 → AF2 →
  AF4 → AF3 → AF5 → AF6/5.2 → remaining 5.1 and Phase 5 gate → Phase 6 →
  AF7/7.1 → AF8/7.2 → AF9 (before hosting). Base verification/recipe smoke may
  start when its prerequisites exist; it does not complete the larger gates.
  AF4 precedes full AF3 because verification reads brief/review metadata.
- **Pending product choices:** default autopilot/co-edit mode, auto-merge policy,
  Agent/Review placement, initial providers and revised order; see
  [the decision register](AGENT_FIRST_PLAN.md#5-decisions-pending-from-the-maintainer).
  Plan adoption does not silently approve these defaults.

`AF0–AF9` names the attachment's A0–A9 without colliding with older packets.
AF6 is UI and AF9 is daemon. Shared 5.2/7.1/7.2 deliverables have one implementation
and one evidence trail; update both cross-references when their gates pass.

---

## Compliance documents (binding for every packet)

> **Adopted 2026-11-05, per user directive.** While working on this plan, the documents below are compliance requirements, not reading suggestions. Where a document conflicts with a packet in the checklist, the document's constraint wins and the packet is reshaped accordingly; record the change in the progress log.

| Document | Type | Binding consequence for plan work |
|---|---|---|
| [AGENTS.md §Provenance and third-party material](../AGENTS.md) (added via [provenance-notice.patch](audit/provenance-notice.patch)) | Repo rule | Study but never paste third-party code; log every reuse in `NOTICE` **in the same change**; only MIT/Apache-2.0/BSD/ISC/Zlib sources; never GPL/AGPL/LGPL deps or non-commercial model weights without maintainer approval; no third-party product names in UI/docs/identifiers; never the ArtCraft/FilmCraft names or logos. |
| [NOTICE](../NOTICE) | Legal record | The FilmCraft derivation is documented (command inventory + `CommandSpec` shape, MIT OR Apache-2.0 texts under `licenses/`). Any new third-party reuse must add an entry here in the same commit. |
| [AGENT_FIRST_PLAN.md](AGENT_FIRST_PLAN.md) | Companion plan adopted 2026-11-08 | AF0–AF9 extends existing plans: human-reviewed profiles, actor isolation/reversible edits, measured verification, brief/provenance and fixed-dock Agent/Review UI. Preserve shipped progress, whole-branch merge v1 and locked colours; new defaults remain pending decisions. |
| [eep-ai-native-plan-review.md](audit/eep-ai-native-plan-review.md) | Independent plan review | Design constraints absorbed into the packets below: no `/dev/shm` ring (Packet 1.1 uses MCP image blocks / `$XDG_RUNTIME_DIR` previews); contact-sheet budget with token controls (Packet 1.1); no invented scalar "Murch score" — measurable telemetry only (Packet 1.2); no ethnicity-labelled skin-tone logic; no OCIO dependency (fix a small WGSL transform set); BLAKE3 is a *fingerprint*, not a content hash (Packet 0.2); branch merge v1 is whole-branch accept/reject only (Packet 0.2); per-session viewport cursors for Packet 5.2 (human/agent do not share one selection); treat media-derived text as untrusted data; curated ~20–30 default agent tool profile for MCP; headless daemon milestone (review §5 stage C) before any hosting work. |
| [dual-licensing-plan.md](planning/dual-licensing-plan.md) | Licensing brief (not legal advice) | **Licence documents/header gate implemented 2026-11-05 (see progress log); policy/packaging follow-ups remain.** AGPL-3.0-or-later is the open licence + a commercial option (`LICENSING.md`, `LICENSE-COMMERCIAL.md`); SPDX headers and cargo-deny run in CI, but the broad AGPL/GPL/LGPL allow-list still needs AF0's scoped dependency policy and package-bundle verification. CLA (CLA.md) required before outside-PR merges (statement-based flow live; CLA Assistant bot is a maintainer follow-up); never bundle x264 or non-commercial model weights (`docs/model-licences.md` gate); lawyer review is pending — any legal contradiction overrides this table. |
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

## Phase 1 — Perceptual Feedback Loop (AI Eyes & Ears) — `status: done (packets 1.1–1.3 committed; all four verification-gate criteria met; matrix re-run 443/448 non-error, identical to the Phase 0 baseline)`

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

## Phase 2 — Local Neural Perception Engine (`saturn-perception`) — `status: DONE 2026-11-06 (packets 2.1–2.3 committed; all gate criteria met with automated evidence)`

### Packet 2.1 — Real Whisper transcription & script sync (Lane 2) — `DONE 2026-11-06`
- [x] Create `crates/saturn-perception` crate — compile checklist superseded the plan's "ort + rusqlite" deps: Packet 2.1 itself needs neither (those land with 2.2's ONNXRuntime embeddings / 2.2+ index sidecar). Deps: `whisper-rs` 0.14 (Unlicense bindings over MIT whisper.cpp), `ureq` (native-tls), `sha2`. Bundled bindings enabled via workspace `.cargo/config.toml` (`WHISPER_DONT_GENERATE_BINDINGS=1` — bindgen chokes on this glibc)
- [x] Implement `src/whisper.rs` (word-level tick alignment; **diarization deferred to its own packet** per plan-review §91 — segments report `speaker: None`; models: tiny/tiny.en (tests), base/small/medium/large-v3-turbo + `.en` twins)
- [x] Replace 4 hardcoded demo sentences in `apps/saturn-tauri/src/commands/audio.rs` with real inference (16 kHz mono decode in `saturn-render::audio_pcm::decode_audio_pcm_mono_16k`; media→timeline tick mapping via first clip; constant-speed mapping, speed-ramp clips flagged `timingApproximated`; engine lock never held across inference; `text_override` kept but labeled `timingSource: "user_text_synthetic"` — user-authored words with synthesized timing, not a transcription claim)
- [x] Implement `speech.script_sync_align` (script vs recorded takes, take ranking) — engine command analysing caller-supplied transcripts (core stays inference-free); fountain-**heuristic** line parsing (headings/cues/transitions/parentheticals dropped), order-preserving window matching with 0.6 score floor and monotonic per-take cursor; desktop orchestrator `align_script_to_takes` transcribes takes then dispatches. **Packet evidence:** `cargo test -p saturn-core` (7 `speech::` unit tests incl. ranking/monotonicity/no-match honesty + `script_sync_align` engine tests in `tests/command_workflow.rs`); end-to-end real-inference proof `tests/whisper_jfk.rs` (canonical jfk.wav; "fellow americans" in real transcript; ticks within 11 s ⇒ centisecond units verified)

### Packet 2.2 — Shot detection, SigLIP search & cut continuity (Lane 2) — `DONE 2026-11-06`
- [x] Implement `src/shot_detect.rs` + `src/embeddings.rs` + sidecar `.saturn-index.sqlite` — shot detection: composite score (0.6·symmetric χ² over [1,2,1]/4-smoothed 32-bin luma histograms + 0.4·mean block |Δluma|), threshold 0.32 with 0.5 s minimum shot gap; **dissolves/fades are not detected (documented limitation)**. Embeddings: pinned `Xenova/siglip-base-patch16-224` @ `4649052…` ONNX exports via `ort` 2.0.0-rc.15 (CPU EP), preprocessing pinned to upstream `preprocessor_config.json` (224×224 bicubic a=−0.5, 1/255 rescale, mean/std 0.5); **both exports expose an on-graph `pooler_output` [1,768] — used directly**, ids-only text input. Sidecar `rusqlite` bundled SQLite keyed `(media_id, tick)` with embedding-revision stamping; stale-revision vectors drop on re-index; plain cosine scan (documented honest deviation — no vector extension at this scale). Colour science split into `color_metrics.rs`: CIEDE2000 verified against 7 Sharma/Wu/Dalal 2005 reference pairs
- [x] Implement `CutContinuityScore` (eye-trace, luma jump, CIEDE2000, flow angle, 180° rule, audio pop) — per plan-review §4.4: every component is measured or honestly `None`. `luma_jump_ire` (|ΔIRE|), `chromatic_delta_e` (mean-color CIEDE2000, stated approximation), `focal_centroid_distance_norm` (**labelled proxy**, not measured gaze), `optical_flow_angle_diff_deg` (16×16 block SSD ±8 px, center-out tie-breaking, measured *within* each shot via decoded neighbor frames; `None` when either side < 0.5 px), `screen_direction_preserved` **always `None`** (180° rule needs a subject-orientation model), `audio_boundary_pop_db` (`None` on silence/missing audio), plus `heuristic_continuity_score` explicitly labelled unvalidated and rule-derived `recommendations`
- [x] Expose `media_semantic_query` + `edit_evaluate_cut_continuity` over MCP — shared payload builders in `saturn-render::preview_tools` (headless `saturn-mcp` + desktop `mcp_bridge` return byte-identical shapes); desktop commands `media_semantic_index` (≈2 s sampling, 400-frame cap per media) and `media_detect_shots`. **Packet evidence:** `cargo test -p saturn-perception` (25 unit tests incl. CIEDE2000 references, block-motion direction, index revision staleness, indexing-loop failure accounting via mock embedder); gated real-inference e2e `tests/siglip_e2e.rs` — deterministic L2-normalized embeddings, cross-modal ranking (green frame → "green grass" beats "the ocean at night"), cosine scan over 502 stored vectors averaged ≪ 50 ms budget; `cargo test -p saturn-mcp` (typed honest errors: unsaved project, empty index, unknown media, missing source, cut at first frame). Model licence rows (SigLIP exports + ONNX Runtime) added to `docs/model-licences.md` in the same change

### Packet 2.3 — Neural matting, depth & motion tracking (Lane 2) — `DONE 2026-11-06`
- [x] Implement `src/matting_ort.rs` (RVM v2 / MODNet) replacing the Gaussian/skin-color heuristic — **RVM refused at the licence gate** (repo + its exported ONNX models are GPL-3.0, upstream-maintainer-confirmed via PaddlePaddle/FastDeploy#1059; permissive-only provenance rule); MODNet alone satisfies the plan's "RVM v2 / MODNet" alternative. New `crates/saturn-perception/src/matting_ort.rs`: pinned `Xenova/modnet` @ `fa2fa54…` fp32 export (25,888,640 B, SHA-256 `07c308cf…`, verified 2026-11-06; Apache-2.0 for code+models+demo since 2022-01 per ZHKKKe/MODNet#70) with fetch-on-first-run download + checksum verify; preprocessing pinned to the export's `preprocessor_config.json` (shortest edge 512, divisibility-by-32, bilinear, 1/255 then mean/std 0.5 → [−1,1]); dynamic `input [B,3,H,W] → output [B,1,H,W]` name-checked before indexing (ort panics on missing output keys); deterministic bilinear resize/upsample back to frame resolution; explicit **EMA-labelled** matte smoother (per-frame model — temporally this is caller-side smoothing, NOT RVM-style recurrent memory, stated in-code); shared prefix-sum `feather_alpha` (output clamped — alpha is semantically bounded). **Portrait-only** limitation documented in code, command response and UI. The fake Gaussian/skin-chroma heuristic and the RVM-shaped `MattingTemporalState` are deleted from `crates/saturn-render/src/matting.rs`; render keeps only `composite_matting` (threshold rescale → shared feather → transparent/green/solid blend) over a *provided measured* alpha — a matte not backed by a real model run can no longer exist. `run_background_matting` command rewritten: takes `media_id` + tick + options (no more RGBA payloads), engine lock held only for the snapshot, decode + MODNet inference + composite on the blocking pool, returns the matted RGBA **plus measured `alphaMean`/`foregroundShare` and the portrait-only note**. UI `BackgroundMattingCard.tsx` rewritten: synthetic 128×128 skin-tone payload and canned success text removed; RVM option removed (model picker is gone — MODNet only); no-clip guard; the returned frame is drawn into a canvas as measured proof, with the coverage % reported numerically
- [x] Implement `src/depth_ort.rs` (Depth-Anything-V2) — **Small model only**: upstream Small weights are Apache-2.0; Base/Large/Giant are CC-BY-NC-4.0 (maintainer-confirmed, DepthAnything issue #81) and are excluded from the catalogue. Pinned `onnx-community/depth-anything-v2-small` @ `4472b73…` int8 export (27,258,801 B, SHA-256 `fcf51f1b…`, verified 2026-11-06; upstream `.pth` recorded for traceability); preprocessing pinned to the `DPTImageProcessor` config (518×518 — both edges multiples of the 14-px ViT patch grid, 1/255, ImageNet mean/std); declared-output name check, finite-value check, exact-dims check. **Honest deviations documented in-code:** relative depth only (never meters; in-frame min–max normalization with a `flat` flag instead of a fake result), fixed square resize vs upstream `keep_aspect_ratio`, int8 quantization drift
- [x] Replace synthetic sine/cosine motion data in `apps/saturn-tauri/src/commands/preview.rs` with real `track_patch_displacement` + 4-corner RANSAC homography — new `crates/saturn-perception/src/motion_track.rs`: exhaustive-SSD patch search with parabolic sub-pixel refinement and **measured ZNCC confidence** (0..1 — flat texture collapses to ~0, replacing the old hardcoded `confidence: 0.95`), Hartley-normalized DLT homography (8×8 gauge h[8]=1, partial-pivot solve, singular → honest `None`), deterministic seeded-LCG RANSAC (all-4-triplet collinearity guard, consensus refit), and `track_region_frames` (3×3 grid of 32 px patches matched from the reference position — non-incremental so one bad match cannot compound — with per-frame RANSAC homography and `None` entries for sub-threshold frames, never interpolation). Command `analyze_motion_tracking` rewritten: `media_id`-based with typed `MotionTrackResult` (points with measured confidences, `ticksDropped`, `meanReprojectionErrorPx`), bounded decode cap (≤48 frames) at a ≤960 px long edge, engine lock held only for the snapshot; the fake `jitter_factor` parameter is gone (no UI caller existed; command stays registered). UI preview card sends the real clip media id
- [x] *(packet addition)* Gated real-inference e2e `crates/saturn-perception/tests/matting_depth_e2e.rs` (`SATURN_PERCEPTION_E2E=1` / pre-cached `SATURN_MODELS_DIR`, mirroring `siglip_e2e.rs`): MODNet — deterministic frame-sized matte in 0..=1, person-less synthetic scene honestly reports low mean alpha, feather + smoother compose in range; DA-V2 — deterministic finite positive 518×518 grids with measurable spread (not flat on textured input), relative normalization in 0..=1, upsample covers the frame. Model licence rows (MODNet export, DA-V2-Small upstream weights + export, **RVM refused entry**) added to `docs/model-licences.md` in the same change

### ✅ Phase 2 Verification Gate — `PASSED 2026-11-06`
- [x] `cargo test -p saturn-perception -p saturn-tauri` green — 43 perception lib tests (whisper 2.1, shot/siglip/index/color 2.2, matting/depth/motion 2.3) + gated e2e `matting_depth_e2e.rs` 2 real-inference tests; saturn-tauri bin 45 tests green
- [x] **Zero fake/hardcoded outputs remain** in `commands/audio.rs` (2.1), `commands/preview.rs` (sine/cosine + hardcoded 0.95 confidence replaced by measured tracking, 2.3), `matting.rs` (Gaussian/skin heuristic + RVM state deleted, composite-only over measured alphas, 2.3)
- [x] `media_semantic_query` returns relevant clips in < 50 ms post-indexing — evidenced in Packet 2.2 (cosine scan over 502 stored vectors ≪ 50 ms; model inference is outside the budget by definition)
- [x] *(2026-11-07, v0.1.2-alpha.1 packages fix)* `ort` downgraded 2.0.0-rc.15 → 2.0.0-rc.10 (ONNX Runtime 1.31 → 1.22 series): pyke's prebuilt runtime binaries from ort-sys rc.11 onward are built against glibc ≥ 2.38 (symbol-bisect of the pyke archives rc.9–rc.14: `__isoc23_strtol/strtoll/strtoull` + `_M_replace_cold` appear first at rc.11, ONNX Runtime 1.23.2), so `libort_sys` failed to link on the release packages job (ubuntu-22.04, glibc 2.35) and would have forced a glibc ≥ 2.38 runtime floor that breaks the Debian 12 install proof. rc.15 ships no older-glibc build target and ort-sys rc.15 has no from-source strategy; rc.10 is the newest ort on the repo's supported floor. Perception follow-through is mechanical (`commit()`, env-free `Session::builder()`, `inputs`/`outputs` fields); all model e2e re-run green on the 1.22 runtime (MODNet matte, DA-V2 depth, SigLIP cross-modal); the Debian 12 / Ubuntu 22.04 package support floor is unchanged

---

## Phase 3 — Scene-Referred 32-Bit Float GPU Color Engine — `status: done (packets 3.1–3.3 and all verification criteria completed; evidence below)`

### Packet 3.1 — Real wgpu `Rgba32Float` pipeline & OCIO v2 ACEScg (Lane 2) — `status: done 2026-11-07`

> **Plan-review revision (docs/audit/eep-ai-native-plan-review.md §4.6):** no OpenColorIO C++ dependency. Implement a **fixed, small WGSL transform set**: ACEScg/ACEScct matrices, a handful of camera log curves, and Rec.709/sRGB/P3 outputs; add more only on user pull. The module keeps the `ocio_pipeline` name for API continuity but must not link OCIO.

- [x] Create `src/gpu_context.rs` + `src/ocio_pipeline.rs` (real `wgpu::Device/Queue`, `Rgba32Float`, Linux DMA-BUF import — **pure WGSL transforms, no OCIO dep**) — `crates/saturn-render` (wgpu 30), WGSL constants generated from the `saturn_core::color` source of truth; GPU-vs-CPU agreement over 6 IDTs × 2 working × 2 ODTs at 2e-4 on real hardware
- [x] Implement `ocio.set_pipeline` + `ocio.set_media_idt` (camera log curves → ACEScg/ACEScct → Rec.709 / P3 output transforms, in WGSL) — schemas `Project.color_management` / `MediaAsset.idt`, imports seed `default_idt`
- **Documented simplification:** display transforms are primaries matrices + transfers (no ACES RRT tone scale); Rec.2020 PQ maps SDR white 1.0 → 100 nits. Camera curves/matrices verified against vendor specs and colour-science anchors (transform module tests).

### Packet 3.2 — WGSL color node graph, power windows & film emulation (Lane 2)
- [x] Create `src/color_nodes_gpu.rs` + `src/film_emulation.rs` (wheels, HDR zones, curves, 3D LUTs, HSL, warper, power windows, halation + grain)
- [x] `color.node_add` / `node_update` / `add_power_window` / `emulate_film_print` wired end-to-end
- [x] Shader hardening per spec §10.2.3 (naga pre-validation, error scopes, no crash on faulty shader)

**Packet 3.2 notes (§4.4 deviations):** seven node kinds shipped (plan listed five — `color_warper_grid` and `film_print_emulation` added with the frame-emulation set). Gate weave ships a documented deterministic model (incommensurate 24/17-frame sinusoidal beat, amplitude = `weave_mm / 36` gate width — not a measured statistical model; later-packet item). Halation bright-pass threshold 1.0, Gaussian σ=2 (9×9, normaliser 23.990704), red-dominant tint (1.0, 0.35, 0.15) and grain 0.10 scale are documented design constants. Curves window gating mixes toward the curved value by gate (full replace would make windows a no-op). Also fixed here: a Packet 3.1 readback bug where every texture row copied texture row 0 (per-row copies used the default `(0,0)` copy origin) — visible in any multi-row GPU readback.

### Packet 3.3 — Automated shot matcher (Lane 2)
- [x] Create `src/shot_match.rs` (`color.match_shot_to_reference` with CDF alignment + skin-tone lock)

**Packet 3.3 notes (§4.4 deviations):** the engine command `color.match_shot_to_reference` is decode-free by design (`saturn-core` owns state, not media): the caller that owns frame decoding — `saturn-render::shot_match_driver`, driven identically by the desktop bridge (`color_match_shot_to_reference`) and the headless MCP tool, both reusing one payload function — measures both clips' ungraded source frames, fits, and hands the fitted wheels to the engine command, which validates them through the exact `color.node_update` contract, checkpoints, and appends/updates a `primary_wheels` node stamped `origin: "shot-match"` (schema `ColorNode.origin`, `None` for operator nodes; re-runs update the machine node instead of stacking). Both anchor clips must exist; bad payloads never become nodes. The plan's command schema gains the `wheels` parameter — documented deviation, forced by the decode-free core — and the command's UI affordance is the agent surface (bridge + MCP tool); no colour-page button yet. Gate metric — **documented reinterpretation of "ΔE < 1.5," not a silent redefinition**: the plan's metric holds as *mean per-pixel CIEDE2000 ΔE < 1.5*; a second *worst 3×3-patch ΔE < 1.5* gate is added; the raw worst single pixel is reported in the failure message only, never gated. Justification (measured, graded `[39,38,40]` vs `[39,39,39]`): at L\*≈15 a single 8-bit code step is worth ΔE ≈ 1.8 on its own, so single pixels sit beyond the threshold for *any* algorithm once the two frames' independent ±half-code roundings are counted — the patch mean (continuous signal averaged per 3×3 block before the Lab conversion) cancels sub-code quantisation and measures the actual fit residual. Skin lock aligns the **chroma hue-direction vector** (magnitude-weighted mean of unit band vectors, `chroma_u/v` on `SkinStats`; `chroma_vector_err_codes` on `SkinLock`), not the vectorscope hue angle: ±0.5 stops of R/B tilt rotates skin hue under 2°, so an angle-only objective traded ten-percent gain changes for quarter-degree hue wins (measured δ=+0.139 disaster). Band statistics are magnitude-weighted means **divided by Σ magnitude, not pixel count**, computed on sampled pixels' decoded, continuously re-encoded signal (symmetric with the graded-side instrument): a count divisor let ~400 warm-grey leaks in the target's band dilute its vector by 27% while the reference's band had none, and raw-code measurement baked a ~0.4–1° round-half chroma bias into the band angle. The gate's synthetic skin patch is **textured** (±3% deterministic jitter): a constant patch rounds every pixel the same direction, baking a phantom ~½-code transverse chroma shift into the frame's band mean. Continuous-domain Lab conversion added as `rgb_continuous_to_lab` in `saturn-perception` (`rgb_to_lab`'s doc corrected: it is the sRGB piecewise decode). File split by area per the 800-line rule (`motion_track_tests.rs` precedent): `shot_match.rs` (722) + `#[path]` siblings `shot_match_tests.rs`, `shot_match_measurement.rs`; the source-size gate's roots do not scan `saturn-render`, but the rule is honored regardless.

### ✅ Phase 3 Verification Gate
- [x] `cargo test -p saturn-render --test color_pipeline` green
- [x] +3.0 stops (Node 1) then −3.0 stops (Node 2) in ACEScg recovers 100% highlights, zero clipping
- [x] Shot match brings mismatched angles within CIEDE2000 ΔE < 1.5 (Packet 3.3) — mean per-pixel ≈ 0.57 graded through the real persisted wheels payload; see the documented gate definition in the Packet 3.3 notes above

---

## Phase 4 — Cinema Audio DSP (`saturn-audio-dsp`) — `status: in_progress`

### Packet 4.1 — Native plugin host & render parity (Lane 2)
- [x] Create `crates/saturn-audio-dsp` crate + `src/clap_host.rs` (CLAP/LV2/VST3 via `clack-host`)
- [x] `dsp.load_plugin` / `dsp.set_plugin_param` wired into preview AND export graph (bit-parity)
- [x] Plugin out-of-process sandbox (`saturn-plugin-worker`) per spec §10.2.4

**Packet 4.1 notes (§4.4 deviations):** hosting is **CLAP only** via
`clack-host = "=0.2.0"`; the schema keeps `lv2`/`vst3` tags round-trippable
but `dsp.load_plugin` rejects non-CLAP formats at the command layer with a
typed `InvalidParameters` (listing the hosted formats), and the sandbox's
chain load carries a second typed `DspError::FormatNotHosted` rejection —
defense in depth, not a silent skip. The trust policy lives in
`saturn-core::project::dsp_trust` (canonicalized-path containment in
`$HOME/.clap`, the XDG `clap` dir, `/usr/{,local/}lib/clap`, and
operator-opt-in `SATURN_PLUGIN_ROOT` entries; 64-slot chain cap;
`HOSTED_PLUGIN_FORMATS`), so the command layer and the plugin host enforce
exactly one policy; `saturn-audio-dsp` only maps the shared failures onto
`DspError::PluginPath*`. Sandbox shape follows plan §10.2.4: a dedicated
`saturn-plugin-worker` child process, UDS control + `/dev/shm` 0600 SPSC
rings (owner unlinks on Drop), strictly one command in flight, a single
typed auto-respawn + buffer replay after a worker crash, and a second
consecutive crash surfacing as typed `DspError::Sandbox`. **UI affordance =
agent surface** (desktop bridge + MCP through the catalogue; commands
`shipped: true` with real engine implementation + tests) — the Packet 3.3
precedent; no panel UI yet. The §4.4 "preview AND export graph bit-parity"
requirement is met as the documented deviation: both the engine preview
render path and the full export render run the **identical**
`saturnaudioplug` GStreamer element backed by the same
`SandboxSession` host, and `tests/dsp_plugin_parity.rs` measures preview
vs export MP4 audio below −90 dBFS plus an element-level exact fixture-gain
run; the web preview stays the plan's JS approximation (no WASM CLAP host
in this packet). Commands count 456 → 458 (`dsp.load_plugin`,
`dsp.set_plugin_param`, both `since` 0.4.0). **Follow-up
(Lane 3 packaging):** `saturn-plugin-worker` and the `SATURN_PLUGIN_ROOT`
contract must be added to the desktop packaging manifest and flatpak
permissions review before any release ships plugin hosting.

### Packet 4.2 — Stems, phase align, room tone & ducking (Lane 2)
- [ ] `src/stems_ort.rs` — `audio.separate_stems` (Dialogue/Music/Ambience/SFX): **deferred by maintainer decision** pending a verified, license-cleared cinema-stem model. The named music model exposes vocals/drums/bass/other; do not relabel its outputs.
- [x] `src/phase_align.rs` — `audio.auto_phase_align` (GCC-PHAT sub-sample)
- [x] `src/room_tone.rs` — `audio.fill_gaps_with_room_tone` (caller-selected profile; in-gap fades, with dialogue-edge crossfades still pending)
- [x] `src/ducking.rs` — `audio.sidechain_duck_envelope` + `audio.loudness_normalize_master`

**Packet 4.2 scope and deviations:** shared `audio_process` tool on MCP and the
desktop bridge; four atomic core commands, 458 → 462 catalogue entries. Saved
projects only; derived float WAVs are project-local, retained for history and
roundtrip, deleted on failed preparation/application. Snapshot digests reject
stale work. Phase alignment handles two single-clip audio tracks and a constant
raw-source delay. Room tone uses a caller-selected speech-free spectral profile
with fades inside gaps, not crossfades of existing dialogue. Ducking is raw-source
RMS energy-keyed, not a speech detector, and requires empty music automation
lanes. Normalization is an undoable GES master bounce: originals remain muted,
solo flags cleared; later source edits require rebouncing. Processing is bounded
to 120 seconds. True peak now uses 4× FIR reconstruction. GES timeline renders
serialize staged effect payloads across export and master bounce. Detailed
contracts, limits and test locations: [audio-processing.md](audio-processing.md).

### ✅ Phase 4 Verification Gate
- [x] `cargo test -p saturn-audio-dsp` green (included in the full workspace run)
- [x] Preview vs rendered MP4 audio match within −90 dBFS (`dsp_plugin_parity` ran locally with codecs, no skip; native engine render paths, not browser playback)
- [x] Boom+lav comb filtering eliminated on 1.7 ms delayed fixture (>45 dB alignment-residual improvement; correlation >0.999)
- [x] Normalization hits `-14.0 ± 0.1 LUFS` and `<= -1.0 dBTP` (lossless master; encoded delivery requires remeasurement)

---

## Phase 5 — Narrative Editing Verbs & Co-Editing UI — `status: in progress`

### Packet 5.1 — Narrative editing verbs (Lane 2)
- [x] Implement `timeline.asymmetric_trim` (J/L-cuts with constant-power crossfade; first-task slice described below)
- [x] Implement `edit.match_cut_align` (eye_trace | motion | transient; bounded heuristics described below)
- [x] Implement `timeline.beat_grid_quantize` (bounded constant-pulse heuristic; explicit downbeat for 4/4 bar/half)
- [ ] Implement `timeline.fluid_morph_cut`
- [ ] Implement `timeline.pin_clip_to_storyline` (magnetic storyline pins in `saturn-timeline`)
- [x] Run `pnpm catalogue:sync` for J/L, match-cut and beat-grid slices (rerun for each remaining verb)

**Packet 5.1, first-task slice — J/L split edits:** select the incoming clip of
an adjacent video pair via **Sequence > Trim > Create J/L Split Edit...**.
Picture and sound deltas are relative to the original cut. The picture cut
rolls without moving the pair's outer edges; a cosine/sine constant-power fade
is centered on the new sound cut (40 ms default, 0 = hard cut, maximum 5 s).
The sound becomes a visible, independently editable stereo float-WAV clip on
a new audio track. Per-clip gain is baked; track gain/mute/solo/group membership
is retained. Schema v16 adds `Clip.audio_enabled` (true for older projects),
so the original video clips supply picture only in native and browser playback.

Saved projects only, at most 120 seconds per pair, with mono/stereo source
audio and sufficient picture/sound handles. The first slice rejects retimed,
linked, multicam, compound, disabled or already-detached clips; effects,
keyframes, tracking, transitions and non-neutral track processing/routing
must be removed first. Static clip colour adjustments are retained. Audio
boundaries are quantized to 48 kHz. The crossfade is baked: trim/move/mix the
resulting audio normally, or undo and rerun to revise the original cut/fade.

MCP uses `audio_process` with `operation: "asymmetric_trim"`; desktop command
dispatch prepares outside the engine lock. Both apply one validated undo
transaction and reject stale snapshots. Derived WAVs are project-local,
removed on failed application, and retained for history and save/reopen.
Core tests cover timing, locks, handles, overflow, stale rejection, history
and migration. The native test generates real AV sources, checks both J/L
fades and changed picture frames, and compares exported audio to a bounce-only
control below -90 dBFS; re-enabling original sound is the negative control.
Browser tests cover scheduling and the menu's IPC/error contract.
**Packet 5.1, second-task slice — match-cut alignment:** **Sequence > Trim >
Align Match Cut...** selects the incoming clip and its unique predecessor.
The `edit_match_cut_align` MCP tool and desktop preparation use raw source
centroids, signed block displacement or attack envelopes, then apply one
validated incoming-source slip. Default search is +/-12 project frames,
maximum 30 and two seconds each side, clamped to source handles. `eye_trace`
is explicitly a contrast-centroid heuristic, not gaze or subject identity.
No-signal analysis errors; no-op results preserve history. Align before J/L
detachment. Native fixtures verify real picture/sound changes, deterministic
sampling and selection, decoy rejection, and unaligned export controls.
See [match-cut-alignment.md](match-cut-alignment.md) for workflow and bounds.
**Packet 5.1, third-task slice — beat-grid quantization:** **Sequence > Trim >
Quantize Cuts to Music...** and `timeline_beat_grid_quantize` share bounded,
model-free raw-music analysis. RMS-rise autocorrelation and a weighted pulse fit
produce a 60–200 BPM grid with measured support/residual statistics. Bar/half
require a caller-supplied downbeat and assume 4/4. All cuts in a continuous
2–128 clip video strip roll atomically, preserving outer edges and source mapping;
collisions, insufficient handles and stale snapshots error. Quantize before
detaching sound. Core history/persistence, native rendered cut/audio controls,
MCP preparation and browser menu/error contracts have focused coverage.
See [beat-grid-quantization.md](beat-grid-quantization.md) for bounds and limits.
Morph cuts, storyline pins and Packet 5.2 remain planned.

### Packet 5.2 — Co-editing React 19 panels (Lane 1)

> **Plan-review constraint (docs/audit/eep-ai-native-plan-review.md §4.9):** human and agent get **separate viewport cursors/selections** — the human's lives in the GUI session, the agent's in its command session; they do not overwrite each other. Viewport state stays out of undo and out of dirty-state. Also gated on the glass-card shell (build after G3 of the UI plan); branch diff ribbon works with Packet 0.2's whole-branch accept/reject v1.

> **Agent-first integration:** implement these same deliverables under AF6 after
> AF2 actor/conflict semantics and AF3–AF5 metadata/verification/provenance.
> `AgentTelemetryDrawer` is dock content, never a floating feature panel. Per-edit
> rejection must wait for AF2's written conflict rule; v1 merge stays whole-branch.

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
> Shared with AF7; add supervised recipes and profile/session/verify controls as
> their backends ship. Early recipes do not complete the deferred VFX CLI group.
- [ ] Add `branch`, `perception`, `ocio`, `telemetry`, `vfx` subcommand groups under `agent-harness/cli_anything_eep/commands/`
- [ ] Update `agent-harness/cli_anything_eep/skills/cli-anything-eep.md` with closed-loop agent skill definitions

### Packet 7.2 — Closed-loop acceptance suite (Lane 3)
> Shared with AF8; reuse this suite for live-bridge acceptance and keep scripted
> smoke results distinct from real-model benchmark results.
- [ ] Create `tests/native/ai_closed_loop_workflow.py` (multi-cam rough cut → continuity check → ACEScg grade → supplied dialogue/music-track ducking + EBU R128 master → H.264/WebM export, over the live socket bridge; cinema-stem separation coverage waits for the verified model and cannot be claimed by this fixture)

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

## Agent-first companion — AF0–AF9 — `status: integrated; implementation pending`

Specification: [AGENT_FIRST_PLAN.md](AGENT_FIRST_PLAN.md). Each milestone below
may need several sequential, single-lane commits. Core/schema = Lane 2;
MCP/harness/docs = Lane 4; UI = Lane 1; packaging/native verification = Lane 3.
Packet IDs are stable labels; follow the dependency queue above.

### AF0 — Baseline, housekeeping and actor design
- [x] Reconcile the supplied plan at `7c20b98`, add companion/cross-links and refresh the local `docs/planning.md` index (kept ignored per repository policy); retain completed work and blocked stems. Move misplaced match-cut evidence into the progress log and correct stale checklist header/Phase 3 status.
- [ ] Inventory dependency licences and actual package contents; record first-party ownership separately from third-party licences (Lane 4).
- [ ] Scope `deny.toml` policy/exceptions to the approved inventory; prove an unapproved copyleft-only dependency fails while owned first-party crates pass (Lane 4; coordinate with Lane 3 for CI).
- [ ] Generate/distribute third-party notices/licence texts and verify .deb/.rpm/Flatpak contents, including plugin-worker packaging follow-up (Lane 3).
- [ ] Write actor/branch/history/lock/Stop and stable-change conflict design; reconcile historical UI decisions and confirm the proposed execution order (Lane 4).
- [ ] **Gate:** dependency negative control, package-content evidence and reviewed actor design recorded; documentation integration alone does not complete AF0.

### AF1 — Reviewed profiles and explicit grants
- [ ] Generate proposed `docs/reference/command-profiles.csv` from the current catalogue plus dedicated MCP tools, wrappers and their effects; **stop for human review** (Lane 4).
- [ ] Record approval and add tier/profile metadata to canonical `saturn-core::catalogue::CommandSpec`; regenerate catalogue references and gate complete assignments (Lane 2).
- [ ] Add profile discovery/loading and enforce grants/profile membership across generic run/batch, dedicated preparation tools and bridge sessions (Lane 4).
- [ ] **Gate:** default core + one profile ≤30 tools; review/manual read-only; typed wrong-profile hint; no permission elevation or wrapper bypass; GUI/CLI catalogue remains complete.

### AF2 — Actors, conflict-safe review, locks, Stop and atomic batches
- [ ] Implement Rust-owned actors, per-actor cursors, selections, targeting and branch context; document shared job ownership (Lane 2).
- [ ] Bind transport connections to actor identities and enforce session modes/resume authority (Lane 4).
- [ ] Attribute history and stable change records; implement agent-only undo/per-change review using the reviewed conflict rule; refuse conflicts with later human edits and stale branch merges (Lane 2).
- [ ] Persist actor-owned clip/region locks with migration/reopen and disconnected-owner handling; enforce on mutations and merge (Lane 2).
- [ ] Add engine-side cancellation/late-apply guards and preparation cleanup hooks (Lane 2).
- [ ] Add session Stop/resume orchestration; propagate cancellation into analysis/render/preparation and reject subsequent calls while stopped (Lane 4).
- [ ] Add opt-in atomic engine transactions with rollback and documented undo boundary; reject unsupported external effects (Lane 2).
- [ ] Expose atomic MCP batches with per-step policy checks and explicit sequential compatibility (Lane 4).
- [ ] **Gate:** multi-actor selection/branch isolation, dependency-conflict rejection, durable locks, failure-after-first-step rollback and measured Stop ≤1 s; no late mutation after cancellation.

### AF4 — Rationale, confidence and human-owned brief
> Depends on AF2; complete before AF3's brief/review-aware gate.
- [ ] Persist per-edit rationale (≤200 characters), declared confidence and human corrections on history/clip/cut records; schema migration from v16, reopen and merge retain provenance (Lane 2).
- [ ] Add Rust-owned `project.brief_set/get` for goal, platform, duration/tolerance, must-keep/banned content, style and loudness; define human authorization for brief changes (Lane 2).
- [ ] Carry metadata through generic command/batch and dedicated mutation tools; keep media-derived content out of trusted brief updates (Lane 4).
- [ ] **Gate:** migration/save/reopen/merge and correction tests; confidence is labelled agent uncertainty, not measured quality.

### AF3 — `verify.run` measured self-check
- [ ] Implement bounded named checks from the companion §2.3: loudness/peak, silence, black/frozen/clipped frames, captions, brief targets, continuity/pops, missing resources and unreviewed uncertain edits (Lane 2 analysis).
- [ ] Expose the shared read-only verifier over MCP/bridge with snapshot/range/coverage, measurements, thresholds, tick/clip evidence and unavailable checks; document presets (Lane 4).
- [ ] **Gate:** seeded bad projects fail at correct ticks/values; clean fixtures pass; missing/truncated measurements never masquerade as full coverage or invented quality scores.

### AF5 — Untrusted media provenance
- [ ] Version and wrap media-derived text consistently across MCP inspection/search/preview/error payloads; propagate server-owned origin information (Lane 4).
- [ ] Add core origin/brief-boundary validation required by the reviewed provenance contract (Lane 2).
- [ ] Enforce origin-aware grants and explicit approval for untrusted-derived destructive/filesystem arguments; cover generic dispatch and batches (Lane 4).
- [ ] **Gate:** adversarial transcript/import/metadata, copied/modified arguments and unknown origins yield approval/refusal with no unauthorized side effects; legitimate approved operations pass.

### AF6 — Agent/Review UI and existing Packet 5.2
> Depends on AF1–AF5 and the existing fixed glass shell. Confirm placement/defaults
> before implementation; all additions use existing tokens and fixed slots.
- [ ] Add always-visible status/Stop and pinned dock status; Agent card session/grants, brief, plan, messages, last-seen evidence and permission requests (Lane 1).
- [ ] Add Review queue, confidence/rationale, before/after previews, verification navigation, whole-branch actions and conflict-safe per-change review backed by AF2 (Lane 1).
- [ ] Add ghost cursor, timeline badges/locks/diff ribbon and actor history/undo; complete the four existing Packet 5.2 panel checkboxes rather than replacing them (Lane 1).
- [ ] Add agent settings/onboarding and fold the existing assistant into the card only after feature/provider parity (Lane 1).
- [ ] **Gate:** demo run → review → accept/reject → merge; later human-edit conflicts preserved; fixed geometry across workspaces and 900–1280 px/wide windows; keyboard/status/Stop and colour-freeze checks pass.

### AF7 — Closed-loop recipes and Packet 7.1 CLI
- [ ] Extend the existing harness/skill with profile/session/brief/verify/review flow and the five companion §2.6 recipes; use supplied dialogue/music sources while stems remain blocked (Lane 4).
- [ ] Complete Packet 7.1 groups as backends ship, including VFX after Phase 6; unsupported actions return errors (Lane 4).
- [ ] **Gate:** scripted podcast cleanup reaches measured verification pass and human review; exact tool shapes and undo/preparation limits documented.

### AF8 — Benchmark and Packet 7.2 closed-loop suite
- [ ] Build 10–20 synthetic/approved-CC0 tasks, provenance records, metric/structure evaluation and labelled model/version/call/token/time reports; unavailable metrics remain explicit (Lane 4).
- [ ] Reuse Packet 7.2 live-bridge suite and add deterministic CI smoke with seeded failure controls (Lane 3).
- [ ] **Gate:** CI smoke and live workflow pass; publish a real-model baseline separately from scripted results; Phase 7 final gates remain required for full completion.

### AF9 — Authenticated long-lived daemon
- [ ] Extend existing MCP/control components with project lifecycle, UDS session tokens, rate/time budgets and bounded JSONL audit records; reuse AF2 Stop and AF5 provenance (Lane 4).
- [ ] **Gate:** reject missing/wrong tokens; prove isolation, audit completeness, budget enforcement, disconnect cleanup and Stop. Required before hosting work.

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
| 2026-11-08 | Render boundary local gates | Final focused correction passed **583 workspace tests plus four cold-process scenarios**, **45 desktop tests**, **174 UI tests**, build, glass/token/style, commands, source-size, catalogue/parity and SPDX gates; all **nine mocked E2E areas** passed with zero browser errors. Final workspace-built beat-grid test also passed under Ubuntu 1.24 runtime libraries. Links, added-line character sweep and whitespace passed. The separate 1.28 software-cadence follow-up below remains open. |
| 2026-11-08 | Render boundary verification | Run [42465005391](https://github.com/zvirox/easy-edits-pro/actions/runs/42465005391) passed the formerly stalled audio suite, then exposed an Ubuntu native beat-grid cut one frame early. Reproduced with Ubuntu 1.24 runtime libraries and confirmed by independent full-stream decoding. Explicit square-pixel caps prevent a mid-cut frame-rate-converter reset; original cuts 82/139 and edited cuts 75/135 now decode correctly with 360 frames and unchanged outer edges on Ubuntu. Added original-cut assertions. A separate, baseline-confirmed **GStreamer 1.28 software-only cadence issue remains open** (original tail one frame late / extra output frame); default hardware-host verification does not cover it. Diagnosis, scope and reproduction: [render-cut-caps.md](audit/render-cut-caps.md). |
| 2026-11-08 | Beat-grid CI outcome | Original run [42462752102](https://github.com/zvirox/easy-edits-pro/actions/runs/42462752102) reached the 30-minute job limit with all four `audio_edits` tests still running; licences and Python harness passed. The follow-up correction is `c817bc4`, with full local verification below. Remote logs match the stalled suite; the lock backtrace is local evidence. |
| 2026-11-08 | Native verification follow-up | The documentation gate rerun exposed a local parallel audio/GES startup deadlock; that interrupted workspace run failed. Centralized full media-runtime initialization across renderer and desktop entry points and added a bounded cold-process regression (frame/audio/hardware-first and simultaneous startup). The new test failed before the correction on missing element registration; all four modes and the parallel audio suite passed after it. Final workspace gate: **583 tests passed plus four child-process scenarios**, 1 existing ignored; desktop **45 passed**, 1 existing ignored. Frontend build and **174 tests**, glass/colour-freeze, commands/source-size, catalogue/parity drift and SPDX checks passed; all **nine mocked E2E areas** passed with zero browser errors. Document links/anchors, changed-line character sweep and whitespace checks passed. Diagnosis and reproduction: [media-runtime-initialization.md](audit/media-runtime-initialization.md). The original beat-grid CI was still in native tests during diagnosis; no remote backtrace established its cause. |
| 2026-11-08 | AF0 (documentation slice) | Added the user-supplied agent-first plan as a companion, reconciled to `7c20b98` and 465 shipped commands. Refreshed the local-only planning index, added dependencies and AF0–AF9 checklist; linked 5.2/7.1/7.2 shared work. Corrected stale status and moved misplaced match-cut evidence here. Verified static catalogue ownership, single viewport/snapshot history, sequential MCP batches, whole-branch merge and existing fixed shell; licensing allow-list/package-bundle follow-ups remain open. Product defaults/order remain pending; AF1 inventory requires human review. Verification and the discovered native initialization correction are recorded in the preceding row. No agent-first feature or licensing fix is claimed by this documentation slice. |
| 2026-11-08 | 5.1 (third task) | **Beat-grid quantization (`7c20b98`):** core validation/atomic cut rolls, bounded model-free pulse analysis, off-lock desktop/MCP preparation, menu and workflow. Native fixture moved cuts from frames 82/139 to 75/135 at 120 BPM while preserving outer edges and audible music; deterministic preparation and undo verified. Local gates: workspace 582 tests, desktop 45, frontend 174, build/glass/commands/source-size/SPDX/catalogue/parity and nine mocked E2E areas passed with zero browser errors. Catalogue: 465 shipped commands. See [beat-grid-quantization.md](beat-grid-quantization.md) and [CI run](https://github.com/zvirox/easy-edits-pro/actions/runs/42462752102) for verification. |
| 2026-11-08 | 5.1 (second task) | **Match-cut alignment:** bounded contrast-centroid, signed block-motion and attack-envelope matching; one incoming-source slip with stale/no-signal rejection, undo/redo and no-op history preservation. Desktop menu and `edit_match_cut_align` MCP tool share preparation. Native moving-video and WAV fixtures verify deterministic selection, matching over a closer transient decoy, changed rendered picture/audio, and unaligned controls. Accurate paused-preroll frame decoding and sample-grid audio starts verified. **Gates:** workspace 572 passed, 1 existing ignored; desktop 45 passed, 1 existing ignored; UI build and 172 tests passed; all nine mocked E2E areas passed with 0 browser errors, including exact match-cut menu request/error display. Initial E2E startup found a leftover server on port 1420; rerun after stopping that server passed. Menu screenshot inspected. Glass/commands/source-size, licence headers and regenerated catalogue/parity drift checks passed (464 shipped commands). Prowl unavailable. Workflow and limitations: [match-cut-alignment.md](match-cut-alignment.md). |
| 2026-11-08 | 5.1 (first task) | **J/L split edits:** `timeline.asymmetric_trim`, desktop menu and MCP preparation, one undoable application, schema v16 audio suppression, persistent editable sound bounce. Native AV evidence covers both cut directions, constant-power fades, changed picture frames, export vs bounce-only audio below -90 dBFS, audible doubled-source negative control, fractional-sample EOF handles, history and cleanup. **Gates:** workspace 562 passed, 1 existing ignored; desktop 45 passed, 1 existing ignored; UI build and 171 tests passed; glass/commands/source-size/parity, licence headers and catalogue drift checks passed. After the fractional-sample fix, the full renderer suite passed (146 passed, 1 existing ignored). All nine mocked-backend E2E areas passed (0 browser errors), including the exact menu request/error contract; the assertion was corrected to include the UI's `Error: ` prefix. Menu screenshot inspected. Catalogue regenerated to 463 shipped commands; added-line/new-file character scan and diff checks passed. Prowl attempted but unavailable. Workflow and bounds: [audio-processing.md](audio-processing.md). |
| 2026-11-08 | 4.2 | **Offline audio operations delivered; cinema stems deferred by maintainer decision.** Shared MCP/desktop `audio_process` prepares phase alignment, caller-profile room tone, energy-keyed duck automation and measured master normalization; atomic Rust commands reject stale snapshots and preserve undo/redo. Project-local float WAVs persist on success and are removed on failure. Room-tone fades are inside gaps; dialogue-edge crossfades remain pending. Lossless transient master: −14.00747 LUFS / −1.0100 dBTP internally; independent FFmpeg meter: −14.0 LUFS / −1.0 dBTP. **Gates:** `cargo test --workspace` 557 passed, 1 existing ignored; desktop tests 45 passed, 1 existing ignored; desktop locked check; UI build, 169 unit tests, glass/commands/source-size/parity and all nine mocked-backend E2E areas passed (0 browser errors). Initial E2E navigation timed out during concurrent Rust builds; rerun after completion passed with the same harness. Explicit native `dsp_plugin_parity -- --nocapture`: both tests executed and passed, including preview/export residual <−90 dBFS and audible plugin-gain control, no codec skip. Catalogue regenerated to 462 shipped commands; new-file junk scan and diff checks passed. Prowl attempted but unavailable in this environment. Contracts and limits: [audio-processing.md](audio-processing.md). |
| 2026-11-06 | 2.3 | **Neural matting, depth & motion tracking.** New `crates/saturn-perception` modules (render-free; compositing stays in saturn-render; commands call both — Packet 2.2 pattern). **Licence gate first:** RVM (plan's "RVM v2 / MODNet") **refused** — repo + exported ONNX models are GPL-3.0 (upstream-maintainer-confirmed, PaddlePaddle/FastDeploy#1059); MODNet satisfies the packet alone (Apache-2.0 for code+models+demos since 2022-01, ZHKKKe/MODNet#70). `matting_ort.rs`: pinned `Xenova/modnet` @ `fa2fa546052fba4c08921230a26cc69a333fca12` fp32 export (25,888,640 B, SHA-256 `07c308cf0fc7e6e8b2065a12ed7fc07e1de8febb7dc7839d7b7f15dd66584df9`, LFS-oid-verified 2026-11-06), fetch-on-first-run + checksum verify, preprocessing pinned to the export's preprocessor_config (shortest edge 512, /32 divisibility, bilinear, 1/255, mean/std 0.5 → [−1,1]), output name-checked (ort panics on missing keys — probed `output [1,1,H,W]` first), deterministic bilinear round-trip to frame size, EMA-labelled smoother (explicitly NOT RVM-style recurrent memory), prefix-sum `feather_alpha` with bounded output. **Portrait-only** documented in code/command/UI. `depth_ort.rs`: **Small only** (Base/Large/Giant CC-BY-NC-4.0 excluded, DepthAnything#81); pinned `onnx-community/depth-anything-v2-small` @ `4472b7362082ad9968fee890ca0f1e5aca36b93d` int8 export (27,258,801 B, SHA-256 `fcf51f1b230362b28690bb9d1809bf0431f29cad20534e3f589bd7285547f20d`; upstream `.pth` recorded for traceability); 518×518 (14-px patch grid), ImageNet mean/std, declared-name + finite + exact-dims checks; honest deviations in-code: relative depth only (normalized in-frame with `flat` flag, never meters), fixed square resize vs upstream keep_aspect_ratio, int8 drift. `motion_track.rs`: exhaustive-SSD patch match + parabolic sub-pixel + **measured ZNCC confidence** (flat texture → ~0, replacing the old fake hardcoded 0.95), Hartley-normalized DLT (gauge h[8]=1, singular → None), seeded deterministic RANSAC (all-triplet collinearity guard, consensus refit), `track_region_frames` (3×3 grid of 32 px patches, **non-incremental** matching so one bad match cannot compound, per-frame homography, `None` instead of interpolation). Fake removals: render `matting.rs` Gaussian/skin-chroma heuristic + RVM-shaped `MattingTemporalState` **deleted** (module is now composite-only over a provided measured alpha — threshold rescale → shared feather → background blend); `analyze_motion_tracking` sine/cosine rewrite replaced with `media_id`-based command returning typed `MotionTrackResult` (measured confidences, `ticksDropped`, `meanReprojectionErrorPx`), bounded decode (≤48 frames @ ≤960 px long edge), engine lock never held across decode/inference; UI `BackgroundMattingCard.tsx` synthetic 128×128 skin-tone payload + canned success removed, RVM option removed, returned frame drawn into a canvas as measured proof with coverage %. Gated e2e `tests/matting_depth_e2e.rs` (SATURN_PERCEPTION_E2E=1 / SATURN_MODELS_DIR): MODNet determinism/range/frame-size + person-less scene reports low alpha honestly + feather/smoother composition; DA-V2 deterministic finite 518×518 grids with real spread (not flat), normalization + upsample shape. **I/O + timing probe evidence (debug build, CPU): MODNet 368 ms @ 512×896, DA-V2-Small int8 675 ms @ 518² (both deterministic; release build will be faster — no release numbers claimed).** `docs/model-licences.md`: MODNet export, DA-V2-Small upstream + export rows, **RVM refused entry**. Gates: `cargo test --workspace` 0 failures (perception 43 lib incl. 6 matting + 4 depth + 8 motion tests; render composite suite 5), tauri workspace 45 green, `pnpm build` + 168 UI tests + e2e suites (0 browser errors), check:glass (tokens + freeze 134) / check:commands (449 unchanged) / check:source-size (≤800) / junk + cyrillic sweep clean (one fabricated CJK token found in the new e2e file mid-session and removed — sweep after every write stands). Phase 2 gate closed: all three criteria ticked with evidence. Commit at packet close (local — not pushed, awaiting user go-ahead). |
| 2026-11-06 | 2.2 | **Shot detection, SigLIP semantic search & cut continuity.** *(retroactive — row was missed when the packet committed as `4072d14`.)* New `shot_detect.rs` (composite hard-cut score: 0.6·symmetric χ² over smoothed 32-bin luma histograms + 0.4·mean block |Δluma|, threshold 0.32, 0.5 s min gap, dissolves/fades documented-not-detected), `embeddings.rs` + `siglip_model.rs` (pinned `Xenova/siglip-base-patch16-224` @ `4649052…`, Apache-2.0, fetch-on-first-run SHA-256-verified, 224×224 bicubic preprocessing pinned to upstream config, on-graph `pooler_output` [1,768]), `index_db.rs` (bundled rusqlite sidecar keyed (media_id, tick) with revision stamping + stale-drop), `media_search.rs` (cosine scan — honest deviation from ANN at this scale), `color_metrics.rs` (CIEDE2000 verified against 7 Sharma/Wu/Dalal reference pairs), `CutContinuityScore` with every component measured or honestly `None` (gaze proxy labelled, 180°-rule always `None`, audio pop `None` on silence); desktop `media_semantic_index` + `media_detect_shots`; MCP `media_semantic_query` + `edit_evaluate_cut_continuity` shared payload builders byte-identical headless/bridge. **Gate evidence:** cosine scan over 502 stored vectors ≪ 50 ms budget; `tests/siglip_e2e.rs` cross-modal ranking (green frame → "green grass") real-inference proof; CIEDE2000 reference suite. Licence rows: SigLIP exports + ONNX Runtime. **Phase 2 gate item "< 50 ms post-indexing" evidenced here.** |
| 2026-11-06 | 2.1 | **Real Whisper transcription + script sync.** New crate `crates/saturn-perception`: `models.rs` — 9-model ggml catalogue (tiny.en…large-v3-turbo) with upstream-exact sizes + LFS SHA-256 hard-coded at pinned revision `5359861c739e955e79d9a303bcbc70fb988958b1` of `huggingface.co/ggerganov/whisper.cpp` (canonical host; MIT; verified via HF API 2026-11-06), fetch-on-first-run downloader (plan-review §92): atomic `.part` rename, SHA-256 over the received bytes (`HashingReader`), corrupt/truncated re-download, offline machines get a typed error naming the exact file+URL; `whisper.rs` — real inference via whisper-rs 0.14 (Unlicense bindings → vendored whisper.cpp), deterministic greedy (best_of 1), token timestamps + word split, BPE word-boundary grouping (spaces/▁ start words, punctuation attaches), shared filler list applied, centisecond→tick exact math (`TICKS_PER_SECOND/100`); **diarization honestly none** (`speaker: None`, review §91). Workspace `.cargo/config.toml` sets `WHISPER_DONT_GENERATE_BINDINGS=1` (bindgen trips this glibc's opaque `_IO_FILE` → const-assert overflow; bundled bindings are deterministic and match the vendored C sources). `saturn-render/audio_pcm.rs` + `decode_audio_pcm_mono_16k` (caps-constrained refactor, verified 11.000 s from jfk.wav). `saturn-core`: script-sync types/functions in `speech.rs` (`parse_script_lines` fountain-**heuristic**, `align_script_line_to_words` order-preserving window match — score floor tightened 0.5→0.6 after testing showed half-overlap false matches on common words; monotonic per-take cursor so a later line cannot lock onto an earlier line's words; compact-span tie-break), `speech.script_sync_align` engine command (analysis-only, **no checkpoint**, core inference-free — takes passed in), catalogue spec `since: "0.3.0"`, docs regen (449 cmds). Desktop: demo-sentence block replaced (Phase 2 exit criterion) — resolve source (requested id → earliest audio-capable timeline clip → first Audio/Video asset), decode 16 kHz mono, transcribe, map media→timeline ticks through first clip (`timeline_start + (media_tick − source_in) × speed`; speed-ramp clips flagged `timingApproximated` rather than silently mislocated; engine lock taken twice, never across inference); provenance in every response (`model`, `language`, `audioSeconds`, `timingSource` whisper vs `user_text_synthetic` — text_override kept, labeled: user-authored words, synthesized timing, confidence 1.0 by definition); new orchestrator `align_script_to_takes`. Tests: 7 `speech::` unit tests + 2 engine tests (ranking, non-mutation, typed errors); end-to-end `tests/whisper_jfk.rs` (canonical jfk.wav fixture committed to `tests/assets/` — US-government public-domain recording, NOTICE + `licenses/whisper.cpp-MIT.txt` recorded) downloads tiny.en on first run and asserts real "fellow americans" transcript + ticks within 11 s (centisecond units verified). `deny.toml` + Unlicense (whisper-rs-sys); `docs/model-licences.md` rows filled. Gate evidence: `cargo test --workspace` 0 failures (saturn-core 111 lib + engine suites; 9 new files: 4 script-sync unit tests + 2 engine dispatch tests; saturn-perception 7 lib), pnpm build (405 ms) + 168 UI tests + **e2e suites passed (0 browser errors)**, check:glass (tokens + freeze) / **check:commands (449, 71 UI-only)** / check:source-size (script-sync split into `script_sync.rs` when speech.rs hit the 800-line gate) / license-headers OK (518 files); junk + cyrillic sweep clean. **End-to-end inference evidence (`SATURN_PERCEPTION_E2E=1 cargo test -p saturn-perception --test whisper_jfk`)**: fetch-on-first-run downloaded `ggml-tiny.en.bin` (SHA-256-verified) then transcribed the committed jfk.wav fixture — transcript contains "fellow americans", word ticks monotonic within the 11 s span (whisper.cpp centisecond units empirically confirmed), 11.35 s wall clock. Commit at packet close (local — not pushed, awaiting user go-ahead). |
| 2026-11-06 | 1.3 | **Audio telemetry.** New `saturn-render/audio_telemetry.rs` (~640 lines): hand-rolled in-crate iterative radix-2 Cooley–Tukey FFT (deterministic float-op order, no new dependency), Parseval-normalized single-sided spectra (bins sum to N·mean-square; band energy = 10·log10(Σ band bins / window_frames)), 10 octave bands per `BAND_CENTER_HZ` (31 Hz..16 kHz, sixth-octave band edges; the plan's `spectral_bands_db: [f32; 10]` layout is octave centers), dialogue-SNR heuristic (20 ms windows of speech-band 300–3400 Hz energy, p25/p75 percentile split, > 3 dB separation required else `None` — labelled heuristic, not ASR), mud (250 Hz) / sibilance (4.7–10 kHz) band-energy warnings vs the 1–2 kHz vocal band (≥ 6 dB), `MAX_ANALYSIS_SECONDS = 120` cap with honest `truncated_to_cap` flag. Per §4.4: **no invented aggregate mix score** (`notes.aggregate_score: null`; warnings are labelled heuristics that gate nothing). `loudness.rs`: stereo phase-correlation accumulation inside `EbuR128Meter` (Σ(L·R)/√(ΣL²·ΣR²) over all processed frames; `None` for mono/silence; `LoudnessReport` unchanged) + `KWeightingFilter` made `pub(crate)` with `steady_state_gain_at` test support. New `saturn-render/audio_pcm.rs` (~250 lines): single-shot GStreamer PCM decode (`uridecodebin → audioconvert → audioresample → appsink F32LE`), the frame extractor's proven Paused→Playing→seek-in-Playing flow (drain then FLUSH|ACCURATE seek in Playing), frame budget recomputed once caps arrive, EOS detected via **`appsink.is_eos()`** — appsink never posts EOS to the pipeline bus, so bus-polling spins forever on short files that fully decode before the first pull (root-caused with GST_DEBUG=appsink:5). `audio_read_telemetry` MCP tool (schema in `server_tools.rs`, dispatch + async impl in `server.rs` bridge→`bridge_call` / headless→`spawn_blocking`, mirror dispatch + handler in `mcp_bridge.rs`, shared payload `audio_telemetry_payload` in `preview_tools.rs` + `parse_standard`/`parse_tick_range`): media mode (`media_id`, optional tick range) and sequence mode (`start_tick`+`end_tick`, renders the sequence mix via `render_project_range_with_settings` software encode → decode → analyze, temp snippet deleted); a `track_id` param is honestly rejected with a typed error (track-scoped telemetry not implemented — §4.4 honesty over silent pretending). Integration evidence `tests/audio_telemetry.rs` (4 green; GStreamer work serialized behind a test gate): **exit criterion — media mode measures integrated −23.0 ± 0.1 LUFS end-to-end** on a synthesized 16-bit WAV whose 1 kHz amplitude is calibrated through the real K-filter's measured steady-state gain (the ITU reference level is defined by that filter, so the test exercises the meter, not filter lore); also asserts the 1 kHz band peaks, phase ≈ +1, `aggregateScore` null; sequence mode mix render/analyze within ±1.0 LU (snippet rides an audio encoder; exact ±0.1 LU covered by media mode + unit test `reference_ebu_r128_signal_measures_minus_23_lufs_plus_minus_tenth`) with snippet-cleanup assertion; param typed errors (modeless, unknown media, track_id, bogus standard). MCP-level typed-error test added (saturn-mcp 6 tests green; tool catalogue unchanged — MCP tool, not a catalogue command). Note: the existing `generate_waveform_peaks` in `apps/saturn-tauri/src/commands/media.rs` is synthetic — real waveform decode is parity debt for a later packet, not 1.3 scope. **Phase 1 verification gate closed: all four criteria now carry automated evidence.** Gates: cargo test --workspace 0 failures (saturn-render 81 lib + audio_telemetry suite; saturn-tauri bin 45; saturn-mcp 6), pnpm build (461 ms) + 168 UI + all e2e suites passed (0 browser errors), check:glass / check:commands (448, 71 UI-only) / check:source-size (max 800) / license-headers OK (513 files); junk + cyrillic sweep clean. **Phase 1 matrix re-run** (`user-tag packet-1.3`, evidence `/tmp/opencode/matrix-p13/result1.json`): ok=372, ui_only=71, error=5, undos=213 → 443/448 non-error, **byte-identical error set to the packet-0.2 baseline** (`edit.select_clip_at_playhead`, `multicam.flatten`, `timeline.ripple_delete_gap`, `trim.next`, `trim.previous` — all previously judged needs-state); both 1.2+1.3 pushed per user go-ahead. |
| 2026-11-06 | 1.2 | **Video scopes telemetry.** New `saturn-render/scopes_gpu.rs` (445 lines): `analyze_rgb_scopes` builds a `ScopeTelemetryReport` from raw RGB via one O(n) deterministic CPU histogram pass (Rec.709 luma IRE min/p10/p50/p90/max, clip ≥ 99.5 IRE / crush ≤ 0.5 IRE %, RGB parade medians, vectorscope mean+p90 saturation in HSV, mean chroma angle; stride sampling above 4 Mpixels with an honest `sampled` flag; skin-band YCbCr chroma statistics measured against the 123° flesh-tone *reference target* — never ethnicity labels or group IRE targets per §4.5; `notes.aggregate_score: null` marks the no-invented-Murch-score constraint §4.4). `color_read_scopes` MCP tool (schema in `server_tools.rs`, dispatch + async impl in `server.rs` bridge→`bridge_call` / headless→`spawn_blocking`, mirror dispatch + handler in `mcp_bridge.rs`): media mode (`media_id`+`tick`, direct decode via new `decode_frame_rgb` — raw RGB, since JPEG quantization would corrupt histograms) and sequence mode (`at_tick`, renders the *graded* frame so a grade's effect is verifiable), optional clamped `roi`. **Latent bug found & fixed:** `render_frame` (backing the uiOnly `export.frame`) deadlocked forever — the bespoke GES preview pull waits ("Waiting for preroll to send initializing seek") on a seek its own FLUSH-then-Paused flow never resumes; reimplemented on the matrix-proven render path (2-frame software snippet via `render_project_range_with_settings` → `decode_frame_rgb` → PNG, snippet cleaned up). Integration evidence `tests/color_scopes.rs` (3 green; GStreamer work serialized behind a test gate — parallel pipelines race global init into a deadlock): media-mode telemetry from `editor-smoke.webm` + sequence-mode render→analyze with temp-frame deletion asserted. Gates: cargo test --workspace 0 failures, saturn-tauri bin 45, pnpm build + 168 UI + e2e suites (0 browser errors), check:glass / check:commands (448, unchanged — tools are MCP tools per §5.1) / check:source-size (all files ≤ 800) / license-headers clean; junk + cyrillic sweep clean. |
| 2026-11-06 | 1.1 | **Frame & contact-sheet extraction.** `saturn-render/lib.rs` split under the 800-line gate: lib 629 lines (render entry points + tests) + `encoders.rs` 294 (format/profile/hardware-encoder selection + rank-override tests) + `ges_timeline.rs` 629 (document slicing, GES graph, render bus loop, eq-strings test). New `frame_extractor.rs` (732 lines): pre-warmed `uridecodebin→videoconvert→appsink(RGB)` decode pipeline per source (preroll to Paused, run to Playing, drain + `FLUSH|ACCURATE` seek per pull); `extract_still` (JPEG q80, max width clamp 64..1280), `cached_still` (BLAKE3-fingerprint-keyed lazy cache in the 0700 previews dir, atomic write, 7-day prune), `SheetRequest::well_ranged/clamped` (≤64 tiles, ≤640×640, warnings reported), `extract_frame_sheet` (canvas composite + hand-drawn bitmap timecode burn-in), 512 KiB `MAX_INLINE_JPEG_BYTES`. New `preview_tools.rs` shared payload builders used byte-identically by headless `saturn-mcp` and the Tauri bridge: `media_preview` upgraded to a real poster frame (`image_block` convention → `RawContent::image` + de-duplicated text block in MCP); `vision_extract_frame_sheet`; `render_preview_clip_proxy` (≤10 s span, 64..1920 resolution, software encode, `include_audio:false` mutes; writes only into the previews dir). saturn-mcp split: tool catalogue + 2 new tool schemas in `server_tools.rs`, tests in `server_tests.rs` (server.rs 701 lines). Preview tools are MCP tools, not catalogue commands (plan §5.1 tool table — 448 command count unchanged). Gates: `cargo test --workspace` 0 failures, saturn-tauri bin tests 45 green, `pnpm build` + 168 UI + all e2e suites passed, check:glass / check:commands (448) / check:source-size / license-headers clean, junk-sweep + cyrillic scan clean. **Gate evidence: 4×4 contact sheet (1280×720, 16 labeled tiles) from `editor-smoke.webm` in 382 ms cold** (`cargo test -p saturn-render --test frame_extractor`, 5 tests green). |
| 2026-11-05 | — | Plan adopted into repo; checklist created; no packets started |
| 2026-11-05 | 0.0 | Security remediation complete. Re-audit found most §10.1 items pre-remediated; completed the remaining unchecked-Tick sweep (`time.rs` helpers + engine/trim/markers-adjacent/effects/captions/xmeml sites), XMEML 50k clip cap, XMEML checked frame math, export-side saturating end math. 4 new robustness tests in `engine/tests/robustness.rs`. Evidence: `cargo test --workspace` clean (0 failures), `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800` pass. Tier-4 matrix deferred to Phase 0 gate. Commit `c8c20a0`. |
| 2026-11-05 | 0.1 | Engine-owned unified viewport state: `UnifiedViewportState` in `viewport_state.rs` (playhead + AI ghost field, clip/gap/media selection, workspace, branch, export queue capped at 500); `viewport_cmds.rs` dispatches 55 ids (playhead 13, workspace 8, timeline.select 4, edit.select 4, bin/marks 3, export.queue 9, viewport.inspect) returning a `viewport` snapshot; 41 catalogue specs flipped backend (`uiOnly:false`, new params), catalogue JSON/COMMANDS.md regenerated. Targeting stays single-sourced in `TargetingState` (snapshot accessor). 16 new engine tests; strict no-junk-params for `params:"{}"` viewport cmds. Gates: cargo test --workspace 0 failures, pnpm build + 168 UI tests green, check:commands (441 cmds / 71 UI-only) + source-size pass. UI convergence deferred to Packet 5.2. Commit `031a7bc`. |
| 2026-11-05 | — | **Compliance docs adopted** (user directive): provenance rules land in `AGENTS.md` + `NOTICE` + `licenses/` via `provenance-notice.patch` (FilmCraft derivation credited; parity checker header updated); `docs/audit/filmcraft-provenance-check.md` (evidence base), `docs/audit/eep-ai-native-plan-review.md` (independent review — design constraints now annotated into Packets 0.2, 1.1, 1.2, 3.1, 5.2), `docs/planning/dual-licensing-plan.md` (licensing brief; CLA-before-merge and no-bundling constraints noted). See "Compliance documents (binding for every packet)" at the top of this file. |
| 2026-11-05 | — | **Dual-licensing plan implemented** (user decisions: AGPL-3.0-or-later open licence; copyright "zvirox and the Easy Edits Pro authors"; SPDX headers applied now). `LICENSE` replaced with verbatim AGPL-3.0 text; new `LICENSING.md`, `LICENSE-COMMERCIAL.md`, `CLA.md`, `TRADEMARKS.md`, `docs/model-licences.md` (weights gate, empty), `deny.toml` (licence gate: allow-list incl. MPL-2.0 + LGPL-3.0 GStreamer dynamic-link carve-out), `scripts/license-headers.mjs` (apply/check). Support emails info@zvirox.in (primary) / zvirox@proton.me on all licence/contact files. SPDX headers stamped into 496 source files (comment-only; .rs/.ts/.tsx/.mjs/.js line comments, .py/.sh hash, .css block; shebangs preserved). `Cargo.toml` licence metadata flipped GPL→AGPL on 11 crates + `agent-harness/pyproject.toml`. CI: new `licences` job runs `cargo-deny check licenses` + header check. `CONTRIBUTING.md`: DCO updated to AGPL, CLA section added (bot pending — statement-based PR-comment flow until then), PR checklist item 5. Follow-ups (maintainer-only): CLA Assistant bot, lawyer review of licence docs, cargo-deny first CI run. Note: `LicenseRef-EEP-Commercial` is not machine-readable in `Cargo.toml` (cargo-deny can't resolve it), so metadata carries AGPL-3.0-or-later and the commercial entitlement lives in `LICENSING.md` + header comment. Gates re-run: cargo test --workspace 0 failures, pnpm build + 168 UI tests, check:contrast/styles/commands 441/71, source-size (gate now excludes the leading licence header block), py_compile, license-headers check. Junk sweep: only pre-existing 👁‍🗨 ZWJ emoji sequences found (legitimate Unicode joins, not introduced here). |
| 2026-11-06 | 0.2 | **Schema v15: BLAKE3 fingerprints & CoW timeline branches.** `branch.rs` (`TimelineBranch`, diff report types, 8 unit tests), `media_fingerprint.rs` (BLAKE3 head/tail/size fingerprint, blake3 v1.8.7 already deny-clear), schema v15 (+fingerprint +branches +relative paths), lossless v14→v15 migration with io roundtrip tests, `engine/branch_cmds.rs` dispatch for `project.branch_create/switch/diff/merge` (merge v1 = whole-branch accept/reject only). Semantics: switching away from any active branch (main included) parks the live state in a record; a reserved record named `main` is allowed by validation (engine-written only — user creation still rejected by `validate_name`); `branch_switch` to `main` with no parked record = no-op restore. `media.relink_suggest` suggests only (fingerprint constraint §4.11): confirmation happens through the existing `project.relink_media`. Catalogue fixes: `edit.select_all_matching`/`select_label_group` `mutating: true → false` (selection is viewport state per §4.9); 6 new specs with `since: "0.3.0"`. Matrix driver hardened (branch/label/queue/select hooks + queue-id capture). Gates: `cargo test --workspace` 0 failures (110 lib tests incl. new savability regression `switching_branches_keeps_the_document_savable`), pnpm build + 168 UI tests + all e2e suites green, check:glass/check:commands (448 cmds, 71 UI-only)/check:source-size/license-headers OK, parity check up to date. Matrix run 5 (`user-tag packet-0.2`, `/tmp/opencode/matrix-p02/result5.json`): ok=372, ui_only=71, error=5, undos=213, drift=1 → **443/448 non-error, Phase 0 exit criterion (≥425) met**. The 5 errors are judged needs-state; `edit.select_clip_at_playhead` proven passing in isolation (diag run) — its full-run failure shares the documented undo-drift root cause (`speech.ripple_delete_silence`), tracked as an engine follow-up. Junk sweep over all touched files clean. Commit `b576be7` (local — not pushed, awaiting user go-ahead). |
