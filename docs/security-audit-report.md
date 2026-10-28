# Security audit report — Easy Edits Pro

**Date:** 2026-10-28 · **Method:** threat model over the three command surfaces (local MCP
JSONL socket, Tauri IPC, headless `saturn-mcp`), parallel deep-dives into the engine's
file/parse/network surface, the web frontend, and crash/DoS robustness — every
high-severity claim verified in source, and the top finding reproduced live over the
bridge. **Commit audited:** `f76a537` (main).

## Trust boundaries

1. **Local MCP bridge** (`apps/saturn-tauri/src/mcp_bridge.rs`) — Unix socket in
   `XDG_RUNTIME_DIR`, mode `0600`, parent `0700`, 1 MiB request cap, method allowlist.
   Caller = the desktop user (or a local MCP agent acting for them).
2. **Imported documents** — `.eep` project files, FCPXML/XMEML/OTIO/EDL/SRT/CSV from
   the internet. *Less privileged than the user*: code that consumes them must not
   gain filesystem reach the file itself shouldn't have.
3. **Remote content in the webview** — Openverse assets, network-import HTML.
4. **Tauri IPC** — 56 `#[tauri::command]` handlers, minimal capability file, no shell
   plugin, no `withGlobalTauri`.

## Findings

### HIGH — one MCP message bricks the editor: Tick overflow panic in `caption.cue_nudge`

`crates/saturn-core/src/engine/captions_cmds.rs:307-308, 318-319` adds a raw `i64`
param with unchecked `+`:

```rust
let new_start = Tick(cue.start_ticks.0 + params.delta_ticks);  // panicked here
```

**Live reproduction** (debug build over the real bridge, message #4 of the session):

```
command_run caption.cue_nudge {track_id:3, id:4, delta_ticks:5000}
→ thread 'tokio-rt-worker' panicked at captions_cmds.rs:307
→ every later command: "Editor state is unavailable"  (poisoned AppState mutex)
```

The cue was created with legal values (`start_ticks` near `i64::MAX`); the *addition*
overflows. `validate_cue_timing` runs **after** the add, and `ProjectDocument::validate`
never bounds `start_ticks`/`end_ticks` against `i64::MAX - delta`, so both a single
legal param and a crafted `.eep` trigger it. Debug builds panic; release wraps silently
(cue corruption).

**Same unchecked-`+` class** (all reachable via crafted documents; verified in source):

- `crates/saturn-core/src/engine/sequence_cmds.rs:407` — `timeline_start.0 + duration.0`
  in `sequence.inspect` (marker ticks *are* `checked_add`-validated at
  `project/mod.rs:318-324`; clip ticks are not — that asymmetry is the root cause).
- `crates/saturn-core/src/engine/timeline_cmds.rs:267-269, 283` — `ripple_delete_gap`
  gap scans.
- `crates/saturn-core/src/engine/compound_cmds.rs` — `latest_end - earliest_start`
  reachable with a hugely negative crafted `timeline_start`.

**Fix:** `checked_add`/`saturating_add` at these sites *and* validate
`timeline_start >= 0` + `checked_add(duration)` in `ProjectDocument::validate()` — that
kills the whole class at the source. **Amplifier:** no `catch_unwind` anywhere on the
dispatch path (`engine/mod.rs:190-215`, bridge at `mcp_bridge.rs:121`), and no
`[profile]` in any `Cargo.toml` (defaults: `overflow-checks` = panic in dev, silent
wrap in release). Recommend `overflow-checks = true` in `[profile.release]` and a
`catch_unwind` → JSON-RPC error wrapper at the bridge boundary.

### MEDIUM — imported documents adopt local paths into the webview/render allowlist

`.eep`, FCPXML (`fcpxml.rs:104-121`), xmeml (`xmeml.rs:134-141`), OTIO
(`otio.rs:200-230`) store embedded media paths as-is; EDL builds
`PathBuf::from(format!("assets/{}", clip_name))` (`edl.rs:106-110`) — the `assets/`
prefix does **not** neutralize `* FROM CLIP NAME: ../../.ssh/id_rsa`. On open, the
shell registers *every* document media path into the Tauri asset-protocol scope and the
media-server allowlist (`commands/project.rs:70-75` → `main.rs:42-47`), and the
renderer opens them via `filename_to_uri` (`saturn-render/src/lib.rs:848-863`). A
malicious project/interchange file therefore points the app at arbitrary local files
(info disclosure into the app's media pipeline, existence leaks via
`media.check_missing`). Mitigations that exist: media server requires origin + a
random constant-time-compared token, so remote web content can't fetch directly.
**Fix:** canonicalize + reject parent escapes on import; import as *unlinked* assets
requiring per-item user confirmation.

### MEDIUM — quadratic xmeml parser pins the engine mutex

`xmeml.rs:134` splits on `"<clipitem"` into k blocks, but each block spans the entire
document tail, and every `extract_tag_content` re-scans it (`xml_utils.rs:17-24`) —
O(document × clipitem occurrences). A ~50 MB import (the Tauri read cap,
`commands/interop.rs:49`) with maximal tokens forces ~10¹² byte scans *under
`state.0.lock()`*, freezing the UI **and** the MCP bridge for minutes; repeatable over
a 1 MiB-import message too. FCPXML's line scan is linear (clean). **Fix:** extract
tags from the block boundaries (track end index of the split), or cap clipitem count.

### MEDIUM-LOW — engine commands bypass the Tauri path validator (confused deputy)

The Tauri wrapper commands validate paths (`commands/path_validation.rs`, used by
subtitles/markers/interop), but the same operations issued by id on the engine surface
(`file.save`, `file.open`, `interop.export_*`, `shortcuts.export`,
`export.presets.export`) take arbitrary paths with no allowlist
(`project_cmds.rs:6-18,47-70`; `settings_cmds.rs:265-279`; `export_cmds.rs:160-175`) —
`save_atomic` even `create_dir_all`s parents anywhere, and `fs::write` follows
planted symlinks. This is not privilege *escalation* (socket is user-owned) but it
hands **an MCP agent a silent "write any file as the user" primitive** — an LLM
following prompt-injected text inside an imported document can overwrite
`~/.bashrc` with no dialog, no validation. **Fix:** apply the existing
`path_validation` allow-roots inside the engine handlers (or at the bridge boundary),
and require UI confirmation for writes outside the project/app-data roots.

### LOW (hardening)

- **Openverse `source_url` rendered as `href` without scheme validation**
  (`ui/src/features/inspector/InspectorPanel.tsx:392`, data from
  `commands/assets.rs:68-72`; only `media_url` is https-enforced at `assets.rs:99-103`).
  `javascript:` is blocked by today's CSP `default-src 'self'`, but validate http(s)
  in Rust before returning it — CSP drift would convert this into stored XSS.
- **`reveal_path` `xdg-open`s any directory without `validate_read_path`**
  (`commands/interop.rs:96-114`) — inconsistent with every sibling command; only
  "open any folder" impact from a compromised renderer, cheap fix.
- **`file.open` uncapped `read_to_end`** (`project/io.rs:56-60`) — memory DoS only via
  a file the caller already chose; cap alongside the other reads.

### Verified clean (no issue found)

- **No XXE / billion-laughs**: interop "XML" is hand-rolled string scanning; no XML
  crate, entities/DOCTYPE never processed.
- **No secrets, no telemetry, no outbound network from `crates/`**; speech
  transcription is local/synthetic (no whisper runtime, nothing leaves the machine).
- **SSRF posture is strong**: `net.rs` blocks loopback/private/link-local/CGNAT/
  multicast (v4+v6), resolves DNS itself and pins resolved IPs (anti-rebinding),
  manual redirect re-validation, no HTTPS downgrade, streaming 512 MiB cap,
  filename sanitization + extension allowlist.
- **No shell/arg injection**: only fixed-arg spawns; `xdg-open` gets a single
  verified directory arg; renderer uses `filename_to_uri`, not string-built commands.
- **Custom shader code is inert data**: stored in the document, never compiled or
  eval'd anywhere (no wgpu/naga/wgsl compile path exists).
- **Web layer**: zero `dangerouslySetInnerHTML/innerHTML/eval`; all untrusted text
  (captions, markers, names, import metadata) rendered through JSX escaping; no
  `fetch`/WebSocket in the UI; Vite bound to `127.0.0.1` with `strictPort`; minimal
  Tauri capabilities; media server behind origin check + random token.
- **Bridge framing**: 1 MiB pre-extend cap (no memory DoS), serde_json 128-level
  recursion limit, document cycles unrepresentable in JSON, engine traversals
  non-recursive.
- **Panic scan of the `.remove()`/index `.unwrap()` family**: preceded by bounds
  checks — no reachable crash found beyond the Tick-arithmetic class above.

### Known bugs from the command-matrix work (context, already documented)

Eight commands mutate the document without pushing an undo entry (`clip.remove_keyframe`,
`edit.redo`, `project.update_media`, `media.check_missing`, `media.auto_relink`,
`media.make_offline`, `media.refresh_changed`, `marker.split`) — see
`docs/mcp-command-matrix-report.md`. Not security, but state-integrity bugs.

## Priority fix list

1. `captions_cmds.rs:307-319` — `checked_add` on `delta_ticks` (or validate before add).
2. `ProjectDocument::validate()` — non-negative `timeline_start`, `checked_add(duration)`
   (eliminates the `sequence.inspect` / `ripple_delete_gap` / compound triggers).
3. `overflow-checks = true` in `[profile.release]`; `catch_unwind` → per-request error
   at the bridge dispatch.
4. Canonicalize + reject parent-escape in interop importers; import media as
   unconfirmed assets.
5. Reuse `path_validation` allow-roots for engine-level path commands (esp. for
   MCP-agent-driven `file.save`/`export*`).
6. Bounded xmeml block extraction; scheme-validate Openverse `source_url`; put
   `reveal_path` behind `validate_read_path`.

## Resolution (post-audit fixes, commits 9791f89 / 3be00c4 / b52e931 / f41f075)

1. **HIGH — Tick-overflow panic: FIXED.** `ProjectDocument::validate()` rejects
   `timeline_start < 0` and `timeline_start + duration` overflow at load;
   `caption.cue_nudge` uses `checked_add` and returns
   "Nudge would overflow the cue timing"; `sequence.inspect` /
   `timeline.ripple_delete_gap` saturate their gap scans; compound-clip
   duration uses `checked_sub`; `[profile.release] overflow-checks = true`.
   The live PoC now returns a clean error, the app stays responsive, and no
   panic reaches stderr.
2. **Panic containment: FIXED.** `execute_command_with_state` (shared by Tauri
   IPC and the MCP bridge) runs the dispatch under `catch_unwind` inside the
   lock scope: a panicking command now yields `Internal engine error in
   '<id>'` instead of poisoning the state mutex. Belt-and-braces behind the
   per-site fixes above.
3. **MEDIUM — imported media paths: FIXED (the traversal class).** All four
   importers (xmeml `pathurl`, FCPXML `src`, OTIO `target_url`, EDL clip
   names) route through a shared lexical sanitizer: percent-decode first,
   strip `file://`, drop `.` components, collapse interior `..` like OS
   resolution. Relative paths that still climb above their root are rejected
   (asset imports unlinked with an empty path); an import can never store a
   parent-escaping `..` path. **Residual:** absolute paths are still adopted
   as media references (same as user-chosen projects); per-item import
   confirmation remains a product decision, not applied here.
4. **MEDIUM — quadratic xmeml parser: VERIFIED FALSE POSITIVE.** Rust's
   `str::split` already bounds each block between successive `<clipitem`
   occurrences. Measured in-crate: 2k→20k→40k clipitems import in
   3 ms→37 ms→104 ms (release), linear even with adversarial missing-close-tag
   content. No parser rewrite was made; a 20k-clipitem scale guard test was
   added so a future quadratic regression cannot land unnoticed.
5. **LOW — Openverse `source_url`: FIXED.** Only http(s) landing URLs are
   accepted in Rust; `javascript:`/`data:` are dropped before they can ever
   render as an `href`.
6. **LOW — `reveal_path`: FIXED** (differently than suggested). It now
   canonicalizes the directory and requires it to exist before `xdg-open`.
   Deliberately *not* put behind `validate_read_path`: reveal targets
   user-chosen project/media directories that the validator's roots do not
   include.
7. **LOW — uncapped project read: FIXED.** `ProjectDocument::load` rejects
   files beyond 256 MiB (stat up front) and bounds the actual read with
   `take()`.
8. **Matrix undo drift: FIXED.** `project.update_media` now checkpoints only
   on real change (engine, 9791f89). The other seven are accounting, not
   engine bugs: `media.check_missing`/`media.refresh_changed` are
   deliberately undo-blind (pinned by `phase4_media.rs`: "a missing-media
   scan changes flags, not undo history"); `edit.redo`'s undo+redo hook pair
   self-restores; no-op success results (`removed:false`, `relinked:0`,
   `offline:0`, `split:0`, `updated:false`) push no undo entry. The matrix
   harness was corrected (f41f075) and a rerun of all eight commands reports
   `ok=8 … undo_drift=0`.

**Still open (tracked, not fixed here):** priority item 5 — reuse the
`path_validation` allow-roots for engine-level path commands
(`file.save`, `export*`, MCP-agent write surface). This needs a product
decision about the confirmation UX for writes outside allow-listed roots and
is the main residual from the original audit.
