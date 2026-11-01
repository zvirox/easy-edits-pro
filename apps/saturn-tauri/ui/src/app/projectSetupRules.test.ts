import assert from "node:assert/strict";
import { test } from "node:test";
import { assertProjectSetup } from "./projectSetupRules.ts";

test("assertProjectSetup accepts a normal welcome-creation setup", () => {
  assert.doesNotThrow(() => assertProjectSetup("Trip", "Main", 1920, 1080));
  assert.doesNotThrow(() => assertProjectSetup("Trip", "Main", 1, 1));
});

test("assertProjectSetup rejects empty names and out-of-range dimensions", () => {
  assert.throws(() => assertProjectSetup("", "Main", 1920, 1080), /names and dimensions/);
  assert.throws(() => assertProjectSetup("Trip", "", 1920, 1080));
  assert.throws(() => assertProjectSetup("Trip", "Main", 0, 1080));
  assert.throws(() => assertProjectSetup("Trip", "Main", -1, 1080));
  assert.throws(() => assertProjectSetup("Trip", "Main", 16_385, 1080));
  assert.throws(() => assertProjectSetup("Trip", "Main", NaN, 1080));
});
