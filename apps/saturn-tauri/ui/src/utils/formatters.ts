export const TICKS_PER_SECOND = 254_016_000_000;

/**
 * Formats rational ticks into standard HH:MM:SS timecode.
 */
export function formatTime(ticks: number): string {
  const seconds = Math.max(0, Math.floor(ticks / TICKS_PER_SECOND));
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  return `${String(hours).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

/**
 * Formats rational ticks into precise HH:MM:SS:FF timecode using the sequence frame rate.
 */
export function formatTimecodeFrames(
  ticks: number,
  frameRate: { numerator: number; denominator: number } = { numerator: 30, denominator: 1 }
): string {
  const totalSeconds = Math.max(0, ticks / TICKS_PER_SECOND);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  const fps = frameRate.numerator / frameRate.denominator;
  const frameTicks = Math.round((TICKS_PER_SECOND * frameRate.denominator) / frameRate.numerator);
  const remainderTicks = Math.max(0, ticks % TICKS_PER_SECOND);
  const frames = Math.min(Math.floor(fps) - 1, Math.floor(remainderTicks / frameTicks));

  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}:${String(frames).padStart(2, "0")}`;
}

/**
 * Formats a stereo pan value (-1.0 to +1.0) into human readable notation (L 50%, C, R 75%).
 */
export function formatPan(pan: number): string {
  if (Math.abs(pan) < 0.05) return "C";
  if (pan < 0) return `L ${Math.round(Math.abs(pan) * 100)}%`;
  return `R ${Math.round(pan * 100)}%`;
}

/**
 * Formats a decibel level into "+3.2 dB", "-12.0 dB", or "-∞ dB".
 */
export function formatDb(db: number): string {
  if (!Number.isFinite(db) || db <= -59.5) return "-∞ dB";
  return `${db > 0 ? "+" : ""}${db.toFixed(1)} dB`;
}

/**
 * Formats byte sizes into human readable units (e.g. 52.4 MB).
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0 || !Number.isFinite(bytes)) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / 1024 ** i).toFixed(1)} ${units[i]}`;
}

/**
 * Clamps a number between min and max inclusive.
 */
export function clamp(val: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, val));
}

/**
 * Snaps a value to the nearest discrete interval.
 */
export function snapToNearest(val: number, interval: number): number {
  if (interval <= 0) return val;
  return Math.round(val / interval) * interval;
}

/**
 * Returns the audio waveform peak data for a clip, generating a naturalistic
 * deterministic fallback if the media asset does not yet have probed peak samples.
 */
export function getClipWaveform(
  item: { waveform?: number[] | null; name?: string; id?: number } | null | undefined,
  seedOffset = 0
): number[] {
  if (item?.waveform && Array.isArray(item.waveform) && item.waveform.length > 0) {
    return item.waveform;
  }
  const count = 72;
  const peaks: number[] = [];
  const baseSeed = (item?.id ?? 1) * 31 + seedOffset * 17 + (item?.name?.length ?? 5);
  for (let i = 0; i < count; i++) {
    const angle = (i / count) * Math.PI * 6;
    const wave = Math.abs(Math.sin(angle)) * 0.7 + Math.abs(Math.sin(angle * 2.3)) * 0.3;
    const noise = (((baseSeed * (i + 11)) % 100) / 100) * 0.35;
    const amp = Math.min(255, Math.max(35, Math.round((wave * 0.75 + noise) * 230)));
    peaks.push(amp);
  }
  return peaks;
}

