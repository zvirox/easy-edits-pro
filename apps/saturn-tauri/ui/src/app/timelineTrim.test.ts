import assert from "node:assert/strict";
import test from "node:test";
import { TICKS_PER_SECOND } from "./projectTypes.ts";
import { MIN_CLIP_TICKS, planTrim } from "./timelineTrim.ts";

const clip = { timeline_start: 2 * TICKS_PER_SECOND, duration: 4 * TICKS_PER_SECOND, source_in: TICKS_PER_SECOND };

test("plain tail trim clamps at the media duration", () => {
  const result = planTrim({
    trackId: 1,
    tool: "select",
    edge: "end",
    deltaTicks: 10 * TICKS_PER_SECOND,
    clipIndex: 0,
    clip,
    mediaDuration: 10 * TICKS_PER_SECOND,
  });
  assert.ok(result.ok);
  assert.equal(result.plan.deltaTicks, 5 * TICKS_PER_SECOND);
  assert.deepEqual(result.plan.preview, {
    timeline_start: 2 * TICKS_PER_SECOND,
    duration: 9 * TICKS_PER_SECOND,
    source_in: TICKS_PER_SECOND,
  });
  assert.equal(result.plan.commands[0].id, "timeline.trim_clip");
  assert.equal(result.plan.commands[0].params.ripple, false);
});

test("ripple head trim stays anchored and clamps before the source start", () => {
  const result = planTrim({
    trackId: 7,
    tool: "ripple",
    edge: "start",
    deltaTicks: -5 * TICKS_PER_SECOND,
    clipIndex: 3,
    clip,
  });
  assert.ok(result.ok);
  assert.equal(result.plan.deltaTicks, -TICKS_PER_SECOND);
  assert.deepEqual(result.plan.preview, {
    timeline_start: 2 * TICKS_PER_SECOND,
    duration: 5 * TICKS_PER_SECOND,
    source_in: 0,
  });
  assert.equal(result.plan.commands[0].params.ripple, true);
});

test("rolling needs both neighbours and keeps a frame on each side", () => {
  const previous = { timeline_start: 0, duration: 2 * TICKS_PER_SECOND, source_in: 0 };
  const next = { timeline_start: 6 * TICKS_PER_SECOND, duration: 2 * TICKS_PER_SECOND, source_in: 0 };
  const missing = planTrim({
    trackId: 1,
    tool: "rolling",
    edge: "end",
    deltaTicks: TICKS_PER_SECOND,
    clipIndex: 0,
    clip,
  });
  assert.equal(missing.ok, false);

  const result = planTrim({
    trackId: 1,
    tool: "rolling",
    edge: "start",
    deltaTicks: -10 * TICKS_PER_SECOND,
    clipIndex: 1,
    clip,
    previous,
    next,
  });
  assert.ok(result.ok);
  assert.equal(result.plan.deltaTicks, -clip.source_in);
  assert.equal(result.plan.commands[0].id, "timeline.roll_edit");
  assert.equal(result.plan.commands[0].params.left_clip_index, 0);
});

test("slip and slide clamp against their neighbours and source bounds", () => {
  const previous = { timeline_start: 0, duration: 2 * TICKS_PER_SECOND, source_in: 0 };
  const next = { timeline_start: 6 * TICKS_PER_SECOND, duration: 2 * TICKS_PER_SECOND, source_in: 0 };

  const slip = planTrim({
    trackId: 1,
    tool: "slip",
    edge: "end",
    deltaTicks: 9 * TICKS_PER_SECOND,
    clipIndex: 1,
    clip,
    mediaDuration: 10 * TICKS_PER_SECOND,
  });
  assert.ok(slip.ok);
  assert.equal(slip.plan.deltaTicks, 5 * TICKS_PER_SECOND);
  assert.equal(slip.plan.preview.source_in, 6 * TICKS_PER_SECOND);
  assert.equal(slip.plan.commands[0].id, "timeline.slip_clip");

  const slide = planTrim({
    trackId: 1,
    tool: "slide",
    edge: "end",
    deltaTicks: -9 * TICKS_PER_SECOND,
    clipIndex: 1,
    clip,
    previous,
    next: { ...next, source_in: 2 * TICKS_PER_SECOND },
  });
  assert.ok(slide.ok);
  assert.equal(slide.plan.deltaTicks, -(previous.duration - MIN_CLIP_TICKS));
  assert.equal(slide.plan.preview.timeline_start, clip.timeline_start - (previous.duration - MIN_CLIP_TICKS));
});

test("rate stretch uses one command at the tail and two at the head", () => {
  const tail = planTrim({
    trackId: 1,
    tool: "rateStretch",
    edge: "end",
    deltaTicks: 2 * TICKS_PER_SECOND,
    clipIndex: 0,
    clip,
  });
  assert.ok(tail.ok);
  assert.equal(tail.plan.commands.length, 1);
  assert.equal(tail.plan.preview.duration, 6 * TICKS_PER_SECOND);

  const head = planTrim({
    trackId: 1,
    tool: "rateStretch",
    edge: "start",
    deltaTicks: TICKS_PER_SECOND,
    clipIndex: 0,
    clip,
  });
  assert.ok(head.ok);
  assert.deepEqual(
    head.plan.commands.map((command) => command.id),
    ["timeline.rate_stretch", "timeline.move_clip"],
  );
  assert.equal(head.plan.preview.timeline_start, 3 * TICKS_PER_SECOND);
});

test("non-trim tools report why nothing happens", () => {
  const result = planTrim({
    trackId: 1,
    tool: "razor",
    edge: "end",
    deltaTicks: TICKS_PER_SECOND,
    clipIndex: 0,
    clip,
  });
  assert.equal(result.ok, false);
  assert.match(result.ok ? "" : result.reason, /does not trim/);
});
