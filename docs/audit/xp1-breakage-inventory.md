# XP1 — Tier 2 build breakage inventory

Baseline: `main` @ `8a4e76f` (2026-11-25), after Phase 5 and Phase 6 closed.
The CI check matrix lives in `.github/workflows/cross-platform-check.yml`.
Tier 1 (Linux) keeps the full gate stack in `.github/workflows/ci.yml`.

## Method

XP1 adds `cargo check --workspace --all-targets` on `macos-latest` and
`windows-latest`. It is a discovery job: a red result is the deliverable, not a
release regression. Tier 2 support begins only when XP2–XP4 turn these jobs
green and add install proofs.

## Static inventory (from source, before CI results)

| Area | Unix-specific code / dependency | Porting packet |
|---|---|---|
| Control bridge | Unix domain sockets in `saturn-mcp/server.rs`, `saturn-control/uds.rs`, desktop `mcp_bridge.rs` | XP3 (named pipe or authenticated loopback) |
| Plugin sandbox | `/dev/shm` rings + UDS in `saturn-audio-dsp` / `saturn-plugin-worker` | XP2/XP3 |
| Plugin trust paths | `$HOME/.clap`, `/usr/lib/clap` in `saturn-core::project::dsp_trust` | XP2 |
| Preview cache | `$XDG_RUNTIME_DIR/easy-edits-pro/previews` in `saturn-render` | XP2 |
| GPU preview | Linux DMA-BUF import in `saturn-render::gpu_context` | XP2 (copy fallback) |
| Hardware encode | VA-API detection in `apps/saturn-tauri/src/gpu_detect.rs` | XP2/XP4 |
| Native deps | GStreamer + GES via pkg-config; brew/Choco setup in the XP1 jobs | XP1/XP2 |
| Rust conditional deps | `[target.'cfg(target_os = "linux")'.dependencies]` in `apps/saturn-tauri/Cargo.toml` | XP4 |

## CI results

| Runner | Run | Result | First root cause |
|---|---|---|---|
| macOS (`macos-latest`) | [45577028503](https://github.com/zvirox/easy-edits-pro/actions/runs/45577028503) | FAIL (exit 101) | `E0432: unresolved import wgpu::hal::vulkan` at `crates/saturn-render/src/gpu_context.rs:17` — the Vulkan HAL module is compiled out on macOS |
| Windows (`windows-latest`) | [45577028503](https://github.com/zvirox/easy-edits-pro/actions/runs/45577028503) | FAIL (exit 1) | `ort-sys v2.0.0-rc.10` build script panics at `build.rs:126` (`Os { code: 3, NotFound }`) while staging DirectML.dll; ONNX Runtime prebuilt setup is Windows-incomplete |

Earlier runs in this sequence exposed two setup issues that are now fixed in
the workflow: a missing `pkg-config` on the Windows runner, and the GStreamer
MSI installing under `C:\Program Files\gstreamer\...` rather than
`C:\gstreamer\...`. Those were runner setup, not product defects.

### Triage

- **macOS:** `gpu_context.rs` imports the Linux Vulkan backend unconditionally.
  XP2 must move backend selection into `saturn-platform` (Metal on macOS,
  DirectX 12 on Windows) with a copy fallback where DMA-BUF is unavailable.
- **Windows:** `ort-sys` cannot stage its DirectML dependency in this runner.
  XP2/XP3 need a deterministic ONNX Runtime provisioning path and a Windows
  shared-library layout before the perception crate compiles.
- The static inventory above remains the roadmap for the remaining Unix paths
  (UDS bridge, `/dev/shm` sandbox, XDG dirs, CLAP trust paths, VA-API).

XP1 gate is **not green** — that is the expected discovery result. It closes
only when XP2–XP4 make both jobs green; no Tier 2 support is claimed here.
