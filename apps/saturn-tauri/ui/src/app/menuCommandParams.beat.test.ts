// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import assert from "node:assert/strict";
import test from "node:test";
import { buildTrimCommandParams, type BatchMenuContext } from "./menuCommandParams.ts";
import { TICKS_PER_SECOND } from "./projectTypes.ts";

const context = { selectedClip: { track_id: 1, clip_index: 1 }, selectedClips: [], playhead: 0.5,
  project: { sequence: { tracks: [{ id: 1, name: "Picture", kind: "video" }, { id: 2, name: "Music", kind: "audio" }] } },
} as BatchMenuContext;
function build(answers: (string | null)[], ctx = context) {
  const previous = (globalThis as { window?: unknown }).window;
  (globalThis as { window?: unknown }).window = { prompt: () => {
    assert.ok(answers.length, "unexpected prompt"); return answers.shift();
  } };
  try { return buildTrimCommandParams("timeline.beat_grid_quantize", ctx); }
  finally { (globalThis as { window?: unknown }).window = previous; }
}
test("beat menu sends tracks, strength, subdivision and explicit bar origin", () => {
  for (const subdivision of ["quarter", "half", "bar"]) {
    const answers = ["1", "2", subdivision, "0.75"];
    if (subdivision !== "quarter") answers.push("0.5");
    assert.deepEqual(build(answers)?.params, { video_track_id: 1, music_track_id: 2,
      subdivision, strength: 0.75, ...(subdivision === "quarter" ? {} : { downbeat_tick: TICKS_PER_SECOND / 2 }) });
  }
  assert.equal(build(["1", "2", "quarter", "0"])?.params?.strength, 0);
});
test("beat menu cancels at every prompt and rejects invalid entries", () => {
  const valid = ["1", "2", "bar", "1", "0.5"];
  for (let i = 0; i < valid.length; i++) {
    assert.equal(build([...valid.slice(0, i), null])?.cancel, true);
    assert.equal(build([...valid.slice(0, i), ""])?.cancel, true);
  }
  for (const answers of [["2"], ["1", "1"], ["1", "2", "eighth"],
    ["1", "2", "quarter", "1.1"], ["1", "2", "quarter", "NaN"],
    ["1", "2", "bar", "1", "-1"], ["1", "2", "bar", "1", "Infinity"]]) {
    assert.equal(build(answers)?.cancel, true);
  }
  assert.equal(build([], { ...context, project: null })?.cancel, true);
});
