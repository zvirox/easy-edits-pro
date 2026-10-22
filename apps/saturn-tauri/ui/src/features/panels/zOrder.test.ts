import assert from "node:assert/strict";
import { test } from "node:test";
import { effectiveZIndex, PANEL_Z_BASE } from "./zOrder.ts";

test("effectiveZIndex is the identity when nothing is pinned", () => {
  const z = { a: PANEL_Z_BASE + 1, b: PANEL_Z_BASE + 2 };
  assert.equal(effectiveZIndex(z, {}), z);
  assert.equal(effectiveZIndex(z, { a: false }), z);
});

test("pinned panels rise above every unpinned panel", () => {
  const z = effectiveZIndex(
    { a: 501, b: 502, c: 503 },
    { a: true },
  );
  assert.ok(z.a > z.b, "pinned a above unpinned b");
  assert.ok(z.a > z.c, "pinned a above unpinned c");
});

test("relative order inside each group is preserved", () => {
  const z = effectiveZIndex(
    { a: 501, b: 502, c: 503, d: 504 },
    { b: true, d: true },
  );
  assert.ok(z.b > z.d === false, "d raised after b keeps priority");
  assert.ok(z.d > z.b, "d raised after b stays above b");
  assert.ok(z.b > z.a, "pinned pair above unpinned a");
  assert.ok(z.d > z.c, "pinned pair above unpinned c");
});

test("pinning never lowers a panel that is already on top", () => {
  const raised = { pinned: 900, other: 501 };
  const z = effectiveZIndex(raised, { pinned: true });
  assert.ok(z.pinned > z.other);
  assert.ok(z.pinned >= 900, "already-raised pinned panel does not move down");
});

test("unpinned panels keep their z untouched", () => {
  const z = effectiveZIndex({ a: 501, b: 502 }, { b: true });
  assert.equal(z.a, 501);
});

test("panels missing from the z map fall back to the base", () => {
  const z = effectiveZIndex({ b: 502 }, { a: true });
  assert.ok(z.a > z.b);
});
