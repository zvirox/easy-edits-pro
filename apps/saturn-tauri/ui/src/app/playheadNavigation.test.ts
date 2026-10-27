import assert from "node:assert/strict";
import test from "node:test";
import {
  findClipAtPlayhead,
  parseTimecode,
  resolveGapCommand,
  resolvePlayheadCommand,
  sequenceDuration,
} from "./playheadNavigation.ts";
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

const gapProject = {
  sequence: {
    tracks: [
      {
        id: 1,
        locked: false,
        clips: [
          { timeline_start: 0, duration: TICKS_PER_SECOND * 2 },
          { timeline_start: TICKS_PER_SECOND * 4, duration: TICKS_PER_SECOND * 2 },
        ],
      },
      {
        id: 2,
        locked: false,
        clips: [
          { timeline_start: TICKS_PER_SECOND * 1, duration: TICKS_PER_SECOND * 2 },
          { timeline_start: TICKS_PER_SECOND * 3, duration: TICKS_PER_SECOND * 2 },
        ],
      },
      {
        id: 3,
        locked: true,
        clips: [{ timeline_start: TICKS_PER_SECOND * 9, duration: TICKS_PER_SECOND }],
      },
    ],
  },
} as unknown as ProjectDocument["project"];

test("next gap picks the smallest gap start past the playhead across unlocked tracks", () => {
  assert.equal(
    resolveGapCommand("sequence.goToNextGap", {
      project: gapProject,
      selected: null,
      playhead: 0.5 * TICKS_PER_SECOND,
    })?.tick,
    TICKS_PER_SECOND * 2,
  );
  // Trailing gap of track 1 (start 6s) loses to track 2's trailing gap (3s).
  assert.equal(
    resolveGapCommand("sequence.goToNextGap", {
      project: gapProject,
      selected: null,
      playhead: TICKS_PER_SECOND * 5,
    })?.tick,
    TICKS_PER_SECOND * 6,
  );
});

test("previous gap picks the largest gap end before the playhead and merges intervals", () => {
  // Track 2's clips touch at 3s, so its gaps are a leading edge at 1s only.
  assert.equal(
    resolveGapCommand("sequence.goToPrevGap", {
      project: gapProject,
      selected: null,
      playhead: TICKS_PER_SECOND * 5,
    })?.tick,
    TICKS_PER_SECOND * 4,
  );
  assert.equal(
    resolveGapCommand("sequence.goToPrevGap", {
      project: gapProject,
      selected: null,
      playhead: 1.5 * TICKS_PER_SECOND,
    })?.tick,
    TICKS_PER_SECOND,
  );
  // Nothing before time zero.
  const atZero = resolveGapCommand("sequence.goToPrevGap", {
    project: gapProject,
    selected: null,
    playhead: 0,
  });
  assert.equal(atZero?.tick, null);
  assert.match(atZero?.notice ?? "", /No gap/);
});

test("in-track gap navigation is restricted to the selected clip's track", () => {
  const selected = { track_id: 1, clip_index: 0 };
  assert.equal(
    resolveGapCommand("sequence.goToNextGapInTrack", {
      project: gapProject,
      selected,
      playhead: 0.5 * TICKS_PER_SECOND,
    })?.tick,
    TICKS_PER_SECOND * 2,
  );
  // Track 2 alone would answer 3s; in-track it must stay on track 1.
  assert.equal(
    resolveGapCommand("sequence.goToPrevGapInTrack", {
      project: gapProject,
      selected: { track_id: 2, clip_index: 1 },
      playhead: TICKS_PER_SECOND * 5,
    })?.tick,
    TICKS_PER_SECOND * 1,
  );
  const noSelection = resolveGapCommand("sequence.goToNextGapInTrack", {
    project: gapProject,
    selected: null,
    playhead: 0.5 * TICKS_PER_SECOND,
  });
  assert.equal(noSelection?.tick, null);
  assert.match(noSelection?.notice ?? "", /Select a clip on the target track first/);
});

test("non-gap ids fall through as null", () => {
  assert.equal(
    resolveGapCommand("playhead.next_edit", {
      project: gapProject,
      selected: null,
      playhead: 0,
    }),
    null,
  );
});

const t = (start: number, duration: number, enabled?: boolean) => ({
  timeline_start: start,
  duration,
  ...(enabled === undefined ? {} : { enabled }),
});
const layeredProject = {
  sequence: {
    tracks: [
      {
        id: 2,
        kind: "video",
        clips: [t(0, 2), t(2, 2)],
      },
      {
        id: 1,
        kind: "video",
        clips: [t(0, 4)],
      },
      {
        id: 3,
        kind: "audio",
        clips: [t(0, 4)],
      },
      {
        id: 4,
        kind: "video",
        clips: [t(0, 1), t(1, 3, false)],
      },
    ],
  },
} as unknown as ProjectDocument["project"];

test("findClipAtPlayhead picks the clip under the playhead, topmost video track", () => {
  assert.deepEqual(findClipAtPlayhead(layeredProject, 0), { track_id: 2, clip_index: 0 });
  assert.deepEqual(findClipAtPlayhead(layeredProject, 3), { track_id: 2, clip_index: 1 });
});

test("findClipAtPlayhead skips disabled clips and falls through to lower tracks", () => {
  // Track 4 is topmost here? No: track 2 covers it. Disable track 2's clips to
  // exercise the fall-through path.
  const disabledTop = {
    sequence: {
      tracks: [
        { id: 2, kind: "video", clips: [t(0, 4, false)] },
        { id: 1, kind: "video", clips: [t(0, 4, true)] },
      ],
    },
  } as unknown as ProjectDocument["project"];
  assert.deepEqual(findClipAtPlayhead(disabledTop, 1), { track_id: 1, clip_index: 0 });
  // Disabled clip fully covering the playhead on the only video track.
  const disabledOnly = {
    sequence: { tracks: [{ id: 5, kind: "video", clips: [t(0, 4, false)] }] },
  } as unknown as ProjectDocument["project"];
  assert.equal(findClipAtPlayhead(disabledOnly, 1), null);
});

test("findClipAtPlayhead returns null off-clip and for null projects", () => {
  assert.equal(findClipAtPlayhead(layeredProject, 99), null);
  assert.equal(findClipAtPlayhead(null, 0), null);
});
