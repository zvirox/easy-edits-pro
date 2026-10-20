---
name: cli-anything-eep
description: "Agent-native CLI for Easy Edits Pro — professional video editing on Linux"
version: "0.1.0"
---

# cli-anything-eep — Easy Edits Pro CLI

Agent-native CLI for Easy Edits Pro video editor. Provides structured command access to all editing capabilities.

## Installation

```bash
pip install cli-anything-eep
# or from source:
cd agent-harness && pip install -e .
```

## Quick Start

```bash
# Create project
cli-anything-eep project create --name "My Video" --template youtube-1080p

# Create timeline
cli-anything-eep timeline create --name "Main" --fps 30 --width 1920 --height 1080

# Add tracks
cli-anything-eep track add --kind video --name "V1"
cli-anything-eep track add --kind audio --name "A1"

# Add clips
cli-anything-eep clip add --source footage.mp4 --track V1 --at 0

# Color grade
cli-anything-eep color grade --clip clip_1 --lift 0.1 0.1 0.1 --gain 1.0 0.9 0.9

# Render
cli-anything-eep render queue --preset youtube --output output.mp4
```

## Command Reference

### Project Management
| Command | Description |
|---------|-------------|
| `project create --name <name> [--template]` | Create new project |
| `project open <path>` | Open existing project |
| `project save [--path]` | Save current project |
| `project close` | Close current project |
| `project inspect <path>` | Inspect project metadata |
| `project templates` | List available templates |

### Timeline Management
| Command | Description |
|---------|-------------|
| `timeline create --name <name> [--fps] [--width] [--height]` | Create timeline |
| `timeline list` | List all timelines |
| `timeline open --id <id>` | Open/select timeline |
| `timeline settings --id <id> [--fps] [--width] [--height]` | Update settings |

### Track Management
| Command | Description |
|---------|-------------|
| `track add --kind <video|audio|subtitle|submix> [--name] [--index]` | Add track |
| `track remove --id <id>` | Remove track |
| `track list` | List tracks |
| `track move --id <id> --index <n>` | Move track |
| `track settings --id <id> [--name] [--volume] [--pan] [--muted] [--solo]` | Update settings |

### Clip Management
| Command | Description |
|---------|-------------|
| `clip add --source <path> --track <id> --at <ticks> [--duration]` | Add clip |
| `clip move --id <id> --track <id> --at <ticks>` | Move clip |
| `clip trim --id <id> --in <ticks> --out <ticks>` | Trim clip |
| `clip remove --id <id>` | Remove clip |
| `clip speed --id <id> --speed <float>` | Change speed |
| `clip reverse --id <id> [--enable/--disable]` | Toggle reverse |

### Rendering
| Command | Description |
|---------|-------------|
| `render queue --preset <name> --output <path> [--timeline]` | Queue render |
| `render status` | Show queue status |
| `render cancel --job-id <id>` | Cancel job |
| `render presets` | List presets |

### Color Grading
| Command | Description |
|---------|-------------|
| `color grade --clip <id> [--lift] [--gamma] [--gain] [--offset] [--contrast] [--saturation] [--temperature] [--tint]` | Apply grade |
| `color reset --clip <id>` | Reset grade |
| `color lut --clip <id> --file <.cube> [--intensity]` | Apply LUT |
| `color wheels --clip <id>` | Show wheel values |

### Audio Mixing
| Command | Description |
|---------|-------------|
| `audio mix --track <id> [--volume] [--pan] [--mute] [--solo]` | Adjust mix |
| `audio effect --clip <id> --name <name> [--params] [--order]` | Add effect |
| `audio meter --track <id>` | Show levels |
| `audio automation --track <id> --param <volume|pan|mute|solo> --mode <read|touch|latch|write>` | Set automation |

### Media Management
| Command | Description |
|---------|-------------|
| `media import --path <path> [--bin] [--recursive] [--proxy]` | Import media |
| `media search --query <text> [--bin] [--limit]` | Natural language search |
| `media list [--bin] [--type]` | List media |
| `media bins` | List bins |
| `media proxy --media-id <id> [--generate/--delete] [--quality]` | Manage proxies |
| `media info --media-id <id>` | Show media info |

### Templates (.sgt)
| Command | Description |
|---------|-------------|
| `template create --name <name> --type <project|graphics|sequence|bin> --spec <json|path>` | Create template |
| `template list [--type]` | List templates |
| `template apply --id <id> [--params]` | Apply template |
| `template export --id <id> --output <path>` | Export .sgt |

### Command Registry
| Command | Description |
|---------|-------------|
| `command list [--category]` | List commands |
| `command execute --id <id> [--params]` | Execute command |
| `command info --id <id>` | Command details |
| `command categories` | List categories |

### Control Server
| Command | Description |
|---------|-------------|
| `daemon start [--port] [--host]` | Start control server |
| `daemon stop` | Stop server |
| `daemon status` | Show status |

### MCP Server
| Command | Description |
|---------|-------------|
| `mcp start [--transport stdio|tcp] [--port]` | Start MCP server |
| `mcp stop` | Stop server |
| `mcp tools` | List tools |
| `mcp resources` | List resources |
| `mcp prompts` | List prompts |

### Utility
| Command | Description |
|---------|-------------|
| `repl` | Enter interactive REPL |
| `undo` | Undo last operation |
| `redo` | Redo last operation |
| `history` | Show undo history |
| `version` | Show version info |

## Output Formats

All commands support:
- `--json` — Structured JSON output (for agents)
- `--format text` — Human-readable (default)
- `--format table` — Tabular data

## Error Format

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable description",
  "details": {},
  "recoverable": true
}
```

## Agent Guidance

- Use `--json` for all automated workflows
- Commands are composable: `project create` → `timeline create` → `track add` → `clip add` → `render queue`
- Use `undo`/`redo` for safe experimentation
- `repl` mode provides interactive session with history
- `history` shows full transaction log

## Backend

Requires `saturn-cli` (Easy Edits Pro) on PATH. The CLI wraps the Rust crates via subprocess with JSON I/O.

## License

Apache-2.0