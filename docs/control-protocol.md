# Easy Edits Pro command interface

The desktop app and `saturn-cli` use the same command IDs and `saturn-core::EditorEngine` dispatcher. UI actions, keyboard shortcuts, CLI operations, and automation should submit a `CommandRequest` instead of changing project state directly.

## JSON Lines control mode

Run `cargo run -p saturn-cli -- control`. The process reads one JSON object per input line and writes one JSON response per line. It keeps one in-memory project session for the lifetime of the process.

Request:

```json
{"id":"project.add_media","params":{"name":"shot.mp4","path":"/media/shot.mp4","kind":"video"}}
```

Success:

```json
{"ok":true,"result":{"media_id":3}}
```

Failure:

```json
{"ok":false,"error":"Command `edit.undo` is disabled: Nothing to undo"}
```

Useful commands include `project.new`, `file.open`, `file.save`, `project.add_media`, `project.update_media`, `timeline.add_clip`, `timeline.move_clip`, `timeline.remove_clip`, `clip.set_color`, `track.set_gain_db`, `clip.set_keyframe`, `clip.remove_keyframe`, `edit.undo`, `edit.redo`, `project.inspect`, and `media.list`. `saturn-cli commands` prints IDs, labels, and shortcuts. The core exposes `command_status()` so clients can query whether Undo and Redo are enabled and why.

## Project files

Desktop projects use the `.saturn` filename extension and the
`easy-edits-pro.project` JSON format (schema version 14). The loader migrates
supported older schemas and accepts the legacy `project-saturn.project` format.
Media is referenced by path rather than copied into the project. Saves write
and sync a temporary sibling file before atomically renaming it into place.

## Current limits

This document describes the `saturn-cli control` JSON Lines interface. It is
distinct from `saturn-control` (HTTP/WebSocket), `saturn-mcp` (MCP stdio and an
optional running-desktop bridge), and the Tauri invoke bridge. These adapters
share the Rust command engine. CLI media import performs extension-based
classification; the desktop app uses GStreamer metadata discovery.
