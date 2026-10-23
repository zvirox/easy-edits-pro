# Design QA

**Updated:** 2026-10-23

## Scope

This document records UI evidence for the Import, Edit, and Deliver surfaces.
It does not certify media decoding, native-window behaviour, hardware encoding,
or installed packages.

## Current UI state

- Import, Edit, and Deliver are primary destinations. Import presents browse and
  project-media actions; Edit groups the source/program monitors, timeline, and
  inspector; Deliver exposes format, destination, dimensions, frame rate, range,
  and encoder preference.
- The timeline exposes labelled tools and supports the browser-smoke interactions
  for pointer drop, zoom, scrubbing, transport, and clip movement.
- The visual system uses the dark burgundy/deep-purple token palette. Blue/cyan
  comparison language from the old reference is not a description of the
  current design.
- The shared panel and button system is in place, but panel persistence,
  responsive behaviour, visual baselines, and accessibility evidence remain
  planned work.

## Evidence and limits

- Earlier local verification recorded UI build, unit, contrast, command, and
  source-size checks passing.
- The browser smoke suite uses mocked Tauri IPC and a synthetic video. It
  verifies only the frontend contract.
- During the latest recorded GitHub Actions run, build/unit/style/catalogue/
  parity/size checks had passed, while the Python harness job failed and the
  Playwright step had not finished. This is not a final QA pass. Check Actions
  for newer results before making a release claim.

## Open UI work

1. Replace the visible export queue placeholder with real queued job state and
   clear error/cancellation feedback.
2. Complete advanced effects and Color parameter controls, effect ordering,
   keyframe handle editing, and Fusion actions only when they change the Rust
   project state.
3. Add keyboard and screen-reader coverage for workspace navigation, panels,
   timeline tools, dialogs, and export controls.
4. Add visual baseline coverage for the burgundy/purple theme and panel layouts.
5. Run the native laptop workflow for preview, import, timeline, save/reopen,
   and render after major UI changes.

## Acceptance rule

A UI capability is complete only when its affordance is understandable, its
action reaches the Rust-owned project model, it has focused regression evidence,
and it reports unsupported work as an error rather than a completed action.
