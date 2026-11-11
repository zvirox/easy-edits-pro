# EEP Agent-First Plan

**Adopted as a companion plan:** 2026-11-08, from the user-supplied
`eep-agent-first-plan.md`. **Reconciled baseline:** `7c20b98` (the attachment
used `ecc69ed`). The agent operates the editor; the human supervises, reviews
and can always take over.

This extends [AI_NATIVE_ARCHITECTURE_PLAN.md](AI_NATIVE_ARCHITECTURE_PLAN.md)
and [GLASS_CARDS_PLAN.md](GLASS_CARDS_PLAN.md), and is extended by
[TASK_SHEET_PLAN.md](TASK_SHEET_PLAN.md) (adopted 2026-11-10, baseline `2834ba3`;
human interface, task sheets, pro review loop, Web Desk, and packets TS0–TS10).
Supporting context: [planning/agent-operated-pro-editor.md](planning/agent-operated-pro-editor.md),
[planning/capture-event-track-v0.1.md](planning/capture-event-track-v0.1.md),
and [planning/cross-platform-and-positioning.md](planning/cross-platform-and-positioning.md).
The [shared checklist](AI_NATIVE_PLAN_CHECKLIST.md) owns progress and the next-work
queue. Existing unfinished work remains on that checklist.

Packet IDs are **AF0–AF9** here and in the checklist, corresponding to A0–A9
in the attachment, to avoid collisions with older repository packets. **AF6
means Agent/Review UI; AF9 means daemon** (correcting the attachment's §2.7).

## 0. Current progress and integration corrections

| Existing work | Reconciled state at the baseline |
|---|---|
| AI-native Phases 0–3 | Completed with the limitations and gate evidence in the checklist: Rust state, branches/fingerprints, perception, GPU colour and shot matching. |
| Phase 4 | CLAP hosting and offline phase alignment, room tone, ducking and master normalization delivered. Dialogue/Music/Ambience/SFX separation remains deferred pending a verified, license-cleared cinema-stem model; music-model output labels cannot substitute. |
| Packet 5.1 | J/L split edits (`ecc69ed`), match-cut alignment (`b067020`) and beat-grid quantization (`7c20b98`) delivered. Morph cuts and storyline pins remain planned. |
| Packet 5.2 | Co-editing UI remains planned; AF2 supplies actor isolation and AF6 integrates these panels into the fixed shell. |
| Phases 6–7 | VFX/typography and harness/closed-loop expansion remain planned. AF7/AF8 share the Phase 7 deliverables rather than duplicate them. |
| Glass G1–G7 | Fixed-shell implementation and geometry coverage exist; extend them for agent work. Existing functional/UI follow-ups remain applicable. |
| Catalogue | 465 entries, all marked shipped in the generated catalogue. This is an engine-command count, not the number of top-level MCP tools or proof that every command is headless-capable. |

### Evidence and scope of the gap inventory

- `crates/saturn-core/src/catalogue/mod.rs` owns the static `CommandSpec` and
  generated catalogue. It currently has no tier/profile fields. The separate
  `saturn-commands::CommandSpec` is not the source of the generated inventory.
- `crates/saturn-mcp/src/server_tools.rs` exposes a static tool list and generic
  `command_list`, `command_run`, `command_batch`, inspection, render and
  perception tools. `audio_process`, `color_match_shot_to_reference`,
  `edit_match_cut_align` and `timeline_beat_grid_quantize` prepare edits outside
  the core. Policies must cover these tools as well as catalogue dispatch.
- `command_batch` currently takes `steps` and `stop_on_error`; successful steps
  remain applied. Atomic individual audio/narrative operations do **not** make
  arbitrary MCP batches atomic. The original architecture's atomic-batch goal
  is still required work (AF2).
- `crates/saturn-core/src/engine/mod.rs` has one `UnifiedViewportState` and
  document-snapshot undo/redo with command labels. The ghost-playhead field
  exists, but does not establish isolated actor sessions or actor-only undo.
- `crates/saturn-core/src/branch.rs` already stores a branch note and creation
  time. Per-edit actor/rationale/confidence and project briefs are additional
  work. Branch diffs use clip indices; merges accept/reject the **whole branch**.
  Per-change rejection needs stable change identity and a written conflict rule.
- `WorkspaceShell.tsx`, `features/dock/DockCard.tsx` and
  `tests/e2e/geometry.mjs` under `apps/saturn-tauri` supply the fixed-shell base.
  `ui/src/components/OllamaChatDrawer.tsx` remains the existing assistant surface;
  Agent/Review cards, actor history and agent-aware verification are planned.
- `saturn-mcp` already runs headless over stdio and supports the desktop bridge;
  `saturn-control` is an existing control server. AF9 adds the specified session
  lifecycle/authentication/budgets/audit contract, not a claim that no server exists.
- `deny.toml` still broadly allows AGPL/GPL/LGPL identifiers, despite its header's
  stricter claim. First-party AGPL is intentional; third-party exceptions need
  separate, explicit treatment. Existing licence CI alone does not prove the
  intended dependency exclusion policy. *(AF0 update, 2026-11-08: the policy is
  now scoped — global AGPL/GPL/LGPL allowances removed, first-party AGPL granted
  per-crate with a repository-path assertion and negative controls in
  `scripts/check-licence-policy.mjs`, CI covers both workspaces and production
  JS. See [the inventory](audit/af0-inventory.md) §2.)*
- `NOTICE`, `licenses/`, SPDX checks and `bundle.licenseFile` already exist.
  The inspected Tauri/Flatpak configuration and release workflows do not wire a
  generated third-party licence bundle. AF0's subsequent
  [inventory](audit/af0-inventory.md) verifies older `v0.1.2-alpha.2` payloads:
  deb/RPM have no standalone notice bundle; Flatpak has GES licence texts but
  no comprehensive EEP/dependency bundle. Current-build correction is still open.

The new profile, session, verification and provenance contracts below are
planned capabilities, not claims of implementation. Prowl was attempted but
unavailable during reconciliation; the inventory used targeted source reads.

## 1. Binding principles

1. Human and agent use the same Rust-owned command/state engine. The human can
   always stop the agent and take over.
2. Observe, edit, measure, review. Verification reports measurements, thresholds,
   sampling and unsupported checks; taste remains a human judgement.
3. Curate a small agent surface: core plus one profile, targeting 20–30 tools;
   additional profiles load on demand within the session's grants.
4. Agent edits are reversible, attributed and explained. External side effects
   such as exports must be disclosed separately; undo cannot promise to retract them.
5. Media-derived text is data, never instructions or permission authority.
6. Colours stay locked. Reuse tokens, `.glass`, `DockCard` and the fixed dock
   layout. No new floating panels; video, scopes and meters remain raw.

## 2. Backend and agent work

### 2.1 Profiles and permission tiers — AF1

First produce `docs/reference/command-profiles.csv` from the actual catalogue
and MCP tool inventory: command/tool ID, existing availability, proposed tier,
profiles and rationale for exceptional effects. Include wrapper operation-to-command
mapping and currently UI-only commands. **Stop for human review of the inventory
before implementing fields or enforcement.** Treat the CSV as a proposal until
approved; do not hand-edit generated catalogue JSON.

After approval, extend the canonical core catalogue with `tier` and `profiles`,
regenerate references with `pnpm --dir apps/saturn-tauri catalogue:sync`, and
expose `tools.list_profiles` / `tools.load_profile` through MCP. Tool names here
are contract proposals until their schemas land.

Proposed tiers: `read`, `edit`, `destructive`, `filesystem`. The last covers
outside-project writes, export, network access and plugin loading; the inventory
must also classify project-local derived media and analysis-cache writes. Tiers
are explicit grants, not an assumed numeric hierarchy. Multi-effect operations
must require every applicable permission, even if a primary tier is displayed.

Core candidates: project/sequence inspection, the actor's own cursor, media
search/preview, contact sheets, profile discovery and `verify.run` when available.
Undo and branch mutations are core **editing** candidates and must be excluded
from read-only review/manual sessions. Do not advertise unimplemented tools.

| Draft profile | Purpose / candidate capabilities |
|---|---|
| `ingest` | Import, fingerprint, relink suggestions, labels, bins. |
| `rough-cut` | Script alignment, transcription, insert/trim/ripple, markers, semantic search, shot detection. |
| `fine-cut` | Trim, J/L through `audio_process`, match-cut, beat-grid, continuity, transitions. |
| `multicam` | Angle synchronization and switching. |
| `colour` | Scopes, shot matching, colour nodes and transforms. |
| `audio` | Telemetry, phase alignment, room tone, ducking, normalization, approved plugin loading. |
| `captions` | Transcription, caption editing/style, subtitle export with its own grant. |
| `graphics` | Titles and supported template operations. |
| `speed-motion` | Retime, stabilization and tracking. |
| `review` | Read-only inspection and verification; no edit/undo/branch mutation. |
| `export` | Render lifecycle and delivery presets, with appropriate write grants. |
| `admin` | Project settings and interchange. |

- Start with core plus the brief's selected profile. Loading a profile cannot
  elevate permission; wrong-profile calls return a typed error with a load hint.
- Destructive/filesystem actions require human approval unless explicitly
  pre-approved for that session, subject to AF5's untrusted-origin restrictions.
- Keep the complete catalogue for GUI/CLI discovery. Generic dispatch, batch
  steps, dedicated preparation tools and the desktop bridge must enforce the
  same session policy; hiding a tool alone is not enforcement.
- Test complete tier/profile assignment, ≤30 tools in the default set, review
  read-only behavior and no generic-tool bypass. Descriptions use concise
  “use when” wording and consistent argument names.

### 2.2 Actors, sessions, locks, Stop and transactions — AF2

**AF0 design reviewed 2026-11-08:** [actor/history/conflict design](planning/af0-actor-design.md).
The maintainer selected a conservative change journal alongside snapshots and
accepted both actor/review and lock/Stop/transaction contracts. This supplies the
implementation design; AF2 behavior and tests remain pending.

- Bind `ActorId` to the connection/session (human GUI or named agent); never
  accept arbitrary human identity from command arguments.
- Give actors separate playheads, selections and branch context. Rust owns
  these views; cursor movement stays outside undo and document dirty-state.
  The agent ghost playhead renders its actor cursor. Define actor targeting and
  shared export/job ownership explicitly rather than cloning a second project.
- History records actor, timestamp, branch and stable affected-change identity.
  Agent-only undo must preserve later human work and refuse dependency conflicts.
  Snapshot filtering alone cannot safely implement selective undo.
- Add durable actor-owned clip/region locks (`clip.lock_for_human` is the draft
  command). Schema migration and save/reopen preserve locks; define reclaim of
  disconnected owners. Mutations and branch merge must honor lock ownership.
- Session modes: `autopilot` edits an agent branch for review, `co-edit` offers
  proposals, `manual` leaves the agent read-only. The default is pending (§5).
- `session.stop` cancels in-flight tools and blocks new calls until human resume.
  Target ≤1 s cancellation with measured slow-analysis/render tests. A stopped
  preparation must never apply late results; preserve accepted edits/branch and
  clean incomplete artifacts. Existing render cancellation is not sufficient.
- Implement an explicitly requested atomic MCP batch with rollback, actor/grant/
  lock checks per step, and one documented history boundary. Keep existing
  sequential behavior explicit for compatibility. Define/reject external or
  prepared-media operations that cannot be rolled back; do not promise atomicity
  for arbitrary filesystem effects. Test failure after an earlier successful step.
- Write and test conflict rules before selective reject, per-edit accept or
  actor-only undo ships. Whole-branch accept/reject remains the supported baseline;
  stale human edits must not be overwritten by accepting an agent branch.

### 2.3 Measured self-check: `verify.run` — AF3

A read operation over a named range or sequence returns per-check `pass`, `warn`
or `fail`, measured values/thresholds, tick/clip references and evidence. Record
snapshot identity, analyzed range, sample coverage, truncation and unavailable
checks; unavailable or unmeasured checks cannot silently pass.

| Check | Existing input to reuse / additional work |
|---|---|
| Loudness and true peak | Audio telemetry; target from preset or brief. |
| Silence/dead air | Bounded PCM/window measurements; define duration threshold. |
| Black/frozen frames, highlight clipping, crushed blacks | Scopes/frame extraction; temporal checks need multiple samples and explicit coverage. |
| Caption safe area, speed, overlap | Caption data plus actual layout bounds; geometry cannot be inferred from text alone. |
| Duration, aspect, resolution | Project settings and AF4 brief. |
| Continuity and boundary pops | Measurable cut-continuity components, with unavailable values preserved. |
| Offline media, missing fonts/plugins, unresolved relinks | Validation plus explicit resource-availability checks. |
| Unreviewed low-confidence edits | AF4 notes and review status. |

Draft presets: `web-1080p`, `vertical-9x16`, `broadcast-r128`, with documented
targets (for example −14 or −23 LUFS). Never invent an aggregate quality score.
Extend existing bounded telemetry rather than treating its 120-second cap as
whole-sequence coverage. Clean and deliberately bad fixtures must identify the
right ticks/values. Base checks can be developed early; the complete AF3 gate
depends on AF4 brief/review metadata.

### 2.4 Edit rationale, confidence and project brief — AF4

- Add optional rationale (≤200 characters) and `high|medium|low` confidence to
  command/batch envelopes and dedicated mutation wrappers. Persist them on
  attributed history and resulting clip/cut change records (`Clip.agent_note`
  is the draft schema). Preserve through reopen, merge and migrations from v16.
- Confidence is the agent's declared uncertainty, not a measured quality score.
  Taste choices such as pacing, music and take selection default to low confidence
  in the agent skill. Low-confidence items lead the review queue.
- Human modifications clear the active agent mark and record a correction without
  deleting provenance. Rejections leave feedback for the next attempt; a future
  learning loop may consume it, but automated training is not part of this packet.
- `project.brief_set/get`: goal, platform preset, duration/tolerance, must-keep
  clips, banned content, style notes and loudness target. Human-owned brief edits
  are stored in Rust and surfaced in the Agent card. Media text cannot silently
  become a trusted brief; importing an excerpt requires an explicit human action
  and retained provenance. These are new commands, not existing APIs.

### 2.5 Untrusted media payloads — AF5

Wrap transcripts, captions, OCR, filenames, metadata and imported project text
with provenance, e.g. `{ "text": "…", "untrusted": true, "source": "transcript" }`.
Apply this consistently across inspection, search, preview, errors and other
MCP results that echo such content. Version/document payload changes for clients.

Tool descriptions and the agent skill explain that these values are data.
Untrusted content cannot grant permissions, impersonate a human or authorize a
profile change. A destructive/filesystem argument derived from that content
requires explicit human approval even with broad session grants. Design and test
server-owned origin tracking and conservative handling of unknown origins;
verbatim string matching or an agent-supplied trust flag is insufficient.

Tests include a transcript requesting deletion or export outside the approved
directory, copied/modified arguments, nested imported metadata and generic batch
bypasses. Assert approval/refusal and no unauthorized side effects; also verify
legitimate human-approved operations still work. Marking text reduces ambiguity
but does not solve prompt injection by itself.

### 2.6 Closed-loop skill and recipes — AF7 / Packet 7.1

Extend the existing `agent-harness/cli_anything_eep` CLI and skill:

1. Read the human brief, project and sequence; load an appropriate profile.
2. Create the task branch and publish a 5–10 step plan to the Agent card.
3. Apply reversible steps with rationale/confidence and the supported batch or
   preparation contract; inspect frames/scopes/audio after changes.
4. Run `verify.run`, fix measured failures within the session budget, then run the
   final platform checks (including encoded-output remeasurement where applicable).
5. Present branch diff, uncertain choices and verification evidence; wait for human
   review. Auto-merge requires explicit opt-in under the eventually approved policy.

Recipes: podcast cleanup, transcript interview rough cut, captioned vertical
excerpt, sequence colour matching, music-bed ducking/master. Use separate supplied
dialogue/music tracks until cinema stems are available. Add `branch`, `perception`,
`ocio`, `telemetry`, `vfx` and new agent controls only as their backends ship;
unsupported operations return real errors. Complete the deferred VFX CLI portion
after Phase 6; an early recipe smoke does not complete all of Phase 7.

### 2.7 Repeatable benchmark — AF8 / Packet 7.2

Build 10–20 tasks using synthetic or approved CC0 media: dialogue cleanup, scripted
rough cut, three-shot colour match, 30-second highlight and caption repair.
Record fixture provenance/licences in the existing records for any new reuse.
Score only AF3 measurements and structural checks (duration, gaps, loudness,
caption timing). Report success rate, tool calls, tokens and wall time; unknown
token usage is unavailable, not zero. CI uses a deterministic scripted smoke
subset. Full real-model runs are manual, labelled with model/version and costs;
publish baselines without presenting scripted CI as model-performance evidence.
Reuse `tests/native/ai_closed_loop_workflow.py` from Packet 7.2 for live-bridge
acceptance. A supplied dialogue/music fixture is not a stem-separation claim.

### 2.8 Headless daemon — AF9

Extend existing server components with long-lived project open/close, authenticated
sessions on a UDS, bounded rate/time budgets and JSONL audit events (actor, tool,
argument hash, tier, result). Reuse AF2 Stop and AF5 provenance. Test missing/wrong
tokens, session isolation, complete audit events, disconnect and Stop cleanup.
Never place secrets or raw sensitive tool arguments in the log. Hosting follows
this verified daemon milestone; it is not part of the current deployment plan.

## 3. Agent and review UI — AF6 / Packet 5.2

### 3.1 Fixed-shell placement

Keep the existing top bar; left dock / Program / right dock upper row; toolbox /
timeline / meters lower row; and status bar. **Do not relocate meters or add a
new strip that changes these slots to match the attachment's schematic.**

AF0 source reconciliation confirms an existing exception: Deliver uses the
full-page `ExportWorkspace` (`WorkspaceRouter.tsx:228-232`), and geometry tests
explicitly cover seven docked modes plus that exception. Preserve this route;
global agent status/Stop must remain visible there as well. Existing shell tests
do not prove a right-dock affordance exists in Deliver.

Proposed placement is permanent Agent and Review tabs in the right `DockCard`,
alongside each workspace's existing tabs. Product confirmation is pending (§5),
but every design must retain an always-visible top-bar agent status chip with
Stop and a pinned right-dock status line across tabs/docked workspaces. The status bar
shows branch, mode, last actor and saved state.

### 3.2 Agent card

Pinned status (idle / thinking / editing / waiting / stopped), current action,
step n/m, elapsed time and Stop; session identity/mode, loaded profiles and grant
badges; editable brief; numbered plan with done/current/pending steps and timeline
jumps; instruction box; collapsed last-seen frame/scope/telemetry strip; inline
permission requests with Allow once / Allow session / Deny and a tab badge.

At 1100–1280 px collapse the body to the pinned status plus “Open agent”, expanding
in place. Also verify the configured 900 px minimum width: no overlap or hidden
Stop. Fold the current assistant drawer into the card only after provider and
existing-function parity; retain a local-model option if approved.

### 3.3 Review card

Open when a run finishes; group changes by plan step, low-confidence first.
Rows show before/after thumbnails, rationale, confidence label, tick range and
tier. Verify results show pass/warn/fail and unavailable/coverage details with
timeline navigation. Whole-branch Accept all / Reject all uses the existing
`project.branch_merge` contract; “Keep editing manually” changes session mode.

Per-change Accept/Reject, “Fix this” feedback and changed-range playback require
AF2's conflict-safe change model, not a range argument invented for v1 merge.
Proposed shortcuts: J/K navigation, A accept, R reject, F fix, Space changed-range
playback. Scope them to the focused review surface and resolve existing playback
shortcut conflicts; never intercept typing in text fields.

### 3.4 Timeline and remaining co-edit panels

- Agent badges and thin top-edge stripes use existing accent tokens; confidence
  has text/shape cues as well as a dot. Human/agent playheads stay distinct.
- `AiBranchDiffRibbon` shows branch changes above tracks without moving the fixed
  shell. Durable locks have indicators and “Lock for me” / “Ask agent to redo
  this” / “Explain this edit” actions.
- Actor-filtered history and agent-only undo obey AF2's dependency checks.
- `AgentTelemetryDrawer` is a dock tab/section despite its historical name: live
  calls, tier/profile, measured deltas, JSONL export and “save as skill”. Export
  is permission-controlled, and saved skills retain provenance.
- Keep Packet 5.2's `ScriptSyncPanel` and `VideoScopesPanel` requirements; scopes
  remain raw canvas. These are shared deliverables, not dropped in favor of chat.
- Settings → Agents: provider/endpoint, default mode, grants, auto-merge policy
  and audit location. First-run flow: connect agent, choose permissions, run a
  sample task on a demo project.
- Extend existing colour-freeze/style/geometry checks; use cached thumbnails,
  keyboard access, `aria-live` status, Stop first in card focus order and reduced
  motion. Test idle/running/stopped/error/waiting states and all workspaces.

## 4. Dependencies and execution

All implementation remains unchecked until tests and the normal repository gates
pass. Split work by existing ownership lanes: core/schema Lane 2, MCP/docs/harness
Lane 4, UI Lane 1, release/native verification Lane 3. A logical AF milestone can
contain several sequential single-lane commits; coordinate shared catalogue/menu
and handler-list edits under `AGENTS.md`.

| Packet | Prerequisite / exit criterion |
|---|---|
| AF0 housekeeping/design | Verify current licensing/package inventory; narrow third-party policy without rejecting owned first-party AGPL; negative dependency fixture fails; package notice bundle verified; actor/review conflict design written. *(2026-11-08: inventory, scoped policy with negative controls and the actor design are recorded — see the checklist AF0 section; notice/worker packaging evidence remains open.)* |
| AF1 profiles/tiers | Human-reviewed inventory first; every command/tool assigned; ≤30 default tools; typed profile hints; grants enforced across wrappers/batches. *(Completed 2026-11-09)* |
| AF2 sessions | AF1 policy + AF0 design; isolated cursors/branches, conflict-safe undo/review, durable locks, atomic-batch failure rollback, Stop ≤1 s with no late apply. *(Completed 2026-11-09)* |
| AF4 rationale/brief | AF2 history/change model; schema migration, reopen and merge preserve notes and brief; human corrections recorded. *(Completed 2026-11-09)* |
| AF3 verification | Existing perception + AF4 for full contract; seeded bad/clean fixtures correctly measured with tick/coverage evidence. *(Completed 2026-11-11)* |
| AF5 untrusted content | AF1 grants + AF2 identity + AF4 brief boundary; adversarial payloads cannot authorize side effects; approved workflows still pass. |
| AF6 Agent/Review UI | AF1–AF5 and existing glass shell; demo run → review → accept/reject → merge with human-edit conflict tests, geometry, colour and accessibility evidence; Packet 5.2 panels complete. |
| AF7 recipes/CLI | AF3–AF6 for supervised loop; existing 7.1 work reused; podcast recipe passes real verification; VFX CLI completion waits for Phase 6. |
| AF8 benchmark | AF7 + AF3; scripted CI smoke, live-bridge suite and labelled real-model baseline; final Phase 7 evidence waits for its remaining backend dependencies. |
| AF9 daemon | AF1–AF5; auth, audit, budgets and Stop verified before hosting. |

**Next actionable work:** **AF5** (Untrusted media provenance). **Recommended queue:** AF0 → AF1 (Done) → AF2 (Done) → AF4 (Done) → AF3 (Done) → AF5 → AF6/5.2 → remaining 5.1 verbs and Phase 5 gate → Phase 6 → AF7/7.1 → AF8/7.2 → AF9.
AF7/7.1 → AF8/7.2 → AF9. Early AF3 base checks and AF7/AF8 smoke are allowed once
their dependencies exist. This reconciles the attachment's “control before VFX”
recommendation without making all remaining narrative editing wait for a daemon.
AF4 precedes full AF3 because brief/confidence checks require it; numeric IDs are
stable labels, not an instruction to ignore dependencies.

Cinema stems remain separately blocked on the verified model. Keep dialogue-edge
room-tone crossfades, diarization, plugin-worker packaging and existing documented
limits visible; this plan does not mark those follow-ups complete.

## 5. Decisions pending and resolved from the maintainer

| Decision | Status / Approved resolution (2026-11-10) |
|---|---|
| New-session mode | Autopilot on an agent branch, then human review; plan approval required by default ([TASK_SHEET_PLAN.md](TASK_SHEET_PLAN.md) §0). |
| Auto-merge | Off by default; any opt-in must obey locks, stale-branch checks and grants. |
| Agent/Review location | Permanent right-dock tabs ("Sheet" and "Review/Notes") with pinned status, preserving fixed geometry ([TASK_SHEET_PLAN.md](TASK_SHEET_PLAN.md) §5). |
| Initial clients/providers | Any compatible MCP client plus the existing local-model option through the same card. |
| Execution priority | Dependency-aware queue: AF0–AF2, AF4 (Done) → AF3 → AF5 → AF6 → Task Sheet v1 (TS0–TS6) → AF7/AF8 → AF9 daemon → TS7–TS9 Web Desk & Runner. |
| Command profiles | Approved 2026-11-09; implemented in AF1. |
| Human interface format | Extended from `ProjectBrief` to **Task Sheet** (`tasksheet/1` schema, [TASK_SHEET_PLAN.md](TASK_SHEET_PLAN.md)). |
| Product family & contract | Shared agent contract for EEP, Audio Labs Pro (ALP), and Screen Studio Pro (SSP); open Capture Event Track (CET v0.1) from day one. |
| Desktop OS GUI priority | macOS first (developer/coding-agent focus), then Windows (creator base). |
| Public positioning wording | To be decided later (candidates preserved in [TASK_SHEET_PLAN.md](TASK_SHEET_PLAN.md) §10). |

The attachment mentions earlier UI decisions D1–D6, but this checked-in glass
plan has no numbered decision register. AF0's
[historical UI reconciliation](planning/af0-actor-design.md#10-historical-ui-reconciliation)
records current source evidence for glow, splitters, assistant location, workspace
names, track gain/pan and gaps without inventing numbered approvals. Locked colours,
fixed geometry and compact track headers remain binding; assistant placement and
the other choices above remain pending.

## 6. Honest limits

Taste and real-time feel need human review; sampled frames/telemetry do not prove
continuous perceptual quality. Track profile-load churn and adjust poor groupings
using measured sessions. Actor history/branch isolation is a substantive engine
change with migration/conflict tests. Provenance marking cannot guarantee model
behavior; enforce grants in the server. Existing licensing/legal review follow-ups
remain in [the licensing plan](planning/dual-licensing-plan.md) and are not resolved
by adding this document.
