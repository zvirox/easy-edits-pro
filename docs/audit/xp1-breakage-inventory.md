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

| Runner | Run | Result | Errors |
|---|---|---|---|
| macOS (`macos-latest`) | pending first matrix run | — | — |
| Windows (`windows-latest`) | pending first matrix run | — | — |

Record the first failing compiler/pkg-config lines here verbatim (trimmed to
the actionable first error per root cause), then link the run. Do not record
"build succeeded" as support: XP1 only checks compilation.
