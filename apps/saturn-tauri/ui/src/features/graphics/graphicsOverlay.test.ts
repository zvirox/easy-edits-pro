import assert from "node:assert/strict";
import { test } from "node:test";
import {
  activeGraphics,
  fillToCss,
  overlayBox,
  shadowToCss,
  shapeBackgroundCss,
  textOverlayStyle,
} from "./graphicsOverlay.ts";
import type { GraphicItem } from "../../engine/playback/types";

const graphic = (overrides: Partial<GraphicItem>): GraphicItem => ({
  id: 1,
  name: "G",
  kind: "rectangle",
  start_ticks: 0,
  duration_ticks: 254_016_000_000,
  layer: 0,
  visible: true,
  locked: false,
  transform: {
    x: 0,
    y: 0,
    width: 960,
    height: 540,
    scale: 1,
    rotation: 0,
    anchor_x: 0.5,
    anchor_y: 0.5,
    opacity: 1,
  },
  fill: { color: "#204060", opacity: 1 },
  stroke: null,
  shadow: null,
  text: null,
  points: null,
  ...overrides,
});

const settings = { width: 1920, height: 1080 };

test("activeGraphics filters by time range and visibility, sorted by layer", () => {
  const items = [
    graphic({ id: 2, start_ticks: 100, layer: 1 }),
    graphic({ id: 1, layer: 5, visible: false }),
    graphic({ id: 3, start_ticks: 150, layer: 3 }),
    graphic({ id: 4, start_ticks: -100, duration_ticks: 50 }),
  ];
  const active = activeGraphics(items, 200);
  assert.deepEqual(
    active.map((item) => item.id),
    [2, 3],
  );
  // Half-open range: at exactly start the graphic shows, at start+duration it does not.
  assert.equal(activeGraphics([graphic({ id: 5, start_ticks: 200 })], 200).length, 1);
  assert.equal(
    activeGraphics([graphic({ id: 6, start_ticks: 0, duration_ticks: 200 })], 200).length,
    0,
  );
});

test("overlayBox converts project pixels to frame percentages", () => {
  const box = overlayBox(graphic({ transform: { x: 480, y: 270, width: 960, height: 540 } }), settings);
  assert.equal(box.left, 25);
  assert.equal(box.top, 25);
  assert.equal(box.width, 50);
  assert.equal(box.height, 50);
});

test("overlayBox clamps negative sizes and is layer-ordered", () => {
  const box = overlayBox(
    graphic({
      layer: -3,
      transform: { x: -100, y: 0, width: -50, height: 10 },
    }),
    settings,
  );
  assert.equal(box.zIndex, 7);
  assert.equal(box.width, 0);
});

test("textOverlayStyle scales typography with the frame and honours alignment", () => {
  const style = textOverlayStyle(
    graphic({
      kind: "text",
      text: {
        content: "Title",
        font_family: "DejaVu Sans",
        font_size: 108,
        tracking: 5,
        leading: 1.4,
        align: "center",
        vertical: false,
      },
    }),
    { width: 1920, height: 540 },
  );
  assert.equal(style.fontSize, "20cqh"); // 108 / 540 * 100
  assert.equal(style.textAlign, "center");
  assert.equal(style.lineHeight, "1.4");
  assert.match(style.fontFamily, /DejaVu Sans/);

  const vertical = textOverlayStyle(
    graphic({ kind: "text", text: { content: "縦", font_family: "Inter", font_size: 42, tracking: 0, leading: 1.2, align: "left", vertical: true } }),
    settings,
  );
  assert.equal(vertical.writingMode, "vertical-rl");
});

test("fill, stroke and shadow convert to CSS", () => {
  assert.equal(fillToCss({ color: "#204060", opacity: 0.5 }), "rgba(32, 64, 96, 0.5)");
  assert.equal(fillToCss({ color: "not-a-hex", opacity: 0.5 }), "not-a-hex");
  assert.deepEqual(shapeBackgroundCss(graphic({ kind: "ellipse" })), {
    background: "rgba(32, 64, 96, 1)",
    borderRadius: "50%",
  });
  const shadow = shadowToCss({ color: "#000000", blur: 54, offset_x: 27, offset_y: 0 }, settings);
  assert.match(shadow ?? "", /2\.5cqh 0cqh 5cqh rgba\(0, 0, 0, 0\.75\)/);
});

import { CREATE_DEFAULT_SIZES, creationBox, draggedBox, projectPoint } from "./graphicsOverlay.ts";

test("creationBox centres new graphics on the click point", () => {
  const box = creationBox("rectangle", 0.5, 0.5, settings);
  assert.equal(box.x, 960 - CREATE_DEFAULT_SIZES.rectangle.width / 2);
  assert.equal(box.y, 540 - CREATE_DEFAULT_SIZES.rectangle.height / 2);
  assert.equal(box.width, 400);
  assert.equal(box.height, 300);
});

test("projectPoint maps pointer events into project pixels", () => {
  const stage = { left: 100, top: 50, width: 960, height: 540 };
  const point = projectPoint(580, 320, stage, settings);
  assert.equal(point.x, 960); // (580-100)/960*1920
  assert.equal(point.y, 540); // (320-50)/540*1080
});

test("draggedBox moves freely and resizes with a minimum size", () => {
  const start = { x: 100, y: 100, width: 200, height: 100 };
  assert.deepEqual(draggedBox(start, 30, -50, "move"), { x: 130, y: 50, width: 200, height: 100 });
  const grown = draggedBox(start, 500, -400, "resize");
  assert.deepEqual(grown, { x: 100, y: 100, width: 700, height: 20 });
  const shrunk = draggedBox(start, -400, -400, "resize", 40);
  assert.deepEqual(shrunk, { x: 100, y: 100, width: 40, height: 40 });
});
