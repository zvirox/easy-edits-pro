# Easy Edits Pro (EEP): Master AI-Native Video Editor Architecture, Prototype Specification & Full Implementation Plan

> **Status:** Adopted 2026-11-05 as the next major plan after the UI Overhaul Plan (`docs/planning.md`).
> **Progress tracking:** [AI_NATIVE_PLAN_CHECKLIST.md](AI_NATIVE_PLAN_CHECKLIST.md) — update the checklist, not this spec, as packets land.

> **Companion plan adopted 2026-11-08:** [AGENT_FIRST_PLAN.md](AGENT_FIRST_PLAN.md)
> adds profiles, actor sessions, measured verification, briefs/provenance and
> human review on the existing [fixed glass shell](GLASS_CARDS_PLAN.md). Follow
> the shared checklist's dependency-aware queue and open decisions. AF6 shares
> Packet 5.2; AF7/AF8 share Packets 7.1/7.2. Completed work stays completed;
> cinema Dialogue/Music/Ambience/SFX separation stays model/license-blocked.
>
> **Reading this specification:** the audit and prototype examples below are
> historical proposals, not current API/status documentation. Implemented
> contracts and review constraints in the checklist take precedence: fingerprints
> suggest relinks, branch merge v1 is whole-branch only, frames use MCP image
> blocks/runtime previews, colour transforms use fixed WGSL (no OCIO dependency),
> and continuity reports measurements rather than an invented aggregate score.
> Per-actor isolation and arbitrary atomic MCP batches remain planned. AF6 must
> use existing accent tokens and fixed dock slots, not add the prototype's hues
> or floating surfaces. Current runtime behavior is in [architecture.md](architecture.md).

## 1. Executive Summary & Core Design Philosophy

**Easy Edits Pro (`zvirox/easy-edits-pro`)** already provides a strong structural foundation for an AI-controllable Linux non-linear video editor (NLE):

* A **Rust-owned project state engine** (`crates/saturn-core`) where neither the React 19 UI nor external clients mutate state directly.

* A **flicker-free rational tick timebase** (`TICKS_PER_SECOND = 254,016,000,000` in `crates/saturn-core/src/time.rs`) that divides evenly across 23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60, and 120 fps plus 44.1/48/96 kHz audio sample rates.

* A **440-command catalogue** (`docs/COMMANDS.md`) shared across the Tauri desktop app, `saturn-cli`, `saturn-control` (JSON-RPC daemon), `saturn-mcp` (Model Context Protocol stdio server), and `cli-anything-eep` (Python agent harness).

* A **same-user Unix domain socket bridge** (`apps/saturn-tauri/src/mcp_bridge.rs` at `$XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock`) that shares the live undo stack with the desktop GUI and emits `"mcp_project_changed"` so the human editor sees AI edits in real time.

### Why Current AI Video Editing Hits a Wall in EEP

A line-by-line audit of the repository shows four fundamental barriers preventing an AI model from editing, grading, and mixing at a Hollywood master level:

1. **The AI Edits Blind and Deaf (Zero Perceptual Feedback)**:

   * In `apps/saturn-tauri/src/mcp_bridge.rs` (lines 370–405), `media_preview` explicitly returns `"still_frame": {"available": false, "reason": "still-frame extraction is not implemented"}`.

   * In `crates/saturn-render/src/loudness.rs`, EEP has a mathematically exact ITU-R BS.1770-4 / EBU R128 dual-biquad K-weighting loudness meter (`EbuR128Meter`), but it is **never exposed to MCP or the command catalogue**.

   * There are no video scopes (Waveform, RGB Parade, Vectorscope, or skin-tone I-line telemetry) readable by the AI agent.

2. **Simulated / Heuristic Stubs in Key AI & Preview Paths**:

   * **Speech Transcription (`apps/saturn-tauri/src/commands/audio.rs`, lines 96–160)**: When `text_override` is omitted, `transcribe_audio_speech` does not run Whisper on the audio file; it returns four hardcoded demo sentences (`"Welcome to Easy Edits Pro, powered by native Rust..."`).

   * **Motion Tracking (`apps/saturn-tauri/src/commands/preview.rs`, lines 4–34)**: `analyze_motion_tracking` generates synthetic sine/cosine curves (`(f as f64 * 3.7).sin()`) instead of decoding frames and calling the real Lucas-Kanade SSD patch matcher in `crates/saturn-render/src/tracking.rs` (`track_patch_displacement`).

   * **AI Background Matting (`crates/saturn-render/src/matting.rs`, lines 55–120)**: `run_background_matting` does not load `RobustVideoMatting` or `MODNet` neural weights; it computes a centered Gaussian spatial prior multiplied by an RGB skin-color heuristic (`r > g && g > b`).

3. **8-Bit CPU Loops Instead of Scene-Referred 32-Bit Float GPU Processing**:

   * **GPU Surface (`crates/saturn-render/src/wgpu_surface.rs`)**: Despite the filename, `GpuPreviewPipeline` allocates CPU `Vec<u8>` buffers (`SurfaceBuffer`) and runs CPU bilinear blitting (`blit_rescale_rgba`).

   * **Color Grading (`crates/saturn-render/src/wheels.rs`)**: `apply_color_wheels_rgba` loops over 8-bit sRGB bytes (`&mut [u8]`) on the CPU and clamps values to `[0.0, 1.0]`, destroying dynamic range and lacking OpenColorIO (OCIO) / ACEScg color management.

4. **112 UI-Only Command Blind Spots & Fragile Path Storage**:

   * Per `docs/mcp-command-matrix-report.md`, **112 of the 440 commands** return `ui_only` (`"handled by the interface"`) over MCP because playhead position, clip selection, and the export queue live in React state rather than `saturn-core`.

   * `.saturn` project files (Schema v14) store raw absolute file paths, causing clips to go offline whenever a project folder moves.

### Core Philosophy of the Ideal AI-Native Video Editor

To surpass traditional NLEs, Easy Edits Pro must evolve from **Open-Loop Command Dispatch** to **Closed-Loop Perceptual Co-Editing**:

* **Observe -> Hypothesize -> Mutate -> Verify**: Every edit, color grade, or audio mix action executed by the AI is immediately verified against extracted contact sheets (`vision_extract_frame_sheet`), quantitative scope telemetry (`color_read_scopes`), cut continuity scores (`edit_evaluate_cut_continuity`), and EBU R128 spectral/loudness reports (`audio_read_telemetry`).

* **Non-Destructive Timeline Branching**: Just as Git lets coding agents work on feature branches, `saturn-core` must support lightweight Copy-on-Write (CoW) timeline branches (`project.branch_create`, `project.branch_diff`, `project.branch_merge`) so an AI agent can propose a tight cut, a rhythmic music cut, or three different color grades without overwriting the human editor's active cut.

## 2. Complete Crate-by-Crate Audit Matrix

| Crate / Directory | Existing Files & Strengths (Keep) | Current Gaps & Technical Debt (Fix) | New Modules & Upgrades to Build | 
| ----- | ----- | ----- | ----- | 
| `crates/saturn-core` | `time.rs` (`TICKS_PER_SECOND`), `command.rs`, `timeline_edits.rs`, `speech.rs` (filler/silence math), `markers.rs`, `subtitles.rs`, `job.rs` (`JobManager`). | Playhead, selection, and export-queue state are missing from `EditorEngine`; Schema v14 lacks content hashes and timeline branches. | Upgrade to **Schema v15** (`BLAKE3` media hashes, relative paths, `TimelineBranch` map, `UnifiedViewportState` converting 112 `ui_only` commands to engine commands). | 
| `crates/saturn-commands` | `registry.rs`, `handler.rs`, `params.rs`, `undo.rs`, `shortcuts.rs`. | Lacks batch transaction rollback (`atomic_batch`) when a multi-step agent macro fails midway. | Add `CommandTransaction` with all-or-nothing rollback and automatic before/after telemetry delta snapshots. | 
| `crates/saturn-timeline` | `sequence.rs`, `track.rs`, `clip.rs`, `edit.rs`, `atomic.rs`. | Purely track-index based; moving a primary dialogue clip does not carry connected B-roll or SFX pins unless manually batched. | Add optional **Magnetic Storyline Pins** (`ClipPinAnchor`) so secondary B-roll, titles, and SFX stay locked to parent dialogue ticks during ripple edits. | 
| `crates/saturn-project` | `manager.rs`, `search_bin.rs`, `template.rs`. | Search bins only match string metadata (`name`, `path`, `tags`), not visual content or spoken dialogue. | Connect `search_bin.rs` to the new semantic vector and transcript index in `saturn-perception`. | 
| `crates/saturn-render` | `lib.rs` (GStreamer + GES export), `loudness.rs` (`EbuR128Meter`), `lut.rs` (`.cube` parser), `curves.rs`, `hsl.rs`, `tracking.rs` (SSD patch tracker), `speedramp.rs`. | `wgpu_surface.rs`, `wheels.rs`, and `matting.rs` are 8-bit CPU implementations; no random-access still-frame extractor; no GPU video scopes. | Build real **`wgpu` `Rgba32Float` compute pipeline**, **OCIO v2 ACEScg** transforms (`ocio_pipeline.rs`), **GPU Scopes** (`scopes_gpu.rs`), and **Frame/Contact-Sheet Extractor** (`frame_extractor.rs`). | 
| `crates/saturn-perception` *(New Crate)* | *(Currently split across stubs in `saturn-tauri/src/commands/ai.rs`, `audio.rs`, `preview.rs`, and `saturn-render/src/matting.rs`)* | Hardcoded demo transcription strings; synthetic sine/cosine motion tracking; heuristic Gaussian matting; no semantic visual search. | Build dedicated **ONNX Runtime (`ort`) + SQLite-vec crate** for offline **Whisper** word timestamps, **SigLIP** visual embeddings, **RVM v2** matting, **Depth-Anything-V2**, and **YuNet** face landmarks. | 
| `crates/saturn-audio-dsp` *(New Crate)* | *(Currently partial logic in `saturn-render/src/loudness.rs` and `automation_element.rs`)* | Denoise is only a high-pass filter; Fairlight VST3 slots are metadata-only; no stem separation or room-tone generator; preview/render DSP mismatch. | Build unified **PipeWire + GStreamer DSP graph** with native **CLAP/LV2/VST3 hosting** (`clack-host`), ONNX **4-stem separation**, sub-sample **phase alignment**, and **room-tone synthesis**. | 
| `crates/saturn-graphics` & `saturn-text` | `.sgt` template parser/writer, shape primitives, font discovery. | `template apply` in `cli-anything-eep` returns `COMMAND_NOT_FOUND`; no per-character/per-word kinetic typography or depth-composited titles. | Wire `graphic.template_apply` end-to-end; add **per-word spring physics** and **face-avoidant smart layout** (`saturn-text`). | 
| `crates/saturn-mcp` & `saturn-control` | 11 MCP stdio tools; Unix socket client; JSON-RPC 2.0 + SSE server. | 1 MiB socket limit; no binary/shared-memory transport for frames or scopes; missing perceptual tools. | Add **POSIX `/dev/shm` ring buffer** (`shm_bridge.rs`) and **7 Perceptual MCP Tools** (`vision_*`, `color_*`, `audio_*`, `media_semantic_query`, `edit_evaluate_cut_continuity`). | 
| `apps/saturn-tauri` | Tauri 2 bridge (`main.rs`, `mcp_bridge.rs`, `media_server.rs`, `write_grants.rs`) + React 19 workspaces. | No visual indicator of where the AI is working on the timeline; no live GPU scope dock; no script-synced multi-take editor. | Add `AiBranchDiffRibbon.tsx`, `VideoScopesPanel.tsx`, `ScriptSyncPanel.tsx`, and `AgentTelemetryDrawer.tsx`. | 
| `agent-harness` (`cli-anything-eep`) | 18 Python CLI command groups + REPL + JSON output. | Lacks subcommands for perceptual telemetry, branching, OCIO color nodes, and stem separation. | Add `perception`, `branch`, `ocio`, and `telemetry` CLI groups + closed-loop agent skill definitions. | 

## 3. Target System Architecture & Data Flow

### 3.1 Closed-Loop Perceptual Co-Editing Architecture

```
flowchart TB
    subgraph AgentClients["1. AI Agent & Automation Layer"]
        MCPClient["Multimodal LLM / MCP Client"]
        PyHarness["cli-anything-eep (Python CLI / REPL)"]
        JSONRPC["saturn-control (TCP / Unix JSON-RPC + SSE)"]
    end

    subgraph TransportLayer["2. Low-Latency Bridge Layer"]
        MCPServer["saturn-mcp (18 MCP Tools over stdio)"]
        UnixSock["Control Socket ($XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock)"]
        ShmRing["Zero-Copy Shared Memory (/dev/shm/eep-shm-ring)"]
    end

    subgraph CoreEngine["3. Deterministic State & Command Engine (crates/saturn-core)"]
        Dispatcher["CommandRequest Dispatcher (480+ Commands + Atomic Batch)"]
        DocV15["ProjectDocument Schema v15 (BLAKE3 Hashes + CoW Timeline Branches)"]
        ViewportState["Unified Playhead, Selection, Trim & Export Queue State"]
    end

    subgraph MediaEngines["4. GPU Color, Audio DSP & Neural Perception Crates"]
        GpuColor["saturn-render (wgpu Rgba32Float, OCIO v2 ACEScg, DAG Nodes, GPU Scopes)"]
        AudioDsp["saturn-audio-dsp (EBU R128 + FFT, CLAP/LV2 Host, 4-Stem Split, Phase Align)"]
        Perception["saturn-perception (ort ONNX: Whisper, SigLIP, RVM v2, Depth-Anything-V2, SQLite-vec)"]
    end

    subgraph DesktopUI["5. React 19 + Tauri 2 Co-Editing Shell (apps/saturn-tauri)"]
        DiffBar["AiBranchDiffRibbon (Ghost Playhead & One-Click Accept/Reject)"]
        ScopeDock["VideoScopesPanel (DMA-BUF Waveform, RGB Parade, Vectorscope)"]
        ScriptSpine["ScriptSyncPanel (Word-Level Editing & Multi-Take Switcher)"]
        TelemetryUI["AgentTelemetryDrawer (Live MCP Feed & Skill Macro Recorder)"]
    end

    MCPClient --> MCPServer
    PyHarness --> MCPServer
    JSONRPC --> Dispatcher
    MCPServer -->|JSON Commands & Queries| UnixSock
    MCPServer <-->|Zero-Copy Frames, Contact Sheets & Scope Matrices| ShmRing
    UnixSock --> Dispatcher
    Dispatcher --> DocV15
    Dispatcher --> ViewportState
    Dispatcher --> GpuColor
    Dispatcher --> AudioDsp
    Dispatcher --> Perception
    GpuColor --> ShmRing
    AudioDsp --> ShmRing
    Perception --> ShmRing
    DocV15 -->|mcp_project_changed + branch_diff| DiffBar
    GpuColor -->|wgpu / DMA-BUF| ScopeDock
    Perception -->|SpeechSegment & Take Graph| ScriptSpine
    UnixSock -->|Command & Telemetry Stream| TelemetryUI

```

### 3.2 Complete Repository Directory Map (Post-Upgrade)

Every file stays strictly under the **800-line limit** enforced by `pnpm check:source-size --max-lines=800`:

```
easy-edits-pro/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── saturn-core/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── time.rs                  # TICKS_PER_SECOND = 254,016,000,000
│   │       ├── command.rs               # CommandRequest, CommandStatus, AtomicTransaction
│   │       ├── viewport_state.rs        # NEW: Playhead, Selection, Trim & Queue state in Rust
│   │       ├── branch.rs                # NEW: CoW TimelineBranch create, diff, merge
│   │       ├── perception_types.rs      # NEW: ScopeTelemetryReport, CutContinuityScore, FrameSheetRequest
│   │       ├── project/
│   │       │   ├── mod.rs               # Schema v15 with BLAKE3 content_hash & relative_path
│   │       │   ├── color_graph.rs       # NEW: ColorSpaceId, ColorNodeGraph, PowerWindow
│   │       │   └── migrations.rs        # v1 -> v15 automatic lossless migration
│   │       └── catalogue/
│   │           └── commands/            # 480+ commands organized by menu domain
│   ├── saturn-perception/               # NEW CRATE: Local Neural Inference & Semantic Index
│   │   ├── Cargo.toml                   # Depends on ort (ONNX Runtime), rusqlite, saturn-core
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── index_db.rs              # SQLite-vec sidecar (.saturn-index.sqlite)
│   │       ├── whisper.rs               # Real Whisper word-level transcription & diarization
│   │       ├── embeddings.rs            # SigLIP / MobileCLIP visual frame embeddings
│   │       ├── shot_detect.rs           # Scene cut detector & CutContinuityScore evaluator
│   │       ├── matting_ort.rs           # Real RVM v2 & MODNet neural alpha matting
│   │       ├── depth_ort.rs             # Depth-Anything-V2 Z-depth map generator
│   │       └── motion_tracker.rs        # Real Lucas-Kanade + planar 4-corner homography tracker
│   ├── saturn-render/                   # UPGRADED CRATE: 32f GPU Pipeline, OCIO & GStreamer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gpu_context.rs           # NEW: Real wgpu::Device, Queue & DMA-BUF import
│   │       ├── ocio_pipeline.rs         # NEW: ACEScg IDT -> Working Space -> ODT shader pipeline
│   │       ├── color_nodes_gpu.rs       # NEW: WGSL shaders for Wheels, HDR Zones, Curves, Warper
│   │       ├── film_emulation.rs        # NEW: Physical halation, 35mm grain & gate weave
│   │       ├── shot_match.rs            # NEW: Optimal-transport CDF + skin-tone anchor matcher
│   │       ├── scopes_gpu.rs            # NEW: Compute shader Waveform, Parade, Vectorscope & I-line
│   │       ├── frame_extractor.rs       # NEW: Fast GStreamer appsink frame & contact-sheet renderer
│   │       └── shm_ring.rs              # NEW: POSIX /dev/shm zero-copy frame/telemetry ring buffer
│   ├── saturn-audio-dsp/                # NEW CRATE: Cinema Audio Mixing & Plugin Host
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ebu_r128.rs              # ITU-R BS.1770-4 meter + 1/3-octave FFT + dialogue SNR
│   │       ├── clap_host.rs             # Native Linux CLAP / LV2 / VST3 plugin host
│   │       ├── stems_ort.rs             # MDX-Net / Demucs 4-stem separation (Dialogue/Music/Amb/SFX)
│   │       ├── phase_align.rs           # Sub-sample GCC-PHAT boom/lav phase alignment
│   │       ├── room_tone.rs             # Ambient noise floor profiler & seamless gap synthesizer
│   │       └── ducking.rs               # Lookahead sidechain dialogue-to-music envelope generator
│   └── saturn-mcp/
│       └── src/
│           ├── main.rs
│           ├── server.rs                # Expanded from 11 to 18 MCP tools
│           └── perceptual_tools.rs      # NEW: Handlers for vision_*, color_*, audio_*, media_semantic_query
└── apps/
    └── saturn-tauri/
        ├── src/
        │   ├── mcp_bridge.rs            # Updated with /dev/shm descriptors & perceptual methods
        │   └── commands/                # Wired to saturn-perception & saturn-audio-dsp (zero fake stubs)
        └── ui/src/features/
            ├── timeline/
            │   ├── TimelineSection.tsx
            │   └── AiBranchDiffRibbon.tsx   # NEW: Ghost playhead & branch diff accept/reject bar
            ├── preview/
            │   ├── PreviewSection.tsx
            │   └── VideoScopesPanel.tsx     # NEW: Live GPU Waveform, Parade & Vectorscope dock
            ├── captions/
            │   └── ScriptSyncPanel.tsx      # NEW: Text-based ripple cutter & multi-take switcher
            └── inspector/
                └── AgentTelemetryDrawer.tsx # NEW: Live MCP stream, scope/LUFS deltas & macro saver

```

## 4. Prototype Rust Engine Specifications

### 4.1 Schema v15: Content-Addressable Media, Timeline Branches & Unified Viewport State (`crates/saturn-core/src/branch.rs`)

```
use crate::project::{Clip, Sequence};
use crate::time::Tick;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Schema v15 upgrade: Every imported media item carries a BLAKE3 content hash
/// so moving a project directory across machines automatically relinks without going offline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaFingerprint {
    pub blake3_head_tail_hash: String, // Fast 64 KiB head + 64 KiB tail + file_size hash
    pub file_size_bytes: u64,
    pub relative_path: Option<String>, // Relative to the .saturn project file parent dir
}

/// Non-destructive Copy-on-Write timeline branch for AI experimentation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineBranch {
    pub name: String,
    pub parent_branch: String,
    pub created_at_unix_ms: u64,
    pub author: String, // e.g. "human" or "mcp:claude-sonnet"
    pub rationale: Option<String>,
    pub sequence: Sequence,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchClipDelta {
    pub track_id: u64,
    pub clip_id: u64,
    pub change_kind: String, // "added" | "removed" | "trimmed" | "moved" | "graded"
    pub before_start: Option<Tick>,
    pub after_start: Option<Tick>,
    pub before_duration: Option<Tick>,
    pub after_duration: Option<Tick>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchDiffReport {
    pub base_branch: String,
    pub target_branch: String,
    pub duration_delta_ticks: Tick,
    pub total_cuts_base: usize,
    pub total_cuts_target: usize,
    pub clip_deltas: Vec<BranchClipDelta>,
}

/// Absorbs the 112 "ui_only" commands into deterministic Rust engine state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedViewportState {
    pub playhead_tick: Tick,
    pub ai_ghost_playhead_tick: Option<Tick>,
    pub selected_clips: Vec<(u64, usize)>, // (track_id, clip_index)
    pub selected_gap: Option<(u64, Tick, Tick)>,
    pub active_workspace: String,          // "media" | "edit" | "fusion" | "color" | "audio" | "deliver"
    pub active_branch: String,             // defaults to "main"
    pub targeted_video_tracks: Vec<u64>,
    pub targeted_audio_tracks: Vec<u64>,
}

```

### 4.2 Perceptual Telemetry & Zero-Copy Shared Memory Ring (`crates/saturn-core/src/perception_types.rs`)

```
use crate::time::Tick;
use serde::{Deserialize, Serialize};

/// Header written at offset 0 of `/dev/shm/eep-shm-<pid>` for zero-copy frame/scope transfer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShmFrameSlotHeader {
    pub magic: u32,           // 0x45455053 ("EEPS")
    pub sequence_number: u64,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,    // 1 = Rgba8, 2 = Rgba32Float
    pub payload_bytes: u32,
    pub timeline_tick: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSheetRequest {
    pub sequence_id: Option<u64>,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub columns: u8,            // e.g. 4
    pub rows: u8,               // e.g. 4 (16 frames sampled evenly across [start_tick, end_tick])
    pub tile_width: u32,        // e.g. 480 px per tile
    pub apply_color_grade: bool,
    pub burn_in_timecode: bool,
    pub burn_in_safe_guides: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeTelemetryReport {
    pub at_tick: Tick,
    pub luma_ire_min: f32,
    pub luma_ire_max: f32,
    pub luma_ire_p10: f32,
    pub luma_ire_p50: f32,
    pub luma_ire_p90: f32,
    pub clipped_highlights_pct: f32, // % of pixels > 100 IRE
    pub crushed_shadows_pct: f32,    // % of pixels < 0 IRE
    pub rgb_parade_medians: [f32; 3],
    pub vectorscope_mean_sat_pct: f32,
    pub skin_tone_detected: bool,
    pub skin_tone_iline_deviation_deg: Option<f32>, // 0.0 deg = exact flesh-tone line
    pub skin_tone_ire_mean: Option<f32>,            // Target: 50.0 .. 68.0 IRE for caucasian/tan, 35..55 IRE dark skin
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutContinuityScore {
    pub cut_tick: Tick,
    pub track_id: u64,
    pub outgoing_clip_index: usize,
    pub incoming_clip_index: usize,
    pub eye_trace_distance_norm: f32,     // Euclidean distance (0..1.414) between salient focal points across the cut
    pub luma_jump_ire: f32,               // Absolute IRE difference between outgoing last frame and incoming first frame
    pub chromatic_delta_e: f32,           // CIEDE2000 color difference across the cut
    pub optical_flow_angle_diff_deg: f32, // 0 deg = smooth match-on-action; 180 deg = jarring direction reversal
    pub screen_direction_preserved: bool, // 180-degree shutter/axis rule check via face orientation
    pub audio_boundary_pop_db: f32,       // Discontinuity at sample boundary if un-crossfaded
    pub overall_murch_score: f32,         // 0.0 (broken cut) .. 100.0 (invisible Hollywood cut)
    pub recommendation: String,           // Actionable advice for the AI agent
}

```

### 4.3 Scene-Referred 32-Bit Float GPU Color Graph (`crates/saturn-core/src/project/color_graph.rs`)

```
use crate::time::Tick;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpaceId {
    #[default]
    AcesCg,                  // Working linear space (AP1 primaries)
    AcesCct,                 // Working log grading space (AP1 primaries)
    ArriLogC4,               // ARRI Alexa 35 IDT
    ArriLogC3,               // ARRI Alexa Mini LF IDT
    RedLog3G10Rwg,           // RED V-Raptor IPP2 IDT
    SonySLog3SGamut3Cine,    // Sony FX3 / FX6 / Venice IDT
    BlackmagicFilmGen5,      // Blackmagic URSA / Pocket IDT
    CanonLog2CinemaGamut,    // Canon C70 / C300 / R5C IDT
    AppleLog,                // iPhone ProRes Log IDT
    Rec709Gamma24,           // Standard SDR Broadcast ODT
    SrgbDisplay,             // Web / Computer Monitor ODT
    DisplayP3D65,            // Wide Gamut Cinema / Mac ODT
    Rec2020PqHdr1000,        // HDR10 1000-nit Mastering ODT
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerWindowMask {
    pub id: u64,
    pub kind: String,        // "ellipse" | "rect" | "polygon_bezier" | "ai_face" | "ai_person" | "ai_sky"
    pub center_x: f32,
    pub center_y: f32,
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation_deg: f32,
    pub feather_inner: f32,
    pub feather_outer: f32,
    pub invert: bool,
    pub tracked_points: Vec<(Tick, f32, f32, f32)>, // (tick, x, y, scale)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ColorNodePayload {
    ColorSpaceTransform {
        src: ColorSpaceId,
        dst: ColorSpaceId,
        tone_map_method: String,
    },
    PrimaryWheels {
        lift: [f32; 4],   // R, G, B, Master in stops
        gamma: [f32; 4],
        gain: [f32; 4],
        offset: [f32; 4],
        temperature_k_delta: f32,
        tint_delta: f32,
        contrast: f32,
        pivot_ire: f32,
        midtone_detail: f32,
    },
    HdrZones {
        black: [f32; 4],
        dark: [f32; 4],
        shadow: [f32; 4],
        light: [f32; 4],
        highlight: [f32; 4],
        specular: [f32; 4],
    },
    ColorWarperGrid {
        rings: u8,
        spokes: u8,
        control_deltas: Vec<(f32, f32)>, // (hue_shift_rad, sat_scale)
    },
    FilmPrintEmulation {
        print_stock: String, // "kodak_2383_d65" | "kodak_2383_d60" | "fujifilm_3513"
        halation_strength: f32,
        halation_radius_px: f32,
        grain_size_mm: f32,  // 16.0, 35.0, or 65.0
        grain_intensity: f32,
        gate_weave_amplitude: f32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorGradeNode {
    pub id: u64,
    pub label: String,
    pub enabled: bool,
    pub blend_opacity: f32,
    pub payload: ColorNodePayload,
    pub power_windows: Vec<PowerWindowMask>,
    pub inputs: Vec<u64>,
}

```

### 4.4 Cinema Audio DSP & Native Plugin Host (`crates/saturn-audio-dsp/src/lib.rs`)

```
use saturn_core::Tick;
use saturn_render::LoudnessReport;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTelemetryReport {
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub loudness: LoudnessReport,         // Momentary, Short-Term, Integrated LUFS, LRA, True Peak dBTP
    pub spectral_bands_db: [f32; 10],     // 31Hz, 63Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz
    pub phase_correlation: f32,           // -1.0 (out of phase cancellation) .. +1.0 (mono coherent)
    pub dialogue_to_background_snr_db: Option<f32>,
    pub mud_warning_250hz: bool,
    pub sibilance_warning_6khz: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClapPluginSlot {
    pub slot_index: usize,
    pub plugin_format: String,            // "clap" | "lv2" | "vst3"
    pub plugin_id: String,                // e.g. "org.surge-synth-team.xt-eq"
    pub plugin_path: String,
    pub bypassed: bool,
    pub parameters: Vec<(u32, String, f64)>, // (param_id, name, normalized_value)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidechainDuckingRequest {
    pub trigger_dialogue_track_id: u64,
    pub target_music_track_id: u64,
    pub threshold_dbfs: f32,              // e.g. -32.0
    pub duck_amount_db: f32,              // e.g. -11.5 dB
    pub lookahead_ms: u32,                // e.g. 60 ms before first spoken consonant
    pub attack_ms: u32,                   // e.g. 120 ms
    pub hold_ms: u32,                     // e.g. 350 ms (prevents pumping between words)
    pub release_ms: u32,                  // e.g. 650 ms smooth S-curve recovery
}

```

## 5. Complete AI Tool & Command Catalogue Specification

### 5.1 MCP Tool Specification

| \# | MCP Tool Name | Status | Input Parameters | Output / Telemetry | 
| ----- | ----- | ----- | ----- | ----- | 
| 1 | `command_list` | Existing | `filter?: str`, `enabled_only?: bool` | Array of `{id, label, menu, shortcut, mutating, enabled, reason}` | 
| 2 | `command_run` | Existing | `id: str`, `params: obj` | Engine mutation result + automatic UI refresh (`mcp_project_changed`) | 
| 3 | `command_batch` | Existing (Upgraded) | `commands: [{id, params}]`, `atomic?: bool`, `stop_on_error?: bool` | Ordered array of per-command results; full rollback if `atomic: true` fails | 
| 4 | `doc_inspect` | Existing | *(none)* | Full `ProjectDocument` v15, active branch, path, and dirty state | 
| 5 | `sequence_inspect` | Existing | `branch?: str` | Active or named branch sequence, tracks, clips, and transitions | 
| 6 | `media_search` | Existing | `query?: str`, `kind?: str`, `limit?: u64` | Filtered list of imported `MediaItem` metadata | 
| 7 | `media_preview` | Upgraded | `media_id: u64`, `extract_poster_tick?: ticks` | Asset metadata, timeline placements, and real extracted poster frame path | 
| 8 | `render_start` | Existing | `output_path: str`, `range?: obj`, `use_mark_range?: bool`, `settings?: obj` | `{job_id, state: "queued", output_path}` | 
| 9 | `render_status` | Existing | `job_id: u64` | `{id, state, progress, phase, output_path, error}` | 
| 10 | `render_cancel` | Existing | `job_id: u64` | `{job_id, cancellation_requested: true}` | 
| 11 | `render_project` | Existing | `output_path: str`, `range?: obj`, `settings?: obj` | Synchronous/compatibility export result | 
| 12 | `vision_extract_frame_sheet` | **New** | `start_tick`, `end_tick`, `columns`, `rows`, `tile_width`, `apply_color_grade` | Contact sheet PNG path + `/dev/shm` slot descriptor + exact tick array | 
| 13 | `color_read_scopes` | **New** | `at_tick`, `roi?: {x, y, w, h}` | `ScopeTelemetryReport` (IRE min/max/p10/p50/p90, RGB balance, skin-tone I-line deg) | 
| 14 | `audio_read_telemetry` | **New** | `start_tick`, `end_tick`, `track_id?: u64`, `standard?: str` | `AudioTelemetryReport` (LUFS, True Peak, 10-band FFT spectrum, phase, SNR) | 
| 15 | `media_semantic_query` | **New** | `query: str`, `shot_scale?: str`, `camera_motion?: str`, `min_duration_ticks?: ticks` | Ranked clip segments matching visual SigLIP embeddings + Whisper dialogue | 
| 16 | `vision_analyze_motion` | **New** | `track_id`, `clip_index`, `mode: "point"|"planar_4corner"|"face_landmarks"` | Per-frame optical-flow trajectories or 4-corner homography matrices | 
| 17 | `edit_evaluate_cut_continuity` | **New** | `track_id`, `cut_at_tick`, `window_frames?: u8` | `CutContinuityScore` (eye-trace jump, luma/color jump, motion angle, Murch score) | 
| 18 | `render_preview_clip_proxy` | **New** | `start_tick`, `end_tick`, `resolution?: "640x360"`, `include_audio?: bool` | Fast sub-second MP4/WebM snippet around an edit point for video-capable LLMs | 

Implemented addition: `edit_match_cut_align` takes `track_id`,
`left_clip_index`, `right_clip_index`, `mode`, and optional `search_frames`;
returns `{applied, telemetry}` after bounded source analysis and one undoable
slip. See [match-cut-alignment.md](match-cut-alignment.md).

### 5.2 Complete New Engine Command Matrix (`crates/saturn-core/src/catalogue/`)

| Category | Command ID | Parameters | Engine Behaviour | 
| ----- | ----- | ----- | ----- | 
| **Branching** | `project.branch_create` | `{"name":str, "from_branch":str?, "rationale":str?}` | Creates a CoW sequence branch in `ProjectDocument.branches`. | 
| **Branching** | `project.branch_switch` | `{"name":str}` | Switches active sequence editing and UI preview to the named branch. | 
| **Branching** | `project.branch_diff` | `{"base":str, "target":str}` | Computes `BranchDiffReport` between two branches. | 
| **Branching** | `project.branch_merge` | `{"source":str, "target":str, "range_ticks":[ticks,ticks]?}` | Merges all or a specific tick range from `source` branch into `target`. | 
| **Narrative Edit** | `timeline.asymmetric_trim` | `{"track_id":int, "clip_index":int, "video_delta":ticks, "audio_delta":ticks, "fade_ticks":ticks?}` | Atomic J-cut / L-cut split-edit with constant-power audio crossfade. | 
| **Narrative Edit** | `edit.match_cut_align` | `{"track_id":int, "left_clip_index":int, "right_clip_index":int, "mode":"eye_trace|motion|transient", "search_frames":int?}` | Bounded incoming source slip using contrast-centroid heuristic, signed block motion, or attack-envelope matching; prepare via `edit_match_cut_align`. |
| **Narrative Edit** | `timeline.beat_grid_quantize` | `{"music_track_id":int, "video_track_id":int, "subdivision":"bar|half|quarter", "strength":num, "downbeat_tick":ticks?}` | Rolls adjacent video cuts to a measured constant-pulse music grid; bar/half require a known downbeat and assume 4/4. |
| **Narrative Edit** | `timeline.fluid_morph_cut` | `{"track_id":int, "cut_at":ticks, "duration":ticks}` | Synthesizes bidirectional optical-flow frames + room-tone crossfade over jump cuts. | 
| **Narrative Edit** | `timeline.pin_clip_to_storyline` | `{"child_track_id":int, "child_clip_index":int, "parent_track_id":int, "parent_clip_index":int}` | Locks B-roll/title/SFX clip to a primary dialogue clip so ripple edits move both together. | 
| **Script & Speech** | `speech.transcribe_media` | `{"media_id":int?, "model":"base|small|medium|large-v3", "diarize":bool?}` | Runs real local Whisper inference (`saturn-perception`) and stores word-level ticks. | 
| **Script & Speech** | `speech.script_sync_align` | `{"script_text":str, "media_ids":[int]?}` | Aligns written script lines against multi-camera recorded takes and ranks best takes. | 
| **Color (OCIO)** | `ocio.set_pipeline` | `{"working_space":"aces_cg|aces_cct", "display_odt":str, "default_idt":str?}` | Configures project-wide OpenColorIO v2 scene-referred color management. | 
| **Color (OCIO)** | `ocio.set_media_idt` | `{"media_id":int, "idt":str}` | Sets camera Input Device Transform (e.g., `arri_logc4`, `sony_slog3_sgamut3_cine`). | 
| **Color (Nodes)** | `color.node_add` | `{"track_id":int, "clip_index":int, "kind":str, "inputs":[int]?}` | Adds a node (`PrimaryWheels`, `HdrZones`, `ColorWarperGrid`, `Lut3D`, `FilmPrintEmulation`) to the clip's DAG. | 
| **Color (Nodes)** | `color.node_update` | `{"track_id":int, "clip_index":int, "node_id":int, "payload":obj}` | Updates parameters on a specific color node in 32-bit float linear space. | 
| **Color (Mask)** | `color.add_power_window` | `{"track_id":int, "clip_index":int, "node_id":int, "mask":obj, "auto_track":bool?}` | Attaches a geometric or AI-segmented (`ai_face`, `ai_sky`) mask and tracks it across the clip. | 
| **Color (Match)** | `color.match_shot_to_reference` | `{"src_track_id":int, "src_clip_index":int, "ref_track_id":int, "ref_clip_index":int, "lock_skin_tones":bool}` | Matches exposure, white balance, and contrast CDF to a reference hero shot. | 
| **Color (Film)** | `color.emulate_film_print` | `{"track_id":int, "clip_index":int, "stock":str, "halation":num, "grain":num, "weave":num}` | Applies physical red-channel halation, 35mm luminance grain, and print stock LUT. | 
| **Audio (Stems, deferred)** | `audio.separate_stems` | `{"track_id":int, "clip_index":int, "stems":["dialogue","music","ambience","sfx"]}` | Requires a verified, license-cleared cinema-stem model. Music-model vocals/drums/bass/other outputs do not meet this contract. |
| **Audio (Sync)** | `audio.auto_phase_align` | `audio_process({"operation":"phase_align", "ref_track_id":int, "target_track_id":int})` | Raw-source GCC-PHAT fractional alignment of two single-clip audio tracks; replaces target source with derived WAV. |
| **Audio (Tone)** | `audio.fill_gaps_with_room_tone` | `audio_process({"operation":"room_tone", "track_id":int, "room_tone_media_id":int, "profile_start_tick":ticks, "profile_end_tick":ticks, "min_gap_ms":int, "crossfade_ms":int})` | Caller-selected speech-free spectral profile; fills internal gaps with in-gap fades. Dialogue-edge crossfades remain pending. |
| **Audio (Duck)** | `audio.sidechain_duck_envelope` | `audio_process({"operation":"duck", "dialogue_track_id":int, "music_track_id":int, "duck_db":num, "lookahead_ms":int, "hold_ms":int, "release_ms":int, "threshold_dbfs":num})` | Raw-source energy-keyed lookahead volume automation; requires empty music volume and pan lanes. |
| **Audio (Plugin)** | `dsp.load_plugin` | `{"track_id":int, "slot":int, "format":"clap|lv2|vst3", "plugin_id":str}` | Loads a native Linux audio plugin into the real-time and export DSP graph. | 
| **Audio (Plugin)** | `dsp.set_plugin_param` | `{"track_id":int, "slot":int, "param_id":int, "value":num}` | Updates a hosted CLAP/LV2/VST3 plugin parameter (`0.0`..`1.0`). | 
| **Audio (Master)** | `audio.loudness_normalize_master` | `audio_process({"operation":"normalize_master", "standard":"ebu_r128|streaming|atsc_a85", "true_peak_dbtp":num})` | Measured K-weighted normalization and linked lookahead limiting with 4× FIR true peak; undoable GES master bounce, originals retained and muted. |
| **VFX & Depth** | `vfx.depth_map_compositor` | `{"track_id":int, "clip_index":int, "z_threshold":num, "foreground_graphic_id":int?, "dof_blur_px":num?}` | Uses `Depth-Anything-V2` depth maps to place titles behind foreground subjects or render depth bokeh. | 
| **VFX & Track** | `vfx.planar_corner_pin` | `{"track_id":int, "clip_index":int, "target_track_id":int, "target_clip_index":int, "roi":obj, "motion_blur":bool}` | Tracks a 4-corner planar perspective surface and pins an insert layer with motion blur. | 
| **VFX & Clean** | `vfx.inpaint_object_removal` | `{"track_id":int, "clip_index":int, "mask_id":int, "start_tick":ticks, "end_tick":ticks}` | Temporally coherent optical-flow clean-plate inpainting to erase boom mics or wires. | 
| **Typography** | `graphic.kinetic_typography` | `{"graphic_id":int, "preset":"spring_words|typewriter|karaoke_highlight", "avoid_faces":bool}` | Animates per-word/per-character text synced to speech ticks while avoiding detected faces. | 

**Implemented Packet 4.2 contract:** the four audio operations above use the
shared MCP/desktop `audio_process` tool. It prepares derived media or automation
outside the engine lock, then applies one atomic core command with an expected
snapshot digest. Low-level `audio.*` commands accept prepared `audio`, `inserts`
or `points` payloads plus `expected_digest`, not the tool arguments above; do not
send the preparation requests through `command_batch`. Save the project first;
keep its sibling `<project-stem>.saturn-audio/` directory with it. Processing is
bounded to 120 seconds. Failed edits clean up new files; successful files persist
for undo/redo and reopening. See [audio-processing.md](audio-processing.md) for
the exact limits. Packet 4.1 plugin hosting is CLAP-only; other format tags are
rejected, and native engine preview/export parity does not cover browser playback.

## 6. UI Elements for Human + AI Co-Editing (`apps/saturn-tauri/ui`)

All UI additions respect the hard rules in `AGENTS.md`: CSS design tokens only (`pnpm check:styles`), `.glass` classes restricted to chrome/panels, **raw unblurred canvases for all video/preview/scope surfaces**, and `< 800` lines per file:

| UI Component File | Workspace Location | Key Visual Affordances & Tauri IPC Bindings | 
| ----- | ----- | ----- | 
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx` | Docked directly above Track `V1` in `EditWorkspace.tsx` | Displays the cyan **AI Ghost Playhead** (`--token-accent-ai`), branch selector pill (`main` vs `ai/tight-cut`), and interactive green/amber diff blocks for every added/trimmed clip with `Accept (Enter)` and `Reject (Backspace)` actions calling `project.branch_merge`. | 
| `ui/src/features/preview/VideoScopesPanel.tsx` | Right/Bottom dock in `ColorWorkspace.tsx` and `EditWorkspace.tsx` | Renders 60fps GPU-computed **Waveform**, **RGB Parade**, **Vectorscope (with skin-tone I-line)**, and **Histogram** onto a raw HTML5 `<canvas>`/WebGL2 surface. Includes a **"Show AI Target Bounds"** toggle displaying the target IRE and hue corridor. | 
| `ui/src/features/captions/ScriptSyncPanel.tsx` | Left dock in `EditWorkspace.tsx` and `MediaWorkspace.tsx` | Displays word-level Whisper transcript synced to the playhead. Strikethrough-deleting text calls `timeline.ripple_delete_ranges` + `audio.fill_gaps_with_room_tone`; clicking a line opens a **Multi-Take Carousel** comparing alternative camera takes by vocal energy and focus score. | 
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx` | Collapsible tab in `InspectorPanel.tsx` | Streams live MCP `CommandRequest` execution cards, before/after `ScopeTelemetryReport` and `AudioTelemetryReport` badges, `CutContinuityScore` warnings, and a **"Save Selection as Agent Skill"** exporter. | 

## 7. Full Phase-Wise Implementation Plan (Phases 0 to 7)

Each phase is divided into atomic packets mapped to the repository's four ownership lanes (`AGENTS.md`: **Lane 1 UI/Glass**, **Lane 2 Engine/Parity**, **Lane 3 Release/Verification**, **Lane 4 Tooling/Docs/MCP**).

The phase IDs remain stable. The [agent-first dependency table](AGENT_FIRST_PLAN.md#4-dependencies-and-execution)
proposes control/review work before VFX, with AF4 brief metadata before the full
AF3 verification gate. Use the shared checklist for the current queue and
maintainer decisions rather than inferring execution order from numbering.

### Phase 0: Unify UI-Only State & Upgrade Project Schema to v15 (Foundation)

**Objective**: Eliminate the 112 `ui_only` MCP command blind spots and add content-addressable media + timeline branching.

* **Packet 0.1 — Rust Viewport & Selection State (Lane 2)**:

  * Create `crates/saturn-core/src/viewport_state.rs` (< 400 lines) holding `UnifiedViewportState` (playhead tick, AI ghost playhead tick, selected clips/gaps, active workspace, targeted V1–V8/A1–A8 tracks, and export queue state).

  * Update `crates/saturn-core/src/catalogue/commands/{edit,sequence,view,file}.rs` so commands previously returning `"handled by the interface"` (`playhead.*`, `timeline.select*`, `edit.select_*`, `export.queue.*`) mutate `UnifiedViewportState` inside `EditorEngine` and return the updated snapshot to both MCP and React.

* **Packet 0.2 — Schema v15: BLAKE3 Media Fingerprints & CoW Timeline Branches (Lane 2)**:

  * Create `crates/saturn-core/src/branch.rs` implementing `TimelineBranch`, `project.branch_create`, `project.branch_switch`, `project.branch_diff`, and `project.branch_merge`.

  * Update `crates/saturn-core/src/project/` to Schema v15 (with v1->v15 migration in `migrations.rs`), computing a fast BLAKE3 hash (first 64 KiB + last 64 KiB + file size) and storing paths relative to the `.saturn` file alongside the canonical path so moved project folders auto-relink on open.

* **Phase 0 Verification Gate**:

  * Run `cargo test -p saturn-core` and `python3 tests/native/mcp_command_matrix.py`.

  * **Exit Criterion**: At least **425 of 440 commands** return `ok` over the headless MCP server and desktop bridge (leaving only OS window controls like `view.fullscreen` as `ui_only`).

### Phase 1: Perceptual Feedback Loop — Give the AI Eyes & Ears

**Objective**: Enable multimodal AI agents to visually see frames/contact sheets and quantitatively read video scopes and audio loudness/spectrum telemetry.

* **Packet 1.1 — POSIX Shared-Memory Ring & Fast Frame/Contact-Sheet Extractor (Lane 2 & Lane 4)**:

  * Create `crates/saturn-render/src/shm_ring.rs` implementing a `0600` POSIX shared memory ring buffer (`/dev/shm/eep-shm-<uid>-<pid>`) for zero-copy frame and scope transfer.

  * Create `crates/saturn-render/src/frame_extractor.rs` using a pre-warmed GStreamer `uridecodebin -> videoconvert -> videoscale -> appsink` pipeline to decode single frames or composite a `columns × rows` tiled contact sheet (`FrameSheetRequest`) with burned-in tick/timecode labels in < 150 ms.

  * Update `media_preview` in `crates/saturn-mcp/src/server.rs` and `apps/saturn-tauri/src/mcp_bridge.rs` to return a real still frame, and add `vision_extract_frame_sheet` and `render_preview_clip_proxy`.

* **Packet 1.2 — Quantitative Video Scopes Telemetry (Lane 2 & Lane 4)**:

  * Create `crates/saturn-render/src/scopes_gpu.rs` computing `ScopeTelemetryReport`: Luma IRE percentiles (`min`, `p10`, `p50`, `p90`, `max`), highlight clipping %, shadow crush %, RGB Parade balance, Vectorscope saturation distribution, and skin-tone I-line deviation in degrees.

  * Expose `color_read_scopes` in `crates/saturn-mcp/src/server.rs` and `apps/saturn-tauri/src/mcp_bridge.rs`.

* **Packet 1.3 — Audio Loudness, FFT Spectrum & SNR Telemetry (Lane 2 & Lane 4)**:

  * Extend `crates/saturn-render/src/loudness.rs` (`EbuR128Meter`) with a 10-band 1/3-octave FFT analyzer, stereo phase correlation meter (`-1.0`..`+1.0`), and speech-to-background SNR calculator.

  * Expose `audio_read_telemetry` as a top-level tool in `saturn-mcp`.

* **Phase 1 Verification Gate**:

  * Run `cargo test -p saturn-render -p saturn-mcp`.

  * **Exit Criterion**: Automated tests verify that `vision_extract_frame_sheet` generates a valid 4×4 PNG contact sheet from `editor-smoke.webm`, `color_read_scopes` accurately flags a synthetic clipped white frame (`clipped_highlights_pct > 95.0`), and `audio_read_telemetry` measures `-23.0 ± 0.1 LUFS` on a reference EBU R128 test signal.

### Phase 2: Local Neural Perception Engine (`crates/saturn-perception`)

**Objective**: Replace all hardcoded/heuristic stubs (`transcribe_audio_speech`, `analyze_motion_tracking`, `run_background_matting`) with real local ONNX Runtime (`ort`) inference and a project-sidecar vector index.

* **Packet 2.1 — Real Offline Whisper Transcription & Script Sync (Lane 2)**:

  * Create `crates/saturn-perception` and implement `src/whisper.rs` using `whisper-rs` / ONNX Whisper (`base`, `small`, `large-v3-turbo`) with word-level tick alignment and speaker turn diarization.

  * Replace the 4 hardcoded demo sentences in `apps/saturn-tauri/src/commands/audio.rs` (`transcribe_audio_speech`) with real calls to `saturn_perception::whisper::transcribe_media`.

  * Implement `speech.script_sync_align` to match `.fountain` / plain-text scripts against all recorded camera takes.

* **Packet 2.2 — Shot Detection, SigLIP Semantic Search & Cut Continuity Evaluator (Lane 2)**:

  * Implement `crates/saturn-perception/src/shot_detect.rs` and `src/embeddings.rs`, storing frame embeddings, shot scale (`closeup`, `medium`, `wide`), camera motion (`static`, `pan`, `tilt`, `handheld`), and face bounding boxes in a sidecar `.saturn-index.sqlite` database.

  * Implement `CutContinuityScore` calculation (eye-trace distance, luma jump, CIEDE2000 delta-E, optical-flow angle continuity, 180-degree eye-line rule, and audio zero-crossing check) and expose `media_semantic_query` and `edit_evaluate_cut_continuity` over MCP.

* **Packet 2.3 — Real Neural Matting, Depth Maps & Optical-Flow Motion Tracking (Lane 2)**:

  * Create `crates/saturn-perception/src/matting_ort.rs` and `src/depth_ort.rs` running `RobustVideoMatting (RVM v2)`, `MODNet`, and `Depth-Anything-V2` via `ort`, replacing the Gaussian/RGB heuristic in `crates/saturn-render/src/matting.rs`.

  * Replace the synthetic sine/cosine generator in `apps/saturn-tauri/src/commands/preview.rs` (`analyze_motion_tracking`) with real frame decoding piped into `saturn-render/src/tracking.rs` (`track_patch_displacement` + 4-corner RANSAC homography).

* **Phase 2 Verification Gate**:

  * Run `cargo test -p saturn-perception -p saturn-tauri`.

  * **Exit Criterion**: Zero fake/hardcoded outputs remain in `commands/audio.rs`, `commands/preview.rs`, or `matting.rs`; `media_semantic_query` retrieves relevant clips by text description in < 50 ms after indexing.

### Phase 3: Scene-Referred 32-Bit Float GPU Color Engine

**Objective**: Replace 8-bit CPU color loops with a 32-bit float linear `wgpu` compute pipeline, OpenColorIO v2 ACEScg color management, composable node graphs, and automatic shot matching.

* **Packet 3.1 — Real `wgpu` `Rgba32Float` Pipeline & OpenColorIO v2 ACEScg (Lane 2)**:

  * Refactor `crates/saturn-render/src/wgpu_surface.rs` and create `src/gpu_context.rs` + `src/ocio_pipeline.rs` using a real `wgpu::Device` and `wgpu::Queue` with `TextureFormat::Rgba32Float` and Linux DMA-BUF zero-copy import (`gpu_detect.rs`).

  * Implement `ocio.set_pipeline` and `ocio.set_media_idt` supporting ARRI LogC4, RED Log3G10, Sony S-Log3, Blackmagic Gen5, Apple Log -> **ACEScg (linear)** -> **ACEScct (log grading)** -> Rec.709 / Display P3 / Rec.2020 PQ HDR ODTs.

* **Packet 3.2 — WGSL Color Node Graph, Power Windows & Film Emulation (Lane 2)**:

  * Create `crates/saturn-render/src/color_nodes_gpu.rs` and `src/film_emulation.rs` porting Primary Wheels, 6-Zone HDR Wheels, Cubic Spline RGB Curves, 3D LUTs, HSL Qualifier, 2D Color Warper Grid, tracked bezier/AI Power Windows (`color.add_power_window`), and physical halation + 35mm grain (`color.emulate_film_print`) to WGSL compute shaders.

* **Packet 3.3 — Automated Optimal-Transport & Skin-Tone Shot Matcher (Lane 2)**:

  * Create `crates/saturn-render/src/shot_match.rs` implementing `color.match_shot_to_reference`, which aligns a target clip's Luma/Chroma CDF to a reference hero shot while anchoring detected facial pixels to the reference shot's vectorscope skin-tone IRE and hue angle.

* **Phase 3 Verification Gate**:

  * Run `cargo test -p saturn-render --test color_pipeline`.

  * **Exit Criterion**: Pushing exposure +3.0 stops in Node 1 and pulling -3.0 stops in Node 2 in `Rgba32Float` ACEScg recovers 100% of highlight detail with zero clipping, and `color.match_shot_to_reference` brings two mismatched camera angles within `CIEDE2000 ΔE < 1.5`.

### Phase 4: Cinema Audio DSP, Stem Separation & Native Plugin Host (`crates/saturn-audio-dsp`)

**Objective**: Achieve full Fairlight audio parity between live preview and final export, with native Linux plugin hosting, AI stem separation, sub-sample phase alignment, and automated dialogue mixing.

* **Packet 4.1 — Native Linux CLAP / LV2 / VST3 Host & Render-DSP Parity (Lane 2)**:

  * Create `crates/saturn-audio-dsp/src/clap_host.rs` using `clack-host` / `lv2` to load native Linux audio plugins (`dsp.load_plugin`, `dsp.set_plugin_param`) and wire the exact same Rust DSP graph into both live preview playback and the GStreamer `automation_element.rs` export pipeline.

* **Packet 4.2 — Stem Separation, Sub-Sample Phase Align, Room Tone & Sidechain Ducking (Lane 2)**:

  * Create `src/stems_ort.rs` (`audio.separate_stems` isolating Dialogue, Music, Ambience, and SFX via ONNX MDX-Net).

  * Create `src/phase_align.rs` (`audio.auto_phase_align` using Generalized Cross-Correlation with Phase Transform, GCC-PHAT, for sub-sample boom/lav alignment).

  * Create `src/room_tone.rs` (`audio.fill_gaps_with_room_tone` extracting spectral noise footprints to fill dialogue cuts seamlessly) and `src/ducking.rs` (`audio.sidechain_duck_envelope` + `audio.loudness_normalize_master`).

* **Phase 4 Verification Gate**:

  * Run `cargo test -p saturn-audio-dsp`.

  * **Exit Criterion**: Preview and rendered MP4 audio buffers match bit-for-bit within `-90 dBFS`; boom+lav comb filtering is eliminated on a 1.7 ms delayed test fixture; master normalization hits `-14.0 ± 0.1 LUFS` and `<= -1.0 dBTP`.

### Phase 5: Narrative Editing Commands & Co-Editing UI Surfaces

**Objective**: Ship high-level Hollywood narrative editing verbs (`J/L-cuts`, match cuts, beat quantize, morph cuts, storyline pins) and build the four React 19 co-editing UI panels.

* **Packet 5.1 — Narrative Editing Verbs & Magnetic Storyline Pins (Lane 2)**:

  * First-task implementation: `timeline.asymmetric_trim` uses `crates/saturn-core/src/asymmetric_trim.rs` for shared timing/handle validation, an atomic engine handler, and `crates/saturn-render/src/asymmetric_trim.rs` for the project-local constant-power audio bounce. Schema v16's `Clip.audio_enabled` suppresses original sound in both playback paths. Select the incoming adjacent video clip; both deltas are relative to the original cut, with a centered 40 ms default fade. The resulting audio clip is independently editable; revising the baked cut requires undo and rerun. See the checklist for supported source states and the 120-second bound.

   * Match-cut implementation: `edit_match_cut_align` prepares raw-source measurements outside the editor lock, then applies `edit.match_cut_align` atomically with stale-snapshot rejection. The desktop menu selects the incoming clip and uniquely adjacent predecessor. Only incoming `source_in` changes; equal matches prefer minimum slip, and no-signal analysis errors. See [match-cut-alignment.md](match-cut-alignment.md) for heuristic definitions, bounds, unsupported states and native render evidence.

   * Beat-grid implementation: `timeline_beat_grid_quantize` and the desktop menu prepare bounded RMS-rise pulse measurements outside the editor lock. `timeline.beat_grid_quantize` rolls every internal cut of a continuous video strip atomically, preserving outer edges and source mapping. Bar/half require a caller-supplied downbeat and assume 4/4; the constant-pulse heuristic does not recognize musical meter. See [beat-grid-quantization.md](beat-grid-quantization.md) for validation, telemetry, limitations and rendered evidence.

   * Remaining: implement `timeline.fluid_morph_cut` and `timeline.pin_clip_to_storyline` in focused core timeline-edit modules.

  * Run `pnpm catalogue:sync` to regenerate `docs/COMMANDS.md` and `docs/reference/command-catalogue.json`.

* **Packet 5.2 — Human + AI Co-Editing React 19 Panels (Lane 1)**:

  * Implement `AiBranchDiffRibbon.tsx`, `VideoScopesPanel.tsx`, `ScriptSyncPanel.tsx`, and `AgentTelemetryDrawer.tsx` under `apps/saturn-tauri/ui/src/features/`, adhering strictly to `.glass` token rules and raw media canvas rules.

* **Phase 5 Verification Gate**:

  * Run `pnpm --dir apps/saturn-tauri build`, `pnpm --dir apps/saturn-tauri test`, `pnpm --dir apps/saturn-tauri check:glass`, and `pnpm --dir apps/saturn-tauri check:commands`.

### Phase 6: VFX Compositing, 3D Depth, Planar Tracking & Kinetic Typography

**Objective**: Enable Hollywood-style visual effects, depth-aware compositing, screen replacements, and dynamic typography directly from engine commands.

* **Packet 6.1 — Z-Depth Compositing, Planar Corner-Pin & Object Removal (Lane 2)**:

  * Implement `vfx.depth_map_compositor` in `crates/saturn-render/src/compositor.rs` using per-frame `Depth-Anything-V2` depth maps to sandwich titles behind foreground subjects or render optical depth-of-field bokeh.

  * Implement `vfx.planar_corner_pin` (4-corner perspective homography with motion blur) and `vfx.inpaint_object_removal` (optical-flow temporal clean-plate inpainting).

* **Packet 6.2 — Kinetic Typography & End-to-End `.sgt` Template Application (Lane 2)**:

  * Fix the `template apply` gap in `crates/saturn-graphics` and `agent-harness` so `.sgt` templates apply cleanly to any sequence tick.

  * Implement `graphic.kinetic_typography` in `crates/saturn-text` with per-word spring physics synced to Whisper `SpeechWord` timestamps and automatic placement that avoids overlapping `YuNet` face bounding boxes.

* **Phase 6 Verification Gate**:

  * Run `cargo test -p saturn-render -p saturn-graphics -p saturn-text` and verify `cli-anything-eep template apply` passes end-to-end.

### Phase 7: Python Agent Harness Expansion, Hardware Encode Matrix & Closed-Loop Verification

**Objective**: Expose the entire upgraded capability set in `cli-anything-eep`, verify hardware encoding (VA-API/NVENC), and prove an autonomous end-to-end AI edit loop.

* **Packet 7.1 — `cli-anything-eep` CLI & Skill Upgrade (Lane 4)**:

  * Add `branch`, `perception`, `ocio`, `telemetry`, and `vfx` subcommand modules under `agent-harness/cli_anything_eep/commands/` and update `agent-harness/cli_anything_eep/skills/cli-anything-eep.md`.

* **Packet 7.2 — Closed-Loop Autonomous AI Editing Acceptance Suite (Lane 3)**:

  * Create `tests/native/ai_closed_loop_workflow.py` driving the live desktop app over `$XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock` through a complete multi-camera rough cut, continuity check, ACEScg shot-matched grade, Fairlight stem ducking + EBU R128 master normalization, and H.264/WebM export.

* **Phase 7 Final Release Gates (`AGENTS.md`)**:

  ```
  cargo test --workspace
  pnpm --dir apps/saturn-tauri build
  pnpm --dir apps/saturn-tauri test
  pnpm --dir apps/saturn-tauri check:glass
  pnpm --dir apps/saturn-tauri check:commands
  pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
  pnpm --dir apps/saturn-tauri test:e2e
  python3 -m pytest agent-harness/tests -q
  python3 tests/native/mcp_command_matrix.py
  python3 tests/native/ai_closed_loop_workflow.py
  
  ```

## 8. Autonomous Coding Agent Playbook: How an AI Coding Agent Builds This in EEP

When an AI coding agent (such as Claude Code, OpenCode, Cursor, or Gemini) is tasked with implementing these packets inside the `easy-edits-pro` repository, it must follow this exact operational protocol:

1. **Navigate with `prowl` Before Reading Files (`AGENTS.md`)**:

   * Run `prowl overview` and `prowl search "<feature>"` or `prowl references <symbol>` to map symbol dependencies before editing any Rust or TypeScript file.

2. **Respect Single-Lane Path Ownership**:

   * Never mix **Lane 1 (`ui/src/styles/`, `ui/src/features/panels/`)** and **Lane 2 (`crates/saturn-core/`, `crates/saturn-render/`)** in the same commit packet.

   * Keep `crates/saturn-core/src/catalogue/commands/<menu>.rs` edits scoped to one menu family per commit, and always run `pnpm catalogue:sync` instead of hand-editing `docs/COMMANDS.md` or `docs/reference/command-catalogue.json`.

3. **Enforce the 800-Line File Ceiling**:

   * Every `.rs`, `.ts`, `.tsx`, and `.css` file must remain `<= 800` lines (`pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`). Note that `crates/saturn-render/src/lib.rs` is currently 1,483 lines and `crates/saturn-mcp/src/server.rs` is 818 lines—any touch to those files should first extract submodules (`frame_extractor.rs`, `perceptual_tools.rs`) to bring them below 800 lines.

4. **Honest Error Reporting & Shipped Gate**:

   * Never return a silent fake success. Every command marked `shipped: true` in the catalogue must have a real Rust engine implementation, a UI affordance, and an automated unit/integration test.

## 9. Reference Closed-Loop Workflow: How an AI Video-Editing Agent Edits a Film in EEP

Once Phases 0–7 are in place, here is the exact sequence of MCP tool calls an AI video editor executes to cut, grade, and mix raw footage into a Hollywood-grade master:

1. **Create an Experimental Branch**:

   * Calls `command_run("project.branch_create", {"name": "ai/director-cut-v1", "rationale": "Tight dialogue assembly with J-cuts and ACEScg warm cinema grade"})`.

2. **Transcribe & Index All Footage**:

   * Calls `command_run("speech.transcribe_media", {"model": "large-v3-turbo", "diarize": true})` and `media_semantic_query({"query": "emotional closeup reaction shot", "shot_scale": "closeup"})`.

3. **Assemble Dialogue Spine & Apply J/L-Cuts**:

   * After assembling and saving an adjacent clip pair, calls `audio_process({"operation":"asymmetric_trim","track_id":1,"clip_index":1,"video_delta":0,"audio_delta":-127008000000})` (0.5 s audio lead; default 40 ms fade). Preparation produces the audio artifact before one atomic `timeline.asymmetric_trim` application; raw headless `command_batch` does not perform the decode/render preparation.

4. **Audit Every Cut for Visual & Audio Continuity**:

   * Calls `edit_evaluate_cut_continuity({"track_id": 1, "cut_at_tick": 762048000000})` and `vision_extract_frame_sheet({"start_tick": 700000000000, "end_tick": 820000000000, "columns": 4, "rows": 2})`.

   * If `overall_murch_score < 70.0` due to a talking-head jump cut, calls `timeline.pin_clip_to_storyline` to place matching B-roll on `V2` or applies `timeline.fluid_morph_cut`.

5. **Execute Closed-Loop ACEScg Color Grade**:

   * Calls `command_run("ocio.set_pipeline", {"working_space": "aces_cct", "display_odt": "rec709_gamma24"})`.

   * Grades the hero clip with `color.node_add` (`PrimaryWheels` + `FilmPrintEmulation` `kodak_2383_d65`) and `color.add_power_window` (`shape: "ai_face"`, `auto_track: true`).

   * Calls `color_read_scopes({"at_tick": 254016000000})` and verifies `skin_tone_ire_mean` is `58.4 IRE` and `skin_tone_iline_deviation_deg` is `+1.2°` (`clipped_highlights_pct == 0.0`).

   * Propagates the look across the scene via `command_run("color.match_shot_to_reference", {"src_track_id": 1, "src_clip_index": 1, "ref_track_id": 1, "ref_clip_index": 0, "lock_skin_tones": true})`.

6. **Mix Cinema Audio & Verify Broadcast Loudness**:

   * Saves the project, then calls `audio_process` sequentially with `operation: "phase_align"`, `"room_tone"` (a selected speech-free profile range), `"duck"` (`duck_db: -11.0`), and `"normalize_master"` (`standard: "streaming"`, `true_peak_dbtp: -1.0`). Each operation prepares against the latest snapshot and applies its own undoable command. Normalization creates an audible master bounce and mutes the retained source tracks.

   * Checks normalization telemetry for `integratedLufs` within `-14.0 ± 0.1` and `truePeakDbtp <= -1.0`. Remeasures encoded delivery audio because lossy encoding can change true peak. Dialogue-to-background SNR verification remains a separate planned capability.

7. **Present Diff to the Human Editor or Render Master**:

   * Calls `command_run("project.branch_diff", {"base": "main", "target": "ai/director-cut-v1"})` so the human editor can inspect and accept the cuts in `AiBranchDiffRibbon.tsx`, or triggers `render_start` to export the mastered MP4.


## 10. Security Audit Remediation & AI Attack-Surface Hardening

Based on the repository's threat model (`docs/security-audit-report.md` and `apps/saturn-tauri/src/write_grants.rs`), an AI-native video editor faces two distinct security challenges: **protecting the host OS from a prompt-injected AI agent** (e.g., an LLM reading malicious instructions embedded inside an imported SRT subtitle or FCPXML file) and **protecting the Rust engine from malformed media, overflow panics, or GPU/DSP hangs**.

### 10.1 Existing Security Audit Findings That Must Be Closed in Phase 0

| Severity | Vulnerability in Current Repo (`docs/security-audit-report.md`) | Root Cause & File Location | Mandatory Remediation in Phase 0 |
|---|---|---|---|
| **HIGH** | **Single MCP Message Bricks the Editor (Tick Overflow Panic & Mutex Poisoning)** | `captions_cmds.rs:307`, `sequence_cmds.rs:407`, `timeline_cmds.rs:267`, and `compound_cmds.rs` use unchecked `+` / `-` on `Tick(i64)`. Overflow panics the worker thread and permanently poisons the `AppState` mutex (`"Editor state is unavailable"`). | Replace all raw `+`/`-` on `Tick` with `Tick::checked_add` / `checked_sub` returning a typed `Err(String)`; validate `timeline_start >= 0` and `timeline_start.checked_add(duration)` inside `ProjectDocument::validate()`; set `overflow-checks = true` in `[profile.release]`; wrap `mcp_bridge.rs::dispatch` in `std::panic::catch_unwind`. |
| **MEDIUM** | **Untrusted Project / Interchange Files Adopt Local Paths into Allowlist** | Opening `.eep`, FCPXML (`fcpxml.rs`), XMEML (`xmeml.rs`), OTIO (`otio.rs`), or EDL (`edl.rs` with `../../.ssh/id_rsa`) automatically registers embedded paths into the Tauri asset scope and media-server allowlist (`commands/project.rs:70`). | Canonicalize paths and reject `..` parent traversal in all interop importers; mark media paths from untrusted/imported interchange files as `unconfirmed: true` until the user explicitly approves relinking or the path resides inside the project directory. |
| **MEDIUM** | **Quadratic $O(N \times K)$ XMEML Parser Freezes UI & MCP Bridge** | `xmeml.rs:134` splits on `"<clipitem"` and re-scans the entire document tail on every tag while holding `state.0.lock()`. | Parse FCPXML/XMEML/OTIO/EDL documents **before** acquiring the `state.0.lock()` mutex, slice each `<clipitem>...</clipitem>` block strictly to its closing tag index, and cap maximum clip items per import (e.g., 50,000). |
| **MEDIUM-LOW**| **State Integrity: 8 "History-Blind" Commands Cause Undo Drift** | Per `docs/mcp-command-matrix-report.md`, `clip.remove_keyframe`, `project.update_media`, `media.check_missing`, `media.auto_relink`, `media.make_offline`, `media.refresh_changed`, and `marker.split` mutate state without pushing an undo snapshot. | Route all 7 mutating commands through the standard transactional undo-push wrapper in `saturn-core` so an AI agent calling `edit.undo` never consumes an earlier baseline edit. |
| **LOW** | **Unvalidated `source_url` Scheme & Uncapped `file.open` Read** | `commands/assets.rs:68` passes `source_url` without `https://` validation; `project/io.rs:56` calls `read_to_end` with no size cap; `commands/interop.rs:96` (`reveal_path`) skips `validate_read_path`. | Enforce `https://` scheme on `source_url`, cap `.saturn`/`.eep` project file reads at 100 MiB, and call `validate_read_path` in `reveal_path`. |

### 10.2 Hardening the New AI Perceptual & GPU/DSP Surfaces (Phases 1–6)

1. **Write-Grant Enforcement on Frame & Proxy Extractors (`write_grants.rs`)**:
   - The new `vision_extract_frame_sheet` and `render_preview_clip_proxy` MCP tools write image/video files to disk. To prevent a prompt-injected MCP agent from overwriting arbitrary user files, these tools must either write exclusively to an isolated, auto-pruned runtime directory (`$XDG_RUNTIME_DIR/easy-edits-pro/previews/` with mode `0700`) or pass any caller-supplied `output_path` through `crate::write_grants::enforce_write_target()`.
2. **POSIX Shared-Memory (`/dev/shm`) Ring Isolation (`shm_ring.rs`)**:
   - Create `/dev/shm/eep-shm-<uid>-<pid>` using `O_CREAT | O_EXCL | O_RDWR` with strict `0600` permissions (never world-readable).
   - Cap the ring buffer to a fixed ceiling (64 MiB across 4 slots) and register an `atexit` / `Drop` guard so crashed sessions never leak `/dev/shm` RAM.
3. **WGSL Custom Shader Validation & GPU Watchdog (`color_nodes_gpu.rs`)**:
   - In the current repo, `clip.add_custom_shader` stores shader strings inertly. When Phase 3 compiles custom shaders onto `wgpu`, parse every shader through `naga` first, reject shaders with unbounded loops or excessive texture bindings, and wrap `queue.submit()` in a `wgpu::ErrorScope` so a faulty shader returns a clean `CommandError` instead of crashing the Wayland/X11 compositor.
4. **Out-of-Process Sandbox for Third-Party CLAP/LV2/VST3 Audio Plugins (`clap_host.rs`)**:
   - Native C/C++ audio plugins can segfault or allocate memory on the real-time audio thread. Host third-party plugins in a dedicated `saturn-plugin-worker` child subprocess communicating over a lock-free shared-memory audio pipe; if a plugin crashes, the worker restarts cleanly without taking down `saturn-tauri` or corrupting the `.saturn` project.

## 11. Multi-Agent Governance, Hard Rules & Task-Card Templates (`AGENTS.md` + `docs/agent-task-card.md`)

When multiple AI coding agents work concurrently on Easy Edits Pro, every agent must obey the institutional rules defined in `AGENTS.md` and `docs/agent-task-card.md`.

### 11.1 Ten Non-Negotiable Institutional Rules for Coding Agents

1. **Query `prowl` First**: Never grep blindly or read entire 700-line files just to find a symbol. Always run `prowl overview`, `prowl search "<concept>"`, `prowl find <symbol>`, `prowl def <symbol>`, and `prowl impact <path>` first.
2. **One Lane Per Agent / Packet**:
   - **Lane 1 (UI / Glass)**: `apps/saturn-tauri/ui/src/styles/`, `ui/src/features/panels/`, `scripts/check-*`, `tests/visual`, `tests/a11y`.
   - **Lane 2 (Engine / Parity)**: `crates/saturn-core/`, `crates/saturn-render/`, `crates/saturn-perception/`, `crates/saturn-audio-dsp/`, `ui/src/features/<domain>`.
   - **Lane 3 (Release / Verification)**: `.github/`, `packaging/`, `apps/saturn-tauri/src/gpu_detect.rs`, `tests/native/`.
   - **Lane 4 (Tooling / Docs / Safety / MCP)**: `scripts/check-parity.mjs`, `docs/`, `crates/saturn-mcp/`, `apps/saturn-tauri/src/mcp_bridge.rs`, `apps/saturn-tauri/src/write_grants.rs`, `agent-harness/`.
3. **Never Hand-Edit Generated Files**: `docs/COMMANDS.md`, `docs/reference/command-catalogue.json`, `docs/PARITY.md`, `docs/glass-tokens.md`, `Cargo.lock`, and `pnpm-lock.yaml` must only be updated by running `pnpm catalogue:sync` (or the token script) and committing the regenerated output.
4. **Strict 800-Line Source Ceiling**: Every `.rs`, `.ts`, `.tsx`, and `.css` file must stay `<= 800` lines (`pnpm --dir apps/saturn-tauri check:source-size --max-lines=800`). Pre-split oversized legacy files (`crates/saturn-render/src/lib.rs` at 1,483 lines and `crates/saturn-mcp/src/server.rs` at 818 lines) before adding new functions.
5. **Design Tokens & Glass Discipline**: Never hard-code hex/RGB colors, border radii, or spacing (`pnpm check:styles`). Apply `.glass` only through the official surface classes on UI chrome, and **never apply blur/glass over video preview, scopes, or audio meter canvases**. Never use `transition: all`.
6. **Honest Stubs & `shipped: true` Gate**: Any unimplemented command or unsupported parameter must return an explicit `Err(...)`—never a fake `"ok": true`. A command may only be marked `shipped: true` when it has (a) a real Rust engine implementation, (b) a working UI affordance, and (c) an automated test.
7. **State Lives Exclusively in Rust**: The React frontend sends `CommandRequest` payloads and renders returned snapshots; never fork project or undo state in TypeScript.
8. **Hotspot Serialization**: In `crates/saturn-core/src/catalogue/commands/<menu>.rs`, assign at most one agent per `<menu>.rs` file at a time; merge `apps/saturn-tauri/src/main.rs` handler registrations in small sequential commits.
9. **Clean Working Tree Hygiene**: Never commit `.claude/`, `.prowl/`, `.mcp.json`, `test-results/`, or temporary render artifacts.
10. **Honest Documentation**: Release notes, comments, and docs may only claim behaviors verified by automated native tests on real media.

### 11.2 Ready-to-Dispatch Agent Task Cards (Initial Foundation Packets)

Integrators can copy-paste these task cards (formatted per `docs/agent-task-card.md`) directly into coding agent sessions to kick off **Phase 0** and **Phase 1**:

```markdown
# Task card: P0.1 — Tick overflow hardening, catch_unwind bridge guard & undo-drift fix

## Goal
Eliminate all unchecked Tick i64 arithmetic overflows identified in docs/security-audit-report.md, wrap MCP bridge dispatch in catch_unwind so a panic never poisons AppState, and fix the 7 mutating commands that fail to push an undo entry.

## Why now
Phase 0 foundation prerequisite: prevents a single malformed MCP parameter or undo rollback from bricking or corrupting the live editor session.

## Owned paths
- crates/saturn-core/src/time.rs
- crates/saturn-core/src/project/mod.rs
- crates/saturn-core/src/engine/
- apps/saturn-tauri/src/mcp_bridge.rs
- Cargo.toml

## Forbidden paths
- Generated files (docs/COMMANDS.md, docs/reference/command-catalogue.json, docs/PARITY.md, docs/glass-tokens.md)
- apps/saturn-tauri/ui/
- .claude/, .prowl/, test-results/

## Acceptance criteria
- [ ] `caption.cue_nudge` with `delta_ticks` near `i64::MAX` returns `{"error": ...}` over the bridge without panicking or poisoning `AppState`
- [ ] `ProjectDocument::validate()` rejects negative `timeline_start` or `timeline_start.checked_add(duration) == None`
- [ ] `[profile.release]` in root `Cargo.toml` sets `overflow-checks = true`
- [ ] All 7 mutating undo-drift commands (`clip.remove_keyframe`, `project.update_media`, `media.check_missing`, `media.auto_relink`, `media.make_offline`, `media.refresh_changed`, `marker.split`) push undo entries and pass rollback verification in `tests/native/mcp_command_matrix.py`
- [ ] Every modified file is <= 800 lines

## Verification commands
cargo test -p saturn-core
cargo test -p saturn-tauri
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
python3 tests/native/mcp_command_matrix.py
```

```markdown
# Task card: P1.1 — Split saturn-render/lib.rs, add GStreamer frame/sheet extractor & wire media_preview

## Goal
Split `crates/saturn-render/src/lib.rs` (currently 1,483 lines) below 800 lines, implement `crates/saturn-render/src/frame_extractor.rs` for fast GStreamer `appsink` still-frame and NxM contact-sheet extraction, and expose `vision_extract_frame_sheet` + real `media_preview` still frames in `saturn-mcp`.

## Why now
Phase 1 perceptual milestone: gives multimodal AI models visual feedback ("eyes") on imported assets and timeline edits.

## Owned paths
- crates/saturn-render/src/lib.rs
- crates/saturn-render/src/frame_extractor.rs
- crates/saturn-mcp/src/server.rs
- crates/saturn-mcp/src/perceptual_tools.rs
- apps/saturn-tauri/src/mcp_bridge.rs

## Forbidden paths
- apps/saturn-tauri/ui/
- Generated catalogue files

## Acceptance criteria
- [ ] `crates/saturn-render/src/lib.rs` and `crates/saturn-mcp/src/server.rs` are both <= 800 lines
- [ ] `media_preview` with `extract_poster_tick` extracts a real PNG frame into `$XDG_RUNTIME_DIR/easy-edits-pro/previews/` and returns `"still_frame": {"available": true, "path": "..."}`
- [ ] `vision_extract_frame_sheet` renders a 4x4 tiled contact sheet with timecode burn-in from `apps/saturn-tauri/tests/fixtures/editor-smoke.webm`
- [ ] Caller-supplied `output_path` values are validated through `enforce_write_target`

## Verification commands
cargo test -p saturn-render -p saturn-mcp -p saturn-tauri
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
```

## 12. Five-Tier Testing & Native Verification Architecture

Per `docs/native-workflow.md` and `AGENTS.md`, browser smoke tests (`pnpm test:e2e`) run against a mocked Tauri IPC backend and a synthetic video—they **never** prove that GStreamer, the Unix socket bridge, GPU shaders, or the Rust command engine work on real media. Every phase must pass all five verification tiers:

| Tier | Test Suite & Command | What It Verifies | When to Run |
|---|---|---|---|
| **Tier 1: Rust Unit & Property Tests** | `cargo test --workspace` | Pure engine math (`Tick` rational arithmetic, ripple/trim invariants, undo/redo stacks, EBU R128 K-weighting filters, LUT parsing, `WriteGrants` path normalization). | On every file save / commit in Lane 2 & Lane 4. |
| **Tier 2: Frontend, Glass & Guardrail Gates** | `pnpm --dir apps/saturn-tauri build && pnpm test && pnpm check:glass && pnpm check:commands && pnpm check:source-size --max-lines=800 && pnpm test:e2e` | TypeScript compilation, WCAG contrast + design-token enforcement (`check:glass`), 100% UI-to-catalogue command parity (`check:commands`), 800-line file limit, and React UI contract (`test:e2e`). | On every commit in Lane 1 & before any PR merge. |
| **Tier 3: Python Agent Harness Suite** | `python3 -m pytest agent-harness/tests -q` | Subprocess CLI verbs (`cli-anything-eep --json ...`), `saturn-control` daemon lifecycle, and real MCP stdio initialize/tool-call handshakes against `saturn-mcp`. | Whenever `saturn-cli`, `saturn-mcp`, `saturn-control`, or `agent-harness` changes. |
| **Tier 4: Live MCP Command-Matrix Torture Suite** | `python3 tests/native/mcp_command_matrix.py` | Launches the real `saturn-tauri` binary inside an isolated `Xvfb :99` session (`HOME`/`XDG_*` sandboxed), executes all 440+ commands over `$XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock`, runs `edit.undo` after each mutating command, and verifies zero state-signature drift and zero panics. | At the end of every Engine/MCP packet (Phases 0–7). |
| **Tier 5: Unmocked Native Media & Package Runbooks** | `python3 tests/native/laptop_workflow.py` · `python3 tests/native/write_policy_workflow.py` · `python3 tests/native/installed_package_workflow.py` · `python3 tests/native/ai_closed_loop_workflow.py` | End-to-end unmocked proof: real `ffprobe` ingest of `editor-smoke.webm`, `.saturn` disk persistence, live MCP edits, `write_grants` security rejection of unauthorized paths, real GStreamer H.264/WebM export, mid-render cancellation cleanup, and closed-loop scope/LUFS verification. | Before marking any milestone complete or cutting an alpha/beta `.deb`, `.rpm`, or Flatpak release. |
