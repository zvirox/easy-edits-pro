# Playwright UI smoke suite (`pnpm test:e2e`)

`node tests/e2e/index.mjs` runs the whole editor flow in Chromium against the
Vite dev server.

## The backend is mocked

This suite does **not** exercise the real Rust engine, GStreamer, or the media
server. `mock-backend.mjs` installs an in-page Tauri mock that answers every
`invoke` from an in-memory project document, and the harness routes
`127.0.0.1:1421/media**` to a synthetic fixture
(`tests/fixtures/editor-smoke.webm`). Command dispatch, undo grouping, and UI
state are covered here; real-engine verification (import, edit, export,
`ffprobe` checks) is the separate native workflow test.

Because the mock decides what "the engine" returns, a green run here is a UI
contract, not an engine claim. Do not mark engine features `shipped` in the
catalogue based on this suite alone.

## Module map

Shared pieces:

- `harness.mjs` - Vite server, Chromium page, mocked media route, shared
  `state`/`dialogPrompt`, page-error collection, teardown.
- `mock-backend.mjs` - in-page Tauri mock (project document, undo stack,
  command recording into `window.__commandRequests`).
- `index.mjs` - runs the area modules in the original smoke order and prints
  the JSON summary.

Area modules (execution order):

1. `shell.mjs` - welcome, catalogue-generated menus, tool rail, layout
   contract, sequence tab strip, tool shortcuts.
2. `panels.mjs` - command palette, shortcut rebinding, preferences
   persistence, view/theme commands, workspace switching, layout presets,
   playhead-context for shortcut commands.
3. `geometry.mjs` - plan v4 §9 acceptance: geometry-invariance walk across the
   seven shell workspace modes at 1920/1600/1280 (monitor, toolbox, timeline
   never move or remount), the no-overlap test (no two card surfaces
   intersect), status-bar visibility everywhere, and the sub-1300px
   menu-overflow contract. Deliver is a full-page workspace by user decision
   (2026-10-23, restore of ExportWorkspace) and gets its own mount/status
   bar/menu assert instead of the shell walk.
4. `timeline.mjs` - media import, pointer drop, speed/duration, clip context
   menu, razor/track-select/ripple tools, keyboard trim, three-point insert,
   playhead commands, zoom, scrub, playback, frame stepping.
4. `audio.mjs` - Fairlight channel strips and EQ DSP toggle.
5. `deliver.mjs` - export workspace render, hardware-encoder/format toggle.
6. `media.mjs` - clip pointer move, bins (create/rename/assign/delete), asset
   metadata.
7. `markers.mjs` - M / Shift+M markers, marker lane, Shift+Right navigation,
   markers panel add/delete.
8. `motion.mjs` - `prefers-reduced-motion` contract.

Screenshots land in `/tmp/easy-edits-pro-*.png`.
Keep every file under 400 lines; split by area, never by convenience.
