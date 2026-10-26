import assert from "node:assert/strict";
import { test } from "node:test";
import {
  defaultMarkerRange,
  markerLeftPx,
  markerShiftTicks,
  markerWidthPx,
  nextMarker,
  previousMarker,
  sortedMarkers,
  splitMarkerTarget,
  splitPosition,
} from "./markerList.ts";

const marker = (id: number, start: number, duration = 0) => ({
  id,
  name: `M${id}`,
  kind: "comment" as const,
  start_ticks: start,
  duration_ticks: duration,
  color: "",
});

test("sortedMarkers orders by start tick then id", () => {
  const ids = sortedMarkers([marker(3, 200), marker(1, 100), marker(2, 100)]).map(
    (item) => item.id,
  );
  assert.deepEqual(ids, [1, 2, 3]);
});

test("navigation skips the marker under the playhead", () => {
  const markers = [marker(1, 100), marker(2, 300), marker(3, 500)];
  assert.equal(nextMarker(markers, 100)?.id, 2);
  assert.equal(nextMarker(markers, 0)?.id, 1);
  assert.equal(nextMarker(markers, 500), null);
  assert.equal(previousMarker(markers, 300)?.id, 1);
  assert.equal(previousMarker(markers, 100), null);
});

test("lane geometry converts ticks to pixels at the current zoom", () => {
  const range = marker(1, 500, 250);
  assert.equal(markerLeftPx(range, 100), (500 / 254_016_000_000) * 100);
  assert.ok(markerWidthPx(range, 100) >= 4, "range markers keep a visible minimum");
  assert.equal(markerWidthPx(marker(2, 0), 100), 0, "point markers render a pin");
});

test("default range prefers ordered in/out marks", () => {
  assert.deepEqual(defaultMarkerRange(50, 10, 400), { start: 10, end: 400 });
  assert.deepEqual(defaultMarkerRange(50, 400, 10), {
    start: 50,
    end: 50 + 254_016_000_000 * 2,
  });
  assert.deepEqual(defaultMarkerRange(50, null, null), {
    start: 50,
    end: 50 + 254_016_000_000 * 2,
  });
});

test("markerShiftTicks converts signed frames to signed ticks", () => {
  const rate30 = { numerator: 30, denominator: 1 };
  assert.equal(markerShiftTicks(12, rate30), Math.round((254_016_000_000 / 30) * 12));
  assert.equal(markerShiftTicks(-3, rate30), -Math.round(254_016_000_000 / 30) * 3);
  assert.equal(markerShiftTicks(0, rate30), 0);
  // Non-integer rational rates keep the same rounding the timecode uses.
  const ntsc = { numerator: 30000, denominator: 1001 };
  assert.equal(markerShiftTicks(5, ntsc), Math.round((254_016_000_000 * 1001) / 30000) * 5);
});

test("split markers expose audio/video positions with shared-start fallback", () => {
  const shared = {
    id: 1,
    name: "Sync",
    kind: "comment" as const,
    start_ticks: 1000,
    duration_ticks: 500,
    color: "",
  };
  assert.equal(splitPosition(shared, "audio"), 1000);
  assert.equal(splitPosition(shared, "video"), 1000);
  assert.equal(splitMarkerTarget(shared, "video", "in"), 1000);
  assert.equal(splitMarkerTarget(shared, "audio", "out"), 1500);

  const split = {
    ...shared,
    audio_start_ticks: 700,
    video_start_ticks: 1100,
  };
  assert.equal(splitPosition(split, "audio"), 700);
  assert.equal(splitPosition(split, "video"), 1100);
  assert.equal(splitMarkerTarget(split, "audio", "in"), 700);
  assert.equal(splitMarkerTarget(split, "audio", "out"), 1200);
  assert.equal(splitMarkerTarget(split, "video", "out"), 1600);
});
