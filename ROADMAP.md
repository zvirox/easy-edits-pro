# Roadmap

This page is a short, honest summary. The detailed, always-current status lives
in [docs/AI_NATIVE_PLAN_CHECKLIST.md](docs/AI_NATIVE_PLAN_CHECKLIST.md); known
limitations are listed in the [README](README.md). Nothing here has a promised
date.

## Where the project is

Easy Edits Pro is an **alpha** Linux video editor with a Rust core, a Tauri 2
and React UI, and GStreamer/GES rendering. The current direction is an
**agent-friendly editor**: AI agents can drive every editing command over MCP,
look at the result with measured tools (frames, scopes, audio telemetry,
semantic search), try changes on branches, and a human reviews the result.

## Done recently

- Command surface over MCP, branches and fingerprints, security hardening.
- Perception: frame sheets, video scopes, audio telemetry, real transcription,
  shot detection, semantic search, matting, depth and motion tracking.
- Colour: scene-referred GPU pipeline, node graph, automated shot matching.
- Audio: native plugin host, offline dialogue clean-up and mastering.
- Editing: J/L split edits, match-cut alignment, beat-grid quantisation.
- A fixed frosted-glass card layout shared by every workspace.
- Agent-first core (Packets AF1, AF2, AF3, AF4): tool profiles and tiers, actor isolation,
  durable locks, Stop control, atomic engine batches, edit rationale, confidence,
  human-owned project brief with correction provenance, and measured self-checks (`verify.run`).
- Contributor onboarding: contributor guide, doctor script, PR template, and codeowners.

## Next

- Agent-first completion: untrusted media provenance (AF5), and Agent and Review UI cards (AF6). See
  [docs/AGENT_FIRST_PLAN.md](docs/AGENT_FIRST_PLAN.md).
- Task Sheet system (TS0–TS6): structured work orders (`tasksheet/1`), plan approval
  gates, structured agent questions, pro review notes and round diffs, delivery
  package exporter, and Capture Event Track (CET v0.1) integration. See
  [docs/TASK_SHEET_PLAN.md](docs/TASK_SHEET_PLAN.md).
- Remaining editing verbs (morph cut, storyline pins).
- Flagship recipes (podcast clips, talking-head/tutorials, webinar recaps) and
  the EEP-Bench public benchmark.

## Later

- Authenticated daemon (`saturn-control` web API) and Web Desk for self-hosted teams (TS7–TS9).
- Cross-platform support: Tier 1 Linux, Tier 2 macOS and Windows (headless CLI/daemon first; macOS desktop GUI first, then Windows).
- Depth compositing, corner-pin, object removal, kinetic typography.
- Shared agent contract extraction for Audio Labs Pro and Screen Studio Pro.

## Where help is welcome

Testing on more distributions, documentation and translations, UI polish,
regression tests, packaging, and the agent tooling. See
[CONTRIBUTING.md](CONTRIBUTING.md) and the issues labelled `good first issue`
and `help wanted`.
