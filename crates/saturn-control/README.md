# saturn-control

Control Server for Easy Edits Pro — external automation via TCP/Unix socket.

## Features

- **JSON-RPC 2.0** over TCP or Unix domain sockets
- **Command execution** — Execute any registered command
- **State queries** — Query timeline state, project metadata, media bins
- **Event subscriptions** — Real-time updates via Server-Sent Events
- **Authentication** — Optional token-based auth

## Usage

```bash
# Start control server on port 9090
cargo run -p saturn-control -- --control 9090

# Or with Unix socket
cargo run -p saturn-control -- --control /tmp/saturn-control.sock
```

## Protocol

### Execute Command
```json
{
  "jsonrpc": "2.0",
  "method": "command.execute",
  "params": {
    "command_id": "timeline.addTrack",
    "params": { "kind": "video" }
  },
  "id": 1
}
```

### Query State
```json
{
  "jsonrpc": "2.0",
  "method": "state.query",
  "params": {
    "path": "timeline.sequence.tracks"
  },
  "id": 2
}
```

### Subscribe to Events
```json
{
  "jsonrpc": "2.0",
  "method": "events.subscribe",
  "params": {
    "events": ["timeline.changed", "playback.started", "render.progress"]
  },
  "id": 3
}
```

## Integration Examples

- **IDE Extensions** — VS Code, Neovim, Zed
- **Stream Deck** — Custom actions via HTTP/TCP
- **OSC/MIDI** — Bridge via `osc2tcp` or similar
- **Custom Scripts** — Python, Node.js, shell

## Running Tests

```bash
cargo test -p saturn-control
```