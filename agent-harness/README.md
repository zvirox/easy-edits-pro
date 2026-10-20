# cli-anything-eep

Agent-friendly command line access to the Easy Edits Pro Rust editing engine. The harness communicates with `saturn-cli` through its JSON-lines control interface and can also reach render jobs through the running desktop application's local socket bridge.

The harness is under active development. Commands that the Rust engine does not support return a structured error instead of reporting a pretend success.

## Install

Requires Python 3.10+ and a built `saturn-cli` binary. From the repository root:

```bash
cargo build --bin saturn-cli
python -m pip install --editable ./agent-harness
```

The harness locates the backend through `SATURN_CLI`, `PATH`, or the repository's `target/debug` and `target/release` directories. For a desktop render queue, Easy Edits Pro must be running.

## Examples

```bash
cli-anything-eep --help
cli-anything-eep --json version
cli-anything-eep project create --name "My Video" --path ./my-video.eep --fps 30
cli-anything-eep --project ./my-video.eep track add --kind video --name V1
cli-anything-eep --json --project ./my-video.eep track list
cli-anything-eep repl
```

In the REPL, enter the same commands without the `cli-anything-eep` prefix. Tab completes command and group names; readline history is stored under `$XDG_STATE_HOME/easy-edits-pro/repl_history` (or `~/.local/state/easy-edits-pro/repl_history`).

## Implemented command areas

Project create/open/save/inspect; timeline list/open/inspect; track add/remove/list and supported settings; clip add/move/remove/list; media import/search/list/bins/info; audio mix and automation; color reset and wheels; command catalogue and command execution; render queue/status/cancel through the desktop bridge; and engine-backed undo/redo.

Undo and redo call the Rust engine and therefore affect the actual project. The Python harness keeps bounded metadata to group related engine snapshots. Project state is restored from the last opened project and recorded in the user's XDG state directory.

## Not yet implemented by the backend

The current Rust command engine does not expose timeline creation/settings, clip trim/speed/reverse, track reordering, full color grading/LUT, audio effects/meters, template persistence/application, or a standalone daemon/MCP lifecycle. These CLI commands return a structured unsupported-command error. Hardware rendering depends on backend and machine capabilities and is not guaranteed by this harness.

Use `cli-anything-eep --json version` to inspect whether the backend is available. Use `--json` or `--format json` for machine-readable output; errors include an error code, message, details, and recoverability.

## Development and verification

```bash
cd agent-harness
python -m pytest -q
```

The repository CI installs the package in editable mode, builds `saturn-cli`, and runs the harness tests. A clean local editable-install check may require access to the Python package index for build dependencies.

## License

Apache-2.0. See [LICENSE](../LICENSE).
