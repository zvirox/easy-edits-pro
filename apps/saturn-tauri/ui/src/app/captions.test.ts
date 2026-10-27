import assert from "node:assert/strict";
import test from "node:test";
import { resolveCaptionTrackId } from "./captions.ts";

// E.4 seam: caption solo across sequence switches. The editor holds
// activeTrackId while documents change; a stale id sent to the engine turns
// "Show Active Track Only" into hide-all/show-nothing (the engine matches
// unknown ids against no track and no-ops, pinned in tests/captions.rs).

const tracks = (ids: number[]) => ids.map((id) => ({ id }));

test("resolveCaptionTrackId keeps a live active track", () => {
  assert.equal(resolveCaptionTrackId(7, tracks([7, 3])), 7);
});

test("resolveCaptionTrackId falls back to the first track when the active id is stale", () => {
  // Sequence switch: track 2 existed in the previous sequence only.
  assert.equal(resolveCaptionTrackId(2, tracks([5, 9])), 5);
});

test("resolveCaptionTrackId resolves null to the first track", () => {
  assert.equal(resolveCaptionTrackId(null, tracks([4])), 4);
});

test("resolveCaptionTrackId returns null when the sequence has no caption tracks", () => {
  assert.equal(resolveCaptionTrackId(2, []), null);
  assert.equal(resolveCaptionTrackId(2, null), null);
  assert.equal(resolveCaptionTrackId(2, undefined), null);
});
