import type { SubtitleItem } from "../../types";
import { TICKS_PER_SECOND } from "../../utils/formatters.ts";

/** Minimal cue shape accepted by the pure helpers. */
export type CueLike = Pick<
  SubtitleItem,
  "id" | "start_ticks" | "end_ticks" | "text"
> &
  Partial<Pick<SubtitleItem, "speaker" | "style_id">>;

/** Timeline order: start tick, then id so equal starts stay stable. */
export function sortedCues<T extends CueLike>(cues: readonly T[]): T[] {
  return [...cues].sort(
    (left, right) => left.start_ticks - right.start_ticks || left.id - right.id,
  );
}

/** The cue covering the playhead, if any. */
export function activeCue<T extends CueLike>(
  cues: readonly T[],
  playheadTicks: number,
): T | null {
  for (const cue of cues) {
    if (playheadTicks >= cue.start_ticks && playheadTicks < cue.end_ticks) {
      return cue;
    }
  }
  return null;
}

/** The next cue starting strictly after the playhead. */
export function nextCue<T extends CueLike>(
  cues: readonly T[],
  from: number,
): T | null {
  let best: T | null = null;
  for (const cue of cues) {
    if (cue.start_ticks <= from) continue;
    if (!best || cue.start_ticks < best.start_ticks) best = cue;
  }
  return best;
}

/** One frame in ticks at the given frame rate (nudge step). */
export function frameTicks(frameRate: { numerator: number; denominator: number }): number {
  const denominator = frameRate.denominator > 0 ? frameRate.denominator : 1;
  const fps = frameRate.numerator / denominator;
  if (!Number.isFinite(fps) || fps <= 0) return 1;
  return Math.max(1, Math.round(TICKS_PER_SECOND / fps));
}

/** Left offset in pixels for the current timeline zoom (pixels per second). */
export function cueLeftPx(cue: CueLike, pixelsPerSecond: number): number {
  return (Math.max(0, cue.start_ticks) / TICKS_PER_SECOND) * pixelsPerSecond;
}

/** Width in pixels for the current timeline zoom (pixels per second). */
export function cueWidthPx(cue: CueLike, pixelsPerSecond: number): number {
  const duration = Math.max(0, cue.end_ticks - cue.start_ticks);
  return Math.max(34, (duration / TICKS_PER_SECOND) * pixelsPerSecond);
}
