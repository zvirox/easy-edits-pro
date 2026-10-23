---
name: cli-anything-eep
description: "Agent-friendly command line access to the Easy Edits Pro editing engine"
version: "0.1.0"
---

# Easy Edits Pro CLI

Use `cli-anything-eep --json ...` for machine-readable operations. The CLI calls the Rust `saturn-cli` backend; it does not simulate successful edits. Commands unavailable in the backend return a structured error.

## Setup

Build the Rust CLI from the repository root with `cargo build --bin saturn-cli`, then install the harness with `python -m pip install --editable ./agent-harness`. The backend is discovered through `SATURN_CLI`, `PATH`, or the workspace `target` directories.

## Supported workflow

```bash
cli-anything-eep --json project create --name "My Video" --path ./my-video.eep --fps 30
cli-anything-eep --json --project ./my-video.eep track add --kind video --name V1
cli-anything-eep --json --project ./my-video.eep track list
cli-anything-eep --json --project ./my-video.eep clip list
cli-anything-eep --json --project ./my-video.eep undo
cli-anything-eep --json --project ./my-video.eep redo
```

The interactive REPL accepts these commands without the executable prefix. It provides command/group tab completion and persistent readline history. Undo and redo are sent to the Rust engine and change the project itself.

## Available backend operations

Project create/open/save/inspect; timeline list/open/inspect; track add/remove/list and supported settings; clip add/move/remove/list; media import/search/list/bins/info; audio mix/automation; color reset/wheels; command catalogue/execution; render queue/status/cancel through the running desktop bridge; and engine-backed undo/redo.

The CLI does not expose every advanced engine parameter as a dedicated typed
option. Use `command list` and `command execute` for supported catalogue
operations, and expect a structured error for commands the backend cannot
perform. Graphics-template application, reverse playback, standalone audio
meters, and timeline options the project model does not represent remain
unsupported. Daemon lifecycle and MCP discovery are implemented; hardware
encoding depends on the render backend and machine.

Project state is remembered between CLI sessions. Pass `--project <file>` before the command group to explicitly select a project. Use `--json` before the command group for structured output and errors.
