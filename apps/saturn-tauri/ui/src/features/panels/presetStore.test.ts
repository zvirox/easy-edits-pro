import assert from "node:assert/strict";
import test from "node:test";
import {
  createCustomPreset,
  makePresetId,
  parseCustomPresets,
  slugForLabel,
} from "./presetStore.ts";

const layout = {
  rects: { media: { x: 0, y: 0, width: 0.2, height: 0.5 } },
  hidden: {},
};

test("custom preset ids are stable slugs with collision suffixes", () => {
  assert.equal(slugForLabel("My Review Layout"), "my_review_layout");
  assert.equal(makePresetId("My Review Layout", []), "custom_my_review_layout");
  assert.equal(
    makePresetId("My Review Layout", ["custom_my_review_layout"]),
    "custom_my_review_layout_2",
  );
  assert.equal(slugForLabel("!!!"), "preset");
});

test("createCustomPreset scopes the layout to its workspace", () => {
  const preset = createCustomPreset("Rough Cut", "edit", layout, []);
  assert.equal(preset.id, "custom_rough_cut");
  assert.equal(preset.workspace, "edit");
  assert.deepEqual(preset.layouts.edit, layout);
});

test("parsing drops malformed presets but keeps valid ones", () => {
  const valid = createCustomPreset("Rough Cut", "edit", layout, []);
  const parsed = parseCustomPresets(
    JSON.stringify([
      valid,
      { id: "broken" },
      { ...valid, id: "empty", layouts: {} },
      null,
    ]),
  );
  assert.deepEqual(parsed, [valid]);
  assert.deepEqual(parseCustomPresets("not json"), []);
  assert.deepEqual(parseCustomPresets(null), []);
});
