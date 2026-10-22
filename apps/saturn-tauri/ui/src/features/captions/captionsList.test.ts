import assert from "node:assert/strict";
import { test } from "node:test";
import {
  activeCue,
  cueLeftPx,
  cueWidthPx,
  frameTicks,
  nextCue,
  sortedCues,
} from "./captionsList.ts";

const cue = (id: number, start: number, end: number, text = `cue ${id}`) => ({
  id,
  start_ticks: start,
  end_ticks: end,
  text,
});

const TPS = 254_016_000_000; // matches TICKS_PER_SECOND

test("sortedCues orders by start then id", () => {
  const cues = sortedCues([cue(2, 5, 6), cue(1, 5, 7), cue(3, 1, 2)]);
  assert.deepEqual(
    cues.map((item) => item.id),
    [3, 1, 2],
  );
});

test("activeCue finds the cue covering the playhead", () => {
  const cues = [cue(1, 0, 10), cue(2, 10, 20)];
  assert.equal(activeCue(cues, 5)?.id, 1);
  assert.equal(activeCue(cues, 10)?.id, 2);
  assert.equal(activeCue(cues, 20), null, "end is exclusive");
  assert.equal(activeCue(cues, -1), null);
});

test("nextCue skips cues at or before the playhead", () => {
  const cues = [cue(1, 0, 10), cue(2, 20, 30), cue(3, 15, 16)];
  assert.equal(nextCue(cues, 10)?.id, 3);
  assert.equal(nextCue(cues, 16)?.id, 2);
  assert.equal(nextCue(cues, 30), null);
});

test("frameTicks converts frame rate to a nudge step", () => {
  const tps = TPS;
  assert.equal(frameTicks({ numerator: 30, denominator: 1 }), Math.round(tps / 30));
  assert.equal(
    frameTicks({ numerator: 30000, denominator: 1001 }),
    Math.round(tps * 1001 / 30000),
    "NTSC rate uses the exact ratio",
  );
  assert.equal(frameTicks({ numerator: 0, denominator: 0 }), 1, "degenerate rate stays positive");
});

test("cue geometry uses the timeline zoom", () => {
  const tps = TPS;
  // 100 px/s keeps a half-second cue (50 px) above the 34 px clamp.
  const px = cueLeftPx(cue(1, TPS / 2, TPS), 100);
  const width = cueWidthPx(cue(1, TPS / 2, TPS), 100);
  assert.equal(Math.round(px), 50);
  assert.equal(Math.round(width), 50);
  assert.ok(cueWidthPx(cue(1, 0, TPS / 1000), 1) >= 34, "min-width clamp applies");
});
