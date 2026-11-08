# Render cut-boundary caps regression

**Date:** 2026-11-08. **Observed baseline:** `b57814f`.

## Ubuntu CI failure

[CI run 42465005391](https://github.com/zvirox/easy-edits-pro/actions/runs/42465005391)
passed all four previously stalled audio-edit tests in 6.65 seconds, then failed
the native beat-grid assertion: frame 74 was blue instead of red, although the
edited cut was at frame 75. Licences and the Python harness passed.

The same failure reproduced with Ubuntu 24.04 GStreamer 1.24 runtime libraries
in a rootless sandbox. This used the host-built test executable and host
loader/libc/libm; it was a runtime comparison, not a complete Ubuntu build.
Independent full-stream decoding confirmed a rendered displacement, rather than
an inaccurate still-frame seek:

| Export | Expected cuts | Before correction | After correction |
|---|---|---|---|
| Original | 82, 139 | 81, 139 | 82, 139 |
| Beat-quantized | 75, 135 | 74, 135 | 75, 135 |

Both corrected Ubuntu files contained 360 frames at 30 fps, with video duration
12 seconds, red starting at frame 30 and black starting at frame 210.

## Cause and correction

The compositor emitted the final pre-cut buffer at 2.466666666 seconds and the
new clip at 2.5 seconds. The `videorate:6` trace showed encodebin's frame-rate
converter receiving a caps change at the cut: `pixel-aspect-ratio=1/1` was added
to otherwise identical caps. GStreamer 1.24 cleared its pending pre-cut buffer
and duplicated the new clip backward to the previous output timestamp.

`ges_timeline::build_timeline` now declares square-pixel aspect ratio alongside
the project width, height and frame rate. This matches the project's existing
pixel-based geometry and keeps that field stable across gaps and clips. The
corrected trace had one initial caps negotiation and no PAR change at the cut.

The native beat-grid test still checks exact edited cuts, outer edges, audible
unchanged music, undo and negative controls. It additionally checks both sides
of the original cuts. The final test passes on the host GStreamer 1.28.8 hardware
path and the Ubuntu 1.24 software path. Temporary fixture-retention code was
removed. Timestamp conversion and encoding-profile experiments are not part of
this correction.

Final local gates passed: 583 workspace tests plus four cold-process scenarios,
45 desktop tests, 174 frontend tests, UI build, glass/token/style gates,
commands, source size, catalogue/parity drift, SPDX headers and all nine mocked
E2E areas with zero browser errors. The final workspace-built native beat-grid
test also passed under the Ubuntu runtime. Document links, added-line character
sweep and whitespace checks passed.

## Open follow-up: GStreamer 1.28 software cadence

Broader diagnostics forced `RenderSettings::use_hardware_encoding = false` on
the host. The original export retained green at frame 210 instead of black;
full decoding found black at 211 and 361 total frames. The edited export had
the expected visible edges but also 361 total frames. This is distinct from
the Ubuntu caps regression and remains unresolved.

**Baseline control:** removing the PAR correction and forcing software encoding
still failed the original-frame-210 assertion, with mean RGB
`[0.5180555555555556, 255.0, 2.0]`. Thus this defect predates the correction.
The 1.28 trace showed a final green fragment at 6.999999999 seconds, followed by
black at 7 seconds, plus chroma-site renegotiation. Simple timestamp rounding
or bypassing encodebin's frame-rate conversion did not consistently preserve
both exact cut indices and frame counts, so those experiments were discarded.

To reproduce with the beat-grid fixture on a 1.28 software host, render both
documents using `render_project_range_with_settings` with hardware encoding
disabled, and assert that `rgb(&old, 210)` is black. Independently decode all
video frames and require 360 frames with the transitions in the table above.
This extra diagnostic is not covered by the default hardware-host gate; passing
that gate does not close the software-cadence follow-up.
