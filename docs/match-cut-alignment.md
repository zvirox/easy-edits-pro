# Match-cut alignment (Packet 5.1)

Select the incoming clip of a hard cut, then choose **Sequence > Trim >
Align Match Cut...**. Choose `eye_trace`, `motion`, or `transient`, and a search
radius in project frames. The incoming clip's `source_in` changes in one
undoable edit. Timeline placement, duration, the outgoing clip, and other
project state stay fixed. Attached sound slips with the incoming picture.
Align before creating a J/L split edit or detaching sound.

## Agent entry point

Call the `edit_match_cut_align` MCP tool (headless or desktop bridge):

```json
{"track_id":1,"left_clip_index":0,"right_clip_index":1,
 "mode":"eye_trace","search_frames":12}
```

It returns `applied` (changed, previous/new source position, slip ticks, mode)
and `telemetry` (measurement method, search bounds, candidate count, distances
or attack-envelope mismatch). The desktop menu uses the same preparation.
The raw Rust command `edit.match_cut_align` requires the prepared
`expected_digest` and `matched_source_in` payload. Headless `command_run` and
`command_batch` do not analyze media: use the dedicated tool.

Preparation reads a project snapshot and decodes outside the editor lock.
The core revalidates the snapshot digest, clip state, search bounds, and source
handles before applying the slip. A stale snapshot errors; rerun the tool.
An unchanged result adds no undo checkpoint and preserves redo history.
No analysis artifacts or additional media files are created. Save/reopen
persists the source slip normally.

## What the modes measure

| Mode | Measurement and selection |
|---|---|
| `eye_trace` | **Squared-contrast centroid heuristic**, in normalized screen coordinates. Minimize distance between the outgoing final-frame centroid and each incoming candidate. This measures neither gaze nor subject identity. Uniform frames have no measurement. |
| `motion` | Mean signed block displacement within each shot, normalized by frame width/height. Minimize vector distance; static/textureless shots are excluded. Blocks are 16 pixels with an 8-pixel search radius at analysis resolution. This is bounded block matching, not dense optical flow or subject tracking. |
| `transient` | Positive 5 ms RMS rises in 16 kHz mono PCM, with a -60 dBFS signal floor and rise threshold of 15% of peak RMS. Match the strongest usable outgoing attack in the final 250 ms against incoming attacks using cosine distance of 40 ms post-attack envelopes. Place the chosen incoming onset at the cut. This is a gain-independent envelope heuristic, not instrument recognition or a crossfade. |

Visual analysis normalizes raw source frames to 160x90, sampling at the center
of each project-frame interval. Motion uses two intervals on each side of the
cut. Candidates are whole-project-frame slips. Accurate paused-preroll decoding
avoids dropping past the requested sample. Equal scores prefer the smallest
slip; a remaining equal-distance tie prefers the earlier source position.

Audio analysis uses the first audio stream, mixed to mono. An onset needs a
preceding energy block and 40 ms of following audio, so attacks at the very
start/end of the analyzed range are not always usable. No usable signal
returns an error, rather than a fabricated alignment. Try another mode, a
wider supported search, or a different cut.

Measurements use raw source media. Existing static grading, clip/track gain,
processing, compositing, other tracks, and final perceptual quality are outside
the measurement domain. Inspect the resulting picture and sound before keeping
the edit; undo restores the prior source timing.

## Bounds and supported state

- Default search: 12 frames each side. Accepts 1-30, at most two seconds each
  side, further clamped to available source handles for the full incoming clip.
- Both clips must be enabled, unretimed, unlinked, and adjacent on the same
  unlocked track and track group, with known source durations.
- Remove transitions and overlapping clips in the pair's range. Compounds,
  multicam, detached sound, effects, keyframes, tracking, node graphs, colour
  nodes, and motion blur are unsupported for this operation.
- Visual modes require video clips on a video track; minimum duration is one
  project frame for centroid matching or two for motion. Transient mode accepts
  mono/stereo audio or video sources, with at least 100 ms per clip.
- Source positions must remain within the probed duration. Missing files,
  truncated audio, failed decoding, and stale preparation report errors.
- Visual pulls wait at most 20 seconds each and check a 120-second budget
  between pulls for each source; audio decoding has its existing 120-second
  per-source ceiling. There is no interactive cancellation control.

## Verification

`cargo test -p saturn-core -p saturn-render --test match_cut` covers bounds,
locks, unsupported states, one-step undo/redo, no-op history, stale rejection,
save/reopen, deterministic real-video sampling and repeated preparation.
Synthetic moving video proves selected timing and changed rendered picture;
unmatched exports provide a negative control. Real WAV attacks prove that a
matching envelope beats a closer decoy, places audible energy at the cut, and
rejects silence. No downloaded model or third-party fixture is used.

Perception unit tests cover signed motion, contrast/no-signal cases, attack
boundaries and gain-independent matching. MCP tests cover typed headless
errors and exact bridge forwarding. Browser tests verify the actual menu,
parameters and error display against the mock backend.
