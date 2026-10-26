import { TICKS_PER_SECOND } from "./projectTypes.ts";

export interface TrimTransportContext {
  /** The keyboard trim session is open. */
  trimActive: boolean;
  /** The cut the trim session pivots around, when active. */
  baseCut: number | null;
  /** J/K/L shuttle into the trim mode handler. */
  handleShuttle: (key: "j" | "k" | "l") => void;
  cancelDynamic: () => void;
  seek: (ticks: number) => void;
  playhead: number;
  frameRate: { numerator: number; denominator: number } | null;
  notice: (message: string) => void;
}

const TRANSPORT_COMMANDS = new Set([
  "trim.shuttle",
  "trim.shuttleStop",
  "trim.tick",
  "trim.playAround",
  "trim.cancelDynamic",
]);

export function isTrimTransportCommand(id: string): boolean {
  return TRANSPORT_COMMANDS.has(id);
}

/**
 * The ui_only dynamic-trim transports: they ride the keyboard trim session
 * (useTrimMode) instead of the engine. Returns false when the id is not one
 * of these commands so the dispatcher can fall through.
 */
export function handleTrimTransportCommand(
  id: string,
  ctx: TrimTransportContext,
): boolean {
  switch (id) {
    case "trim.shuttle": {
      if (!ctx.trimActive) {
        ctx.notice("Enter Trim Mode first (Ctrl+Alt+E)");
        return true;
      }
      const input = window.prompt("Shuttle direction (-1 back, 0 stop, 1 forward)", "1");
      if (input === null) return true;
      const direction = Number(input.trim());
      if (!Number.isFinite(direction)) {
        ctx.notice("Enter -1, 0 or 1");
        return true;
      }
      ctx.handleShuttle(direction < 0 ? "j" : direction > 0 ? "l" : "k");
      return true;
    }
    case "trim.shuttleStop":
      if (!ctx.trimActive) {
        ctx.notice("Enter Trim Mode first (Ctrl+Alt+E)");
        return true;
      }
      ctx.handleShuttle("k");
      return true;
    case "trim.tick": {
      if (!ctx.trimActive) {
        ctx.notice("Enter Trim Mode first (Ctrl+Alt+E)");
        return true;
      }
      const input = window.prompt("Advance trim playback by signed frames", "1");
      if (input === null) return true;
      const frames = Number(input.trim());
      if (!Number.isInteger(frames)) {
        ctx.notice("Enter a signed integer frame count");
        return true;
      }
      const rate = ctx.frameRate;
      const fps = rate ? rate.numerator / rate.denominator : 30;
      const ticks = Math.round((frames * TICKS_PER_SECOND) / fps);
      ctx.seek(Math.max(0, ctx.playhead + ticks));
      return true;
    }
    case "trim.playAround": {
      if (!ctx.trimActive) {
        ctx.notice("Enter Trim Mode first (Ctrl+Alt+E)");
        return true;
      }
      const base = ctx.baseCut ?? ctx.playhead;
      ctx.seek(Math.max(0, base - 2 * TICKS_PER_SECOND));
      ctx.handleShuttle("l");
      return true;
    }
    case "trim.cancelDynamic":
      ctx.cancelDynamic();
      return true;
    default:
      return false;
  }
}
