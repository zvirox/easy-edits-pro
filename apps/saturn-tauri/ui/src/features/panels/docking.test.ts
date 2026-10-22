import assert from "node:assert/strict";
import { test } from "node:test";
import {
  DOCK_ZONE_THRESHOLD_PX,
  dockZoneAt,
  zonePixels,
  zoneRect,
} from "./docking.ts";

const region = { width: 1280, height: 720 };

test("dockZoneAt resolves edge-proximate pointers", () => {
  assert.equal(dockZoneAt(10, 360, region), "left");
  assert.equal(dockZoneAt(1270, 360, region), "right");
  assert.equal(dockZoneAt(640, 8, region), "top");
  assert.equal(dockZoneAt(640, 712, region), "bottom");
});

test("dockZoneAt returns null in the middle of the region", () => {
  assert.equal(dockZoneAt(640, 360, region), null);
  assert.equal(dockZoneAt(400, 300, region), null);
});

test("dockZoneAt honours the threshold", () => {
  const justOutside = DOCK_ZONE_THRESHOLD_PX + 1;
  assert.equal(dockZoneAt(justOutside, 360, region), null);
  assert.equal(dockZoneAt(DOCK_ZONE_THRESHOLD_PX, 360, region), "left");
});

test("dockZoneAt resolves corners by the closer axis", () => {
  // Top-left corner: horizontal (left) distance equals vertical (top).
  assert.equal(dockZoneAt(4, 4, region), "left");
  // 40px from the left edge, 8px from the top: vertical wins.
  assert.equal(dockZoneAt(40, 8, region), "top");
  // 8px from the left edge, 40px from the top: horizontal wins.
  assert.equal(dockZoneAt(8, 40, region), "left");
});

test("dockZoneAt degenerates safely on an empty region", () => {
  assert.equal(dockZoneAt(0, 0, { width: 0, height: 0 }), null);
});

test("zoneRect covers the matching half in fractions", () => {
  assert.deepEqual(zoneRect("left", region), {
    x: 0,
    y: 0,
    width: 0.5,
    height: 1,
  });
  assert.deepEqual(zoneRect("right", region), {
    x: 0.5,
    y: 0,
    width: 0.5,
    height: 1,
  });
  assert.deepEqual(zoneRect("top", region), {
    x: 0,
    y: 0,
    width: 1,
    height: 0.5,
  });
  assert.deepEqual(zoneRect("bottom", region), {
    x: 0,
    y: 0.5,
    width: 1,
    height: 0.5,
  });
});

test("zonePixels covers the matching half in pixels", () => {
  assert.deepEqual(zonePixels("left", region), {
    x: 0,
    y: 0,
    width: 640,
    height: 720,
  });
  assert.deepEqual(zonePixels("bottom", region), {
    x: 0,
    y: 360,
    width: 1280,
    height: 360,
  });
});
