# TEST.md — Easy Edits Pro CLI Test Plan & Results

## Test Strategy

| Layer | Tool | Coverage Target |
|-------|------|-----------------|
| Unit | pytest + mocks | ≥90% |
| E2E | pytest + real saturn-cli | ≥80% critical paths |
| CLI Subprocess | Installed command verification | 100% commands |

---

## Unit Tests (`tests/unit/`)

### `test_state.py`
- [ ] `ProjectState` load/save roundtrip
- [ ] `ProjectState` recent projects tracking
- [ ] Global state singleton behavior

### `test_output.py`
- [ ] JSON output valid structure
- [ ] Text output formatting
- [ ] Table output with various data shapes
- [ ] Error output formatting
- [ ] Progress indicator

### `test_errors.py`
- [ ] `EepError` to_dict() structure
- [ ] Error code classification
- [ ] Specific error types (ProjectNotOpenError, etc.)
- [ ] Validation error creation

### `test_undo.py`
- [ ] Single operation undo/redo
- [ ] Transaction group undo/redo
- [ ] History limits
- [ ] Clear operation

### `test_commands_unit.py` (per command group)
- [ ] Parameter validation
- [ ] Help text generation
- [ ] Output format switching

---

## E2E Tests (`tests/e2e/`)

### `test_project_workflow.py`
- [ ] Create project → save → open → inspect
- [ ] Template selection (youtube-1080p, cinema-4k, vertical-9x16)
- [ ] Project with custom settings
- [ ] Multiple projects in sequence

### `test_timeline_editing.py`
- [ ] Create timeline → add tracks → add clips → trim → move
- [ ] Multi-track editing (V1, V2, A1, A2)
- [ ] Clip speed/reverse
- [ ] Nested sequences

### `test_render_pipeline.py`
- [ ] Queue render with presets (youtube, tiktok, cinema-4k)
- [ ] Render status polling
- [ ] Cancel render
- [ ] Output file validation

### `test_color_audio.py`
- [ ] Color grade Lift/Gamma/Gain/Offset
- [ ] LUT application
- [ ] Audio mix volume/pan
- [ ] Audio effects chain
- [ ] Automation modes

### `test_media_management.py`
- [ ] Media import (file + directory)
- [ ] Natural language search
- [ ] Proxy generation
- [ ] Bin organization

### `test_templates.py`
- [ ] Template create from spec
- [ ] Template apply with params
- [ ] .sgt export/import

### `test_mcp_integration.py`
- [ ] MCP server start/stop
- [ ] Tool discovery
- [ ] Resource access

### `test_repl.py`
- [ ] REPL command execution
- [ ] History persistence
- [ ] Undo/redo in REPL

---

## CLI Subprocess Tests (`tests/test_cli_subprocess.py`)

- [ ] `pip install -e .` succeeds
- [ ] `cli-anything-eep --help` works
- [ ] `cli-anything-eep --version` works
- [ ] All commands return valid JSON with `--json`
- [ ] Error codes map correctly
- [ ] REPL entry works

---

## Fixtures (`tests/fixtures/`)

| Fixture | Description |
|---------|-------------|
| `sample_project.eep` | Minimal valid project |
| `sample_timeline.json` | Timeline with tracks/clips |
| `sample_footage.mp4` | 1-frame test video (100KB) |
| `sample_audio.wav` | 1-second test audio |
| `sample_lut.cube` | 17x17x17 identity LUT |
| `sample_template.sgt` | Minimal graphics template |

---

## Running Tests

```bash
# Unit tests only (fast)
pytest tests/unit -v

# E2E tests (requires saturn-cli)
pytest tests/e2e -v

# All tests with coverage
pytest --cov=cli_anything_eep --cov-report=html

# Specific test file
pytest tests/unit/test_undo.py -v

# CLI subprocess verification
pytest tests/test_cli_subprocess.py -v
```

---

## CI Configuration

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Build saturn-cli
        run: cargo build --release --bin saturn-cli
      - name: Install CLI
        run: pip install -e ./agent-harness
      - name: Unit tests
        run: pytest tests/unit -v
      - name: E2E tests
        run: pytest tests/e2e -v
      - name: CLI verification
        run: pytest tests/test_cli_subprocess.py -v
```

---

## Test Results Log

| Date | Commit | Unit | E2E | CLI Subprocess | Notes |
|------|--------|------|-----|----------------|-------|
| | | | | | |

---

## Performance Baselines

| Operation | Target | Actual |
|-----------|--------|--------|
| Project create | <500ms | |
| Timeline create | <200ms | |
| Clip add | <100ms | |
| Render queue | <1s | |
| Color grade | <50ms | |
| REPL startup | <1s | |