import { TICKS_PER_SECOND } from "./projectTypes.ts";

/**
 * Calculates ticks per single video frame given a rational frame rate.
 */
export function calculateFrameTicks(frameRate: { numerator: number; denominator: number }): number {
  if (!frameRate.numerator || !frameRate.denominator) return Math.round(TICKS_PER_SECOND / 30);
  return Math.round((TICKS_PER_SECOND * frameRate.denominator) / frameRate.numerator);
}

/**
 * Snaps a timeline tick value to a frame boundary, and optionally to nearby snap points.
 */
export function snapTimelineTicks(
  rawTicks: number,
  frameTicks: number,
  snapPoints: number[] = [],
  thresholdTicks: number = frameTicks * 3
): number {
  // First snap to frame grid
  const framed = Math.round(rawTicks / frameTicks) * frameTicks;
  if (snapPoints.length === 0) return Math.max(0, framed);

  // Look for any snap point within threshold
  let closestPoint: number | null = null;
  let smallestDist = Infinity;

  for (const point of snapPoints) {
    const dist = Math.abs(rawTicks - point);
    if (dist <= thresholdTicks && dist < smallestDist) {
      smallestDist = dist;
      closestPoint = point;
    }
  }

  return closestPoint !== null ? closestPoint : Math.max(0, framed);
}

/**
 * Calculates a clip split at playhead ticks. Returns null if cut is outside the clip.
 */
export function calculateClipSplit<T extends { timeline_start: number; duration: number; source_in: number }>(
  clip: T,
  atTicks: number
): { left: T; right: T } | null {
  if (atTicks <= clip.timeline_start || atTicks >= clip.timeline_start + clip.duration) {
    return null;
  }
  const delta = atTicks - clip.timeline_start;
  const left: T = {
    ...clip,
    duration: delta,
  };
  const right: T = {
    ...clip,
    timeline_start: atTicks,
    source_in: clip.source_in + delta,
    duration: clip.duration - delta,
  };
  return { left, right };
}

/**
 * Calculates ripple shift for clips when a gap or clip is deleted.
 */
export function calculateRippleShift<T extends { timeline_start: number }>(
  clips: T[],
  deletedStart: number,
  deletedDuration: number
): T[] {
  if (deletedDuration <= 0) return clips;
  return clips.map((clip) => {
    if (clip.timeline_start >= deletedStart + deletedDuration) {
      return {
        ...clip,
        timeline_start: clip.timeline_start - deletedDuration,
      };
    }
    return clip;
  });
}
