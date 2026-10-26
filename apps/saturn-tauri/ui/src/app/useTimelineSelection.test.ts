import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  findGapAtPlayhead,
  stepSelectionIndex,
  TIMELINE_SELECTION_COMMANDS,
} from "./useTimelineSelection.ts";

const tps = 254_016_000_000;

const track = (
  id: number,
  name: string,
  clips: Array<[number, number]>,
  locked = false,
) => ({
  id,
  name,
  locked,
  clips: clips.map(([start, end]) => ({
    timeline_start: start,
    duration: end - start,
  })),
});

test("findGapAtPlayhead returns the bounded gap containing the playhead", () => {
  const tracks = [
    track(1, "V1", [
      [0, 2 * tps],
      [5 * tps, 8 * tps],
    ]),
  ];
  const gap = findGapAtPlayhead(tracks, 3 * tps);
  assert.ok(gap);
  assert.equal(gap.trackId, 1);
  assert.equal(gap.trackName, "V1");
  assert.equal(gap.start, 2 * tps);
  assert.equal(gap.end, 5 * tps);

  // On a clip: no gap.
  assert.equal(findGapAtPlayhead(tracks, 1 * tps), null);
  // Past the last clip: unbounded, not selectable.
  assert.equal(findGapAtPlayhead(tracks, 9 * tps), null);
  // Before the first clip: bounded from zero.
  const head = findGapAtPlayhead(
    [track(2, "V1", [[3 * tps, 4 * tps]])],
    1 * tps,
  );
  assert.ok(head);
  assert.equal(head.start, 0);
  assert.equal(head.end, 3 * tps);
});

test("findGapAtPlayhead skips locked tracks and prefers document order", () => {
  const tracks = [
    track(1, "V1", [[0, 4 * tps]], true),
    track(2, "A1", [
      [0, 1 * tps],
      [2 * tps, 3 * tps],
    ]),
  ];
  const gap = findGapAtPlayhead(tracks, 1.5 * tps);
  // The locked video track fully covers the playhead; the audio track's gap
  // between its clips is the answer.
  assert.ok(gap);
  assert.equal(gap.trackId, 2);
  assert.equal(gap.start, 1 * tps);
  assert.equal(gap.end, 2 * tps);
});

test("stepSelectionIndex clamps and handles the no-selection anchor", () => {
  assert.equal(stepSelectionIndex(-1, 5, true), 0, "no anchor + Next → first");
  assert.equal(stepSelectionIndex(-1, 5, false), 4, "no anchor + Prev → last");
  assert.equal(stepSelectionIndex(0, 5, false), 0, "prev clamps at 0");
  assert.equal(stepSelectionIndex(4, 5, true), 4, "next clamps at end");
  assert.equal(stepSelectionIndex(2, 5, true), 3);
  assert.equal(stepSelectionIndex(2, 5, false), 1);
  assert.equal(stepSelectionIndex(-1, 0, true), -1, "empty timeline");
});

test("TIMELINE_SELECTION_COMMANDS match the shipped ui_only catalogue ids", () => {
  // catalogue:sync exports the catalogue next to docs; the glob-based node
  // runner resolves relative to the package root (apps/saturn-tauri).
  const raw = readFileSync(
    new URL("../../../../../docs/reference/command-catalogue.json", import.meta.url),
    "utf8",
  );
  const catalogue = JSON.parse(raw) as Array<Record<string, unknown>>;
  for (const id of TIMELINE_SELECTION_COMMANDS) {
    const spec = catalogue.find((item) => item.id === id);
    assert.ok(spec, `${id} missing from the command catalogue`);
    assert.equal(spec.uiOnly, true, `${id} must stay ui_only`);
    assert.equal(spec.shipped, true, `${id} must be shipped`);
  }
  assert.deepEqual([...TIMELINE_SELECTION_COMMANDS].sort(), [
    "timeline.select",
    "timeline.selectGap",
    "timeline.selectNextClip",
    "timeline.selectPrevClip",
  ]);
});
