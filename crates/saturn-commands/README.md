# saturn-commands

Unified command registry for Easy Edits Pro — single source of truth for Tauri IPC, CLI, MCP server, and Control Server automation.

## Features

- **CommandSpec** — Full command specification with JSON Schema parameter validation
- **CommandRegistry** — Categories, priorities, execution modes (Sync/Async/Background)
- **ShortcutManager** — Global keybindings with modifier support (Ctrl/Cmd, Shift, Alt, Meta)
- **UndoManager** — Transactional undo/redo with grouped operations and checkpoints
- **Parameter System** — Rich parameter types (String, Integer, Float, Boolean, Enum, Path, Media, Track, Timecode, Color, KeyframeEasing)

## Usage

```rust
use saturn_commands::{
    CommandRegistry, CommandSpec, CommandId, CommandCategory, CommandPriority, ExecutionMode,
    params::{ParamSchema, ParamValidator}, shortcuts::ShortcutManager, undo::UndoManager,
};

// Create registry
let mut registry = CommandRegistry::new();

// Register a command
let spec = CommandSpec::new(CommandId::new("timeline.addTrack"), "Add Track")
    .category(CommandCategory::Timeline)
    .priority(CommandPriority::Normal)
    .execution_mode(ExecutionMode::Sync)
    .param(ParamSchema::Track { kind: None })
    .handler(|ctx, params| {
        // Implementation
        Ok(CommandResult::success())
    });

registry.register(spec);

// Execute via IPC/CLI/MCP/Control
let result = registry.execute(&CommandId::new("timeline.addTrack"), params, context);
```

## Running Tests

```bash
cargo test -p saturn-commands
```

## Architecture

- **No framework coupling** — Pure Rust, works with Tauri, CLI, MCP, TCP server
- **Single command definition** — Register once, execute everywhere
- **Parameter validation** — JSON Schema with custom validators
- **Undo/Redo integration** — Commands can declare undo operations