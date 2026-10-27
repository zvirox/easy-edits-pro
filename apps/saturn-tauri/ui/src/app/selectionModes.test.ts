import assert from "node:assert/strict";
import { test } from "node:test";
import {
  expandLinkedClip,
  isLinkedSelection,
  isSelectionModeCommand,
  isSelectionFollowsPlayhead,
  setLinkedSelection,
  setSelectionFollowsPlayhead,
  toggleSelectionModeCommand,
} from "./selectionModes.ts";

const project = {
  sequence: {
    tracks: [
      { id: 1, kind: "video", clips: [{ group_id: 7 }, { group_id: null }] },
      { id: 2, kind: "audio", clips: [{ group_id: 7 }] },
      { id: 3, kind: "video", clips: [{ group_id: 9 }] },
    ],
  },
};

test("isSelectionModeCommand covers exactly the two Sequence-mode ids", () => {
  assert.ok(isSelectionModeCommand("sequence.linked_selection"));
  assert.ok(isSelectionModeCommand("sequence.selection_follows_playhead"));
  assert.ok(!isSelectionModeCommand("sequence.settings"));
  assert.ok(!isSelectionModeCommand("clip.set_enabled"));
});

test("toggleSelectionModeCommand flips each flag and reports a notice", () => {
  setLinkedSelection(false);
  setSelectionFollowsPlayhead(false);

  assert.equal(toggleSelectionModeCommand("sequence.linked_selection"), "Linked selection on: selecting a clip selects its link group");
  assert.ok(isLinkedSelection());
  assert.equal(toggleSelectionModeCommand("sequence.linked_selection"), "Linked selection off");
  assert.ok(!isLinkedSelection());

  assert.equal(toggleSelectionModeCommand("sequence.selection_follows_playhead"), "Selection follows the playhead");
  assert.ok(isSelectionFollowsPlayhead());
  assert.equal(toggleSelectionModeCommand("sequence.selection_follows_playhead"), "Selection no longer follows the playhead");
  assert.ok(!isSelectionFollowsPlayhead());

  assert.equal(toggleSelectionModeCommand("sequence.settings"), "");
});

test("expandLinkedClip returns the whole link group with all members", () => {
  assert.deepEqual(expandLinkedClip(project, { track_id: 1, clip_index: 0 }), [
    { track_id: 1, clip_index: 0 },
    { track_id: 2, clip_index: 0 },
  ]);
});

test("expandLinkedClip returns null for unlinked clips, empty projects, and out-of-range rows", () => {
  assert.equal(expandLinkedClip(project, { track_id: 1, clip_index: 1 }), null);
  assert.equal(expandLinkedClip(project, { track_id: 3, clip_index: 0 }), null);
  assert.equal(expandLinkedClip(null, { track_id: 1, clip_index: 0 }), null);
  assert.equal(expandLinkedClip(project, { track_id: 99, clip_index: 0 }), null);
});
