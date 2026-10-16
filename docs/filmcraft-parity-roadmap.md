# FilmCraft-Informed Development Roadmap

**Status:** planning baseline, 2026-10-16

**Reference:** [storytold/filmcraft](https://github.com/storytold/filmcraft) and the attached `filmcraft-main.zip` snapshot.

## Goal and scope

Use FilmCraft as a reference for the workflows and engineering quality Easy Edits Pro should grow toward: dependable media ingest, editing, playback, effects, titles, audio, export, project recovery, and automation. This is a product roadmap, not a promise to reproduce every FilmCraft feature or achieve Premiere Pro parity in one release. FilmCraft itself describes its project as alpha and its public roadmap is updated frequently; its feature-readiness figures are maintainer estimates, not independent measurements.

Keep Easy Edits Pro on its existing **Tauri 2 + React/TypeScript UI and Rust core**. Use the attached source and FilmCraft documentation to learn from its boundaries, data model, command flow, and test strategy. The archive includes contributor instruction files; they were treated as reference material and were not used as instructions for this project.

## What the source review found

### Easy Edits Pro today

- The application already has a Tauri/React shell, welcome and project-selection flow, media import, project/workspace UI, timeline interactions, program preview, and export UI.
- `saturn-core` is framework-independent and owns a versioned `.saturn` document, project settings, external media references, sequence tracks/clips, exact rational frame timing, mutation commands, and snapshot-based undo/redo. The schema is currently version 4.
- Existing timeline operations include placement, movement, split, slip, remove, ripple-delete, and keyframe/color/gain updates. The model supports video/audio tracks, clip color (exposure, contrast, saturation), and position/scale/rotation/opacity keyframes.
- `saturn-render` builds a GStreamer Editing Services timeline and exports MP4 or WebM. Export applies the current clip color controls and track gain; the root README says keyframe transforms are preview-only.
- The React UI has source/program preview and transport code, timeline zoom/scrubbing, clip dragging, and media-bin pointer drag/drop. Those paths have been reported as not working reliably in prior project requests, so treat them as **implemented but not yet accepted as dependable**.
- The main product gaps visible in the current model/render path are title/graphic clips, transition data and rendering, a reusable ordered effect stack, a compositing preview that agrees with export, richer editing operations, audio mixing/effects, and broad automated media workflow coverage.

### FilmCraft reference

The archive snapshot is a sizable Rust workspace, not just a UI example. It has separate desktop, web/WASM, and CLI applications and crates for time, media, project data, editing, engine commands, rendering, GPU, codecs, interchange, captions, text, audio DSP, and testing. Its documentation describes an engine-first design: UI actions and automation go through one command system, edits use exact tick time, project state is plain data with invariants, and CPU rendering serves as a reference for accelerated rendering. Its project-file docs cover schema migration, atomic saves, autosaves, and crash recovery. Its test docs cover property tests, render comparisons, and scripted UI flows.

The public README describes a wide feature set: multi-track editing and trimming, grading/scopes, effects and transitions, keyframes, audio mixing, graphics/captions, broad codecs and exports, interchange formats, and agent commands. Those feature lists are not evidence that every feature is equally mature. The public status page explicitly calls FilmCraft alpha and separates feature breadth from readiness for real work. Its current roadmap also prioritizes stability, real-world workflow coverage, and playback/format gaps alongside new features.

## Recommended implementation principles

1. **Keep a single source of truth.** Project mutations should go through stable Rust commands, regardless of whether a user invokes them from the Tauri UI, CLI, or automation. Give every new command parameter validation, enabled-state feedback, undo behavior, and persistence.
2. **Extend the `.saturn` schema intentionally.** Add versioned migrations before adding effect stacks, graphics, transitions, markers, or audio routing. Keep project files portable and handle missing/relinked source media explicitly.
3. **Separate editing data from UI state.** Store sequence content and settings in the project; store panel layout, selection, zoom, and similar preferences separately or under an explicitly versioned view section.
4. **Preserve exact time math.** Continue using integer ticks and rational frame rates for edits. Convert to seconds only at media/UI boundaries. Add tests for common fractional frame rates and audio sample boundaries.
5. **Make preview and export share render semantics.** Define one render/effect contract and verify that the program monitor and exported frames agree. Keep GStreamer as the current media/export backend unless measured needs justify a separate compositor; avoid building a custom codec stack as the first step.
6. **Build reliability evidence with each workflow.** Keep a small redistributable fixture corpus and add core, render, and UI acceptance coverage for import → edit → save/reopen → preview → export. Track failures and performance on supported Linux environments.
7. **Treat parity as tested behavior.** Track feature existence separately from quality, supported formats, stability, and usability. Mark features complete only when their end-to-end acceptance criteria pass.

## Phased roadmap

| Phase | Priority | Work | Exit criteria |
|---|---|---|---|
| 0. Reliability baseline | **P0 — in progress** | Fix reported play/pause, seeking, frame stepping, playhead scrubbing, pointer drag/drop, clip movement, and zoom. Add an end-to-end fixture project and repeatable UI workflow. Record known Linux runtime/build requirements and errors. A Playwright smoke test now covers the welcome flow, pointer placement, seek/playback, zoom, and clip movement with mocked Tauri IPC; native backend acceptance remains. | A user can create/open a project, import a short supported clip, place it with the pointer, scrub and play it, save/reopen it, and export a file that opens and plays. Each action has a regression check. |
| 1. Project integrity and editing foundation | **P0** | Add project schema migrations and validation for clip/track invariants; strengthen atomic-save and recovery behavior; stable clip IDs instead of relying on array indexes; multi-selection commands; snapping, markers, keyboard shortcuts, undo grouping; ripple/roll/slide/rate-stretch and trim workflows. | Timeline operations preserve source ranges and durations, undo as expected, and survive save/reopen. Invalid or overlapping edits are rejected or represented explicitly by design. |
| 2. Playback and render consistency | **P0** | Define a frame evaluation path used by both preview and export. Add thumbnail/waveform generation, background decode/render jobs, cancellation, cache invalidation, and a render status/error surface. Establish performance targets from representative media before optimizing. | A fixed timestamp and project state yields visually matching monitor/export frames; seeking and playback remain stable across cuts, gaps, and supported frame rates. |
| 3. Titles, graphics, and transitions | **P1** | Add title/graphic clip types with editable text, font, alignment, fill, stroke/background, transform, opacity, and duration. Add transition objects between clips, beginning with fade/dissolve and audio crossfade. Persist all data and render it in preview and export. | Titles and transitions can be created, edited, moved, saved/reopened, previewed, and exported with matching timing and appearance. Include Unicode and font fallback checks. |
| 4. Effects and color | **P1** | Replace fixed color fields with a typed, ordered effect stack and a registry of effect definitions/parameters. Migrate exposure/contrast/saturation into it. Start with transform, opacity, brightness/contrast, saturation, blur, and crop; add keyframes per parameter and basic scopes later. | Effects can be reordered, enabled/disabled, adjusted, animated, undone, persisted, and rendered identically in monitor/export. Unsupported effects fail clearly rather than silently disappearing. |
| 5. Audio editing and mixing | **P1** | Add clip-level gain/fades, volume automation, waveform display, meters, mute/solo/pan, track routing, crossfades, and a small set of DSP effects. Keep audio timing sample-aware and provide device/playback failure feedback. | A multi-track sequence mixes predictably in playback and export, with saved automation and no audible discontinuities at common edit boundaries. |
| 6. Media management and delivery | **P2** | Add robust relink/offline-media UX, proxies or optimized media if performance measurements require them, export presets/queue/cancel, progress and output validation, additional commonly requested formats, and project collect/consolidate. Add interchange only after core project behavior is stable. | Projects can be moved between machines with media relink/collection; exports report supported settings clearly and can be validated by metadata and playback checks. |
| 7. Product polish and extensibility | **P2** | Improve workspace/layout management, accessibility, shortcuts/preferences, localization foundations, automation/control API, and later plugin or AI integrations. Keep these as clients of the same Rust command layer. | Major editing operations are discoverable and automatable, and preferences/layout survive restarts without changing edit content. |

### Suggested first milestone

Start with Phase 0 and a narrow “short edit” acceptance project, before adding a large effect catalogue:

1. Open the welcome screen and create a project at a chosen resolution and frame rate.
2. Import a short test video and audio file; place both with pointer drag/drop.
3. Scrub to a chosen frame, play/pause, seek, and step one frame in either direction.
4. Split, move, and remove a clip; undo and redo each operation.
5. Save, close/reopen, and confirm media relinks or reports missing files clearly.
6. Export MP4 and WebM and verify duration, dimensions, frame rate, audio presence, and playable output.

The current code already exposes much of this surface, but the user's reported interaction bugs make acceptance and regression coverage the highest-value next work.

## Out of scope for the first parity milestones

Do not begin by implementing FilmCraft's custom codecs, every Premiere effect, GPU compositor, multicamera editing, plugin hosting, `.prproj` import, or every interchange/export format. These are large projects with substantial correctness and support costs. Prioritize proven editing workflows and rendering correctness, then add media or GPU capabilities in response to user needs and profiling.

The attached FilmCraft snapshot includes MIT and Apache-2.0 license files and an attribution inventory, while Easy Edits Pro is GPL-3.0-only. This plan uses FilmCraft as a design and behavior reference; it does not copy its implementation or assets. If direct reuse is considered later, review the specific file's license, attribution requirements, third-party notices, and compatibility with the project's licensing before importing it.

## Research sources

The attached archive (`filmcraft-main.zip`) was inspected at its included **2026-10-16 08:56** file timestamps. These upstream links point to the live `main` branch and may have changed since that snapshot; re-check them before turning roadmap items into implementation issues.

- [FilmCraft README](https://github.com/storytold/filmcraft/blob/main/README.md) — product scope, workflows, supported formats, automation, and stated status.
- [FilmCraft ROADMAP](https://github.com/storytold/filmcraft/blob/main/ROADMAP.md) and [detailed roadmap](https://github.com/storytold/filmcraft/blob/main/docs/roadmap.md) — project stage, current focus, milestones, and progress notes.
- [FilmCraft architecture](https://github.com/storytold/filmcraft/blob/main/docs/architecture.md) — crate layering, command model, time model, data model, playback/render path, and CPU/GPU relationship.
- [FilmCraft project files](https://github.com/storytold/filmcraft/blob/main/docs/project-files.md) — schema evolution, atomic persistence, autosave, recovery, media relink, proxies, and project collection.
- [FilmCraft testing](https://github.com/storytold/filmcraft/blob/main/docs/testing.md) — unit/property/render-oracle/UI tests and performance checks.
- [FilmCraft graphics](https://github.com/storytold/filmcraft/blob/main/docs/graphics.md) and [captions](https://github.com/storytold/filmcraft/blob/main/docs/captions.md) — graphic clip, text, and caption models/workflows.
- [FilmCraft gaps](https://github.com/storytold/filmcraft/blob/main/docs/gaps.md) and [target-app parity](https://github.com/storytold/filmcraft/blob/main/docs/target-app-parity.md) — the maintainers' own caveats and their distinction between feature breadth and readiness. Their estimates are specific to FilmCraft and must not be reused as Easy Edits Pro estimates.
- [FilmCraft MIT license](https://github.com/storytold/filmcraft/blob/main/LICENSE-MIT) and [Apache-2.0 license](https://github.com/storytold/filmcraft/blob/main/LICENSE-APACHE) — licenses present in the reference repository.

## How to keep this plan current

When work starts, turn each phase into small issues with acceptance checks. Update this roadmap when a feature's scope changes or its exit criteria pass. Keep a short status table in the main README, but keep detailed sequencing and technical decisions here. Revisit upstream FilmCraft docs periodically; do not treat a live `main` branch or its reported percentages as a stable specification.
