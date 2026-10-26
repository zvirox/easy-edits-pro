import assert from "node:assert/strict";
import test from "node:test";
import { buildTrimCommandParams } from "./menuCommandParams.ts";

const TICKS_PER_SECOND = 254_016_000_000;

type PromptTarget = { prompt: (message?: string) => string | null };

const ctx = (playhead: number, selectedClip: unknown = null) =>
  ({
    selectedClip,
    selectedClips: [],
    playhead,
    project: null,
  }) as Parameters<typeof buildTrimCommandParams>[1];

/** The builders use the native prompt idiom; stub `window` for node runs. */
function withPrompt(answer: string | null, run: (calls: string[]) => void) {
  const calls: string[] = [];
  const target: PromptTarget = {
    prompt: (message) => {
      calls.push(String(message));
      return answer;
    },
  };
  const previous = (globalThis as { window?: unknown }).window;
  (globalThis as { window?: unknown }).window = target;
  try {
    run(calls);
  } finally {
    (globalThis as { window?: unknown }).window = previous;
  }
}

test("buildTrimCommandParams anchors at-commands to the playhead", () => {
  const at = Math.round(2.5 * TICKS_PER_SECOND);
  for (const id of [
    "trim.extendToPlayhead",
    "trim.extendNextEdit",
    "trim.extendPreviousEdit",
    "trim.rippleNext",
    "trim.ripplePrevious",
    "trim.next",
    "trim.previous",
    "trim.selectNearest",
    "trim.monitor",
  ]) {
    const built = buildTrimCommandParams(id, ctx(2.5));
    assert.ok(built, id);
    assert.deepEqual(built.params, { at });
  }
});

test("buildTrimCommandParams passes structured trim commands through", () => {
  for (const id of [
    "trim.edit",
    "trim.forward",
    "trim.backward",
    "trim.forwardMany",
    "trim.backwardMany",
    "trim.toggleType",
    "trim.clear",
    "trim.applyDefaultTransition",
  ]) {
    const built = buildTrimCommandParams(id, ctx(0));
    assert.ok(built, id);
    assert.deepEqual(built.params, {});
  }
});

test("buildTrimCommandParams selects the edit point from the selection", () => {
  const selected = { track_id: 3, clip_index: 1 };
  withPrompt("start", (calls) => {
    const built = buildTrimCommandParams("trim.selectEditPoint", ctx(0, selected));
    assert.deepEqual(built, { params: { track_id: 3, clip_index: 1, edge: "start" } });
    assert.equal(calls.length, 1);
  });
});

test("buildTrimCommandParams refuses edit-point selection without a clip", () => {
  const built = buildTrimCommandParams("trim.selectEditPoint", ctx(0));
  assert.equal(built?.cancel, true);
  assert.match(built?.notice ?? "", /Select a clip/);
});

test("buildTrimCommandParams prompts a signed frame count for nudges", () => {
  withPrompt("-12", () => {
    const built = buildTrimCommandParams("trim.nudge", ctx(0));
    assert.deepEqual(built, { params: { frames: -12 } });
  });
  withPrompt("1.5", () => {
    const built = buildTrimCommandParams("trim.nudge", ctx(0));
    assert.equal(built?.cancel, true);
  });
  withPrompt(null, () => {
    const built = buildTrimCommandParams("trim.nudge", ctx(0));
    assert.equal(built?.cancel, true);
  });
});

test("buildTrimCommandParams ignores ids outside the trim family", () => {
  assert.equal(buildTrimCommandParams("timeline.trim_clip", ctx(0)), null);
});

import { buildMarkerCommandParams } from "./menuCommandParams.ts";

test("buildMarkerCommandParams marks clips from the selection", () => {
  const selected = { track_id: 2, clip_index: 3 };
  const single = buildMarkerCommandParams("timeline.mark_clip", ctx(0, selected));
  assert.deepEqual(single, { params: { track_id: 2, clip_index: 3 } });

  const multi = buildMarkerCommandParams("timeline.mark_selection", ctx(0, selected));
  assert.deepEqual(multi, { params: { clips: [{ track_id: 2, clip_index: 3 }] } });

  const multiCtx = ctx(0, selected);
  multiCtx.selectedClips = [
    { track_id: 2, clip_index: 3 },
    { track_id: 1, clip_index: 0 },
  ] as never;
  const batch = buildMarkerCommandParams("timeline.mark_selection", multiCtx);
  assert.deepEqual(batch, {
    params: {
      clips: [
        { track_id: 2, clip_index: 3 },
        { track_id: 1, clip_index: 0 },
      ],
    },
  });
});

test("buildMarkerCommandParams refuses marking without a selection", () => {
  const clip = buildMarkerCommandParams("timeline.mark_clip", ctx(0));
  assert.equal(clip?.cancel, true);
  const selection = buildMarkerCommandParams("timeline.mark_selection", ctx(0));
  assert.equal(selection?.cancel, true);
});

test("buildMarkerCommandParams ignores non-mark ids", () => {
  assert.equal(buildMarkerCommandParams("marker.add", ctx(0)), null);
  assert.equal(buildMarkerCommandParams("trim.forward", ctx(0)), null);
});
