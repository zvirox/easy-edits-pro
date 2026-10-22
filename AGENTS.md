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

- size: 410 files, 9402 symbols, 3891 edges (resolved 2294, external deps 694, unresolved 903)
- languages: rust:129 typescript:78 tsx:69 python:39 css:34 markdown:27 toml:12 json:9
- subsystems: apps/saturn-tauri(172,typescript) · crates/saturn-core(116,rust) · crates/saturn-commands(12,rust) · agent-harness/cli_anything_eep(5,python) · apps/saturn-tauri(4,typescript) · apps/saturn-tauri(3,javascript) · agent-harness/cli_anything_eep(2,python)
- entrypoints: apps/saturn-tauri/ui/src/features/timeline/TimelineSection.tsx · apps/saturn-tauri/ui/src/features/dialogs/EditorModals.tsx · apps/saturn-tauri/ui/src/features/inspector/InspectorPanel.tsx · apps/saturn-tauri/ui/src/features/preview/PreviewSection.tsx · apps/saturn-tauri/ui/src/features/media-pool/MediaPanel.tsx · apps/saturn-tauri/ui/src/features/timeline/TimelineTrackRow.tsx · apps/saturn-tauri/ui/src/engine/playback/scheduler.ts · apps/saturn-tauri/ui/src/features/panels/PanelHost.tsx · (+18 more)
- central files (most depended-on): crates/saturn-core/src/catalogue/mod.rs · crates/saturn-core/src/catalogue/commands/mod.rs · crates/saturn-commands/src/lib.rs · apps/saturn-tauri/ui/src/engine/playbackEngine.ts · apps/saturn-tauri/ui/src/components/interior/interior-button.tsx
- read these guides first: README.md · CONTRIBUTING.md · docs/architecture.md · docs/design-qa.md

Depth on demand: `prowl find|def|outline|references <name>`, `search <text>`, `context search "<question>"`, `sketch <ui>`.
<!-- /prowl-agent:map -->
