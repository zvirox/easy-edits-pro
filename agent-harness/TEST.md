# CLI Harness Verification

## Test command

Run from the repository root:

```bash
python -m pytest -q agent-harness/tests
```

The Rust integration tests use workspace binaries when available and skip when they are not built. Unit tests use a protocol-compatible fake backend for control-process error handling.

## Coverage

- Every command id in the engine registry has a named CLI verb, and the suite drives the editing verbs against the real `saturn-cli` engine: timeline split/ripple/transitions/marks, clip effects/keyframes/motion blur, media bins, subtitles, titles, node graph, collaboration, and multicam creation.
- Real `saturn-control` lifecycle: `daemon start`/`status`/`stop` against `/health`, duplicate-start refusal, and stale-pid cleanup.
- Real MCP stdio handshake: `mcp tools`, `mcp resources`, and `mcp prompts` against `saturn-mcp`.
- Real `.sgt` template round trip through `saturn-graphics`: create, list, inspect, export, plus an honest `COMMAND_NOT_FOUND` for `template apply`.
- Engine-delegated `undo`/`redo`, including a subprocess REPL round trip.
- The CLI command journal (`history`), including capping, ordering, and corrupt-line recovery.
- Structured backend errors and JSON-lines request/response handling.
- Project CLI error mapping and persisted project state.
- Backend-backed clip listing and render cancellation errors.
- Rust control-process project/track/undo/redo integration.
- Every test runs against an isolated `XDG_STATE_HOME`, so a developer's saved project or daemon cannot leak into the suite.

## Latest local result

| Date | Result | Notes |
|------|--------|-------|
| 2026-10-20 | 62 passed | `python -m pytest -q` from `agent-harness`; includes real engine, control-server, MCP, and template integration. |
| 2026-10-20 | 140 passed, 0 warnings | `cargo test --workspace` and `cargo check --workspace --all-targets`, including `saturn-control`, `saturn-graphics`, and `saturn-project`. |

## Latest remote result

GitHub Actions run #21 (2026-10-23) installed the package, built the Rust
backends, then failed the harness verification step in the media-bin/subtitle
test. The local result above is historical evidence, not a statement that the
current branch is green. The next task is to reproduce and fix that failure
before publishing another release.

## Packaging and CI

The harness has a single `pyproject.toml` with a `[project]` table; `setup.py` and `requirements.txt` are gone. A clean editable install and a wheel build were verified locally in a fresh virtual environment using `--no-build-isolation` with `setuptools` installed from a local wheel cache, because the default interpreter has no `setuptools` and an isolated build cannot reach the package index. The built wheel contains the package, the console script, and `skills/cli-anything-eep.md`. CI exercises the normal isolated editable install on GitHub Actions.

`.github/workflows/ci.yml` (renamed from `rust.yml`) contains the `python-cli-harness` job, which installs the harness, builds `saturn-cli`, `saturn-mcp`, and `saturn-control`, and runs the pytest suite.

## Known test limits

- Render queue/status/cancel require the desktop application's local socket bridge and are not exercised by the standalone control smoke test.
- Undo/redo history lives in the engine process: a one-shot `undo` invocation with no prior edit in that process reports `NOTHING_TO_UNDO`. Undo across commands works inside a REPL session, where one engine process stays alive.
- Speech commands accept caller-supplied segments/peaks; the engine does not perform transcription or waveform analysis itself, and the suite only validates parameter plumbing.
- `template apply` is intentionally unsupported until graphics templates can target the engine's sequence model.
- Hardware encoder selection/output is not verified by these CLI harness tests.
