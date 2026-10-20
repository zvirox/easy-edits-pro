# CLI Harness Verification

## Test command

Run from the repository root:

```bash
python -m pytest -q agent-harness/tests
```

The Rust integration test uses the workspace `saturn-cli` binary when available and skips when it is not built. Unit tests use a protocol-compatible fake backend for control-process behavior.

## Coverage

- Structured backend errors and JSON-lines request/response handling.
- Project CLI error mapping and persisted project state.
- REPL command dispatch and persistent readline history.
- Real engine-backed undo/redo, including grouped operations.
- Backend-backed clip listing and render cancellation errors.
- Rust control-process project/track/undo/redo integration.
- Existing CLI subprocess, output formatting, error, and undo-manager behavior.

## Latest local result

| Date | Result | Notes |
|------|--------|-------|
| 2026-10-20 | 45 passed (previous run) | Ran `python -m pytest -q` from `agent-harness`, including a real `saturn-cli control` smoke run for project creation, track add/list, undo, and redo. Final review then added option validation and failure-safe grouped undo; rerun the suite before release. |

## Packaging and CI

`.github/workflows/rust.yml` includes a `python-cli-harness` job that installs the harness in editable mode, builds `saturn-cli`, and runs the pytest suite.

A local clean editable install could not be verified on this machine: Python's isolated build environment could not reach the package index to obtain `setuptools`, and the configured interpreter does not have `setuptools` for a no-build-isolation install. CI now exercises the clean install on GitHub Actions.

## Known test limits

- Render queue/status/cancel require the desktop application's local socket bridge and are not exercised by the standalone Rust control smoke test.
- Hardware encoder selection/output is not verified by these CLI harness tests.
- Commands unsupported by the current Rust backend are expected to return structured errors; they are not counted as implemented functionality.
