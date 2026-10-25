# Changelog

All notable changes to **Easy Edits Pro** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-alpha.16] - 2026-10-25

Twelfth and final parity fulfilment task: the planned-command list
reaches **zero**. The last three entries — `workspace.library`,
`workspace.ai`, `workspace.export` — were placeholder switchers for
workspaces that do not exist (`DavinciWorkspace` has no such values,
and the generic `workspace.*` menu interception would have bricked the
router on first use). They are purged under the G2 precedent, with the
phase2 purge test extended so the ids resolve to the standard
unknown-command error. The real surfaces are served by shipped
switchers: the Media pool is the library, Export is the Deliver
workspace, and AI lives in the transcription and media-pool flows.

### Campaign summary (Tasks 5–12, alphas 5–16)
The three-way audit (engine dispatch + UI affordance + test) has now
verified or dispositioned every command in the catalogue:

| Metric | alpha.4 (start) | alpha.16 (now) |
|---|---|---|
| Shipped commands | 115 | **210 of 210 in catalogue (0 planned)** |
| Tier-1 coverage | 15% | **31% (138/443)** |
| Tier-2 coverage | 8% | **9% (19/207)** |
| Tier-3 coverage | 0% | **18% (3/17)** |

Three dynamic-dispatch finds (InteropModal, the dead speech flows, the
already-wired `shortcuts.set`), two dead-flow repairs, and one purge
with regression test. **Beta 1 planning is now unblocked** — the
original gate was closing the Tier-1 command gaps.

---

## [0.1.0-alpha.15] - 2026-10-25

Eleventh parity fulfilment task: caption styling and engine-backed
shortcut sync. `caption.style_set` gains a style section in the
Captions panel (size, fill, stroke, and background with Apply — first
save creates the track's reusable style, later saves update it), and
`shortcuts.set` is flipped as a dynamic-dispatch case: the
ShortcutsDialog editor and its sync hook were already dispatching it;
only the catalogue flag was stale. The catalogue stands at 213
commands with 210 shipped and 3 planned; both commands are Tier-1, so
Tier-1 coverage moves 136 → 138/443 (31%).

### Added
- **Caption style section.** The Captions panel gains a compact style
  bar — font size, fill / stroke / background colours, stroke width,
  and Apply — saved to the selected track through `caption.style_set`.
  Graphics > Captions > Set Caption Style opens the panel.
- **New engine test** `caption_style_workflow_with_undo`: rejection of
  unknown tracks, empty names, and unknown style ids; create with
  engine defaults filling omitted fields; partial update semantics;
  undo walking back both the update and the creation.

### Fixed
- The ShortcutsDialog doc comment claimed overrides lived only in
  localStorage; they are mirrored there and synced to engine settings
  (`settings.json`) through `shortcuts.set` / `shortcuts.list`.

---

## [0.1.0-alpha.14] - 2026-10-25

Tenth parity fulfilment task: media metadata re-probe. The media seam
deferred since Task 5 is closed — the importer's GStreamer probe is
now exposed as a Tauri command and wired to `project.update_media`,
which ships with a Media pool affordance and the File menu. The
catalogue stands at 213 commands with 208 shipped and 5 planned;
because this is a Tier-1 project-family command, Tier-1 coverage
moves 135 → 136/443 (31%).

### Added
- **Update metadata.** The Media pool toolbar gains an Update button
  (next to Scan, enabled when an asset is selected): it re-probes the
  selected file through the same Discoverer pass the import flow uses
  and applies the fresh kind/duration/dimensions to the engine, whose
  None-means-keep rule preserves existing waveforms. File > Update
  Media Metadata routes to the same operation.
- **Rust seam.** New `probe_media_for_update` Tauri command — it
  returns probe results for the engine dispatch, so project state
  stays owned by the core through the normal command pipeline.

---

## [0.1.0-alpha.13] - 2026-10-25

Ninth parity fulfilment task: the compositing Nodes panel. All six
`nodegraph.*` commands are shipped behind a real graph editor —
draggable node cards with SVG connection wires, per-node
enable/remove, a connect row with socket fields, per-edge disconnect,
and a roto-spline mask list. The Effects menu now renders in the menu
bar (it ships the nodegraph commands). The catalogue stands at 213
commands with 207 shipped and 6 planned; nodegraph is an extension
family with no FilmCraft tier, so tier coverage is unchanged (Tier-1
135/443, 30%; Tier-2 19/207, 9%).

### Added
- **Nodes panel** (Edit and docked workspaces): a dotted canvas where
  nodes are placed by the engine, dragged (position commits through
  `nodegraph.update_node` on drop), toggled enabled/disabled, and
  removed (the engine cleans up attached edges). Connections are
  drawn between the affected cards and listed below with one-click
  disconnect; Add Mask creates a four-point rectangular matte.
- **Menu dispatch.** Effects > Nodes items route to the hook: add
  node/mask dispatch immediately; the node- and edge-specific
  commands open the Nodes panel.
- `nodegraph_disconnect_workflow_with_undo` engine test covering the
  previously untested disconnect command (unknown edge reports
  `false`, real edge removal, undo restores).
- The e2e shell contract now asserts the Effects menu renders with
  shipped commands (previously it asserted the opposite).

---

## [0.1.0-alpha.12] - 2026-10-25

Eighth parity fulfilment task: live multicam editing. The multicam
monitor in the preview already had an angle grid, keyboard cuts, and a
Sync Waveforms button, but its callbacks were stubbed no-ops. They now
drive the real engine commands, and `multicam.create`,
`multicam.switch_angle`, and `multicam.sync` are shipped. The catalogue
stands at 213 commands with 201 shipped and 12 planned; multicam is a
Tier-2 family, so Tier-1 coverage is unchanged at 135/443 (30%) and
Tier-2 moves to 19/207 (9%).

### Added
- **Live cuts.** Clicking an angle tile in the monitor (or pressing
  1–9) switches the selected timeline clip's multicam angle via
  `multicam.switch_angle`; the engine links an unlinked clip to the
  group containing that angle. A notice asks for a timeline selection
  when none exists.
- **Waveform sync.** The Sync Waveforms button cross-correlates the
  first multicam group's angle waveforms (via the
  `sync_multicam_by_waveform` Tauri command) and applies the resulting
  per-angle offsets through `multicam.sync`; unsyncable angles fall
  back to zero offset with an honest notice.
- **Creation flow.** Clip > Multicam > Create Multicam Clip builds a
  group from the project's video/audio media assets (up to nine
  angles) and dispatches `multicam.create`; it is guarded when fewer
  than two assets are imported.

---

## [0.1.0-alpha.11] - 2026-10-25

Seventh parity fulfilment task: multi-track caption management. The
three `caption.track_*` commands were verified (engine dispatch +
panel affordance + test) and shipped. The Captions panel now manages
multiple caption tracks — pick, add, rename, remove — and cue editing
operates on the selected track. The catalogue stands at 213 commands
with 198 shipped and 15 planned; Tier-1 coverage is 135/443 (30%).

### Added
- **Caption track bar.** The Captions panel shows every caption track
  with its language, and can add a track ("Captions N"), rename the
  selected one, or remove it — removal is disabled on the last track,
  matching the engine's guard. Cue editing, import, and export all
  apply to the selected track.
- **Menu paths.** Graphics > Captions carries the track commands;
  rename prompts for a name.
- `caption_track_management_workflow_with_undo` engine test: lazy
  first-track creation, add/rename, empty-name and unknown-id
  rejection, undo of both the rename and the add, and the last-track
  removal guard.

---

## [0.1.0-alpha.10] - 2026-10-25

Sixth parity fulfilment task, and a bug-fix release: the speech
feature set (transcription, silence removal, filler removal) has been
dead in the UI since it was built — every flow invoked Tauri command
names that were never registered. This release repairs the seams and
ships the three `speech.*` commands behind them. The catalogue stands
at 213 commands with 195 shipped and 18 planned.

### Fixed
- **Speech flows work for the first time.** The transcription modal,
  silence detection, silence ripple deletion, and filler removal now
  call the registered commands (`transcribe_audio_speech`,
  `detect_timeline_silence`, `ripple_delete_silence`,
  `ripple_delete_fillers`) with the right arguments. Previously every
  one of them failed at runtime with an unknown-command error.

### Added
- **Transcription records what it heard.** The transcribe wrapper now
  returns the word-timed speech segments alongside the subtitle cues,
  and the UI keeps them so filler removal consumes exactly what the
  transcription produced instead of a synthetic guess.
- **Silence analysis uses real media waveforms.** Detection and
  silence ripple feed the first media asset's waveform peaks into the
  engine; without an audio-bearing asset the flows explain what they
  need instead of failing.
- **Menu affordances:** Audio > Speech opens the transcription modal;
  Edit > Speech entries open the speech tools that dispatch
  `speech.ripple_delete_silence` and `speech.ripple_delete_fillers`.
- `speech_ripple_delete_silence_workflow_with_undo` engine test: a 2s
  silent stretch is padding-trimmed to 1.9s, removed by ripple, and
  restored by undo.

### Changed
- `useSpeechOperations.ts` now owns the transcribe/silence modal state
  and handlers (useProjectOperations had passed the 800-line gate).
- Tier-3 coverage rises to 3/17 (speech is a Tier-3 family); Tier-1
  stands at 132/443 (30%).

---

## [0.1.0-alpha.9] - 2026-10-25

Fifth FilmCraft Tier-1 parity fulfilment task: the three small
singulars. `project.inspect`, `marker.ripple`, and `prefs.reset` were
verified (engine dispatch + real UI affordance + test) and shipped.
The catalogue stands at 213 commands with 192 shipped and 21 planned;
Tier-1 coverage reaches 30% (132/443) and the prefs family is fully
closed.

### Added
- **Project JSON viewer.** File > Inspect Project opens a dialog that
  reads the full project document from `project.inspect` — the engine
  source of truth — with a one-click clipboard copy. Useful for
  debugging and for parity comparisons.
- **Marker ripple.** The Markers panel gains a "Shift from playhead"
  control: enter a signed frame count and every marker at or after the
  playhead moves by it (the engine clamps at zero and reports what it
  shifted). Markers > Ripple Markers prompts for the same shift in
  seconds. Frame-to-tick conversion is unit-tested, including NTSC
  fractional rates.
- **Preference reset reaches the engine.** The Preferences dialog's
  Reset flow now also clears the engine's persisted prefs store, not
  just the in-memory state.

### Changed
- Engine tests: `prefs_reset_clears_stored_preferences` (set + reset
  round-trips through the settings file and a reload stays empty) and
  `project_inspect_reflects_edited_document` (the inspect snapshot
  reflects project name, media, and timeline tracks).

### Notes
- Accurate record for alpha.8: the interop importers' Import modal
  (InteropModal via the `import_timeline_interop` Tauri command) already
  dispatched the same engine commands when alpha.8 shipped; alpha.8's
  direct File > Import flow remains a second entry point to them.
- Deferred: `project.update_media` stays planned — it needs a file
  re-probe command exposed to the UI before an honest affordance can
  be built (`probe_media_file` is internal to the Tauri media
  commands today).

---

## [0.1.0-alpha.8] - 2026-10-25

Fourth FilmCraft Tier-1 parity fulfilment task: the interop importers.
The four `interop.*` commands gained their real File > Import UI
affordances and are now shipped — EDL, FCPXML, Premiere XML, and OTIO
timelines can be brought in from the app for the first time. The
catalogue stands at 213 commands with 189 shipped and 24 planned.

### Added
- **File > Import now works end to end.** `interop.import_edl`
  (CMX 3600), `interop.import_fcpxml`, `interop.import_premiere_xml`,
  and `interop.import_otio` open a file dialog with format-specific
  filters, read the chosen file through the sandboxed
  `read_text_file` Tauri command, and hand the contents to the engine
  (the importers parse contents, not paths). EDL and OTIO imports pass
  the filename stem as the project name.
- A successful import resets the editing surface the same way opening
  a project does — selections cleared, playhead to zero — and lands
  the user in the editor with a "Timeline imported" notice. Parse
  failures surface through the standard notice path.

### Changed
- Tier-2 coverage rises to 16/207; the interop family is a Tier-2
  family, so Tier-1 stands at 129/443 (29%).

---

## [0.1.0-alpha.7] - 2026-10-25

Seventh engineering alpha: the third FilmCraft Tier-1 parity fulfilment
task. Twenty-eight commands across the track, bus, media, project, prefs,
shortcuts, and edit families were verified (engine dispatch + real UI
affordance + test) and flipped to shipped. Tier-1 coverage now 29%
(129/443); the catalogue stands at 213 commands with 185 shipped and only
28 planned.

### Changed
- **Mixer and Fairlight wiring is official:** `track.set_gain_db`,
  `set_pan`, `set_muted`, `set_solo`, `set_locked`,
  `set_audio_processing`, and `set_bus_route` (channel-strip faders,
  header toggles, bus routing) plus `bus.set_gain_db` (Fairlight master
  bus fader) are shipped — the UI existed on day one; the catalogue now
  says so.
- **Media pool with folder bins:** `media.list`, `create_bin`,
  `rename_bin`, `delete_bin`, `move_to_bin`, and `update_properties`
  shipped (pool refresh via `media.list` powering the F1 media server).
- **Project plumbing:** `project.status` (the ProjectProvider status
  poll) and `project.switch_sequence` (header sequence switcher).
- **Editing-session gestures:** `edit.begin_gesture` / `edit.end_gesture`
  (the trim and drag gesture brackets used by the undo system).
- **Graphics and captions:** `title.update` (graphics title editor) and
  `subtitle.add` / `update` / `remove` / `clear` (inspector subtitle
  editor).
- **Preferences and shortcuts:** `prefs.get` / `prefs.set` (settings
  sync) and the four shortcut-override commands (`list`, `set`, `reset`,
  `import`).
- **Catalogue hygiene for plumbing commands:** menu-less shipped specs
  gained their natural menus per the validator (`edit.*` gesture pair and
  shortcut overrides under Edit, `project.status` / `media.list` /
  `prefs.*` under File), and params are now documented instead of empty.

### Added
- `shortcuts_import_and_reset_workflow` engine test: import rejects
  unknown command ids before persisting, the imported map survives a
  settings-file reload, and reset round-trips through the same file.

---

## [0.1.0-alpha.6] - 2026-10-25

Sixth engineering alpha: the second FilmCraft Tier-1 parity fulfilment task.
Twelve commands gained their real UI affordances and flipped to shipped —
the **clip and timeline Tier-1 families are now fully closed**, with
Tier-1 coverage reaching 25% (110/443).

### Added
- **Track folder groups become fully manageable:** the new
  `useTrackGroupOperations` hook backs the existing create/collapse/rename
  flows and adds folder deletion (group header × button — tracks stay, they
  just leave the folder) and per-track folder assignment (a folder button on
  every track label). Menu route via prompt-based
  `timeline.assign_track_group` / `timeline.delete_track_group`.
- **Batch timeline editing surfaces:** `timeline.ripple_delete_batch` appears
  in the clip context menu and the Edit menu (selection-aware);
  `timeline.ripple_delete_ranges` works from mark in/out when set (otherwise
  prompts for a range); `timeline.move_clips_batch` moves the selection to
  the playhead preserving relative offsets, and multi-selection drags now
  move all selected clips together through the batch command. New pure
  `menuCommandParams` module builds the parameters.
- **New engine tests:** `ripple_delete_batch` (empty selection rejected;
  two clips removed, middle clip ripples left) and `ripple_delete_ranges`
  (interval cut, tail ripples left, report counters asserted) workflows in
  `command_workflow.rs`.

### Changed
- **Effects list reorder is real:** the up/down buttons in the Effects dock
  and inspector now dispatch `clip.reorder_effects` — both call sites were
  silent no-op stubs before.
- **Custom GLSL shaders round-trip:** the shader editor applies through
  `clip.update_custom_shader` (routed by payload kind); other effect
  payloads persist through `clip.update_effect`, replacing the
  non-catalogue `clip.update_effect_payload` call.
- **Color grade studio persists:** the RGB curves and HSL qualifier editors
  now save through `clip.update_rgb_curves` / `clip.update_hsl_qualifier`
  with live draft state (previously literal empty stubs), and the color
  studio `runCommand` path is wired.
- **Motion tracking panel is honest:** mode pills, stabilization
  smoothness, and pin-target now persist through `clip.set_motion_tracking`
  and `clip.pin_to_track`. Three phantom commands that had no backend
  (`clip.run_motion_tracker`, `clip.set_stabilization_smoothness`,
  `clip.pin_to_motion_track`) are gone. Note: tracking *point analysis*
  still happens at render in a later workstream — `saturn-render` already
  ports the evaluators (`evaluate_tracking_at`, `compute_stabilization_offset`);
  the setup is persisted in projects today.
- Catalogue: 213 commands, **157 shipped / 56 planned**. clip 17/17,
  timeline 35/35 shipped; `docs/COMMANDS.md`, `docs/PARITY.md`,
  `docs/reference/command-catalogue.json` regenerated.

### Known limits
- Motion-tracking point capture (the actual per-frame analysis) is not yet
  computed by the live pipeline; the inspector persists the tracking setup
  for the render stage, where the evaluators are already ported.

---

## [0.1.0-alpha.5] - 2026-10-24

Fifth engineering alpha: the first FilmCraft Tier-1 parity fulfilment task.
Thirty commands across the clip and timeline families were verified against
the verified-before-done rule (engine dispatch, UI affordance, test) and
flipped to shipped, moving Tier-1 coverage from 15% to 22%. No behaviour
change beyond a Clip-menu entry for slip — this alpha makes the catalogue
tell the truth about what the editor already does.

### Changed
- **Clip family fulfilment (0 → 10 shipped of 17 planned):** `clip.set_color`,
  `clip.set_keyframe`, `clip.remove_keyframe`, `clip.add_effect`,
  `clip.remove_effect`, `clip.update_effect`, `clip.toggle_effect`,
  `clip.set_color_wheels`, `clip.set_speed_ramp`, and
  `clip.set_motion_blur` are now shipped — each already had a working engine
  dispatch arm and a real UI affordance (Inspector operations, color docks);
  the missing `remove_keyframe` workflow test covers the nonexistent-time
  no-op (`removed: false`, no mutation) and targeted removal.
- **Timeline family fulfilment (10 → 30 shipped of 25 planned — 20 flipped):**
  `timeline.add_clip`, `timeline.move_clip`, `timeline.trim_clip`,
  `timeline.slip_clip`, `timeline.roll_edit`, `timeline.slide_clip`,
  `timeline.add_track`, `timeline.remove_track`, `timeline.link_clips`,
  `timeline.unlink_clips`, `timeline.create_compound_clip`,
  `timeline.decompose_compound_clip`, `timeline.add_transition`,
  `timeline.update_transition`, `timeline.remove_transition`,
  `timeline.add_title`, `timeline.create_track_group`,
  `timeline.toggle_track_group_collapsed`, `timeline.set_track_group_properties`,
  and `timeline.ripple_delete_gap` are now shipped with existing
  affordances (timeline context menus, trim/slip handles, header menu,
  panels).
- `timeline.slip_clip` gains its `Clip` menu entry so the catalogue validator
  accepts it as shipped (it was already reachable from the trim handles).
- Catalogue is now 213 commands (145 shipped, 68 planned) — `docs/COMMANDS.md`,
  `docs/PARITY.md`, and `docs/reference/command-catalogue.json` regenerated
  via `pnpm catalogue:sync`.

### Remaining in these families (next parity task)
- UI affordances still to build for 12 planned commands: the advanced
  inspector surfaces (`clip.reorder_effects`, motion tracking, pin-to-track,
  RGB curves, HSL qualifier, custom GLSL shaders ×2) plus ripple-delete
  ranges/batch and track-group assign/delete.

---

## [0.1.0-alpha.4] - 2026-10-24

Fourth engineering alpha: the glass-cards overhaul plan v4 is complete —
functional workstream F1–F6 (reactive selection, media relinking, transport,
keyframed exports, async encoder probe, Fairlight 4-band EQ), G-workstream
debt closed (typed per-workspace shell column presets, 1100–1280 right-dock
collapse), and the placeholder plugin-slot system removed.

### Removed
- **Fairlight placeholder plugin slots:** Removed the fake "PLUGIN SLOTS ·
  BUILT-IN EFFECT HOST PENDING" rack from the Fairlight channel strips, and
  the dead `track.add_audio_plugin` / `track.toggle_audio_plugin` commands
  plus the `Track.plugins` schema field behind them (catalogue is now 213
  commands; the real track DSP rack — 4-band parametric EQ, dynamics
  compressor, denoise — is the binding).
- **Collaborative multi-user "Lead Editor" subsystem:** Removed the header
  collab pill and identity, the dead Phase 12.1 modal state, the collab CSS,
  the `collab.acquire_lock` / `collab.release_lock` / `collab.record_change`
  commands and their engine dispatch, the `CollaborationState` project field
  (older projects simply drop the key on next save), and the Tauri
  `*_collaboration_lock` handlers. Catalogue is now 213 commands (113 shipped).

### Added
- **Per-workspace shell column presets & right-dock collapse (G7 debt):** The
  workspace switcher now applies plan §5's fixed column-ratio table
  (Edit/media/effects/fusion/animation `28/48/24`, Color `34/42/24`,
  Fairlight `40/36/24`) through a typed table that drives the shell's dock
  columns; in the 1100–1280 viewport band the right dock card collapses so
  the persistent monitor, toolbox, and timeline keep their room. The
  geometry-invariance walk verifies the preset re-ratio, the collapse band
  boundaries, and that the status-bar and no-overlap invariants hold
  throughout.
- **True 4-band track EQ (F6):** The Fairlight track EQ is a real 4-band
  parametric end-to-end — low shelf (80 Hz), mid bell (1 kHz), a new
  high-mid bell (2.5 kHz), and high shelf (8 kHz) — with the fourth band in
  the schema (backward-compatible default), the preview WebAudio chain, the
  DSP rack UI, and MP4 exports. Export now actually applies track EQ at all
  (a silent invalid-property bug previously dropped it), with failures
  surfaced instead of swallowed.
- **Async hardware encoder probe (F5):** GStreamer hardware-encoder detection
  now runs once at startup on a background job and is cached process-wide;
  the Deliver workspace reads the cached result (or the follow-up event) and
  shows the real encoder readout instead of a permanent "Probing..." placeholder.
- **Keyframes baked into exports (F4):** Clip position, scale, rotation, and
  opacity keyframes now appear in exported MP4s, matching the preview. The
  preview's cubic-bezier evaluator is ported 1:1 to Rust and a new
  `saturnkfvid` GStreamer element warps each frame around centre with
  keyframed opacity (straight alpha), attaching only to clips with visual
  keyframes; verified by live-pipeline tests.
- **Transport & frame export (F3):** The Program monitor transport now has
  Add Marker, Mark In/Out, Lift/Extract (`sequence.lift`/`sequence.extract`
  operating on the marked range — Lift keeps gaps, Extract ripples them
  closed) and Export Frame (single-frame PNG render via GES), plus monitor
  resolution telemetry and a Fit/50–200% zoom dropdown; the playhead
  timecode is accented violet.
- **MCP editing workflow:** Re-enabled `saturn-mcp` in the Cargo workspace with command discovery/execution, project and sequence inspection, media search and asset-placement preview, and asynchronous render jobs with progress and cancellation.
- **Live desktop MCP bridge:** Added a same-user Unix socket bridge so MCP edits use the open desktop project's shared editor state and undo history. Requests are size-limited and the runtime socket directory and socket use restrictive permissions.
- **Fairlight automation playback:** Added interpolated Read-mode volume and pan automation playback, plus a dedicated audio workspace hook and regression tests.
- **Render workflow coverage:** Added render range handling, cancellation/progress coverage, and expanded project command workflow tests.
- **Development plan:** Updated the main README and MCP documentation with verified capabilities, limitations, and the active delivery plan.
- **Control server:** `saturn-control` now builds and exposes `GET /health`, `GET /commands`, `POST /commands/:id/execute`, and a `GET /ws` event stream backed by the shared command registry, with route-level tests.
- **Graphics and project templates:** `saturn-graphics` and `saturn-project` are back in the Cargo workspace, with each type defined once in its owning module and library roots reduced to module declarations and re-exports.
- **CLI command journal:** `cli-anything-eep history` lists the commands the harness executed, stored as JSON lines in the XDG state directory.
- **Full CLI command surface:** Every command id in the engine registry now has a named CLI verb, including timeline split/ripple/slip/slide/roll/rate-stretch, transitions, marks and titles, clip effects/keyframes/motion blur/motion tracking/speed ramps/color wheels/curves/HSL/shader, media bins, project relink, subtitles, node graph, multicam, EDL/FCPXML/OTIO/Premiere import, collaboration locks, speech edits, and bus routing.
- **Managed control daemon:** `daemon start` launches `saturn-control`, records pid/port/log in the XDG state directory, `daemon status` probes `/health`, and `daemon stop` terminates only the recorded process.
- **Real MCP introspection:** `mcp tools`, `mcp resources`, and `mcp prompts` perform an MCP initialize handshake against `saturn-mcp` over stdin/stdout; `mcp start` runs the server in the foreground for a client.
- **Graphics template CLI:** `saturn-cli template create|list|inspect|export` reads and writes the `.sgt` format through `saturn-graphics`, and the harness exposes the same operations as `cli-anything-eep template ...`.
- **Sequence marks:** `timeline.set_mark_in` and `timeline.set_mark_out` are implemented in the engine as undoable sequence state, alongside `timeline.clear_marks`.

### Changed
- **App icon:** Replaced the old camera icon with the EEP hummingbird mark
  (`assets/icons/eep-icon.png`); the full Tauri desktop icon set (32–512 px,
  `.icns`, `.ico`) is generated from it and the header/welcome logos now use
  the same mark.
- **Title bar:** The app-name text next to the header logo is gone — the
  header leads with the brand mark alone (the window/Native title is
  unchanged).
- **CLI undo/redo:** `cli-anything-eep undo` and `redo` now delegate straight to the engine's `edit.undo`/`edit.redo` commands; the duplicate in-process undo manager and its fake inverse replay are gone.
- **Workspace membership:** `saturn-control`, `saturn-graphics`, and `saturn-project` are compiled and tested by `cargo test --workspace` again.
- **Python packaging:** The CLI harness now uses one `pyproject.toml` with a `[project]` table and the correct GPL-3.0-or-later license; `setup.py` and `requirements.txt` are removed, and unused `httpx`/`pydantic` dependencies are dropped.
- **CI workflow:** Renamed `.github/workflows/rust.yml` to `ci.yml`; its Python job now builds `saturn-cli`, `saturn-mcp`, and `saturn-control` before running the harness suite.
- **Warning cleanup:** Removed unused imports and variables across the workspace, replaced deprecated `IndexMap::remove` calls with `shift_remove`, and fixed an unreachable modifier pattern in shortcut parsing. `cargo check --workspace --all-targets` is warning-free.
- Expanded media/project IPC and path validation support, and updated editor smoke coverage for audio and export workspace interactions.
- Formatted and aligned Rust command, timeline, render, and project schema modules with current workspace conventions.

## [0.1.0-alpha.1] - 2026-10-23

First public alpha pre-release of Easy Edits Pro.

### Added

- A Linux Tauri desktop editor with project creation, persistence, recovery,
  media import, multi-track editing surfaces, and Rust-backed undo/redo.
- Media, Edit, Color, Fairlight, Keyframes, Fusion, and Deliver workspaces.
- GStreamer MP4/WebM rendering controls with software-encoder fallback.
- saturn-mcp, saturn-cli, and saturn-control integration with the shared Rust
  command engine.
- GitHub prerelease automation that published .deb, .rpm, and SHA-256 assets.
- Local media-server, path, and network-import hardening.

### Known limitations

- Software x264 output is the verified render path on the development laptop.
  Encoder detection exists, but hardware-encoded output is not verified.
- The published packages have not yet been recorded as installed and launched.
- The Unix desktop MCP bridge has no Windows implementation.
- Advanced Color, Fusion, effects, keyframe, export-queue, Fairlight DSP, and
  VST3-hosting work remains incomplete. See docs/planning.md for the active
  priorities and evidence required before a feature is called complete.
