export interface RunStats {
  frames: number;
  fps: number;
  p50: number;
  p95: number;
  p99: number;
  max: number;
  longFrames: number;
}

export interface MeasurementOptions {
  durationMs: number;
  warmupMs: number;
  onFrame?: (elapsedMs: number) => void;
}

function percentile(sorted: number[], fraction: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(fraction * sorted.length) - 1));
  return sorted[index];
}

export function summarize(samples: number[]): RunStats {
  const deltas = samples.filter((value) => Number.isFinite(value) && value > 0);
  const sorted = [...deltas].sort((a, b) => a - b);
  const total = deltas.reduce((sum, value) => sum + value, 0);
  return {
    frames: deltas.length,
    fps: total > 0 ? (deltas.length * 1000) / total : 0,
    p50: percentile(sorted, 0.5),
    p95: percentile(sorted, 0.95),
    p99: percentile(sorted, 0.99),
    max: sorted.length > 0 ? sorted[sorted.length - 1] : 0,
    longFrames: deltas.filter((value) => value > 25).length,
  };
}

/**
 * Measures real animation-frame deltas. Warmup frames are discarded so the
 * first blur allocation does not dominate the result.
 */
export function measure(options: MeasurementOptions): Promise<RunStats> {
  const { durationMs, warmupMs, onFrame } = options;
  return new Promise((resolve) => {
    const samples: number[] = [];
    let startedAt = 0;
    let measuredAt = 0;
    let last = 0;
    let warm = false;

    const tick = (now: number) => {
      if (startedAt === 0) {
        startedAt = now;
        last = now;
      }
      if (!warm) {
        if (now - startedAt >= warmupMs) {
          warm = true;
          measuredAt = now;
          last = now;
        }
      } else {
        samples.push(now - last);
        last = now;
      }

      onFrame?.(now - startedAt);

      if (warm && now - measuredAt >= durationMs) {
        resolve(summarize(samples));
        return;
      }
      requestAnimationFrame(tick);
    };

    requestAnimationFrame(tick);
  });
}

export function medianStats(runs: RunStats[]): RunStats {
  const pick = (key: keyof RunStats) => {
    const values = runs.map((run) => run[key]).sort((a, b) => a - b);
    if (values.length === 0) return 0;
    const middle = Math.floor(values.length / 2);
    return values.length % 2 === 0 ? (values[middle - 1] + values[middle]) / 2 : values[middle];
  };
  return {
    frames: pick("frames"),
    fps: pick("fps"),
    p50: pick("p50"),
    p95: pick("p95"),
    p99: pick("p99"),
    max: pick("max"),
    longFrames: pick("longFrames"),
  };
}
