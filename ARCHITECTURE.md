# Easy Edits Pro — Architecture

**Overview:** Easy Edits Pro (EEP) separates thin reactive UI rendering (React 19 + Tauri 2 Webview) from deterministic Rust timeline state (`saturn-core`), GStreamer/`wgpu` media pipelines (`saturn-render`), on-device ML perception (`saturn-perception`), isolated audio plugin hosting (`saturn-plugin-worker`), and authenticated agent/team daemons (`saturn-mcp`, `saturn-control`).

---

## 1. Technology Stack

| Layer | Technology | Responsibility |
| :--- | :--- | :--- |
| **Desktop Shell** | **Tauri 2** (`apps/saturn-tauri`) | Frameless `1280×800` (`min 1024×640`) native Linux window, IPC bridge, file dialogs, autosave, DMA-BUF WebKit config |
| **Frontend UI** | **React 19 + TypeScript + Vite 8** | Docked workspace shell, menu/shortcut dispatch, timeline & inspector controls (`ui/src/`) |
| **Styling System** | **Tailwind CSS v4 + Self-Hosted Inter** | Locked Linear matte solid palette (`linear-theme.css`, `app-accent.css`, `linear-layout.css`), zero blur |
| **Scopes & Viewport** | **HTML5 Canvas (`VideoScopes.tsx`) + CSS** | Live waveform, vectorscope, RGB parade, histogram, quad, and raw video preview surfaces |
| **Core Engine** | **Rust (`saturn-core`)** | Schema v19 project model, rational ticks, CoW `TimelineBranch`, undo/redo history, 498-command catalogue |
| **Media & Render** | **GStreamer + `wgpu` (`saturn-render`)** | Decoding, frame extraction, hardware encoder probing (`nvenc`/`vaapi`/`videotoolbox`/`qsv`), MP4/WebM export |
| **Perception / ML** | **ONNX Runtime (`ort`) + `whisper-rs` (`saturn-perception`)** | Offline Whisper speech, SigLIP embeddings, MODNet matting, Depth-Anything-v2-Small, SSD+RANSAC tracking |
| **Audio DSP Worker** | **Out-of-process Host (`saturn-plugin-worker`)** | Crash-isolated CLAP host + internal loudness/dynamics DSP (`saturn-audio-dsp`) |
| **Control & MCP** | **`axum` + `rmcp` + `starship-plugin`** | Headless daemon (`saturn-control`), MCP stdio/bridge (`saturn-mcp`), Starship Control `eep.sock` UDS integration |
| **Database / State** | **SQLite (`rusqlite`) + Atomic JSON** | Semantic index sidecar (`index_db.rs`), `.saturn` project documents, bounded JSONL audit logs |

---

## 2. System Topology & Process Boundaries

```text
+------------------------------------------------------------------------------------------------+
|                         FRONTEND SHELL (React 19 · Tauri Webview / Web Desk)                    |
|                                                                                                |
|  TitleBar (40px, exclusive ThemeToggle) · Sidebar (240/56px) · Breadcrumb · WorkspaceShell      |
|  Workspaces: Welcome | Media | Edit | Fusion | Color | Fairlight | Effects | Keyframes | Deliver|
|  Web Desk (?mode=desk): JobsBoard | NewJobModal | JobDetailView | WebReviewView                 |
+--------------------+---------------------------------------------------+-----------------------+
                     | Tauri invoke(...) / Channel                       | HTTP/WS (Web Desk)
                     v                                                   v
+--------------------------------------------------------+  +------------------------------------+
|       DESKTOP HOST (apps/saturn-tauri/src/main.rs)     |  |    HEADLESS DAEMON (saturn-control)|
|                                                        |  |                                    |
|  - WriteGrants allowlist & dialog consent enforcement  |  |  - Axum REST + /ws + UDS JSONL     |
|  - media_server.rs (token-authenticated HTTP streams)  |  |  - AuthManager bearer tokens       |
|  - net.rs (DNS pinning, SSRF / private-IP guard)       |  |  - OIDC / SAML 2.0 SSO & RBAC      |
|  - recovery.rs (autosave loop) & gpu_detect.rs         |  |  - RunnerService (leases/budgets)  |
|  - mcp_bridge.rs (local UDS for --bridge)              |  |  - AuditLogger (sha256 args)       |
+--------------------+-----------------------------------+  +-----------------+------------------+
                     | Mutex + catch_unwind + AppState                        |
                     v                                                        v
+------------------------------------------------------------------------------------------------+
|                          RUST DOMAIN CORE (crates/saturn-core & saturn-commands)               |
|                                                                                                |
|  - Rational TICKS_PER_SECOND = 254,016,000,000 (checked Tick arithmetic)                       |
|  - EditorEngine & 498-command Catalogue (Tiers, Profiles, Transaction Undo/Redo)               |
|  - UnifiedViewportState, ActorContext isolation, durable LockRegistry, TaskSheet state machine |
|  - TimelineBranch CoW diffs/merges, OriginTracker provenance, EDL/FCPXML/OTIO/XMEML interop    |
+----------+-----------------------------+-------------------------------+-----------------------+
           |                             |                               |
           v                             v                               v
+-----------------------+   +--------------------------+   +-------------------------------------+
|     saturn-render     |   |    saturn-perception     |   |   saturn-audio-dsp & plugin-worker  |
|                       |   |                          |   |                                     |
|  GStreamer pipelines, |   |  Whisper, SigLIP,        |   |  EBU R128 loudness, phase align,    |
|  wgpu color/LUTs,     |   |  MODNet, DA-V2-Small,    |   |  ducking, room-tone, out-of-process |
|  hardware encoders    |   |  CIEDE2000, SQLite vecs  |   |  CLAP sandbox (saturn-plugin-worker)  |
+-----------------------+   +--------------------------+   +-------------------------------------+

External Local Cockpit:
  Starship Control <---> $XDG_RUNTIME_DIR/starship/bridges/eep.sock (SO_PEERCRED) + saturn-mcp (--mcp / --bridge)
```

---

## 3. Cargo Workspace & Repository Layout

```text
easy-edits-pro/
├── Cargo.toml                     # Workspace root (14 crates, release overflow-checks = true)
├── DESIGN.md / PRD.md / ARCHITECTURE.md / API.md
├── AGENTS.md                      # Engineering, styling & Starship ecosystem invariants
├── NOTICE & licenses/             # Third-party attribution & bundled canonical SPDX texts
├── apps/saturn-tauri/
│   ├── src/                       # Tauri host (main.rs, commands/*, media_server.rs, mcp_bridge.rs, write_grants.rs, net.rs, recovery.rs)
│   ├── ui/src/                    # React 19 UI (app/, components/linear/, components/VideoScopes.tsx, features/, workspaces/, styles/)
│   ├── scripts/                   # check-linear.mjs, check-contrast.mjs, check-styles.mjs, check-parity.mjs, catalogue sync
│   └── tests/e2e/                 # 13-area Playwright E2E suite (mocked IPC, 6 viewports × 9 workspaces)
├── crates/
│   ├── saturn-core/               # ProjectDocument (v19), EditorEngine, 498-command catalogue, TaskSheet, branches, interop
│   ├── saturn-commands/           # Registry, AppContext, CommandId, shortcut manager, transactions
│   ├── saturn-render/             # GStreamer playback/export, frame decode, encoder detection, beat-grid/match-cut prep
│   ├── saturn-perception/         # Whisper, SigLIP, MODNet, DA-V2-Small, optical/SSD motion tracking, shot detect
│   ├── saturn-audio-dsp/          # Native audio DSP graph, EBU R128, room tone, ducking, CLAP abstractions
│   ├── saturn-plugin-worker/      # Isolated subprocess binary hosting third-party CLAP plugins
│   ├── saturn-plugin-fixture/     # Deterministic test CLAP fixture for parity tests
│   ├── saturn-mcp/                # MCP stdio server (--project / --bridge) + starship_bridge.rs
│   ├── saturn-control/            # Long-lived Axum HTTP/WS + UDS JSONL daemon, RunnerService, SSO/RBAC
│   ├── saturn-cli/                # Headless CLI & JSONL `control` protocol
│   ├── saturn-timeline/           # Timeline domain primitives
│   ├── saturn-graphics/           # Vector/title/shape overlay primitives
│   ├── saturn-project/            # Project model helpers
│   └── saturn-text/               # Subtitle & text formatting primitives
├── agent-harness/cli_anything_eep/# Python automation CLI (`branch`, `daemon`, `runner`, `bench`, §2.6 recipes)
├── scripts/                       # license-headers.mjs, check-licence-policy.mjs, generate-notice-bundle.py, doctor.sh
└── tests/native/                  # Python native integration workflows (mcp_command_matrix, installed_package_workflow, ai_closed_loop_workflow)
```

---

## 4. Key Architectural Invariants

1. **Single Authoritative Mutation Path:** Both interactive UI actions and headless/MCP automation construct a `CommandRequest` targeting `saturn-core::EditorEngine`. Tauri wraps execution in `std::panic::catch_unwind` under the state lock (`commands/project.rs`) so panics never poison state.
2. **Off-Lock Heavy Media & ML Execution:** Frame decoding, GStreamer export, Whisper transcription, SigLIP indexing, MODNet/Depth inference, and beat-grid/match-cut preparation run **outside** the engine mutex; mutations validate against the project snapshot digest before committing atomically.
3. **Security Boundaries:**
   - `write_grants.rs`: Enforces write containment (`app_data_dir`, `app_cache_dir`, active project directory, or UI-dialog-registered `register_write_grant` exact paths).
   - `media_server.rs`: Serves local media assets only after allowlist registration + per-session bearer token verification.
   - `net.rs`: Resolves DNS once and rejects loopback/private/link-local IPs to block SSRF when importing network media or Openverse assets.
   - `mcp_bridge.rs` & `starship_bridge.rs`: Socket directories restricted to `0700` / sockets `0600`, same-UID kernel `SO_PEERCRED` auth, 1 MiB message caps, and strict tier/profile policy gating.
4. **800-Line Source Cap:** Every `.rs`, `.ts`, `.tsx`, and `.css` source file is strictly capped at 800 lines (`pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`).

---

*Authored by Zvirox · Easy Edits Pro.*
