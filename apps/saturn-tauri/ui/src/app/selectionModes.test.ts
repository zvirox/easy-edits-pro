import assert from "node:assert/strict";
import { test } from "node:test";
import {
  expandLinkedClip,
  isLinkedSelection,
  isSelectionModeCommand,
  isSelectionFollowsPlayhead,
  sequenceAutoSelectIdentity,
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

// E.4 seam: F1 auto-select across projects. Every fresh project's default
// sequence has id 1, so a guard keyed on the bare sequence id meant opening a
// second project never auto-selected (Inspector docks came up dead). The key
// must mix in the document identity so the same sequence id in another
// document produces a different key.

test("sequenceAutoSelectIdentity separates two projects sharing sequence id 1", () => {
  const first = sequenceAutoSelectIdentity("/a/movie.saturn", "Movie A", 1);
  const second = sequenceAutoSelectIdentity("/b/movie.saturn", "Movie B", 1);
  assert.notEqual(first, second, "project B's sequence 1 must not match project A's");
});

test("sequenceAutoSelectIdentity is stable for the same document and sequence", () => {
  const key = sequenceAutoSelectIdentity("/a/movie.saturn", "Movie A", 1);
  assert.equal(sequenceAutoSelectIdentity("/a/movie.saturn", "Movie A", 1), key);
  // A different sequence id in the same document re-arms the auto-select
  // (switching tabs should pick that sequence's first clip).
  assert.notEqual(sequenceAutoSelectIdentity("/a/movie.saturn", "Movie A", 2), key);
});

test("sequenceAutoSelectIdentity distinguishes an unsaved draft from a saved path", () => {
  assert.notEqual(
    sequenceAutoSelectIdentity(null, "Draft", 1),
    sequenceAutoSelectIdentity("/tmp/draft.saturn", "Draft", 1),
  );
});
