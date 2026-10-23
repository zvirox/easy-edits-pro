# Agent task-card template (packet G1)

Every work packet handed to an agent starts life as a task card copied from
this template. One card = one small, verified, self-contained change
("packet"). The card is pasted into the agent session (or issue) as-is; the
agent fills nothing in — the integrator writes the card before dispatch.

Rules of thumb: one lane per card (see path ownership in `AGENTS.md`), a card
never spans lanes, and if the card cannot state its acceptance criteria it is
not ready to dispatch.

---

```markdown
# Task card: <packet-id> — <short title>

## Goal
One or two sentences: what exists after this packet that does not exist now.

## Why now
Which milestone/workstream this serves (see docs/planning.md).

## Owned paths
- <paths this agent may create or modify>

## Forbidden paths
- Everything else, in particular: generated files (docs/COMMANDS.md,
  docs/reference/command-catalogue.json, docs/PARITY.md, docs/glass-tokens.md,
  Cargo.lock, pnpm-lock.yaml — regenerate via scripts, never hand-edit),
  files owned by another lane, and agent/tooling files (.claude/, .prowl/,
  .mcp.json, test-results/) which never enter commits.

## Background
Facts the agent cannot rediscover quickly: relevant command ids, schema
versions, prior packet ids, log lines, known traps.

## Acceptance criteria
- [ ] <observable, testable statement>
- [ ] <...>

## Verification commands
cargo test --workspace
pnpm build
pnpm test
pnpm check:glass
pnpm check:commands
pnpm check:source-size --max-lines=800
pnpm test:e2e
(<packet-specific gates first, e.g. cargo test -p saturn-render; if the
catalogue changed: pnpm catalogue:sync and commit the generated output)

## Constraints
- Green main, always: small logical commit(s), no wip.
- shipped: true only with a UI affordance and a test.
- Stubs must return an error.
- Design tokens only; glass only through .glass; media surfaces never glass;
  no transition: all; 800-line limit.
- Follow the honest-work rule: docs and screenshots may claim only what the
  gates verified.

## Report back
- Commit hash(es) and one-line message(s)
- Gate results (pass/fail per command)
- Anything discovered that invalidates assumptions (this is valuable —
  report it even if the packet could not finish)
```

---

## Worked example (truncated)

```markdown
# Task card: D3 — parity backlog generator

## Goal
`scripts/parity-backlog.mjs` prints the missing FilmCraft ids for one family,
so agents get task lists without guessing.

## Owned paths
- apps/saturn-tauri/scripts/parity-backlog.mjs
- apps/saturn-tauri/package.json (script entry only)

## Acceptance criteria
- [ ] `pnpm parity:backlog --family clip` lists missing Tier-1 clip ids with
      labels and tiers, sourced from the same reference files as PARITY.md
- [ ] Unknown family name exits non-zero with the list of valid families
- [ ] No file over 400 lines; no new dependencies
```
