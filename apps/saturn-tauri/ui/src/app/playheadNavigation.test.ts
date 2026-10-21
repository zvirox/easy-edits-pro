import assert from "node:assert/strict";
import test from "node:test";
import { parseTimecode, resolvePlayheadCommand, sequenceDuration } from "./playheadNavigation.ts";
import type { ProjectDocument } from "./projectTypes.ts";

const TICKS_PER_SECOND = 254_016_000_000;

const project = {
  sequence: {
    mark_in: TICKS_PER_SECOND,
    mark_out: TICKS_PER_SECOND * 8,
    tracks: [
      {
        id: 1,
        clips: [
          { timeline_start: 0, duration: TICKS_PER_SECOND * 2 },
          { timeline_start: TICKS_PER_SECOND * 4, duration: TICKS_PER_SECOND * 2 },
        ],
      },
    ],
  },
} as unknown as ProjectDocument["project"];

test("sequence duration follows the furthest clip end", () => {
  assert.equal(sequenceDuration(project), TICKS_PER_SECOND * 6);
});

test("next and previous edit points skip the current cut", () => {
  assert.equal(
    resolvePlayheadCommand("playhead.next_edit", {
      project,
      selected: null,
      playhead: 0,
    }),
    TICKS_PER_SECOND * 2,
  );
  assert.equal(
    resolvePlayheadCommand("playhead.previous_edit", {
      project,
      selected: null,
      playhead: TICKS_PER_SECOND * 4,
    }),
    TICKS_PER_SECOND * 2,
  );
});

test("in and out command targets use the sequence marks", () => {
  assert.equal(
    resolvePlayheadCommand("playhead.go_to_in", {
      project,
      selected: null,
      playhead: 0,
    }),
    TICKS_PER_SECOND,
  );
  assert.equal(
    resolvePlayheadCommand("playhead.go_to_out", {
      project,
      selected: null,
      playhead: 0,
    }),
    TICKS_PER_SECOND * 8,
  );
});

test("clip commands need a selected clip", () => {
  assert.equal(
    resolvePlayheadCommand("playhead.go_to_clip_start", {
      project,
      selected: null,
      playhead: 0,
    }),
    null,
  );
  assert.equal(
    resolvePlayheadCommand("playhead.go_to_clip_end", {
      project,
      selected: { track_id: 1, clip_index: 1 },
      playhead: 0,
    }),
    TICKS_PER_SECOND * 6,
  );
});

test("timecode entry accepts frames and rejects malformed values", () => {
  assert.equal(parseTimecode("00:00:01:15"), TICKS_PER_SECOND + TICKS_PER_SECOND / 2);
  assert.equal(parseTimecode("1:02"), TICKS_PER_SECOND * 62);
  assert.equal(parseTimecode("00:99:00"), null);
  assert.equal(parseTimecode("00:00:00:30"), null);
  assert.equal(parseTimecode("nope"), null);
});

test("frame nudges use the project frame rate and never go negative", () => {
  assert.equal(
    resolvePlayheadCommand("playhead.nudge_back_1", {
      project,
      selected: null,
      playhead: TICKS_PER_SECOND,
    }),
    TICKS_PER_SECOND - Math.round(TICKS_PER_SECOND / 30),
  );
  assert.equal(
    resolvePlayheadCommand("playhead.nudge_back_5", {
      project,
      selected: null,
      playhead: 1,
    }),
    0,
  );
});
