<!-- prowl-agent -->
## Prowl project context

This repo has a Prowl index of its files, symbols, and how they connect. For any
semantic or structural question -- where code is, what it does, who calls it, or
what a change touches -- **run the read-only prowl CLI first**; do not grep or
read whole files just to locate things. Prowl reindexes what changed before each
query, so answers stay current and are cited to file:line, returned in one call
instead of a grep hit list you then open files to disambiguate.

| Question | First command |
|---|---|
| Map the repository | `prowl overview` |
| Locate a feature or concept | `prowl search "<question>"` |
| Locate a named symbol | `prowl find <name>` |
| Read one symbol's source | `prowl def <name-or-id>` |
| Inspect a file's structure | `prowl outline <path>` |
| Trace who uses a symbol | `prowl references <name-or-id>` |
| Size a change's blast radius | `prowl impact <path>` |
| Inspect uncommitted work | `prowl wip` / `prowl changed` |
| Read a located line range | `prowl peek <file:start-end>` |

Keep grep for exact literal or regex text and glob for filename patterns. CLI
output is token-lean TOON by default; add --format human|toon|json|markdown. If
your harness also wires Prowl as an MCP server, the same index is reachable
there; the CLI needs no server and is the first choice.
<!-- /prowl-agent -->

<!-- prowl-agent:map -->
## Prowl project map

Auto-generated from the Prowl index, refreshed on each `overview`/`init`. Prefer retrieving from Prowl (and reading the cited files) over grepping or relying on training memory; this is the current shape of the repo.

- size: 475 files, 10474 symbols, 4228 edges (resolved 2464, external deps 780, unresolved 984)
- languages: rust:144 typescript:98 tsx:75 css:39 python:39 markdown:34 javascript:17 toml:13
- subsystems: apps/saturn-tauri(203,typescript) · crates/saturn-core(131,rust) · crates/saturn-commands(12,rust) · apps/saturn-tauri(11,javascript) · agent-harness/cli_anything_eep(5,python) · apps/saturn-tauri(4,typescript) · apps/saturn-tauri(3,javascript) · agent-harness/cli_anything_eep(2,python)
- entrypoints: apps/saturn-tauri/ui/src/features/timeline/TimelineSection.tsx · apps/saturn-tauri/ui/src/features/dialogs/EditorModals.tsx · apps/saturn-tauri/ui/src/features/inspector/InspectorPanel.tsx · apps/saturn-tauri/ui/src/features/preview/PreviewSection.tsx · apps/saturn-tauri/ui/src/features/panels/PanelHost.tsx · apps/saturn-tauri/ui/src/features/captions/CaptionsPanel.tsx · apps/saturn-tauri/ui/src/features/graphics/GraphicsPanel.tsx · apps/saturn-tauri/ui/src/features/markers/MarkersPanel.tsx · (+21 more)
- central files (most depended-on): crates/saturn-core/src/catalogue/mod.rs · crates/saturn-core/src/catalogue/commands/mod.rs · apps/saturn-tauri/ui/src/engine/playbackEngine.ts · crates/saturn-commands/src/lib.rs · apps/saturn-tauri/ui/src/components/interior/interior-button.tsx
- read these guides first: README.md · AGENTS.md · CONTRIBUTING.md · docs/architecture.md · docs/design-qa.md

Depth on demand: `prowl find|def|outline|references <name>`, `search <text>`, `context search "<question>"`, `sketch <ui>`.
<!-- /prowl-agent:map -->

# Hard rules for agents (packet A4)

These rules are enforced by gates or review. Violating them is a failed packet.

## Design and styling

- **Design tokens only.** Never hard-code colours, radii, or spacing in
  components; use the token files. `pnpm check:styles` gates the raw-colour
  ratchet.
- **Never `transition: all`.** Name the properties you animate; respect
  `prefers-reduced-motion`.
- **Glass only through the `.glass` surface classes.** Never hand-roll
  `backdrop-filter`/blur in a component.
- **Media surfaces never glass.** Video/preview/audio-render surfaces stay
  raw; blur over them wastes frame budget and skews colour.

## Code and engine

- **800-line limit per source file.** `pnpm check:source-size --max-lines=800`
  gates it; split by area, never by convenience.
- **Stubs must return an error.** An unimplemented command or handler
  reports a real error; silent fake success is worse than absence.
- **`shipped: true` only with a UI affordance and a test.** Planned commands
  stay unshipped until the verified-before-done rule is met.
- **Project state is owned by the Rust core.** The UI sends commands and
  renders snapshots; never fork state client-side.

## Generated files — never hand-edit or hand-merge

- `docs/COMMANDS.md`, `docs/reference/command-catalogue.json`,
  `docs/PARITY.md`, `docs/glass-tokens.md`, `Cargo.lock`, `pnpm-lock.yaml`.
- After any Rust catalogue change run `pnpm catalogue:sync` (and the token
  docs script after token changes), then commit the regenerated output.

## Path ownership (one agent per lane at a time)

| Lane | Paths | Packets |
|---|---|---|
| 1 UI/glass | `ui/src/styles/`, `ui/src/features/panels/`, `scripts/check-*` (glass), `tests/visual`, `tests/a11y` | C3-C13 |
| 2 engine/parity | `crates/saturn-core/src/catalogue/`, `crates/saturn-core/src/engine/`, `ui/src/features/<area>` | A1, D4, E* (one family per agent) |
| 3 release/verification | `.github/`, `packaging/`, `apps/saturn-tauri/src/gpu_detect.rs`, `scripts/make-corpus.sh`, `tests/native/` | A2, B*, C1-C2, F* |
| 4 tooling/docs/safety | `scripts/check-parity.mjs`, `docs/`, `crates/saturn-mcp`, `mcp_bridge.rs` | A4, D1-D3, G*, M* |

Conflict hotspots: `crates/saturn-core/src/catalogue/commands/<menu>.rs` +
`commands/mod.rs` (one menu per agent), `ui/src/styles/index.css` (lane 1
only), `apps/saturn-tauri/src/main.rs` handler list (separate small commits,
merged in order).

## Before you report a packet done — run the gates

```
cargo test --workspace
pnpm build          # typecheck + vite build
pnpm test
pnpm check:glass    # contrast + styles + tokens
pnpm check:commands
pnpm check:source-size --max-lines=800
pnpm test:e2e       # UI contract against a mocked backend - not an engine claim
```

Green gates, the sweep for junk characters in new files, and a clean
`git status` (agent/tooling files such as `.claude/`, `.prowl/`,
`test-results/` stay out of commits) are part of done.
