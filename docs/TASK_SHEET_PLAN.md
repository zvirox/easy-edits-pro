# EEP Task Sheet Plan: the human interface for agent-operated post-production

Baseline: `main` @ `2834ba3` (2026-11-10). Companion to `docs/AGENT_FIRST_PLAN.md`; it extends AF4's `ProjectBrief` and plugs into the AF1/AF2 profile, tier and actor system. Wording for all public text: **"Agents do the craft, with help from humans."**

---

## 0. Decisions recorded

| Question | Decision |
|---|---|
| Task sheet as the main human interface | Yes |
| First three job types | Podcast social clips, talking-head/screen tutorials, webinar recaps (see §1) |
| Where the Task Sheet desk lives | **Both**: in the desktop app and as a separate web app (B2B licensees run it on their own platform) |
| Audio Labs Pro shares the agent contract | Yes. Screen Studio Pro (SSP, agent-first screen recording) will also use it |
| Public tagline / wording | **To be decided later** (candidates noted in §10) |
| UI name | **Task Sheet** (code name `tasksheet`) |
| Plan approval | **Required by default.** Auto-approval may be offered later per job type, as an explicit opt-in per organisation |
| Web Desk v1 accounts | **Local accounts only.** SSO (OIDC/SAML) is a later packet (TS10) |
| SSP capture event track | **Documented open format from day one** (CET v0.1: [planning/capture-event-track-v0.1.md](planning/capture-event-track-v0.1.md)) |
| CET naming | **Capture Event Track (CET)** confirmed |
| CET licence | **CC BY 4.0** for specification; **Apache-2.0** for JSON Schema, test vectors, and reference libraries |
| CET repository | **Dedicated standalone public repository**: [github.com/zvirox/capture-event-track](https://github.com/zvirox/capture-event-track) |
| CET key capture | **Class-only by default** (`letter`, `digit`, `nav`...); explicit opt-in for named shortcuts (`privacy.named_shortcuts="on"`) |
| Recipe 2 fixture | **Synthetic generator (`cet-gen`) first** with test vectors before physical SSP recordings exist |
| Cross-platform tiers | **Tier 1 (Linux)** and **Tier 2 (macOS and Windows)**, headless-first parity (`saturn-cli` and `saturn-mcp`) before Tauri GUI ports |
| Desktop OS GUI priority | **macOS first** (developer/coding-agent focus), then **Windows** (creator base) |

---

## 1. The three jobs to win first (my pick)

Chosen because the work is rule-heavy and measurable (so agents can check themselves), the demand is large, and the same recipes feed your other products.

| # | Job | Why | Typical sheet | Measurable acceptance (suggested defaults; confirm per platform) |
|---|---|---|---|---|
| 1 | **Podcast and interview to social clips** | Huge demand; transcript-driven; clear specs; fits ALP (audio) | One long recording → 3–10 vertical or square clips with captions | Loudness target and true-peak limit; captions present and inside safe area; no dead air over a set length; duration within tolerance; banned words absent |
| 2 | **Talking-head, tutorial and product videos** (including screen recordings) | Large B2B and creator use; shares clean-up, chapters, captions; fits SSP | Raw take(s) → cleaned, captioned, chaptered master + short cut | Filler/silence removal within limits; loudness; caption timing; chapter markers present; export specs |
| 3 | **Webinar, event and training/compliance recaps** | Strong B2B and privacy story (on-prem); long source to highlights | Long recording → highlight reel + full cleaned version + summary chapters | Duration target; coverage of required topics (from transcript); loudness; caption accuracy sample; no flagged content |

**Deliberately not first:** drama, comedy, music video and advertising (taste-heavy, hard to measure), and generative video (a different category).

---

## 2. Product architecture

```
                       ┌──────────────── Task Sheet core (Rust) ────────────────┐
 Human surfaces        │ schema · state machine · validation · rounds · notes   │      Agent surfaces
 ───────────────       │ questions · budgets · delivery package · audit log     │      ───────────────
 Desktop Sheet tab ───▶│                                                        │◀──── MCP tools (profile "tasksheet")
 Web Desk (browser) ──▶│  runs inside the EEP daemon (saturn-control: axum+WS)  │◀──── Runner (picks up ready sheets,
                       │  uses the same engine, branches, verify, journal       │      launches an agent session)
                       └────────────────────────────────────────────────────────┘
```

- **One core, two surfaces.** The desktop panel and the Web Desk call the same commands. The crate `saturn-control` already depends on `axum` with WebSocket support, so the web API can extend it rather than start fresh.
- **Web Desk is a thin client.** It never touches raw media; it receives low-resolution proxy video over HTTP range requests, metadata, notes and QC reports.
- **Runner.** A small service that watches for sheets in state `ready`, starts an agent session with the right profile, permission tiers and budget, and records the run. Models are bring-your-own (cloud or local/on-prem endpoints).
- **Agent contract package.** Keep the product-agnostic parts (task sheet, review rounds, notes, questions, verify result shape, tiers/profiles, actors, audit log) in a clearly separated module with its own JSON schemas and conformance tests. **Extract it into a shared crate only when ALP is the second real user** (SSP third). Extracting before there are two users usually produces the wrong abstraction. Until then, keep the boundary clean inside EEP.
- **Domain parts stay per product:** EEP commands and checks (video), ALP (audio), SSP (capture). SSP can also supply a **capture event track** (cursor, clicks, key presses, window focus) as an input that EEP agents use for zooms, cuts and chapters.

---

## 3. Task Sheet schema v1

A sheet is one JSON document, versioned, stored in the project (and in the daemon's job store for the web).

```jsonc
{
  "schema": "tasksheet/1",
  "id": "ts_01",
  "title": "Ep. 42 social cuts",
  "requester": "human:maria",
  "inputs": [
    { "id": "in1", "media": "…", "role": "main_video" },
    { "id": "in2", "media": "…", "role": "isolated_audio" },
    { "id": "in3", "media": "…", "role": "transcript", "trust": "untrusted" }
  ],
  "deliverables": [
    { "id": "d1", "name": "Clip A", "preset": "vertical-9x16", "duration": {"target_s": 45, "tolerance_s": 10},
      "loudness_lufs": -14, "true_peak_dbtp": -1, "captions": true, "must_include": ["moment:12:31"] }
  ],
  "style": { "house_style": "brand_kit_id | inline", "pacing": "medium", "music": "none" },
  "must_not": { "words": [], "people": [], "content": [] },
  "acceptance": { "auto": ["verify.preset:vertical-9x16", "loudness", "captions_safe_area", "no_dead_air"],
                  "human": ["hook_in_first_3s", "tone_matches_brand"] },
  "budget": { "wall_clock_min": 60, "model_cost_usd": 5, "render_min": 30 },
  "escalate_when": ["ambiguous_selection", "low_confidence_taste_call", "destructive_action"],
  "handoff": { "exports": true, "project_file": true, "interchange": ["fcpxml","otio"], "qc_report": true, "change_log": true },
  "status": "draft",
  "rounds": []
}
```

**State machine:** `draft → ready → planning → awaiting_plan_approval → running ⇄ needs_input → in_review → revision_requested → running (next round) … → approved → delivered`, plus `failed`, `cancelled`, `stopped`. Every transition records actor, time and reason. `needs_input` is the structured-question state.

**Relation to `ProjectBrief`:** keep `project.brief_set/get` as the single-deliverable shorthand. Migration: a brief becomes a one-deliverable sheet; both stay readable by `verify.run`.

---

## 4. Commands and tools (fit the existing profile/tier system)

New profile `tasksheet` (plus `core` read tools). Tiers follow AF1.

| Tool | Tier | Who | Purpose |
|---|---|---|---|
| `tasksheet.create / update / validate / submit` | edit | human (or agent drafting for confirmation) | Build and submit the sheet |
| `tasksheet.get` | read | all | Read sheet, status, rounds |
| `tasksheet.plan_submit` | edit | agent | Post the plan (steps, risks, questions) |
| `tasksheet.plan_approve / plan_reject` | edit | human | Approval gate (configurable: auto-approve for trusted job types) |
| `tasksheet.question_ask` | edit | agent | Pause with a structured question (choices, thumbnails, default, deadline) |
| `tasksheet.question_answer` | edit | human | Resolve a question; agent resumes |
| `tasksheet.run_report` | edit | agent | Step progress, budget use, checkpoint |
| `review.note_add / note_list / note_resolve` | edit | human adds, agent resolves | Timecoded revision notes with status |
| `review.round_open / round_close` | edit | system/human | Round boundaries, diff against previous round |
| `delivery.package` | filesystem | agent (with grant) | Build exports, project, interchange files, QC report, change log, provenance |

Rules: media-derived text (transcripts, captions) stays `untrusted` and can never edit the sheet, raise a tier or approve a plan (AF5). Questions must offer at least two choices and a safe default; unanswered by deadline means the job pauses, never guesses on destructive steps.

---

## 5. Desktop UI (on the glass shell; colours unchanged)

Extends the AF6 Agent and Review cards; no new floating panels.

- **Right dock tab "Sheet":** compact form for the sheet (inputs with roles, deliverables, acceptance, budget, escalation), template picker (the three job templates), validation messages, Submit.
- **Agent card additions:** sheet title and status chip, budget meters (time, cost, render), current step, **Inbox** (agent questions as small cards with choices and thumbnails; keyboard answer).
- **Review card additions:** round selector (v1, v2…), **Notes** list (timecoded, status), "Add note at playhead" shortcut, diff against the previous round, QC report with pass/fail rows that jump to the tick.
- **Timeline:** note markers on the ruler, agent clip marks (from AF6), low-confidence dots.
- **Status bar:** job status and cost so far.
- Narrow windows: the Sheet tab collapses to a summary row that opens in place.

---

## 6. Web Desk (browser app for B2B self-hosting)

**Pages**
1. **Jobs:** list and board by state; filters by requester, job type, due date; cost and time per job.
2. **New job:** pick a template (the three jobs first), upload or choose media, fill the sheet, validate, submit.
3. **Job detail:** sheet, plan (approve/reject), live run log (tool calls with tier and reason), questions inbox, budget.
4. **Review:** proxy video player, timecoded notes, QC report, round diffs, approve or request revision.
5. **Deliveries:** download package, interchange files, provenance.
6. **Admin:** users and roles, agents/models and keys, budgets and limits, audit log export, branding.

**Rules**
- Roles: requester, reviewer, approver, admin. Approval cannot be self-granted by an agent.
- Auth: local accounts first; OIDC/SAML for enterprise SSO later.
- Security: loopback by default; remote access requires TLS and tokens; signed URLs for proxies; per-job media access; full audit log (AF9).
- Reuse: same React components and the same design tokens (colours locked) as the desktop app, to avoid two design systems.
- **AGPL §13:** a web UI that serves a modified version over a network must offer users the source. Add a visible "Source and licence" link in the Web Desk. Customers who cannot accept that buy the commercial licence, which is exactly your dual-licence offer.
- Packaging: container image and a systemd unit first; Helm chart later. Single-tenant per deployment first; multi-tenant hosting only after isolation work.

---

## 7. Phases (after the existing AF queue; each is a separate, testable packet)

| ID | Work | Exit criterion |
|---|---|---|
| TS0 | Design freeze: schema JSON Schema files, state machine doc, template drafts for the three jobs | Reviewed; examples validate |
| TS1 | Core: sheet types, validation, state machine, migration from `ProjectBrief`, commands, journal entries (depends on AF3, AF5) | Unit tests for every transition; save/reopen; brief migration test |
| TS2 | Desktop Sheet tab and Inbox (with AF6) | e2e: create sheet → submit → agent question → answer → resume |
| TS3 | Questions and plan approval flow | Agent paused until answered; unanswered deadline pauses safely |
| TS4 | Review notes and rounds | Notes survive save/reopen; round diff correct; agent marks each note addressed or returns it with a reason |
| TS5 | `delivery.package` | Package contains exports, project, interchange file, QC report, change log, provenance; checksums verify |
| TS6 | Three flagship recipes + EEP-Bench (scripted conformance suite) tasks for each (`--agent` mode planned separately) | Scripted conformance suite passes in CI; real-agent `--agent` baseline before public claims |
| TS7 | Daemon web API (REST + WS) on `saturn-control` (after AF9 auth/audit) | Auth required; audit log complete; API conformance tests |
| TS8 | Web Desk MVP (Jobs, New job, Job detail, Review) | Playwright e2e against a daemon with a scripted agent |
| TS9 | Runner (queue, worker leasing, budgets, BYO model endpoints) | Two concurrent jobs complete within budgets; kill switch works |
| TS10 | Enterprise SSO (OIDC/SAML) | SSO login flow and role-mapping tests pass |
| TS-X | Extract the shared agent-contract crate when ALP is the second user | ALP passes the same conformance tests |

### Agent task cards (copy into the checklist)

- **TS0/TS1:** "Add `tasksheet/1` JSON Schema and Rust types in a module separate from EEP domain code (so it can later be extracted). Implement the state machine and validation, migrate `ProjectBrief` to a one-deliverable sheet, add `tasksheet.*` commands with tiers and a `tasksheet` profile, and write journal entries per transition. Tests for every transition and for refusing any sheet change that originates from `untrusted` text."
- **TS2/TS3:** "Add the Sheet tab and Inbox to the Agent card using existing glass tokens; implement question pause/resume and plan approval end-to-end; extend the geometry e2e."
- **TS4:** "Add `review.*` notes and rounds with a round-diff view; migrate saved projects; keyboard-first note entry at the playhead."
- **TS5:** "Implement `delivery.package` with checksums, provenance and a QC report; reuse existing exporters."
- **TS7/TS8:** "Expose the task sheet and review APIs over `saturn-control` with token auth and audit; build the Web Desk using the shared components and tokens; add a visible Source and Licence link."

---

## 8. The three recipes in a bit more detail

Each recipe is a versioned skill (steps for the agent), a sheet template, a set of `verify` checks and a bench task.

1. **Podcast to clips:** find candidate moments from transcript and semantic search; rank with measurable signals (speech energy, topic match to the sheet, completeness of sentences); cut on word boundaries; reframe; add captions; loudness and true-peak pass; QC; flag hook, tone and ending as human-judgement items.
2. **Talking head and tutorial (with screen recordings):** remove silence and fillers within limits; clean audio (room tone, phase, ducking); cut to chapters from transcript; captions; use SSP's event track (clicks, window changes) to propose zooms and cuts; master loudness; QC.
3. **Webinar and training recap:** chapter from transcript; coverage check against required topics in the sheet; highlight selection with must-include moments; cleaned full version plus highlight reel; caption sample check; flagged-content check against `must_not`.

---

## 9. Commercial angle

- **On-premises B2B:** the Web Desk plus daemon plus runner is the product a company installs on its own platform. The commercial licence lets them keep their changes private; support and updates are the recurring revenue.
- **Keep one codebase.** Because you hold the copyright through the CLA, you could later sell closed add-ons (SSO, audit tooling), but that splits the story and the work. My suggestion: sell the licence, support and hosting, and keep features in the open code until there is a clear reason.
- **Price anchors to research later:** per-seat for the desk, per-agent-run or per-render-minute for the runner. Not decided here.

---

## 10. Messaging

- **Tagline:** "Agents do the craft, with help from humans."
- **Sub-line:** "Hand your agent a task sheet. Get back a checked cut and a review queue."
- **B2B line:** "Run it on your own platform. Your footage never leaves."
- **Proof to publish:** a 90-second demo of one sheet → three deliverables → QC report → one revision round; EEP-Bench `--agent` results (distinct from the scripted conformance suite) for the three jobs; interchange round-trip results.

---

## 11. Risks and decisions

- **Scope:** the Web Desk and runner are a second product. Do TS1–TS6 in the desktop first, then TS7–TS9.
- **Over-engineering the shared contract:** extract only when ALP is the second user.
- **Quality:** agent edits will sometimes be wrong; the plan depends on QC plus review. Track the acceptance pass rate and the fraction of agent edits humans change; those are your honest quality numbers.
- **Security:** a web surface and remote agents widen the attack surface. AF5 (untrusted content) and AF9 (auth, audit, budgets) are prerequisites for TS7 onward.
- **Licensing:** close the open items from the licence audit before any B2B sale (`deny.toml` allow list, third-party notices, codec packages, CLA entity and lawyer review).

**Decisions closed (2026-11-10 & 2026-11-11)**
- **Task Sheet**: Name "Task Sheet"; plan approval required by default; local accounts for Web Desk v1 (SSO in TS10); desktop Sheet tab and Agent Inbox UI.
- **Capture Event Track**: Name confirmed as Capture Event Track (CET); licence split CC BY 4.0 (spec) and Apache-2.0 (schema/code); dedicated public repo created at [github.com/zvirox/capture-event-track](https://github.com/zvirox/capture-event-track); class-only keys by default with named shortcut opt-in; recipe 2 starts with synthetic generator (`cet-gen`).
- **Cross-platform**: Tier 1 Linux, Tier 2 macOS and Windows; headless first (`saturn-cli` and `saturn-mcp`); macOS desktop GUI first, then Windows.
- **Positioning**: Public tagline to be decided later (candidates preserved in §10).

**Decisions still open**
- None currently blocking. All governance and structural decisions closed.
