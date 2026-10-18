import test from "node:test";
import assert from "node:assert/strict";
import {
  formatTime,
  formatTimecodeFrames,
  formatPan,
  formatDb,
  formatBytes,
  clamp,
  snapToNearest,
} from "./formatters.ts";
import { TICKS_PER_SECOND } from "../engine/playbackEngine.ts";

test("formatTime formats zero and basic seconds", () => {
  assert.equal(formatTime(0), "00:00:00");
  assert.equal(formatTime(TICKS_PER_SECOND), "00:00:01");
  assert.equal(formatTime(TICKS_PER_SECOND * 65), "00:01:05");
  assert.equal(formatTime(TICKS_PER_SECOND * 3661), "01:01:01");
});

test("formatTimecodeFrames formats fractional ticks with frames", () => {
  const fps = { numerator: 30, denominator: 1 };
  assert.equal(formatTimecodeFrames(0, fps), "00:00:00:00");
  // 15 frames at 30 fps is 0.5s = half ticks
  const halfSecTicks = Math.floor(TICKS_PER_SECOND / 2);
  assert.equal(formatTimecodeFrames(halfSecTicks, fps), "00:00:00:15");
  // 1 full second
  assert.equal(formatTimecodeFrames(TICKS_PER_SECOND, fps), "00:00:01:00");
});

test("formatPan formats pan dial values correctly", () => {
  assert.equal(formatPan(0), "C");
  assert.equal(formatPan(0.02), "C"); // within deadzone
  assert.equal(formatPan(-0.03), "C");
  assert.equal(formatPan(-0.5), "L 50%");
  assert.equal(formatPan(-1.0), "L 100%");
  assert.equal(formatPan(0.75), "R 75%");
  assert.equal(formatPan(1.0), "R 100%");
});

test("formatDb formats decibel levels correctly", () => {
  assert.equal(formatDb(-60), "-∞ dB");
  assert.equal(formatDb(-100), "-∞ dB");
  assert.equal(formatDb(0), "0.0 dB");
  assert.equal(formatDb(3.5), "+3.5 dB");
  assert.equal(formatDb(-12.4), "-12.4 dB");
});

test("formatBytes formats data byte sizes accurately", () => {
  assert.equal(formatBytes(0), "0 B");
  assert.equal(formatBytes(500), "500.0 B");
  assert.equal(formatBytes(1024), "1.0 KB");
  assert.equal(formatBytes(1024 * 1024 * 5.5), "5.5 MB");
  assert.equal(formatBytes(1024 * 1024 * 1024 * 2.1), "2.1 GB");
});

test("clamp keeps value bounded within range", () => {
  assert.equal(clamp(5, 0, 10), 5);
  assert.equal(clamp(-5, 0, 10), 0);
  assert.equal(clamp(15, 0, 10), 10);
});

test("snapToNearest snaps values accurately to intervals", () => {
  assert.equal(snapToNearest(14, 10), 10);
  assert.equal(snapToNearest(16, 10), 20);
  assert.equal(snapToNearest(25, 10), 30);
  assert.equal(snapToNearest(100, 0), 100);
});
