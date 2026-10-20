import assert from "node:assert/strict";
import test from "node:test";
import { evaluateAutomation } from "./audioAutomation.ts";

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
