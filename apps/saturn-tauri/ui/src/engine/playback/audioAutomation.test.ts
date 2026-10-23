import assert from "node:assert/strict";
import test from "node:test";
import {
  AUTOMATION_RECORD_GAP_TICKS,
  clampAutomationValue,
  evaluateAutomation,
  evaluateAutomationLane,
  findPointIndexNear,
  recordLanePoint,
} from "./audioAutomation.ts";
import type { AutomationPoint } from "./types.ts";

test("automation interpolates between timeline points and holds its end values", () => {
  const points = [{ time: 100, value: -12 }, { time: 300, value: 0 }];
  assert.equal(evaluateAutomation(points, 0, -3), -12);
  assert.equal(evaluateAutomation(points, 200, -3), -6);
  assert.equal(evaluateAutomation(points, 600, -3), 0);
});

test("empty or invalid automation falls back to the static parameter", () => {
  assert.equal(evaluateAutomation([], 100, -4), -4);
  assert.equal(evaluateAutomation([{ time: Number.NaN, value: 1 }], 100, -4), -4);
});

test("constant interpolation holds the previous point", () => {
  const points: AutomationPoint[] = [
    { time: 0, value: 0 },
    { time: 1_000, value: 12 },
  ];
  assert.equal(evaluateAutomationLane(points, 500, 0, "constant"), 0);
  assert.equal(evaluateAutomationLane(points, 1_000, 0, "constant"), 12);
});

test("smooth interpolation eases with the half-cosine law", () => {
  const points: AutomationPoint[] = [
    { time: 0, value: 0 },
    { time: 1_000, value: 12 },
  ];
  const quarter = evaluateAutomationLane(points, 250, 0, "smooth");
  const linear = evaluateAutomationLane(points, 250, 0, "linear");
  assert.ok(quarter < linear - 0.5, "smooth lags linear at the quarter");
  assert.ok(
    Math.abs(quarter - 12 * 0.5 * (1 - Math.cos(0.25 * Math.PI))) < 1e-9,
  );
  const middle = evaluateAutomationLane(points, 500, 0, "smooth");
  assert.ok(Math.abs(middle - 6) < 1e-9);
});

test("volume clamp matches the commanded domain", () => {
  assert.equal(clampAutomationValue("volume", 999), 12);
  assert.equal(clampAutomationValue("volume", -999), -60);
  assert.equal(clampAutomationValue("pan", -9), -1);
  assert.equal(clampAutomationValue("pan", 1.4), 1);
});

test("latch record collapses inside the record gap and keeps order", () => {
  const first = recordLanePoint(undefined, 0, 0);
  assert.deepEqual(first, [{ time: 0, value: 0 }]);
  // A write ~2 ms later collapses into the previous point (fresh value wins).
  const closer = recordLanePoint(first, AUTOMATION_RECORD_GAP_TICKS / 2, 4.2);
  assert.equal(closer.length, 1);
  assert.equal(closer[0].value, 4.2);
  assert.equal(closer[0].time, AUTOMATION_RECORD_GAP_TICKS / 2);
  // A later write becomes a new point.
  const later = recordLanePoint(closer, AUTOMATION_RECORD_GAP_TICKS * 2, 8);
  assert.equal(later.length, 2);
});

test("point lookup snaps within the edit window", () => {
  const points: AutomationPoint[] = [
    { time: 0, value: -6 },
    { time: 10_000_000_000, value: 3 },
  ];
  assert.equal(findPointIndexNear(points, 9_500_000_000), 1);
  assert.equal(findPointIndexNear(points, 10_000_000_000), 1);
  assert.equal(findPointIndexNear(points, 4_000_000_000), -1);
});
