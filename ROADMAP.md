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

## Next

- Agent-first layer: reviewed tool profiles and permissions, human and agent
  sessions with locks and a stop control, measured self-checks, edit rationale,
  and an Agent and Review UI. See [docs/AGENT_FIRST_PLAN.md](docs/AGENT_FIRST_PLAN.md).
- Remaining editing verbs (morph cut, storyline pins).
- Closed-loop acceptance tests and a public benchmark.

## Later

- Depth compositing, corner-pin, object removal, kinetic typography.
- Authenticated long-running daemon for headless use.

## Where help is welcome

Testing on more distributions, documentation and translations, UI polish,
regression tests, packaging, and the agent tooling. See
[CONTRIBUTING.md](CONTRIBUTING.md) and the issues labelled `good first issue`
and `help wanted`.
