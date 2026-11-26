# Easy Edits Pro — API & Protocol Reference

Easy Edits Pro exposes five coordinated control surfaces built over the shared `saturn-core::EditorEngine` command dispatcher (`498` shipped commands): Tauri Desktop IPC, Headless HTTP/WS Daemon (`saturn-control`), Unix Domain Sockets (`saturn-control` UDS, desktop `mcp_bridge`, and Starship Control `eep.sock`), Model Context Protocol (`saturn-mcp`), and the CLI (`saturn-cli` / `cli-anything-eep`).

---

## 1. Unified Command Engine (`CommandRequest`)

All editor state transitions share one typed JSON envelope (detailed per-ID schemas live in `docs/COMMANDS.md` and `docs/reference/command-catalogue.json`):

```json
{
  "id": "timeline.add_clip",
  "params": {
    "trackId": "v1",
    "mediaId": 1,
    "timelineStartTicks": 0
  }
}
```

- **Command Catalogue Metadata:** Each `CommandSpec` specifies `id`, `category`, `mutating`, `ui_only`, `shipped`, permission `tiers` (`Read`, `Edit`, `Destructive`, `Filesystem`), and canonical `profiles` (`core`, `ingest`, `rough-cut`, `fine-cut`, `colour`, `audio`, `speed-motion`, `graphics`, `review`, `export`, etc.).
- **CLI JSON Lines (`saturn-cli control`):** Reads one `CommandRequest` JSON object per stdin line and emits `{"ok": true, "result": ...}` or `{"ok": false, "error": "..."}` to stdout. See `docs/control-protocol.md`.

---

## 2. Tauri Desktop IPC Commands (`invoke(...)`)

Registered in `apps/saturn-tauri/src/main.rs` (`tauri::generate_handler!`):

| Domain | Registered IPC Commands |
| :--- | :--- |
| **Core Dispatch & Security** | `execute_command`, `register_write_grant`, `command_status`, `get_command_catalogue` |
| **Recovery & Diagnostics** | `recovery_status`, `restore_autosave`, `discard_autosave`, `get_system_diagnostics`, `glass_environment`, `save_glass_report` |
| **Media & Perception** | `import_media`, `check_media_availability`, `probe_media_for_update`, `prepare_video_preview`, `get_media_server_port`, `get_media_server_token`, `media_semantic_index`, `media_detect_shots`, `generate_synthetic_media`, `import_network_media` |
| **Openverse & Local AI** | `search_openverse`, `download_openverse_asset`, `ask_local_ai`, `run_background_matting` |
| **Audio & Speech** | `measure_ebu_r128_loudness`, `sync_multicam_by_waveform`, `transcribe_audio_speech`, `align_script_to_takes`, `detect_timeline_silence`, `ripple_delete_silence`, `ripple_delete_fillers` |
| **Color, Preview & FX** | `load_lut_cube_file`, `get_lut_preset_cube`, `analyze_motion_tracking`, `synthesize_speed_ramp_frames`, `get_gpu_preview_metrics`, `present_gpu_preview_frame` |
| **Subtitles, Markers & Interop** | `import_subtitles_file`, `export_subtitles_file`, `read_text_file`, `read_markers_file`, `write_markers_file`, `export_timeline_interop`, `import_timeline_interop`, `reveal_path` |
| **Export & Job Queue** | `start_export`, `detect_encoder_capabilities`, `export_frame`, `queue_export`, `get_active_jobs`, `get_all_jobs`, `cancel_job`, `cancel_all_jobs`, `clear_job_history`, `retry_export`, `move_queued_export`, `remove_queued_export`, `pause_job_queue`, `resume_job_queue`, `is_job_queue_paused` |
| **Agent Session Supervision** | `agent_session_stop`, `agent_session_resume`, `agent_session_status`, `agent_undo_change` |

**Security note:** `register_write_grant` is callable only after a native Tauri OS file dialog choice and is never exposed over `mcp_bridge`.

---

## 3. Headless Daemon REST & WebSocket API (`saturn-control`)

Mounted in `crates/saturn-control/src/server.rs` and `crates/saturn-control/src/api/mod.rs` with `origin_guard`, bearer auth (`Authorization: Bearer <token>` or `x-auth-token`), sliding-window rate limiting, session time budgets, AF2 Stop fencing, and SHA-256 argument audit logging. Most `/api/v1/*` routes are dual-mounted at root and under `/api/v1`.

### 3.1 Core & Command Endpoints

| Method | Path | Purpose |
| :--- | :--- | :--- |
| `GET` | `/health` | Unauthenticated liveness probe (deliberately omits command/session inventory) |
| `POST` | `/auth/token` | Issue scoped bearer session token from master secret or claims |
| `POST` | `/project/open`, `/project/save`, `/project/close` | Manage active headless `.saturn` document |
| `GET` | `/project/status` | Inspect active project path, dirty flag, and schema status |
| `POST` | `/session/stop`, `/session/resume` | Trigger or release generation-fenced AF2 emergency Stop |
| `GET` | `/session/status` | Query Stop fence, rate limit, and remaining time budget |
| `GET` | `/commands` | List catalogue commands visible to authenticated session |
| `POST` | `/commands/:id/execute` | Execute catalogue command through `EditorEngine` |
| `GET` | `/ws` | Real-time WebSocket stream broadcasting `AppEvent`, tasksheet, question, review, verify, and stop/resume events |

### 3.2 Task Sheets, Questions, Review, Verification & Runner (`TS7`–`TS10`)

| Group | Endpoints (mounted at root & `/api/v1`) |
| :--- | :--- |
| **Task Sheet (`tasksheet.rs`)** | `GET /tasksheet`, `POST /tasksheet`, `POST /tasksheet/submit`, `POST /tasksheet/plan/submit`, `POST /tasksheet/plan/approve`, `POST /tasksheet/plan/reject`, `GET /tasksheet/report`, `POST /tasksheet/report`, `POST /tasksheet/run_report` |
| **Questions (`questions.rs`)** | `GET /tasksheet/questions`, `POST /tasksheet/questions`, `POST /tasksheet/questions/:id/answer` |
| **Review Rounds (`review.rs`)** | `GET /review/notes`, `POST /review/notes`, `POST /review/notes/:id/resolve`, `GET /review/changes`, `POST /review/round/open`, `POST /review/round/close` |
| **Verify & Delivery (`verify.rs`)** | `POST /verify/run`, `POST /delivery/verify`, `POST /delivery/approve` |
| **Runner (`runner.rs`)** | `GET /runner/status`, `POST /runner/start`, `POST /runner/stop`, `GET /runner/jobs`, `POST /runner/jobs/:id/cancel` |
| **Enterprise SSO (`sso.rs`)** | `GET /auth/sso/status`, `GET /auth/sso/roles`, `GET\|POST /auth/sso/oidc/login`, `POST /auth/sso/oidc/callback`, `GET\|POST /auth/sso/saml/login`, `POST /auth/sso/saml/callback` |

---

## 4. Unix Domain Socket (UDS) & Starship Bridge Protocols

1. **`saturn-control` UDS (`uds.rs`):**
   - `0600` socket accepting newline-delimited JSON (`DEFAULT_MAX_UDS_LINE_BYTES = 1 MiB`), authenticating via session bearer token and releasing locks / aborting pending work automatically on EOF disconnect.
2. **Desktop Live MCP Bridge (`apps/saturn-tauri/src/mcp_bridge.rs`):**
   - Binds `saturn_core::local_control::mcp_socket_path()` (`0700` dir / `0600` socket), enforces `SessionPolicy` tiers (`Read`, `Edit`, `Destructive`, `Filesystem`) and active profiles, and shares project write-root containment (`SATURN_ALLOW_ANY_WRITE_TARGETS=1` only for test harnesses).
3. **Starship Control Plugin Bridge (`crates/saturn-mcp/src/starship_bridge.rs`):**
   - Publishes discovery metadata (`schema_version: "1.0"`, `app_id: "eep"`) to `$XDG_DATA_HOME/starship/plugins/eep.json`.
   - Listens on `$XDG_RUNTIME_DIR/starship/bridges/eep.sock` with kernel `SO_PEERCRED` verification via `starship-plugin` SDK, exposing Tier 2 desktop-orchestrated tools including `eep.timeline.cut_interval`.

---

## 5. Model Context Protocol (`saturn-mcp`)

Run via `saturn-mcp [--project <path> | --bridge] [--profile <name>] [--mode <autopilot|coedit|review|manual>] [--grant-tiers <tiers>]`:

| Tool Group | Registered MCP Tools (`crates/saturn-mcp/src/server_tools.rs`) |
| :--- | :--- |
| **Profiles & Session** | `tools.list_profiles`, `tools.load_profile`, `session.stop`, `session.resume`, `session.status`, `history.undo_change` |
| **Commands & Inspection** | `command_list`, `command_run`, `command_batch`, `doc_inspect`, `sequence_inspect`, `media_search`, `media_preview` |
| **Perception & Continuity** | `vision_extract_frame_sheet`, `media_semantic_query`, `capture_read_events`, `edit_evaluate_cut_continuity`, `verify.run` |
| **Audio, Color & Cut Prep** | `color_read_scopes`, `color_match_shot_to_reference`, `edit_match_cut_align`, `timeline_beat_grid_quantize`, `audio_process`, `audio_read_telemetry` |
| **Render & Proxy** | `render_preview_clip_proxy`, `render_start`, `render_status`, `render_cancel`, `render_project` |

---

*Authored by Zvirox · Easy Edits Pro.*
