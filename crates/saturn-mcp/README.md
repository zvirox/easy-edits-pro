# saturn-mcp

Model Context Protocol stdio server for Easy Edits Pro. The server calls
`saturn-core::EditorEngine` directly, so the exposed command list and edits use
the same Rust command dispatcher as the desktop app and CLI.

## Available tools

| Tool | Purpose |
|---|---|
| `command_list` | Discover engine command ids, labels, menu paths, shortcuts, and current enabled state. Supports a text filter. |
| `command_run` | Run one engine command with its JSON parameters. |
| `command_batch` | Run several commands in order, with optional stop-on-error behavior. Each successful command is applied separately. |
| `doc_inspect` | Read the project document, active sequence, current path, and dirty state. |
| `sequence_inspect` | Read the active sequence and its tracks/clips. |
| `media_search` | Search imported media by name/path and optional type. |
| `media_preview` | Inspect one asset and its timeline placements; returns the source path for client preview. |
| `render_start` | Start an asynchronous MP4/WebM render and return its job id. |
| `render_status` | Read queued/running progress and terminal render state. |
| `render_cancel` | Cancel a queued or active render and remove partial output. |
| `render_project` | Compatibility export tool; use the job tools for progress/cancellation. |

The bridge workflow has been smoke-tested against the running desktop app with
video and audio media: MCP added both to a sequence, split the video clip,
updated track EQ/compression, saved an `.eep` project, and rendered an H.264/AAC
MP4. A native desktop workflow also exercised media search/placement preview,
Color, Effects, Keyframes, Fusion node creation, Fairlight automation/DSP state,
transition rendering, render cancellation, and an increasing progress report on
a 30-second render. This machine used the x264 software encoder. `--bridge`
connects to the project open in the desktop app; commands share
the app's undo history and the UI refreshes after MCP edits. Bridge requests are
local and limited to 1 MiB. Render job progress/cancellation and project media
search/asset inspection are implemented. `media_preview` returns asset metadata,
the source path, and sequence placements; it does not yet extract a still frame.
The desktop bridge currently uses a private Unix socket and is unavailable on
Windows. UI automation and MCP resources/prompts remain future work. See the
[delivery plan](../../docs/planning.md) for the next MCP milestones and
verification rules.

## Run

```bash
# Start with a new, unsaved project
cargo run -p saturn-mcp

# Start with an existing Easy Edits Pro project file
cargo run -p saturn-mcp -- --project /path/to/project.eep

# Connect to the project in the running desktop app (Unix desktop)
cargo run -p saturn-mcp -- --bridge
```

The server uses MCP stdio transport. Configure the MCP client with the executable
and optional arguments, for example:

```json
{
  "mcpServers": {
    "easy-edits-pro": {
      "command": "cargo",
      "args": ["run", "--quiet", "-p", "saturn-mcp", "--", "--bridge"]
    }
  }
}
```

Use `command_list` first to retrieve valid command ids and their menu metadata;
pass the chosen id and command-specific parameters to `command_run`. Use
`media_search` and `media_preview` to locate assets and inspect their timeline
placements. Start exports with `render_start`, then poll `render_status` using
the returned `job_id`; call `render_cancel` to stop a queued or running export.
Pass an absolute `.mp4` or `.webm` `output_path`; settings default to the project
sequence. To export a selected clip or section, pass
`range: {"start_ticks": ..., "end_ticks": ...}` in project ticks, or use
sequence In/Out marks with `use_mark_range: true`.
