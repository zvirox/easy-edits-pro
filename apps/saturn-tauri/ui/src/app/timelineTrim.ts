import { TICKS_PER_SECOND, type TimelineTool } from "./projectTypes.ts";

export type TrimEdge = "start" | "end";

export interface TrimSession {
  trackId: number;
  clipIndex: number;
  tool: TimelineTool;
  edge: TrimEdge;
  deltaTicks: number;
}

/** One frame at 30 fps; the shortest clip a trim gesture may leave behind. */
export const MIN_CLIP_TICKS = TICKS_PER_SECOND / 30;

export interface TrimClipLike {
  timeline_start: number;
  duration: number;
  source_in: number;
}

export interface TrimPlanInput {
  trackId: number;
  tool: TimelineTool;
  edge: TrimEdge;
  deltaTicks: number;
  clipIndex: number;
  clip: TrimClipLike;
  previous?: TrimClipLike | null;
  next?: TrimClipLike | null;
  mediaDuration?: number | null;
}

export interface TrimCommand {
  id: string;
  params: Record<string, unknown>;
}

export interface TrimPlan {
  deltaTicks: number;
  commands: TrimCommand[];
  notice: string;
  preview: TrimClipLike;
}

export type TrimPlanResult =
  | { ok: true; plan: TrimPlan }
  | { ok: false; reason: string };

const TRIM_TOOLS = new Set<TimelineTool>([
  "select",
  "ripple",
  "rolling",
  "slip",
  "slide",
  "rateStretch",
]);

function clamp(value: number, min: number, max: number): number {
  if (max < min) return min;
  return Math.min(max, Math.max(min, value));
}

function seconds(ticks: number): string {
  const value = Math.abs(ticks) / TICKS_PER_SECOND;
  return `${value.toFixed(2)}s`;
}

function signedSeconds(ticks: number): string {
  return `${ticks < 0 ? "-" : "+"}${seconds(ticks)}`;
}

/**
 * Turns a trim tool + pointer delta into concrete engine commands. Pure so the
 * same maths drives the live Trim Monitor and the command that commits on
 * pointer-up. Clamps keep a gesture from proposing an invalid edit.
 */
export function planTrim(input: TrimPlanInput): TrimPlanResult {
  const { clip, edge } = input;
  if (!TRIM_TOOLS.has(input.tool)) {
    return { ok: false, reason: "This tool does not trim clips" };
  }

  const mediaEnd = input.mediaDuration ?? Number.POSITIVE_INFINITY;
  const minDuration = Math.min(MIN_CLIP_TICKS, clip.duration);

  if (input.tool === "select" || input.tool === "ripple") {
    const ripple = input.tool === "ripple";
    if (edge === "start") {
      const lower = Math.max(-clip.source_in, ripple ? Number.NEGATIVE_INFINITY : -clip.timeline_start);
      const deltaTicks = Math.round(
        clamp(input.deltaTicks, Math.max(lower, -(clip.duration - minDuration)), clip.duration - minDuration),
      );
      const preview = {
        timeline_start: ripple ? clip.timeline_start : clip.timeline_start + deltaTicks,
        duration: clip.duration - deltaTicks,
        source_in: clip.source_in + deltaTicks,
      };
      return {
        ok: true,
        plan: {
          deltaTicks,
          commands: [
            {
              id: "timeline.trim_clip",
              params: {
                track_id: input.trackId,
                clip_index: input.clipIndex,
                edge,
                delta: deltaTicks,
                ripple,
              },
            },
          ],
          notice: `${ripple ? "Ripple" : "Trim"} head ${signedSeconds(deltaTicks)}`,
          preview,
        },
      };
    }

    const maxByMedia = mediaEnd - (clip.source_in + clip.duration);
    const deltaTicks = Math.round(
      clamp(input.deltaTicks, -(clip.duration - minDuration), maxByMedia),
    );
    return {
      ok: true,
      plan: {
        deltaTicks,
        commands: [
          {
            id: "timeline.trim_clip",
            params: {
              track_id: input.trackId,
              clip_index: input.clipIndex,
              edge,
              delta: deltaTicks,
              ripple,
            },
          },
        ],
        notice: `${ripple ? "Ripple" : "Trim"} tail ${signedSeconds(deltaTicks)}`,
        preview: {
          timeline_start: clip.timeline_start,
          duration: clip.duration + deltaTicks,
          source_in: clip.source_in,
        },
      },
    };
  }

  if (input.tool === "rolling") {
    const leftIndex = edge === "start" ? input.clipIndex - 1 : input.clipIndex;
    const left = edge === "start" ? input.previous : clip;
    const right = edge === "start" ? clip : input.next;
    if (!left || !right) {
      return { ok: false, reason: "Rolling edit needs a clip on both sides of the cut" };
    }
    const deltaTicks = Math.round(
      clamp(
        input.deltaTicks,
        Math.max(-(left.duration - minDuration), -right.source_in),
        right.duration - minDuration,
      ),
    );
    return {
      ok: true,
      plan: {
        deltaTicks,
        commands: [
          {
            id: "timeline.roll_edit",
            params: { track_id: input.trackId, left_clip_index: leftIndex, delta: deltaTicks },
          },
        ],
        notice: `Roll ${signedSeconds(deltaTicks)}`,
        preview:
          edge === "start"
            ? {
                timeline_start: clip.timeline_start + deltaTicks,
                duration: clip.duration - deltaTicks,
                source_in: clip.source_in + deltaTicks,
              }
            : {
                timeline_start: clip.timeline_start,
                duration: clip.duration + deltaTicks,
                source_in: clip.source_in,
              },
      },
    };
  }

  if (input.tool === "slip") {
    const deltaTicks = Math.round(
      clamp(
        input.deltaTicks,
        -clip.source_in,
        Math.max(-clip.source_in, mediaEnd - (clip.source_in + clip.duration)),
      ),
    );
    return {
      ok: true,
      plan: {
        deltaTicks,
        commands: [
          {
            id: "timeline.slip_clip",
            params: { track_id: input.trackId, clip_index: input.clipIndex, delta: deltaTicks },
          },
        ],
        notice: `Slip source ${signedSeconds(deltaTicks)}`,
        preview: { ...clip, source_in: clip.source_in + deltaTicks },
      },
    };
  }

  if (input.tool === "slide") {
    if (!input.previous || !input.next) {
      return { ok: false, reason: "Slide needs a clip on both sides" };
    }
    const deltaTicks = Math.round(
      clamp(
        input.deltaTicks,
        Math.max(
          -(input.previous.duration - minDuration),
          -input.next.source_in,
          -clip.timeline_start,
        ),
        input.next.duration - minDuration,
      ),
    );
    return {
      ok: true,
      plan: {
        deltaTicks,
        commands: [
          {
            id: "timeline.slide_clip",
            params: { track_id: input.trackId, clip_index: input.clipIndex, delta: deltaTicks },
          },
        ],
        notice: `Slide clip ${signedSeconds(deltaTicks)}`,
        preview: { ...clip, timeline_start: clip.timeline_start + deltaTicks },
      },
    };
  }

  // Rate stretch: the opposite edge stays anchored. A head stretch also moves
  // the clip, so the two commands are committed inside one undo gesture.
  const deltaTicks = Math.round(
    edge === "start"
      ? clamp(input.deltaTicks, -clip.timeline_start, clip.duration - minDuration)
      : clamp(input.deltaTicks, -(clip.duration - minDuration), Number.POSITIVE_INFINITY),
  );
  const newDuration =
    edge === "end" ? clip.duration + deltaTicks : clip.duration - deltaTicks;
  if (newDuration < minDuration) {
    return { ok: false, reason: "Rate stretch would leave less than one frame" };
  }
  const speed = clip.duration / newDuration;
  const commands: TrimCommand[] = [
    {
      id: "timeline.rate_stretch",
      params: {
        track_id: input.trackId,
        clip_index: input.clipIndex,
        new_duration: newDuration,
      },
    },
  ];
  const preview = {
    timeline_start: edge === "start" ? clip.timeline_start + deltaTicks : clip.timeline_start,
    duration: newDuration,
    source_in: clip.source_in,
  };
  if (edge === "start") {
    commands.push({
      id: "timeline.move_clip",
      params: {
        track_id: input.trackId,
        clip_index: input.clipIndex,
        target_track_id: input.trackId,
        timeline_start: preview.timeline_start,
      },
    });
  }
  return {
    ok: true,
    plan: {
      deltaTicks,
      commands,
      notice: `Rate stretch ${speed.toFixed(2)}×`,
      preview,
    },
  };
}

export function formatTrimStatus(session: TrimSession): string {
  const seconds = Math.abs(session.deltaTicks) / TICKS_PER_SECOND;
  return `${trimMonitorLabel(session.tool, session.edge)} ${session.deltaTicks < 0 ? "−" : "+"}${seconds.toFixed(2)}s`;
}

export function trimMonitorLabel(tool: TimelineTool, edge: TrimEdge): string {
  switch (tool) {
    case "ripple":
      return `Ripple ${edge === "start" ? "head" : "tail"}`;
    case "rolling":
      return "Rolling edit";
    case "slip":
      return "Slip";
    case "slide":
      return "Slide";
    case "rateStretch":
      return "Rate stretch";
    default:
      return edge === "start" ? "Trim head" : "Trim tail";
  }
}
