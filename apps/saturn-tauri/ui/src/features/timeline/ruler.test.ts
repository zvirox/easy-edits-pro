import test from "node:test";
import assert from "node:assert/strict";
import { rulerSteps } from "./ruler.ts";

test("rulerSteps picks wide label steps when zoomed out", () => {
  // 0.5 px/s: 300 s = 150px is the first step over 110px.
  assert.deepEqual(rulerSteps(0.5), { major: 300, minor: 50 });
  // 10 px/s: 15 s = 150px labels.
  assert.deepEqual(rulerSteps(10), { major: 15, minor: 3 });
});

test("rulerSteps keeps minor ticks at least 12px apart", () => {
  const { major, minor } = rulerSteps(100);
  assert.equal(major, 2);
  assert.equal(minor, 1);
  assert.ok(minor * 100 >= 12);
});

test("rulerSteps returns 1s majors when zoomed in", () => {
  const { major, minor } = rulerSteps(400);
  assert.equal(major, 1);
  assert.equal(minor, 1);
});

test("rulerSteps tolerates zero and negative zoom", () => {
  assert.deepEqual(rulerSteps(0), { major: 3600, minor: 3600 });
  assert.deepEqual(rulerSteps(-5), { major: 3600, minor: 3600 });
});
