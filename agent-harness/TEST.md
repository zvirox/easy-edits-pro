# CLI Harness Verification

## Test command

Run from the repository root:

```bash
python -m pytest -q agent-harness/tests
```

The Rust integration tests use the workspace `saturn-cli` binary when available and skip when it is not built. Unit tests use a protocol-compatible fake backend for control-process behavior.

## Coverage

- Structured backend errors and JSON-lines request/response handling.
- Project CLI error mapping and persisted project state.
- REPL command dispatch and persistent readline history.
- Engine-delegated `undo`/`redo`, including a subprocess REPL round trip against the real `saturn-cli` binary.
- The CLI command journal (`history`), including capping, ordering, and corrupt-line recovery.
- Backend-backed clip listing and render cancellation errors.
- Rust control-process project/track/undo/redo integration.
- Every test runs against an isolated `XDG_STATE_HOME`, so a developer's saved project cannot leak into the suite.

## Latest local result

| Date | Result | Notes |
|------|--------|-------|
| 2026-10-20 | 52 passed | Ran `python -m pytest -q` from `agent-harness`; the suite includes a real `saturn-cli control` REPL round trip (`project create`, `track add`, `undo`, `redo`). |
| 2026-10-20 | 139 passed | Ran `cargo test --workspace`, now including `saturn-control`, `saturn-graphics`, and `saturn-project`. |

## Packaging and CI

`.github/workflows/rust.yml` includes a `python-cli-harness` job that installs the harness in editable mode, builds `saturn-cli`, and runs the pytest suite.

A local clean editable install could not be verified on this machine: Python's isolated build environment could not reach the package index to obtain `setuptools`, and the configured interpreter does not have `setuptools` for a no-build-isolation install. CI now exercises the clean install on GitHub Actions.

## Known test limits

- Render queue/status/cancel require the desktop application's local socket bridge and are not exercised by the standalone Rust control smoke test.
- Hardware encoder selection/output is not verified by these CLI harness tests.
- Undo/redo history lives in the engine process: a one-shot `undo` invocation with no prior edit in that process reports `NOTHING_TO_UNDO`. Undo across commands works inside a REPL session, where one engine process stays alive.
- Commands unsupported by the current Rust backend are expected to return structured errors; they are not counted as implemented functionality.
