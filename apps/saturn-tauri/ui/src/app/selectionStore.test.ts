// F1: the selection store's contract. The store keeps the legacy orthogonal
// setState semantics (primary clip / multi-selection) while guaranteeing the
// primary clip is always part of the multi-selection and both clear together.
import test from "node:test";
import assert from "node:assert/strict";
import {
  getClipSelection,
  setPrimaryClip,
  setMultiClips,
  pickFirstTimelineClip,
} from "./selectionStore.ts";

function reset() {
  setPrimaryClip(null);
}

test("selecting a clip collapses the selection to that clip", () => {
  reset();
  setPrimaryClip({ track_id: 1, clip_index: 0 });
  assert.deepEqual(getClipSelection(), { clip: { track_id: 1, clip_index: 0 }, clips: [{ track_id: 1, clip_index: 0 }] });
});

test("clearing the primary clip clears the multi-selection", () => {
  reset();
  setMultiClips([{ track_id: 1, clip_index: 0 }, { track_id: 1, clip_index: 1 }]);
  setPrimaryClip(null);
  assert.deepEqual(getClipSelection(), { clip: null, clips: [] });
});

test("the primary clip within a multi-selection survives setMultiClips", () => {
  reset();
  setPrimaryClip({ track_id: 1, clip_index: 1 });
  setMultiClips([{ track_id: 1, clip_index: 1 }, { track_id: 2, clip_index: 0 }]);
  const selection = getClipSelection();
  assert.deepEqual(selection.clip, { track_id: 1, clip_index: 1 });
  assert.equal(selection.clips.length, 2);
});

test("a primary clip outside a new multi-selection becomes the first entry", () => {
  reset();
  setPrimaryClip({ track_id: 3, clip_index: 2 });
  setMultiClips([{ track_id: 1, clip_index: 0 }, { track_id: 1, clip_index: 1 }]);
  assert.deepEqual(getClipSelection().clip, { track_id: 1, clip_index: 0 });
});

test("setting the same selection is a no-op (stable snapshot)", () => {
  reset();
  setPrimaryClip({ track_id: 1, clip_index: 0 });
  const before = getClipSelection();
  setPrimaryClip({ track_id: 1, clip_index: 0 });
  setMultiClips([{ track_id: 1, clip_index: 0 }]);
  assert.equal(getClipSelection(), before);
});

test("pickFirstTimelineClip picks the first video track's first clip", () => {
  const project = {
    sequence: {
      tracks: [
        { id: 5, kind: "video", clips: [{ id: 100 }, { id: 101 }] },
        { id: 6, kind: "audio", clips: [{ id: 200 }] },
      ],
    },
  };
  assert.deepEqual(pickFirstTimelineClip(project), { track_id: 5, clip_index: 0 });
});

test("pickFirstTimelineClip returns null for an empty or missing sequence", () => {
  assert.equal(pickFirstTimelineClip(null), null);
  assert.equal(
    pickFirstTimelineClip({ sequence: { tracks: [{ id: 5, kind: "video", clips: [] }] } }),
    null,
  );
});
