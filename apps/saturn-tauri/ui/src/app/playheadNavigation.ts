import type { Clip, ProjectDocument, SelectedClip } from "./projectTypes.ts";
import { TICKS_PER_SECOND } from "../engine/playback/constants.ts";

export type PlayheadContext = {
  project: ProjectDocument["project"] | null;
  selected: SelectedClip | null;
  playhead: number;
};

export function sequenceDuration(project: ProjectDocument["project"] | null): number {
  return Math.max(
    0,
    ...(project?.sequence.tracks ?? []).flatMap((track) =>
      track.clips.map((clip) => clip.timeline_start + clip.duration),
    ),
  );
}

export function selectedClipFrom(
  project: ProjectDocument["project"] | null,
  selected: SelectedClip | null,
): Clip | null {
  if (!project || !selected) return null;
  return (
    project.sequence.tracks.find((track) => track.id === selected.track_id)
      ?.clips[selected.clip_index] ?? null
  );
}

/** Unique cut points on every track, sorted ascending. */
export function editPoints(project: ProjectDocument["project"] | null): number[] {
  return Array.from(
    new Set(
      (project?.sequence.tracks ?? []).flatMap((track) =>
        track.clips.flatMap((clip) => [
          clip.timeline_start,
          clip.timeline_start + clip.duration,
        ]),
      ),
    ),
  ).sort((left, right) => left - right);
}

/**
 * Parse `HH:MM:SS`, `MM:SS`, or `HH:MM:SS:FF` into ticks. Returns null for
 * malformed input or an out-of-range frame count.
 */
export function parseTimecode(
  input: string,
  frameRate: { numerator: number; denominator: number } = {
    numerator: 30,
    denominator: 1,
  },
): number | null {
  const parts = input.trim().split(":").map(Number);
  if (parts.some((part) => !Number.isFinite(part) || part < 0)) return null;
  let hours = 0;
  let minutes = 0;
  let seconds = 0;
  let frames = 0;
  if (parts.length === 2) {
    [minutes, seconds] = parts;
  } else if (parts.length === 3) {
    [hours, minutes, seconds] = parts;
  } else if (parts.length === 4) {
    [hours, minutes, seconds, frames] = parts;
  } else {
    return null;
  }
  const fps = frameRate.numerator / frameRate.denominator;
  if (minutes >= 60 || seconds >= 60 || frames >= fps) return null;
  const totalSeconds = hours * 3600 + minutes * 60 + seconds;
  return (
    totalSeconds * TICKS_PER_SECOND +
    Math.round((frames * TICKS_PER_SECOND * frameRate.denominator) / frameRate.numerator)
  );
}

function frameTicks(project: ProjectDocument["project"] | null): number {
  const frameRate = project?.settings?.frame_rate ?? { numerator: 30, denominator: 1 };
  return Math.round((TICKS_PER_SECOND * frameRate.denominator) / frameRate.numerator);
}

/**
 * Resolve a `playhead.*` command to an absolute tick target. Returns null when
 * the command is unknown or the move is impossible (for example a clip command
 * with no selection), so the caller can leave the playhead untouched.
 */
export function resolvePlayheadCommand(
  id: string,
  context: PlayheadContext,
): number | null {
  const { project, selected, playhead } = context;
  const duration = sequenceDuration(project);
  const clip = selectedClipFrom(project, selected);
  const frame = frameTicks(project);

  switch (id) {
    case "playhead.start":
      return 0;
    case "playhead.end":
      return duration;
    case "playhead.next_edit": {
      const points = editPoints(project);
      return points.find((tick) => tick > playhead + 1) ?? null;
    }
    case "playhead.previous_edit": {
      const points = editPoints(project);
      return [...points].reverse().find((tick) => tick < playhead - 1) ?? null;
    }
    case "playhead.go_to_in":
      return project?.sequence.mark_in ?? 0;
    case "playhead.go_to_out":
      return project?.sequence.mark_out ?? duration;
    case "playhead.go_to_clip_start":
      return clip ? clip.timeline_start : null;
    case "playhead.go_to_clip_end":
      return clip ? clip.timeline_start + clip.duration : null;
    case "playhead.nudge_back_1":
      return Math.max(0, playhead - frame);
    case "playhead.nudge_forward_1":
      return playhead + frame;
    case "playhead.nudge_back_5":
      return Math.max(0, playhead - frame * 5);
    case "playhead.nudge_forward_5":
      return playhead + frame * 5;
    default:
      return null;
  }
}

/** Result of {@link promptGoToTimecode}: cancelled, bad input, or a tick target. */
export type GoToTimecodeResult =
  | { status: "cancelled" }
  | { status: "error"; message: string }
  | { status: "ok"; ticks: number };

/**
 * Prompt for a timecode and parse it against the project frame rate. Pure
 * input handling: the caller owns seeking and notices.
 */
export function promptGoToTimecode(
  frameRate: ProjectDocument["project"]["settings"]["frame_rate"] | null | undefined,
): GoToTimecodeResult {
  const value = window.prompt("Go to timecode (HH:MM:SS:FF)", "00:00:00:00");
  if (value === null) return { status: "cancelled" };
  const target = parseTimecode(value, frameRate ?? undefined);
  if (target === null) {
    return { status: "error", message: "Enter a timecode such as 00:01:23:12" };
  }
  return { status: "ok", ticks: target };
}

/** Outcome of a gap-navigation command: a seek tick or a user-facing notice. */
export type GapResolution = { tick: number | null; notice: string | null };

const GAP_COMMAND_IDS = new Set([
  "sequence.goToNextGap",
  "sequence.goToPrevGap",
  "sequence.goToNextGapInTrack",
  "sequence.goToPrevGapInTrack",
]);

/** Merge a track's clips into covered [start, end) intervals, ascending. */
function coveredIntervals(
  clips: { timeline_start: number; duration: number }[],
): { start: number; end: number }[] {
  const sorted = [...clips]
    .filter((clip) => clip.duration > 0)
    .sort((a, b) => a.timeline_start - b.timeline_start);
  const intervals: { start: number; end: number }[] = [];
  for (const clip of sorted) {
    const last = intervals[intervals.length - 1];
    if (last && clip.timeline_start <= last.end) {
      last.end = Math.max(last.end, clip.timeline_start + clip.duration);
    } else {
      intervals.push({ start: clip.timeline_start, end: clip.timeline_start + clip.duration });
    }
  }
  return intervals;
}

/**
 * Resolve the sequence.goTo*Gap family to a playhead tick. "In sequence"
 * scans every unlocked track; "in track" restricts to the selected clip's
 * track. Returns null when `id` is not a gap command; `tick: null` means
 * nothing to seek at and `notice` explains why.
 */
export function resolveGapCommand(
  id: string,
  ctx: PlayheadContext,
): GapResolution | null {
  if (!GAP_COMMAND_IDS.has(id)) return null;
  const { project, selected, playhead } = ctx;
  const inTrack = id.endsWith("InTrack");
  const tracks = (project?.sequence.tracks ?? []).filter(
    (track) => (inTrack ? track.id === selected?.track_id : true) && !track.locked,
  );
  if (inTrack && !selected) {
    return { tick: null, notice: "Select a clip on the target track first" };
  }
  let best: number | null = null;
  for (const track of tracks) {
    const intervals = coveredIntervals(track.clips);
    for (let index = 0; index <= intervals.length; index += 1) {
      // Gap between the previous interval's end and this interval's start;
      // the trailing gap uses the last interval's end as its start.
      const gapStart = index === 0 ? 0 : intervals[index - 1].end;
      if (id.startsWith("sequence.goToNext")) {
        if (gapStart > playhead && (best === null || gapStart < best)) best = gapStart;
      } else {
        const gapEnd = index === intervals.length ? null : intervals[index].start;
        if (gapEnd !== null && gapEnd < playhead && (best === null || gapEnd > best)) {
          best = gapEnd;
        }
      }
    }
  }
  if (best === null) {
    return {
      tick: null,
      notice: inTrack
        ? "No gap on the selected track in that direction"
        : "No gap in the sequence in that direction",
    };
  }
  return { tick: best, notice: null };
}
