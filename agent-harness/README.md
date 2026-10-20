# cli-anything-eep

Agent-friendly command line access to the Easy Edits Pro Rust editing engine. The harness communicates with `saturn-cli` through its JSON-lines control interface, starts `saturn-control` as a managed daemon, speaks MCP to `saturn-mcp`, and can also reach render jobs through the running desktop application's local socket bridge.

The harness is under active development. Commands that the engine cannot perform return a structured error instead of reporting a pretend success.

## Install

Requires Python 3.10+ and built Rust binaries. From the repository root:

```bash
cargo build --bin saturn-cli --bin saturn-mcp --bin saturn-control
python -m pip install --editable ./agent-harness
```

The harness locates binaries through `SATURN_CLI` / `SATURN_MCP` / `SATURN_CONTROL`, then `PATH`, then the repository's newest `target/debug` or `target/release` build. For a desktop render queue, Easy Edits Pro must be running.

## Examples

```bash
cli-anything-eep --help
cli-anything-eep --json version
cli-anything-eep project create --name "My Video" --path ./my-video.eep --fps 30
cli-anything-eep --project ./my-video.eep track add --kind video --name V1
cli-anything-eep --json --project ./my-video.eep track list
cli-anything-eep --json timeline split --track 1 --clip-index 0 --at 127008000000
cli-anything-eep --json clip add-effect --track 1 --clip-index 0 --kind gaussian_blur
cli-anything-eep --json daemon start --port 9090
cli-anything-eep --json mcp tools
cli-anything-eep repl
```

In the REPL, enter the same commands without the `cli-anything-eep` prefix. Tab completes command and group names; readline history is stored under `$XDG_STATE_HOME/easy-edits-pro/repl_history` (or `~/.local/state/easy-edits-pro/repl_history`).

## Command coverage

Every command id in the engine registry has a named CLI verb: `project`, `timeline`, `track`, `clip`, `title`, `media`, `subtitle`, `color`, `audio`, `multicam`, `nodegraph`, `interop`, `collab`, `speech`, `template`, `daemon`, `mcp`, `command`, plus `undo`/`redo`/`history`. Editing verbs take typed options where the engine has a stable schema and JSON options where the payload is a nested structure (color wheels, curves, masks, speed ramps, speech segments).

`cli-anything-eep command list` prints the registry catalogue and `command execute <id> --params '<json>'` is the escape hatch for any id.

## Engine ownership

Undo and redo call the engine's `edit.undo` / `edit.redo`; the harness keeps no second undo stack. `history` prints an append-only journal of the commands this CLI ran, stored as JSON lines in the XDG state directory, and says so in its output. Project state is restored from the last opened project.

`daemon start` launches `saturn-control` and records its pid, port, and log under `$XDG_STATE_HOME/easy-edits-pro/`; `daemon status` probes `/health`, and `daemon stop` stops only the process this CLI started. `mcp tools`, `mcp resources`, and `mcp prompts` perform a real MCP initialize handshake against `saturn-mcp`.

Graphics templates are real `.sgt` files written by `saturn-graphics`: `template create`, `list`, `inspect`, and `export` round-trip through the Rust parser. `template apply` is not supported and returns `COMMAND_NOT_FOUND`.

## Not supported

A few engine gaps remain and are reported honestly: applying a graphics template to a sequence, reverse playback, standalone audio meters, and timeline creation options the engine does not model. Hardware rendering depends on backend and machine capabilities and is not guaranteed by this harness.

Use `cli-anything-eep --json version` to inspect whether the backend is available. Use `--json` or `--format json` for machine-readable output; errors include an error code, message, details, and recoverability.

## Development and verification

```bash
cd agent-harness
python -m pytest -q
```

The repository CI installs the package in editable mode, builds `saturn-cli`, `saturn-mcp`, and `saturn-control`, and runs the harness tests, including real engine, control-server, and MCP integration tests. See [TEST.md](TEST.md) for the current result and known limits.

## License

GPL-3.0-or-later. See [LICENSE](../LICENSE).
