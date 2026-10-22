import assert from "node:assert/strict";
import { test } from "node:test";
import {
  clampPanelRect,
  sanitizePanelLayout,
  snapEdge,
  snapRect,
} from "./snapping.ts";
import {
  defaultLayout,
  EDIT_DEFAULT_RECTS,
  EDIT_CONSTRAINTS,
  EFFECTS_CONSTRAINTS,
  COLOR_CONSTRAINTS,
  GLOBAL_CONSTRAINTS,
} from "./layouts.ts";
import { DEFAULT_CONSTRAINTS } from "./types.ts";

const viewport1280 = { width: 1280, height: 720 };
const viewport1600 = { width: 1600, height: 900 };

test("default edit layout rects are valid on a 1280x720 viewport", () => {
  const layout = defaultLayout("edit");
  assert.ok(layout);
  for (const [id, rect] of Object.entries(layout)) {
    const clamped = clampPanelRect(rect, viewport1280, EDIT_CONSTRAINTS[id]);
    assert.ok(
      clamped.width >= EDIT_CONSTRAINTS[id].minWidth,
      `${id} keeps min width`,
    );
    assert.ok(
      clamped.height >= EDIT_CONSTRAINTS[id].minHeight,
      `${id} keeps min height`,
    );
    assert.equal(
      clamped.x + clamped.width <= 1,
      true,
      `${id} stays inside the viewport horizontally`,
    );
    assert.equal(
      clamped.y + clamped.height <= 1,
      true,
      `${id} stays inside the viewport vertically`,
    );
  }
});

test("effects, color, and global layouts survive the smallest viewport", () => {
  const suites = [
    { workspace: "effects", constraints: EFFECTS_CONSTRAINTS },
    { workspace: "color", constraints: COLOR_CONSTRAINTS },
    { workspace: "global", constraints: GLOBAL_CONSTRAINTS },
  ] as const;
  for (const { workspace, constraints } of suites) {
    const layout = defaultLayout(workspace);
    assert.ok(layout, `${workspace} has a default layout`);
    for (const [id, rect] of Object.entries(layout)) {
      const clamped = clampPanelRect(rect, viewport1280, constraints[id]);
      assert.ok(
        clamped.width >= constraints[id].minWidth,
        `${workspace}/${id} keeps min width`,
      );
      assert.ok(
        clamped.height >= constraints[id].minHeight,
        `${workspace}/${id} keeps min height`,
      );
      assert.ok(
        clamped.x + clamped.width <= 1 && clamped.y + clamped.height <= 1,
        `${workspace}/${id} stays inside the viewport`,
      );
    }
  }
});

test("clampPanelRect enforces constraints and bounds", () => {
  const huge = {
    x: -0.5,
    y: -0.5,
    width: 2,
    height: 2,
  };
  const clamped = clampPanelRect(huge, viewport1600);
  assert.ok(clamped.width <= DEFAULT_CONSTRAINTS.maxWidth);
  assert.ok(clamped.height <= DEFAULT_CONSTRAINTS.maxHeight);
  assert.equal(clamped.x, 0);
  assert.equal(clamped.y, 0);

  const tiny = { x: 0.5, y: 0.5, width: 0.01, height: 0.01 };
  const grown = clampPanelRect(tiny, viewport1600);
  assert.ok(grown.width >= DEFAULT_CONSTRAINTS.minWidth);
  assert.ok(grown.height >= DEFAULT_CONSTRAINTS.minHeight);
});

test("clampPanelRect keeps a minimised panel anchored at its right edge on resize", () => {
  // Simulate the resize hook: left handle dragged past the right edge.
  const rect = { x: 0.9, y: 0, width: -0.5, height: 0.5 };
  const clamped = clampPanelRect(rect, viewport1600);
  assert.ok(clamped.width >= DEFAULT_CONSTRAINTS.minWidth);
  assert.equal(clamped.x + clamped.width <= 1, true);
});

test("snapRect snaps a dragged rect to the viewport edge within the threshold", () => {
  const viewport = { width: 1600, height: 900 };
  const rect = { x: 10, y: 40, width: 300, height: 400 };
  const snapped = snapRect(rect, { viewport, threshold: 12, margin: 0 });
  assert.equal(snapped.x, 0, "10px from the edge snaps to 0");
  assert.equal(snapped.width, rect.width, "snapping never resizes");
});

test("snapRect ignores an edge beyond the threshold", () => {
  const viewport = { width: 1600, height: 900 };
  const snapped = snapRect(
    { x: 40, y: 40, width: 300, height: 400 },
    { viewport, threshold: 12, margin: 0 },
  );
  assert.equal(snapped.x, 40, "40px from the edge is not a snap");
});

test("snapRect snaps to a neighbouring panel's edge", () => {
  const viewport = { width: 1600, height: 900 };
  const others = [
    { id: "other", rect: { x: 400, y: 0, width: 300, height: 900 } },
  ];
  const snapped = snapRect(
    { x: 395, y: 10, width: 300, height: 400 },
    { viewport, others, threshold: 12, margin: 0 },
  );
  assert.equal(snapped.x, 400, "5px short of a neighbour snaps flush");
});

test("snapEdge moves only the passed edge", () => {
  const others = [{ rect: { x: 800, y: 0, width: 400, height: 900 } }];
  const viewport = { width: 1600, height: 900 };
  assert.equal(snapEdge(795, "left", { viewport, others, threshold: 12 }), 800);
  assert.equal(
    snapEdge(400, "top", { viewport, others, threshold: 12 }),
    400,
    "falls through to viewport margin range",
  );
  assert.equal(snapEdge(8, "top", { viewport, threshold: 12, margin: 0 }), 0);
  assert.equal(
    snapEdge(30, "left", { viewport, threshold: 12, margin: 0 }),
    30,
    "beyond threshold stays",
  );
});

test("sanitizePanelLayout drops unknown panels and clamps known ones", () => {
  const raw = {
    media: { x: 0.01, y: 0.01, width: 0.24, height: 0.9 },
    rogue: { x: 0.1, y: 0.1, width: 0.3, height: 0.8 },
    broken: { x: "abc", y: 0.1, width: 0.3, height: 0.8 },
    empty: { x: 0.1, y: 0.1, width: 0, height: 0.8 },
  };
  const rects = sanitizePanelLayout(raw, ["media", "inspector"], viewport1280);
  assert.deepEqual(Object.keys(rects), ["media"]);
  assert.ok(rects.media.width > 0);
  assert.notDeepEqual(Object.keys(rects), [
    "rogue",
    "broken",
    "empty",
    "media",
  ]);
});

test("sanitizePanelLayout survives non-object payloads from disk", () => {
  assert.deepEqual(
    Object.keys(sanitizePanelLayout(null, ["media"], viewport1280)),
    [],
  );
  assert.deepEqual(
    Object.keys(sanitizePanelLayout("nope", ["media"], viewport1280)),
    [],
  );
  assert.deepEqual(
    Object.keys(sanitizePanelLayout([], ["media"], viewport1280)),
    [],
  );
});

test("default media and inspector defaults mirror the old dock columns", () => {
  const { media, inspector } = EDIT_DEFAULT_RECTS;
  assert.ok(
    media.width > 0.2 && media.width < 0.3,
    "media column stays near 25vw",
  );
  assert.ok(
    inspector.width > 0.18 && inspector.width < 0.28,
    "inspector column stays near 22vw",
  );
});
