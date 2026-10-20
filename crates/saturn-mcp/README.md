# saturn-mcp

MCP (Model Context Protocol) server for Easy Edits Pro — AI-assisted video editing.

## Features

- **Tools** — AI-callable editing operations
- **Resources** — Timeline state, project metadata, media bins
- **Prompts** — Guided workflows for common editing tasks
- **Standard MCP** — Compatible with Claude, Cursor, Continue, etc.

## Tools

| Tool | Description |
|------|-------------|
| `saturn.analyzeTimeline` | Analyze structure, detect gaps/overlaps, suggest optimizations |
| `saturn.suggestEdit` | Suggest edits based on context, selection, playhead |
| `saturn.batchOperation` | Execute multiple commands atomically with single undo |
| `saturn.searchMedia` | Natural language media search (e.g., "close up person talking") |
| `saturn.createTemplate` | Create project/graphics/sequence/bin templates from spec |

## Resources

```
saturn://project/{id}/timeline     — Current timeline state
saturn://project/{id}/media        — Media bin contents
saturn://project/{id}/sequences    — All sequences
saturn://project/{id}/tracks       — Track layout
```

## Prompts

- `editing-workflow` — Guided edit session from rough cut to final
- `color-grading` — Step-by-step color correction workflow
- `audio-mixing` — Fairlight console mixing guide
- `multi-cam-sync` — Multi-camera synchronization workflow

## Usage

```bash
# Start MCP server (stdio transport for Claude Desktop)
cargo run -p saturn-mcp

# Or with custom transport
cargo run -p saturn-mcp -- --transport tcp --port 3000
```

## Configuration (Claude Desktop)

```json
{
  "mcpServers": {
    "easy-edits-pro": {
      "command": "saturn-mcp",
      "args": []
    }
  }
}
```

## Example AI Interaction

**User:** "Add a cross dissolve between the two clips on track V1"

**AI:** Calls `saturn.suggestEdit` with context, returns:
```json
{
  "command": "timeline.addTransition",
  "params": {
    "track_id": "V1",
    "clip_id": "clip_2",
    "transition": "cross_dissolve",
    "duration": 1.0
  }
}
```

## Running Tests

```bash
cargo test -p saturn-mcp
```