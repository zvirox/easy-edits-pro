# Installed Flatpak MCP bridge stops responding after `media_preview`

**Status:** Reproduced once during a manual smoke test; root cause not yet known  
**Date:** 2026-11-08  
**Severity:** High for MCP bridge workflows; the desktop process remained alive

## Environment

- Installed app: Flatpak `io.github.zvirox.EasyEditsPro`, version `0.1.0-alpha.26`
- Desktop: KDE Plasma on Wayland
- MCP client: workspace `saturn-mcp` built locally; MCP server reported version `0.1.0`
- Bridge socket: `/run/user/1000/easy-edits-pro-mcp.sock`
- Media: `apps/saturn-tauri/tests/fixtures/editor-smoke.webm` (6 seconds, VP8, 640×360, 30 fps)

The MCP client and installed app have different version strings, so compatibility should be checked before attributing this solely to the Flatpak build.

## Steps to reproduce

1. Start the installed Flatpak and connect `saturn-mcp --bridge` to its private socket.
2. Initialize MCP and call `doc_inspect`; it returned the clean, unsaved `Untitled project` document.
3. Call `command_run` with `project.new`, then save the project under the app's allowed data directory.
4. Import `editor-smoke.webm` with `project.add_media` (returned `media_id: 3`).
5. Call `media_search` for `editor-smoke.webm`; it returned the imported asset.
6. Call `timeline.insert_clip` for media 3 on track 1, from tick 0 for 1,524,096,000,000 ticks; it returned clip index 0.
7. Call `media_preview` with `{"media_id":3}`.
8. After the MCP client timed out at 60 seconds without a result, call `doc_inspect` again.

## Expected

`media_preview` returns the media metadata and a poster frame (or an explicit unavailable reason). Subsequent bridge calls continue to respond.

## Actual

- `media_preview` produced no MCP result within 60 seconds.
- The follow-up `doc_inspect` produced no result within 10 seconds.
- The Flatpak process and Unix socket listener were still present afterward; this was not an observed process crash.

## Evidence and limits

- MCP process-capture evidence for the timed-out preview: `ev_72395a5bb3bb559a386efdea9716691e4e50cdabf2e05546f1c82f736f11d`
- MCP process-capture evidence for the subsequent `doc_inspect` timeout: `ev_93903e41ef103e799ff7c5c681a9f60202bcfb62ff40bcdea6d872431a6b551b`
- Earlier `doc_inspect`, `project.new`, `file.save`, `project.add_media`, `media_search`, and `timeline.insert_clip` calls returned successfully.
- The last successful project save preceded media import and timeline insertion. Those later edits were not verified as saved.
- No root cause is established. The result points to a bridge request that blocks later bridge work, but a client/server version mismatch or preview decoding path may also contribute.
