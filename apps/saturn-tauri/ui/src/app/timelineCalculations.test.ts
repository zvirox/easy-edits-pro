import test from "node:test";
import assert from "node:assert/strict";
import {
  calculateFrameTicks,
  snapTimelineTicks,
  calculateClipSplit,
  calculateRippleShift,
} from "./timelineCalculations.ts";
import { getTrackGaps } from "./projectTypes.ts";
import { TICKS_PER_SECOND } from "../engine/playback/constants.ts";

test("calculateFrameTicks computes correct ticks for standard frame rates", () => {
  // 30 fps
  const fps30 = calculateFrameTicks({ numerator: 30, denominator: 1 });
  assert.equal(fps30, TICKS_PER_SECOND / 30);

  // 60 fps
  const fps60 = calculateFrameTicks({ numerator: 60, denominator: 1 });
  assert.equal(fps60, TICKS_PER_SECOND / 60);

  // 24 fps
  const fps24 = calculateFrameTicks({ numerator: 24, denominator: 1 });
  assert.equal(fps24, TICKS_PER_SECOND / 24);

  // Zero / fallback
  const fallback = calculateFrameTicks({ numerator: 0, denominator: 1 });
  assert.equal(fallback, Math.round(TICKS_PER_SECOND / 30));
});

test("snapTimelineTicks snaps accurately to frame grid", () => {
  const frameTicks = calculateFrameTicks({ numerator: 30, denominator: 1 });

  // Right on frame 0
  assert.equal(snapTimelineTicks(0, frameTicks), 0);

  // Slightly off frame 1 (e.g. 1.2 frames) snaps to frame 1
  assert.equal(snapTimelineTicks(frameTicks * 1.1, frameTicks), frameTicks);

  // 1.8 frames snaps to frame 2
  assert.equal(snapTimelineTicks(frameTicks * 1.8, frameTicks), frameTicks * 2);
});

test("snapTimelineTicks snaps to nearby snap points within threshold", () => {
  const frameTicks = calculateFrameTicks({ numerator: 30, denominator: 1 });
  const cutPoint = TICKS_PER_SECOND * 5; // 5 seconds
  const snapPoints = [cutPoint];

  // Near the cut point (within 3 frames)
  const nearby = cutPoint + frameTicks * 1.5;
  assert.equal(snapTimelineTicks(nearby, frameTicks, snapPoints, frameTicks * 3), cutPoint);

  // Far from the cut point (outside threshold) snaps to frame grid, not cut point
  const farAway = cutPoint + frameTicks * 5;
  const snappedFar = snapTimelineTicks(farAway, frameTicks, snapPoints, frameTicks * 3);
  assert.notEqual(snappedFar, cutPoint);
});

test("calculateClipSplit divides clip at valid playhead location", () => {
  const clip = {
    id: 1,
    timeline_start: 0,
    duration: TICKS_PER_SECOND * 10,
    source_in: 0,
  };

  // Split at 4s
  const split = calculateClipSplit(clip, TICKS_PER_SECOND * 4);
  assert.notEqual(split, null);
  assert.equal(split!.left.duration, TICKS_PER_SECOND * 4);
  assert.equal(split!.left.timeline_start, 0);

  assert.equal(split!.right.timeline_start, TICKS_PER_SECOND * 4);
  assert.equal(split!.right.source_in, TICKS_PER_SECOND * 4);
  assert.equal(split!.right.duration, TICKS_PER_SECOND * 6);
});

test("calculateClipSplit returns null if cut is at or outside clip bounds", () => {
  const clip = {
    timeline_start: TICKS_PER_SECOND * 2,
    duration: TICKS_PER_SECOND * 5,
    source_in: 0,
  };

  // Cut before clip start
  assert.equal(calculateClipSplit(clip, TICKS_PER_SECOND), null);
  // Cut at clip start
  assert.equal(calculateClipSplit(clip, TICKS_PER_SECOND * 2), null);
  // Cut at clip end
  assert.equal(calculateClipSplit(clip, TICKS_PER_SECOND * 7), null);
  // Cut after clip end
  assert.equal(calculateClipSplit(clip, TICKS_PER_SECOND * 10), null);
});

test("calculateRippleShift shifts trailing clips by deleted duration", () => {
  const clips = [
    { id: 1, timeline_start: 0, duration: TICKS_PER_SECOND * 4 },
    { id: 2, timeline_start: TICKS_PER_SECOND * 4, duration: TICKS_PER_SECOND * 2 },
    { id: 3, timeline_start: TICKS_PER_SECOND * 6, duration: TICKS_PER_SECOND * 4 },
  ];

  // Ripple delete clip 2 (start: 4s, duration: 2s)
  const remaining = clips.filter((c) => c.id !== 2);
  const shifted = calculateRippleShift(remaining, TICKS_PER_SECOND * 4, TICKS_PER_SECOND * 2);
  assert.equal(shifted[0].timeline_start, 0); // Clip 1 unmoved
  assert.equal(shifted[1].timeline_start, TICKS_PER_SECOND * 4); // Clip 3 shifted left by 2s (from 6s to 4s)
});

test("getTrackGaps finds empty intervals between clips accurately", () => {
  // Empty track -> no gaps
  assert.deepEqual(getTrackGaps([]), []);

  // Continuous clips -> no gaps
  const continuous = [
    { id: 1, timeline_start: 0, duration: 100 },
    { id: 2, timeline_start: 100, duration: 200 },
  ] as any;
  assert.deepEqual(getTrackGaps(continuous), []);

  // Clips with gap at start and middle
  const gapped = [
    { id: 1, timeline_start: 50, duration: 100 }, // gap 0 to 50
    { id: 2, timeline_start: 250, duration: 100 }, // gap 150 to 250
  ] as any;
  const gaps = getTrackGaps(gapped);
  assert.equal(gaps.length, 2);
  assert.deepEqual(gaps[0], { start: 0, end: 50 });
  assert.deepEqual(gaps[1], { start: 150, end: 250 });
});
