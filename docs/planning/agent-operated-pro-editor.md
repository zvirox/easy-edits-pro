# EEP as a pro-grade editor for agents: product philosophy, the task sheet, and what to build around it

Repo checked: `main` @ `2834ba3` (2026-11-10). This builds on `docs/AGENT_FIRST_PLAN.md`; it does not replace it. It is the "what would I design if the editor were for me" answer, taken one level further: from an editor an agent can drive to a *production system* an agent can run from a work order.

---

## 1. The idea in one paragraph

DaVinci Resolve and Premiere are built around a human hand on a mouse: scrub, drag, tweak, watch. EEP should be built around a different loop: **a human gives a work order; an agent does the craft; measured checks prove the result; a human approves or sends notes.** The editor is the agent's workshop and the product around it is the producer's desk. That is a different product from "a chat box on top of a timeline", and from a prompt-to-video generator.

**Category name to own:** *the agent-operated pro editing system* (or "agentic post-production"). Not "AI video editor".

---

## 2. What "pro grade" must mean for an agent

Pro editors do not buy features, they buy **predictable deliverables and safe handoffs**. A pro-grade agent editor needs the boring things that make post-production trustworthy:

| Pro requirement | What it means for agents | Where EEP is |
|---|---|---|
| Colour-managed pipeline | Known working space, input transforms, output transforms; no accidental grading in the wrong space | Built (scene-referred GPU pipeline, IDT/ODT, nodes, shot matching) |
| Audio to spec | Loudness, true peak, dialogue clean-up, stems | Largely built (telemetry, ducking, room tone, master normalise, plugin host); stems not available |
| Conform and interchange | Take work from and return work to Resolve/Premiere/Final Cut (XML, FCPXML, OTIO, EDL), keep metadata | Importers/exporters exist; round-trip fidelity needs a published test suite |
| Deterministic, replayable edits | The same command sequence gives the same timeline; every step is logged and reversible | Strong (commands, undo, branches, journal) |
| QC before delivery | Automated checks against a delivery spec with a report | In progress (`verify.run`, AF3) |
| Versioning and revision rounds | v1, v2, v3 with diffs and notes | Branches and diff exist; revision-notes workflow missing (§5) |
| Provenance and rights | Where each asset came from, licences, AI-generated flags | Fingerprints exist; rights ledger missing |
| House style | Client or studio rules (fonts, lower-thirds, pacing, loudness, colour look) | `style_notes` free text only; needs structure (§4) |
| Scale | Many jobs, queues, budgets, unattended runs | Daemon planned (AF9) |

**Honest claim:** you can call it pro-grade *for the parts that are measured and tested*. Taste (pacing, humour, emotional rhythm) is the part agents are weakest at today, so the product should make that part cheap for a human to review, not pretend it is solved.

---

## 3. The human interface becomes a **task sheet** (work order)

Today EEP has one `ProjectBrief` (goal, platform, one target duration, must-keep clips, banned content, style notes, loudness target). Real post-production work orders are richer. Extend the brief into a **Task Sheet**: a structured document a human (or another system) fills in, and the agent reads.

```text
TASK SHEET
  job:            "Ep. 42 podcast, three social cuts"
  inputs:         media folder, camera/audio roles, transcript (optional), reference cuts
  deliverables:   [ {name, platform preset, resolution, duration ± tolerance, loudness, captions, aspect, codec} ... ]
  style:          house style id (or inline): fonts, colours (from brand kit), lower-third template, pacing, music rules
  must / must-not: keep these moments; never include these words/people/music
  acceptance:     list of measurable checks (verify.run presets) + human-judgement items to flag
  budget:         time, token/model cost, render minutes
  escalation:     ask the human when: ambiguous selection, low-confidence taste call, anything destructive
  handoff:        deliver as: exports + project file + XML for Resolve/Premiere + QC report + change log
```

**How it works**
1. **Intake:** the producer fills the sheet in a form (or an agent drafts it from an email or chat; the human confirms).
2. **Plan:** the agent proposes a plan (steps, risks, questions). The human can approve with one click or edit.
3. **Work:** the agent edits on a branch, looks at its own output with measured tools, fixes problems, and logs every step with reasons and confidence.
4. **QC report:** automatic pass/fail on every acceptance check with evidence (tick, value, frame).
5. **Review:** the human sees only what needs judgement: low-confidence items first, previews of changed ranges.
6. **Revision round:** the human leaves timecoded notes; they become structured revision tasks; the agent addresses them and shows a diff (§5).
7. **Delivery package:** files, project, interchange export, QC report, change log, provenance.

This is also the product's sales story: **"Hand it a task sheet, get back a QC'd deliverable and a review queue."** No competitor roundup I read describes this; they describe chat in a sidebar.

---

## 4. What I would want in the editor (and what I would drop)

**Need (agent-first features)**
- **Readable project state:** a compact, diffable text view of the whole timeline with stable IDs; patches and atomic batches (largely there).
- **Cheap perception:** contact sheets, scopes, loudness and spectrum, transcript with word timing, shot boundaries, semantic search, continuity measurements (there).
- **Self-check:** `verify.run` with delivery presets (AF3).
- **Reasons and confidence on edits** and a human-correction record (AF4, done).
- **Structured questions to the human:** the agent can pause and ask a multiple-choice question ("Which of these three takes?", with thumbnails) and carry on when answered; the answer is stored. Needed for unattended work.
- **Asset intelligence:** a project-wide index of what is in every clip (speakers, topics, shots, quality problems, rights flags), updated at import and queryable.
- **House style as data:** brand kit and style rules the agent can apply and `verify.run` can check (fonts, colours, safe areas, lower-third template, music rules).
- **Revision notes:** timecoded comments, one per issue, with status (open, addressed, rejected).
- **Budgets and checkpoints:** time, model-cost and render budgets; automatic checkpoints before risky steps.
- **Idempotent, bounded tools:** safe to retry, time-limited, with typed errors that tell the agent what to do next (profiles, tiers, locks).
- **Reproducibility:** record model/agent version, tool versions and seeds in the journal so a result can be explained or re-run.
- **Skills and recipes as first-class:** versioned, testable workflows ("podcast to three shorts", "conform and grade a sequence") that agents load, and humans or partners can publish.

**Don't need (for the agent)**
- Real-time playback tools, scrubbing, magnifiers, drag handles, multi-window layouts, per-pixel mouse tools.
- Dozens of overlapping ways to do one thing; fewer, sharper commands are better.
- Hidden state in dialogs, global modal settings, UI-only commands (71 are UI-only today and should stay out of agent profiles).
- Opaque "magic" one-button AI effects without measurable output.

**Keep for the human (supervisor), small and fast**
- The familiar timeline and preview for verification and take-over.
- Review queue, agent card, notes, approve/reject.
- A "manual mode" so a human editor can still cut, because pros will not trust a system they cannot take over.

---

## 5. The pro review loop (the feature that makes the product real)

Frame.io-style review is how professionals work. Build it natively:
- A reviewer (human) plays the preview and drops a **note at a timecode or range** ("tighten this", "wrong shot", "too loud", "use take 2").
- Each note becomes a **structured revision task** linked to clips; the agent receives them as a list with status.
- The agent resolves each note on a new branch version (v2), marks it addressed with a reason, and the review card shows **only the diffs against the previous round**.
- Notes the agent cannot do are returned with a question or reason, never silently dropped.
- Export the whole round as a change log for the client.

Optional later: notes from outside (client web link, email, Slack) feed the same queue.

---

## 6. A product family, not only one app

You said you want to build whole products around agents. The common pieces are a shared **agent contract**: tools with tiers and profiles, actors and locks, rationale and confidence, measured verification, task sheets, review. Build that once and reuse it:

| Product | Role | Status |
|---|---|---|
| **EEP engine + daemon** | The editing system (headless, MCP, CLI) | Core built; daemon planned |
| **EEP Studio (desktop)** | Human review, take-over and direct editing | Built; agent/review cards in progress |
| **Task Sheet and Review desk** | Producer-facing intake, approvals and revision notes (web or desktop) | Not started |
| **Recipes and skills library** | Versioned, tested workflows; partner-publishable | Not started |
| **EEP-Bench** | Public benchmark and certification of agents/models on EEP tasks | Not started |
| **Audio Labs Pro (your planned audio editor)** | Same agent contract for audio-only work, sharing tiers, verification and task sheets | Planning |
| **Hosted farm (later)** | Run many task sheets in parallel for teams | After daemon and licence cleanup |

This is how "agent-native pro tools" becomes a platform story: one contract, several tools. It also gives the commercial licence and hosting something real to sell.

---

## 7. Positioning, sharpened

**Against prompt-based editors:** they take a prompt and return a file or an edit you cannot inspect. EEP takes a *task sheet* and returns a *reviewed, measured, reversible* result with a change log.
**Against Resolve/Premiere:** they are tools for a human to operate. EEP is a system for an agent to operate and a human to supervise; and it interoperates with them (XML/OTIO in and out), so it fits a pro pipeline instead of asking to replace it overnight.

**Headline candidates**
1. "The pro editing system built for agents. Hand it a task sheet, get back a QC'd cut."
2. "Post-production for the agent era: open, local, measured."
3. "Your agent's editing suite."

**Message hierarchy:** (1) work orders in, deliverables out; (2) every claim measured; (3) human stays in control; (4) open source and local; (5) works with the agent you already use.

**Proof you must produce before the loud launch:** EEP-Bench v0; a 90-second demo of a task sheet turning into three deliverables with a QC report and one revision round; a round-trip test with Resolve/Premiere interchange; a case study from a real job.

---

## 8. The "replace human pro editing" goal: how to say it and what to watch

- **Direction is reasonable; wording matters.** Say "agents do the craft; humans direct and approve", and show the hours saved. "Replace editors" invites a defensive reaction from the very professionals who would adopt it, and it is not yet true for taste-heavy work.
- **Be specific about which jobs first:** podcasts, interviews, talking-head social cuts, product/tutorial videos, event recaps, compliance and training videos, localisation. These are rule-heavy and measurable. Narrative drama and comedy come last.
- **Rights and trust:** keep provenance and licence records; some clients will ask who or what made each cut.
- **Model dependence:** you do not control the agents. Support several (cloud and local) and publish benchmark results per model, so the product does not depend on one vendor.
- **Cost honesty:** the editor is free/open; the agent's reasoning has a cost. Show an estimate per task sheet.
- **Legal:** AI-written code and AI-assisted output raise copyright questions that differ by country; keep human review records and take advice before making ownership claims in contracts.

---

## 9. What to build next (order)

1. Finish AF3 `verify.run`, AF5 untrusted-content handling, AF6 Agent and Review cards (already queued).
2. **Task Sheet v1:** extend `ProjectBrief` to multiple deliverables, inputs with roles, acceptance list, budget, escalation rules, handoff. Add `tasksheet.*` commands and a form in the Agent card.
3. **Structured questions** from agent to human (pause/resume).
4. **Revision notes** and the round-by-round diff view.
5. **House style as data** (brand kit) and checks for it in `verify.run`.
6. **Delivery package** command: exports + XML/OTIO + QC report + change log + provenance.
7. **Skills/recipes format** and three flagship recipes with EEP-Bench tasks.
8. Interchange round-trip test suite with published results.
9. Then daemon, hosting, and the producer desk as a separate surface.

**Decisions for you**
1. Adopt "Task Sheet" as the main human interface, with the Agent card as its runtime view? (recommended)
2. Which first three job types do you want to win? (my suggestion: podcast clips, talking-head social, tutorial/product video)
3. Is the Task Sheet desk part of the desktop app first, or a separate web app?
4. Is Audio Labs Pro going to share the same agent contract?
5. Wording: "replace" or "agents do the craft, humans direct"?
