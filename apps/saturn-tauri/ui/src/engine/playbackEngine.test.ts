import test from "node:test";
import assert from "node:assert/strict";
import {
  TICKS_PER_SECOND,
  solveCubicBezier,
  getActiveSubtitle,
  interpolateKeyframe,
  computeTransitionBlend,
  type Clip,
  type Transition,
  type SubtitleItem,
} from "./playbackEngine.ts";

test("solveCubicBezier calculates standard curve values", () => {
  // Boundary values
  assert.equal(solveCubicBezier(0, 0, 0, 1, 1), 0);
  assert.equal(solveCubicBezier(1, 0, 0, 1, 1), 1);

  // Linear easing
  const midLinear = solveCubicBezier(0.5, 0, 0, 1, 1);
  assert(Math.abs(midLinear - 0.5) < 0.01, `Expected ~0.5, got ${midLinear}`);

  // Ease-in (slow start, fast end)
  const midEaseIn = solveCubicBezier(0.5, 0.42, 0, 1, 1);
  assert(midEaseIn < 0.5, `Ease-in at 0.5 should be < 0.5, got ${midEaseIn}`);
});

test("getActiveSubtitle locates active subtitle item by playhead", () => {
  const subs: SubtitleItem[] = [
    { id: 1, start_ticks: 0, end_ticks: TICKS_PER_SECOND * 2, text: "First Line" },
    { id: 2, start_ticks: TICKS_PER_SECOND * 3, end_ticks: TICKS_PER_SECOND * 5, text: "Second Line" },
  ];

  // At 1s: First line
  const at1s = getActiveSubtitle(subs, TICKS_PER_SECOND);
  assert.equal(at1s?.id, 1);
  assert.equal(at1s?.text, "First Line");

  // At 2.5s (gap): null
  const atGap = getActiveSubtitle(subs, Math.floor(TICKS_PER_SECOND * 2.5));
  assert.equal(atGap, null);

  // At 4s: Second line
  const at4s = getActiveSubtitle(subs, TICKS_PER_SECOND * 4);
  assert.equal(at4s?.id, 2);
  assert.equal(at4s?.text, "Second Line");

  // Undefined subtitles returns null
  assert.equal(getActiveSubtitle(undefined, 0), null);
});

test("interpolateKeyframe evaluates static value or animated keyframes", () => {
  // Case A: Clip without keyframes falls back to default
  const plainClip: Clip = {
    media_id: 10,
    timeline_start: 0,
    duration: TICKS_PER_SECOND * 5,
    source_in: 0,
    speed: 1.0,
    color: { brightness: 1, contrast: 1, saturation: 1, hue: 0 },
    keyframes: [],
  };

  assert.equal(interpolateKeyframe(plainClip, "opacity", 0, 1.0), 1.0);

  // Case B: Clip with linear keyframes
  const keyframedClip: Clip = {
    ...plainClip,
    keyframes: [
      { property: "opacity", time: 0, value: 0, interpolation: "linear" },
      { property: "opacity", time: TICKS_PER_SECOND * 2, value: 1.0, interpolation: "linear" },
    ],
  };

  // At start
  assert.equal(interpolateKeyframe(keyframedClip, "opacity", 0, 1.0), 0);
  // At midpoint (1s)
  const midOpacity = interpolateKeyframe(keyframedClip, "opacity", TICKS_PER_SECOND, 1.0);
  assert(Math.abs(midOpacity - 0.5) < 0.05, `Expected ~0.5, got ${midOpacity}`);
  // At end
  assert.equal(interpolateKeyframe(keyframedClip, "opacity", TICKS_PER_SECOND * 2, 1.0), 1.0);
});

test("computeTransitionBlend evaluates crossfade and wipe progress", () => {
  const trans: Transition = {
    id: 1,
    cut_at: TICKS_PER_SECOND * 5,
    duration: TICKS_PER_SECOND * 2, // from 4s to 6s
    kind: "crossfade",
  };

  // Before transition: progress 0
  const before = computeTransitionBlend(trans, TICKS_PER_SECOND * 3);
  assert.equal(before.progress, 0);

  // Exact cut point (5s): progress 0.5
  const mid = computeTransitionBlend(trans, TICKS_PER_SECOND * 5);
  assert.equal(mid.progress, 0.5);
  assert.equal(mid.deckAStyle.opacity, 0.5);
  assert.equal(mid.deckBStyle.opacity, 0.5);

  // After transition: progress 1.0
  const after = computeTransitionBlend(trans, TICKS_PER_SECOND * 7);
  assert.equal(after.progress, 1.0);
});
