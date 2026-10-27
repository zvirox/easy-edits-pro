/**
 * Timeline ruler step sizing, ported from the reference editor's `ruler_steps`:
 * labels sit at least ~110px apart and minor ticks at least ~12px apart,
 * so the density adapts to the current zoom (px per second).
 */
const LABEL_STEPS = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1800, 3600];
const LABEL_MIN_PX = 110;
const TICK_MIN_PX = 12;

export interface RulerSteps {
  /** Seconds between labelled major ticks. */
  major: number;
  /** Seconds between unlabelled minor ticks (divisor of `major`). */
  minor: number;
}

export function rulerSteps(zoomPxPerSecond: number): RulerSteps {
  const zoom = zoomPxPerSecond > 0 ? zoomPxPerSecond : Number.EPSILON;
  let major = LABEL_STEPS[LABEL_STEPS.length - 1];
  for (const step of LABEL_STEPS) {
    if (step * zoom >= LABEL_MIN_PX) {
      major = step;
      break;
    }
  }
  let minor = major;
  for (let divisor = 9; divisor >= 1; divisor -= 1) {
    if (major % divisor === 0 && (major / divisor) * zoom >= TICK_MIN_PX) {
      minor = major / divisor;
      break;
    }
  }
  return { major, minor };
}
