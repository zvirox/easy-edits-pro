import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { WORKSPACE_COLUMN_RATIOS, columnRatiosFor } from "./columnRatios.ts";

describe("workspace column-ratio presets (GLASS_CARDS_PLAN §5)", () => {
  it("every shell mode has a preset", () => {
    for (const mode of ["edit", "media", "effects", "fusion", "keyframes", "color", "audio"]) {
      assert.ok(columnRatiosFor(mode), `mode ${mode} must resolve`);
    }
  });

  it("presets are the plan's small fixed table", () => {
    assert.deepEqual(columnRatiosFor("edit"), [28, 48, 24]);
    assert.deepEqual(columnRatiosFor("color"), [34, 42, 24]);
    assert.deepEqual(columnRatiosFor("audio"), [40, 36, 24]);
  });

  it("every preset sums to 100 with the right dock locked at 24%", () => {
    for (const [mode, [left, center, right]] of Object.entries(WORKSPACE_COLUMN_RATIOS)) {
      assert.equal(left + center + right, 100, `${mode} must fill the shell row`);
      assert.equal(right, 24, `${mode} right dock must stay at the plan's 24%`);
      assert.ok(left > 0 && center > 0, `${mode} left/center must stay positive`);
    }
  });

  it("unknown modes fall back to the edit split", () => {
    assert.deepEqual(columnRatiosFor("deliver"), columnRatiosFor("edit"));
    assert.deepEqual(columnRatiosFor("nonsense"), [28, 48, 24]);
  });
});
