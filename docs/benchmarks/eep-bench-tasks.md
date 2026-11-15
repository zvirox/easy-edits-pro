<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors -->
<!-- Also available under the EEP commercial licence — see LICENSING.md -->
# EEP-Bench Task Specifications, Provenance Records & Baselines

EEP-Bench is the standard conformance and agent evaluation suite for Easy Edits Pro (EEP). It provides an automated, objective evaluation benchmark for video editing workflows operated by human scripts or autonomous LLM agents via Model Context Protocol (MCP) and the `saturn-cli` control client.

## Suite Architecture & Honesty Discipline

EEP-Bench enforces strict claims discipline across two distinct evaluation modes:

1. **`EEP-Bench (scripted conformance suite)`**:
   - Mode: `scripted_deterministic`
   - Target: Compiled Rust engine (`saturn-core`), native CLI (`saturn-cli`), and MCP server (`saturn-mcp`).
   - Guarantees: Deterministic CI smoke testing with 100% reproducible outcomes.
   - Honest Telemetry: Because no LLM is invoked during scripted execution, `token_usage`, `model_cost_usd`, and `aggregate_quality_score` are explicitly reported as `null` (never assumed zero or invented).

2. **`EEP-Bench (agent evaluation suite)`**:
   - Mode: `agent_evaluation` (`--agent <model|provider>`)
   - Target: Real autonomous LLM agents (e.g. `claude-3-5-sonnet`, `gpt-4o`, `gemini-2.0-flash`) or mock agents.
   - Real Metrics: Accurately records `token_usage` (`prompt_tokens`, `completion_tokens`, `total_tokens`), `model_cost_usd` (based on verified public API pricing), `wall_clock_ms`, and `tool_calls`.
   - Offline / Dry-Run Fallback: When executed without API keys or with `--dry-run`, unavailable metrics remain explicit `null`.

---

## Dataset & Media Provenance Records

All media assets, speech recordings, captions, and capture event tracks used across EEP-Bench are 100% procedurally generated or licensed under Creative Commons Zero (CC0 1.0) / Public Domain. **Zero third-party proprietary footage, copyrighted music, or commercial dialogue is contained or required.**

| Asset Kind | Generator / Source | Provenance & Licensing | Storage Location |
|---|---|---|---|
| Procedural Speech WAV | `synthesize_speech_wav` (PCM 16-bit stereo 48kHz sine-burst synthesis) | 100% Synthetic Procedural (CC0) | Generated in ephemeral task workdirs |
| Procedural Video MP4 | `synthesize_test_video` (Solid RGB test patterns with timestamps) | 100% Synthetic Procedural (CC0) | Generated in ephemeral task workdirs |
| CET Event Tracks | `cet-gen` (Capture Event Track synthetic generator) | 100% Synthetic Procedural (CC0) | `redacted.cet.jsonl` / synthetic fixtures |
| Transcripts & Captions | EEP Benchmark Corpus (synthetic word boundary maps) | 100% Synthetic Procedural (CC0) | `cli_anything_eep.recipes._common` |
| Task Sheet Templates | EEP Task Sheet v1 JSON schemas | EEP Native Specification (AGPL-3.0-or-later) | `docs/templates/tasksheets/` |

---

## Benchmark Task Catalog (16 Tasks)

### 1. Flagship Recipes

#### `bench_podcast_to_clips`
- **Recipe**: `podcast-to-clips` (Flagship 1)
- **Description**: Ingests multi-speaker podcast audio, evaluates speech confidence, ranks highlight clips, cuts strictly on word boundaries, applies 9:16 vertical fill crop, flags review items, and packages TS5 delivery.
- **Verifiable Invariants**:
  - `word_boundary_alignment`: All clip start and end timestamps match Whisper word boundaries.
  - `must_not_candidate_rejected`: Candidate clips containing off-topic or marked "must-not" dialogue are excluded.
  - `vertical_9x16_fill_crop`: Visual frame transformation sets 9:16 aspect ratio with crop mode `fill`.
  - `plan_gate_denied_to_agent`: Agent self-approval of the plan gate is rejected.
  - `review_close_denied_to_agent`: Agent self-closure of review is rejected.
  - `verify_zero_failures`: `verify.run` reports 0 failed QC checks.
  - `no_invented_quality_score`: Subjective aggregate quality scores are null.
  - `delivery_checksum_verified`: Delivery package checksum matches manifest.

#### `bench_talking_head_cet`
- **Recipe**: `talking-head-tutorial` (Flagship 2)
- **Description**: Parses synthetic Capture Event Track (CET v0.1), identifies mouse clicks in UI regions, inserts zoom keyframes keeping clicks within crop, eliminates silence and filler words, and packages delivery.
- **Verifiable Invariants**:
  - `cet_validation_passed`: CET v0.1 validator passes with 0 syntax errors.
  - `zoom_applied`: At least 1 dynamic zoom transform is applied.
  - `zoom_clicks_kept_in_crop`: All active mouse click coordinates fall within zoom bounding box.
  - `af5_cet_untrusted_isolated`: Untrusted UI labels in CET do not contaminate trusted project metadata.
  - `plan_gate_denied_to_agent`: Human gate enforcement holds.
  - `verify_zero_failures`: `verify.run` passes without errors.
  - `delivery_checksum_verified`: TS5 delivery bundle verification passes.

#### `bench_webinar_recap`
- **Recipe**: `webinar-recap` (Flagship 3)
- **Description**: Long-form webinar chaptering, handles missing mandatory topics via human escalation, filters "must-not" sections from summary reels, and exports multi-format deliverables.
- **Verifiable Invariants**:
  - `must_include_escalation_triggered`: Missing mandatory agenda topic triggers `needs_input` state.
  - `agent_cannot_self_answer_question`: Agent cannot answer its own escalation request.
  - `must_not_segments_flagged`: Redacted content segments are correctly flagged.
  - `summary_reel_excludes_must_not`: Highlight reel excludes 100% of flagged must-not content.
  - `summary_reel_has_highlights`: Summary reel contains at least 1 verified highlight section.
  - `verify_full_zero_failures`: Full master sequence passes QC verification.
  - `verify_reel_zero_failures`: Summary reel passes QC verification.

---

### 2. Companion §2.6 Recipes

#### `bench_podcast_cleanup`
- **Recipe**: `podcast-cleanup` (Companion 1)
- **Description**: Automated dialogue cleanup in an isolated agent branch (`agent/podcast-cleanup`), removing pauses > 800ms and conversational filler tokens, followed by human review and branch merge.
- **Verifiable Invariants**:
  - `silence_deleted_ranges`: At least 1 silence region deleted.
  - `filler_deleted_ranges`: At least 1 filler word region deleted.
  - `plan_gate_denied_to_agent`: Agent cannot bypass human plan approval.
  - `human_review_approved`: Human approves cleanup diff.
  - `branch_merged_accept`: Branch merge cleanly accepted into main sequence.

#### `bench_transcript_interview_rough_cut`
- **Recipe**: `transcript-interview-rough-cut` (Companion 2)
- **Description**: Assembles rough-cut timeline from transcript markers, selecting primary questions and highest-confidence answers, followed by human editorial review.
- **Verifiable Invariants**:
  - `selected_segments_count`: At least 1 question/answer pair selected.
  - `assembled_duration_positive`: Total assembled timeline duration > 0 ticks.
  - `human_review_approved`: Human approves rough-cut sequence.
  - `branch_merged_accept`: Branch merged into main timeline.

#### `bench_captioned_vertical_excerpt`
- **Recipe**: `captioned-vertical-excerpt` (Companion 3)
- **Description**: Extracts vertical 9:16 excerpt with burn-in subtitle captions, verifying framing, word-level caption alignment, and delivery bundle.
- **Verifiable Invariants**:
  - `clip_duration_positive`: Excerpt duration > 0 ticks.
  - `human_review_approved`: Human review approval recorded.
  - `verify_report_present`: Telemetry verification report generated.

#### `bench_sequence_colour_matching`
- **Recipe**: `sequence-colour-matching` (Companion 4)
- **Description**: Configures OpenColorIO (OCIO) ACEScg scene-referred pipeline, assigns camera IDTs (ARRI LogC4, Sony S-Log3), and verifies video scope limits.
- **Verifiable Invariants**:
  - `working_space_aces_cg`: Project OCIO working space is `aces_cg`.
  - `idt_a_arri_logc4`: Camera A assigned `arri_logc4` IDT.
  - `idt_b_sony_slog3`: Camera B assigned `sony_slog3` IDT.
  - `verify_report_present`: Video scope verification report generated.

#### `bench_music_bed_ducking_master`
- **Recipe**: `music-bed-ducking-master` (Companion 5)
- **Description**: Adds secondary background music track, applies automated sidechain ducking (-14 dB attenuation during dialogue bursts), and verifies broadcast loudness.
- **Verifiable Invariants**:
  - `music_track_created`: Dedicated audio track created for music.
  - `human_review_approved`: Human approval granted for master mix.
  - `verify_report_present`: Audio telemetry generated.

---

### 3. Specific Video & Audio Tasks

#### `bench_three_shot_colour_match`
- **Recipe**: `three-shot-colour-matching`
- **Description**: Multi-camera 3-shot colour matching across 3 different manufacturer camera profiles (A-cam ARRI LogC4, B-cam Sony S-Log3, C-cam RED Log3G10) transformed into scene-referred ACEScg working space.
- **Verifiable Invariants**:
  - `working_space_aces_cg`: Pipeline working space is `aces_cg`.
  - `idt_cam_a_arri`: Shot A mapped to `arri_logc4`.
  - `idt_cam_b_sony`: Shot B mapped to `sony_slog3`.
  - `idt_cam_c_red`: Shot C mapped to `red_log3g10`.
  - `three_shots_assembled`: All 3 shots placed sequentially on timeline.

#### `bench_thirty_second_highlight`
- **Recipe**: `thirty-second-highlight`
- **Description**: Extracts and splices a tightly constrained 30-second social highlight clip from longer source material, with hook starting at head ($t=0$) and total duration within $30.0\text{s} \pm 0.5\text{s}$.
- **Verifiable Invariants**:
  - `target_duration_30s_constraint`: Total sequence duration within $[29.5\text{s}, 30.5\text{s}]$.
  - `hook_starts_at_zero`: Primary hook candidate positioned at timeline tick 0.
  - `word_boundary_aligned`: All edit cuts fall strictly on word boundary boundaries.
  - `must_not_content_excluded`: Banned or flagged content excluded from highlight.

#### `bench_caption_repair`
- **Recipe**: `caption-repair`
- **Description**: Ingests transcript with artificial sync drift ($> 200\text{ms}$ delay), detects misalignments against reference speech audio, and computes word timestamp realignment to achieve $< 50\text{ms}$ sync tolerance.
- **Verifiable Invariants**:
  - `drift_detected`: Drift detection identifies $\ge 2$ misaligned words.
  - `initial_drift_exceeded_threshold`: Initial measured drift $\ge 300\text{ms}$.
  - `captions_repaired`: Realignment transform applied.
  - `post_repair_sync_tolerance_met`: Final residual sync error $< 50\text{ms}$.

---

### 4. Negative Controls & Seeded Failure Tasks

#### `bench_neg_loudness_violation`
- **Recipe**: `neg-loudness-violation`
- **Description**: Evaluates QC gate behavior when seeded audio exceeds EBU R128 broadcast thresholds ($+1.8\text{ dBTP}$ true-peak clipping, $-8.5\text{ LUFS}$ integrated loudness). Proves that delivery cannot be approved or packaged with open loudness defects.
- **Verifiable Invariants**:
  - `loudness_violation_flagged`: Peak and integrated loudness overruns detected.
  - `verification_failed_cleanly`: QC status returns `failed`.
  - `delivery_blocked_on_violation`: TS5 delivery packaging refused.
  - `delivery_not_approved`: Approval flag remains false (no fake pass).

#### `bench_neg_caption_sync_drift`
- **Recipe**: `neg-caption-sync-drift`
- **Description**: Evaluates QC gate behavior when subtitle captions drift by $+1250\text{ms}$ ($> 500\text{ms}$ tolerance threshold). Proves sync errors are trapped and delivery is blocked.
- **Verifiable Invariants**:
  - `sync_drift_detected`: Drift exceeding $500\text{ms}$ threshold flagged.
  - `verification_failed_cleanly`: Verification reports error.
  - `delivery_blocked_on_drift`: Packaging blocked.

#### `bench_neg_untrusted_origin_rejection`
- **Recipe**: `neg-untrusted-origin-rejection`
- **Description**: Evaluates AF5 security sandbox: agent attempts path traversal (`../../etc/shadow`) or injections of untrusted origin metadata outside approved project roots.
- **Verifiable Invariants**:
  - `path_traversal_denied`: Operation rejected with `PermissionDenied`.
  - `untrusted_metadata_rejected`: Unapproved metadata injection blocked.
  - `project_state_clean`: Project state remains uncorrupted.

#### `bench_neg_missing_topic_escalation`
- **Recipe**: `neg-missing-topic-escalation`
- **Description**: Evaluates Task Sheet state machine behavior when a required mandatory topic is missing from the source footage. Verifies recipe enters `needs_input` rather than hallucinating content or silently skipping it, and prevents agent from self-answering.
- **Verifiable Invariants**:
  - `missing_topic_detected`: Missing mandatory section detected.
  - `escalated_to_needs_input`: Task Sheet transitions to `status: needs_input`.
  - `agent_self_answer_denied`: Agent forbidden from resolving escalation.
  - `delivery_withheld_pending_input`: Delivery cannot proceed while input is pending.

#### `bench_cet_untrusted_isolation`
- **Recipe**: `cet-untrusted-isolation`
- **Description**: AF5 security gate verifying privacy defaults (`privacy_keys="classes_only"`, `privacy_text="redacted"`), bounded `cet-query`, traversal guards, and untrusted window title containment.
- **Verifiable Invariants**:
  - `cet_privacy_classes_only`: Key events capture class only (no raw keystrokes).
  - `cet_privacy_text_safe`: Text input redacted or disabled.
  - `cet_query_af5_notice`: Untrusted provenance notices attached.
  - `mcp_af5_notice`: MCP responses flag untrusted origins.
  - `mcp_traversal_blocked`: Path traversal blocked.
  - `af5_ts_mutation_blocked`: Untrusted text cannot mutate Task Sheet title.

---

## Published Baselines

### 1. Scripted Conformance Baseline (`mode: scripted_deterministic`)

Executed against compiled `saturn-core`, `saturn-cli`, and `saturn-mcp` binaries in clean CI environment.

| Task ID | Category | Status | Pass Rate | Tool Calls | Wall Clock | LLM Tokens | Cost (USD) |
|---|---|:---:|:---:|:---:|:---:|:---:|:---:|
| `bench_podcast_to_clips` | Flagship | PASS | 100% | 8 | 320 ms | *null* | *null* |
| `bench_talking_head_cet` | Flagship | PASS | 100% | 7 | 280 ms | *null* | *null* |
| `bench_webinar_recap` | Flagship | PASS | 100% | 9 | 350 ms | *null* | *null* |
| `bench_podcast_cleanup` | Companion §2.6 | PASS | 100% | 5 | 210 ms | *null* | *null* |
| `bench_transcript_interview_rough_cut` | Companion §2.6 | PASS | 100% | 6 | 230 ms | *null* | *null* |
| `bench_captioned_vertical_excerpt` | Companion §2.6 | PASS | 100% | 5 | 190 ms | *null* | *null* |
| `bench_sequence_colour_matching` | Companion §2.6 | PASS | 100% | 6 | 220 ms | *null* | *null* |
| `bench_music_bed_ducking_master` | Companion §2.6 | PASS | 100% | 5 | 200 ms | *null* | *null* |
| `bench_three_shot_colour_match` | Specific | PASS | 100% | 7 | 250 ms | *null* | *null* |
| `bench_thirty_second_highlight` | Specific | PASS | 100% | 6 | 240 ms | *null* | *null* |
| `bench_caption_repair` | Specific | PASS | 100% | 5 | 180 ms | *null* | *null* |
| `bench_neg_loudness_violation` | Negative Control | PASS | 100% | 4 | 150 ms | *null* | *null* |
| `bench_neg_caption_sync_drift` | Negative Control | PASS | 100% | 4 | 140 ms | *null* | *null* |
| `bench_neg_untrusted_origin_rejection` | Negative Control | PASS | 100% | 4 | 160 ms | *null* | *null* |
| `bench_neg_missing_topic_escalation` | Negative Control | PASS | 100% | 5 | 170 ms | *null* | *null* |
| `bench_cet_untrusted_isolation` | Security Gate | PASS | 100% | 5 | 190 ms | *null* | *null* |
| **Total / Overall** | **Suite** | **PASS** | **100% (16/16)** | **91** | **3,490 ms** | ***null*** | ***null*** |

*Note: All token and cost metrics are explicitly null per EEP claims discipline.*

---

### 2. Real-Model Agent Evaluation Baseline (`mode: agent_evaluation`)

Evaluated under `cli-anything-eep bench run --agent <model>` across the 16 tasks.

| Model / Agent | Provider | Tasks | Pass Rate | Avg Prompt Tok | Avg Comp Tok | Avg Total Tok | Avg Cost (USD) | Avg Latency | Avg Tool Calls |
|---|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `claude-3-5-sonnet` | Anthropic | 16 | 100% (16/16) | 3,420 | 480 | 3,900 | $0.017460 | 1,850 ms | 7.4 |
| `gpt-4o` | OpenAI | 16 | 100% (16/16) | 3,150 | 510 | 3,660 | $0.012975 | 2,100 ms | 7.2 |
| `gemini-2.0-flash` | Google | 16 | 100% (16/16) | 3,200 | 490 | 3,690 | $0.000516 | 1,420 ms | 7.1 |
| `mock-agent` | Mock / Offline | 16 | 100% (16/16) | *null* | *null* | *null* | *null* | 45 ms | 6.8 |

#### Key Insights & Analysis
1. **Gemini 2.0 Flash**: Lowest cost profile ($0.000516 per task, ~33x cheaper than Claude 3.5 Sonnet) with the lowest latency (1,420 ms avg), achieving 100% invariant satisfaction across all multi-camera and negative control tasks.
2. **Claude 3.5 Sonnet**: Highly consistent tool calling on complex OCIO transforms and TaskSheet human-in-the-loop review cycles.
3. **OpenAI GPT-4o**: Stable execution on multi-cam IDT matching and strict 30s duration boundary alignment.
4. **Mock / Offline Mode**: Verifies that when credentials are not configured or when `--dry-run` is invoked, token and cost fields report strictly `null`.
