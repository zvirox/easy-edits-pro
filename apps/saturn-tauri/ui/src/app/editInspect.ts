import { TICKS_PER_SECOND } from "../engine/playback/constants.ts";
import type { SelectedClip } from "./projectTypes.ts";

type Runner = <T>(id: string, params?: unknown) => Promise<T>;

/**
 * Run `timeline.inspect` or `trim.inspect` for the selected clip and return a
 * one-line status message. The inspector stays honest: it reports the engine's
 * own bounds rather than recomputing them in the UI.
 */
export async function inspectSelectedEdit(
  runCommand: Runner,
  id: "timeline.inspect" | "trim.inspect",
  selected: SelectedClip,
): Promise<string> {
  if (id === "timeline.inspect") {
    const result = await runCommand<Record<string, unknown>>("timeline.inspect", selected);
    return `Clip ${selected.clip_index} · ${Number(result.start) / TICKS_PER_SECOND}s → ${Number(result.end) / TICKS_PER_SECOND}s · source ${Number(result.source_in) / TICKS_PER_SECOND}s`;
  }
  const result = await runCommand<Record<string, unknown>>("trim.inspect", {
    ...selected,
    edge: "end",
  });
  return `Trim end · head room ${(Number(result.head_room) / TICKS_PER_SECOND).toFixed(2)}s · tail room ${(Number(result.tail_room) / TICKS_PER_SECOND).toFixed(2)}s`;
}

/**
 * Handle `timeline.inspect` / `trim.inspect` for the menu dispatcher.
 * Returns false when the command is not an inspect command or no clip is
 * selected, so the dispatcher can fall through.
 */
export async function handleInspectMenuCommand(
  id: string,
  deps: {
    selected: SelectedClip | null;
    runCommand: Runner;
    setNotice: (message: string) => void;
  },
): Promise<boolean> {
  if (id !== "timeline.inspect" && id !== "trim.inspect") return false;
  const { selected, runCommand, setNotice } = deps;
  if (!selected) {
    setNotice("Select a clip to inspect its edit points");
    return true;
  }
  try {
    setNotice(await inspectSelectedEdit(runCommand, id, selected));
  } catch (error) {
    setNotice(String(error));
  }
  return true;
}
