# Starter issue template (for maintainers)

Use this when writing a `good first issue`. A good starter issue can be done in
one to three hours by someone who has never seen the code.

```markdown
**Title:** <small, specific outcome>

**Context**
Why this matters, in two or three sentences.

**What to change**
- Files to look at: `path/to/file.rs`, `path/to/other.tsx`
- What the current behaviour is, and what it should be.

**How to know it is done**
- [ ] Test or manual check that proves it (give the exact command).
- [ ] `cargo test --workspace` and the relevant `pnpm` checks pass.

**Size / difficulty:** easy | medium
**Mentor:** @maintainer (ask here or in Discussions)
**Labels:** good first issue, area: <ui|engine|render|audio|agent|docs|packaging>
```

Rules of thumb: link the exact files, give the acceptance check, keep it under
about 150 changed lines, and never leave a claimed issue silent for more than
14 days.

Suggested labels: `good first issue`, `help wanted`, `needs design`,
`needs repro`, `blocked`, `question`, `difficulty: easy|medium|hard`,
`area: ui|engine|render|audio|agent|docs|packaging`. Create them with
`scripts/github-labels.sh`.
