# Quantize cuts to music

Use **Sequence > Trim > Quantize Cuts to Music...**. Choose the video track,
music reference track, subdivision and strength. All internal cuts on that
video track participate. The strip's first start and last end stay fixed.

- `quarter`: nearest detected pulse.
- `half`: every two pulses, anchored to a known bar start.
- `bar`: every four pulses, anchored to a known bar start.
- Strength `0` keeps the cuts; `1` moves them to the nearest grid point.
  Intermediate values blend the displacement before rounding to project frames.
- Half and bar require a known downbeat position in timeline seconds in the
  menu, or `downbeat_tick` in automation. They assume **4/4 meter**. The supplied
  position must be within 50 ms or half a project frame (whichever is larger)
  of a measured pulse. The detector does not recognize meter or downbeats.

## Automation and transaction

Call the headless or desktop-bridge MCP tool `timeline_beat_grid_quantize`:

```json
{
  "music_track_id": 2,
  "video_track_id": 1,
  "subdivision": "quarter",
  "strength": 0.75
}
```

Preparation decodes and measures outside the editor lock. The core command
`timeline.beat_grid_quantize` applies a prepared grid and project digest in one
undoable transaction. Direct core callers must supply `expected_digest` and
`grid: {origin, period, beats}` in addition to the public parameters; the MCP
preparation tool is the normal automation route. Preparation and core application
both validate every proposed cut. A changed project rejects stale preparation.
No-op results preserve undo/redo. Save/reopen retains ordinary clip timings;
there is no schema change or generated media.

The response contains `applied` (changed-cut count, old/new ticks and deltas) and
`telemetry` (tempo, supported grid span, onset count, matched onsets, normalized
periodicity, strength-weighted onset support, beat occupancy and RMS timing error).
These are measured fit statistics, not musical-quality confidence scores.

## Supported material and bounds

- An unlocked video track with **2–128** enabled, contiguous, frame-aligned
  video clips spanning at most **120 seconds**, at least one frame each.
- No gaps, overlaps, transitions, retiming, link groups, compound/multicam
  clips, effects, keyframes, tracking, node graphs or motion blur. Static clip
  colour and gain settings are retained. Unlock any parent track group.
- Quantize **before detaching sound**. Original embedded audio follows its video
  clip's rolled boundaries. Use J/L editing afterward for separate sound cuts.
- One enabled mono/stereo music clip on an audio track covering the entire strip,
  with known source duration and no transitions, effects, processing, automation
  or plugins. Analysis uses the raw first audio stream mixed to mono; clip/track
  gain, pan, mute/solo and bus/master processing are not analyzed.
- Analyze **4–120 seconds**, adding up to two seconds per side when available.
  If that exceeds 120 seconds, use the strip's range. Decode at 16 kHz with the
  source start rounded forward to a sample boundary and mapped back to timeline
  time. Missing/truncated source files report an error.
- Cuts must lie within the supported pulse span. The grid does not extrapolate
  through leading/trailing silence. At least six supported pulses at **60–200
  BPM** are required.

Each boundary is planned from the original snapshot. Equal-distance targets
prefer the earlier pulse. Frame-displacement half ties round away from zero.
Every clip's `source_in - timeline_start` stays constant, preserving which source
frame maps to each retained timeline position. Colliding cuts, crossed outer
edges or insufficient source handles reject the entire edit. Reduce strength,
choose a finer subdivision, or provide more source handles to resolve these errors.

## Detector limitations

The model-free detector extracts RMS rises at 10 ms hops, refines attack sample
positions, autocorrelates the attack train, then fits a constant-period grid with
strength-weighted regression. It rejects low periodicity, insufficient supported
attacks, low grid occupancy, large timing residuals and long internal gaps.
Syncopation, weak attacks, swing, changing tempo, stereo phase cancellation and
half/double-tempo ambiguity can defeat this energy heuristic. A steady click
fixture is evidence of timing, not a benchmark of general music understanding.
Listen to the result and undo if the chosen pulse level is unsuitable.

## Verification

Local verification on 2026-11-08: workspace 582 passed (one existing ignored
test), desktop 45 passed (one existing ignored test), frontend 174 passed;
build, glass/style, catalogue/drift, parity, source-size and SPDX gates passed.
All nine browser E2E areas passed with zero browser errors; the menu screenshot
was inspected. Native beat-grid decode/render assertions ran without skipping.

- `saturn-perception/src/beat_grid_tests.rs`: tempo endpoints, gain changes,
  two-minute drift, silence, sparse and irregular attacks.
- `saturn-core/tests/beat_grid.rs`: subdivisions, supplied phase, ties, partial
  strength, fractional frame rates, source mapping, atomic collision/handle
  rejection, stale work, no-op history, undo/redo and save/reopen.
- `saturn-render/tests/beat_grid.rs`: real generated video and WAV, offset music
  source, deterministic preparation, changed encoded cut frames versus the
  original negative control, fixed outer edges and unchanged audible music.
- MCP tests exercise headless measurement/application and exact bridge forwarding.
- Menu unit tests cover parameters, cancellation and bad input. Browser E2E
  verifies the menu IPC and analysis-error display against the mocked backend.
