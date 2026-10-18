import { useCallback, type KeyboardEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  DavinciWorkspace,
  ProjectStatus,
  SelectedClip,
  TimelineTool,
} from "./projectTypes";

export type { DavinciWorkspace, TimelineTool, SelectedClip };
export type SelectedTransition = { track_id: number; transition: { id: number; kind: string; cut_at: number; duration: number } };
export type ShortcutCommand = { id: string; shortcut: string | null };
export type ShortcutTimelineTool = { id: TimelineTool; shortcut: string };

type Options = {
  commands: ShortcutCommand[];
  timelineTools: ShortcutTimelineTool[];
  project: ProjectStatus["document"]["project"] | null;
  multicamMode: boolean;
  selectedTransition: SelectedTransition | null;
  selectedClip: SelectedClip | null;
  timelineZoom: number;
  toggleProgramPlayback: () => void;
  switchMulticamAngle: (angle: number) => void | Promise<void>;
  handleShuttleKey: (key: "j" | "k" | "l") => void;
  stepProgramFrame: (direction: -1 | 1) => void;
  seekProgramTo: (time: number) => void;
  removeSelectedTransition: () => void | Promise<void>;
  removeSelectedClip: () => void | Promise<void>;
  addTitleToTimeline: () => void | Promise<void>;
  addTransitionToTimeline: () => void | Promise<void>;
  unlinkSelectedClips: () => void | Promise<void>;
  linkSelectedClips: () => void | Promise<void>;
  decomposeSelectedCompoundClip: () => void | Promise<void>;
  setCompoundClipNameDraft: (name: string) => void;
  setCompoundClipModalOpen: (open: boolean) => void;
  toggleTheme: () => void;
  selectDavinciWorkspace: (workspace: DavinciWorkspace) => void;
  setTimelineTool: (tool: TimelineTool) => void;
  setSnapEnabled: (updater: (enabled: boolean) => boolean) => void;
  zoomTimeline: (factor: number) => void;
  invokeMenuCommand: (id: string) => Promise<unknown>;
  setNotice: (notice: string) => void;
};

export function useShortcuts(options: Options) {
  return useCallback((event: KeyboardEvent<HTMLDivElement>) => {
    const target = event.target;
    if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
    if (event.key === " " || event.code === "Space") { event.preventDefault(); options.toggleProgramPlayback(); return; }
    if (options.multicamMode && ["1", "2", "3", "4"].includes(event.key) && !(event.ctrlKey || event.metaKey || event.altKey)) {
      event.preventDefault(); void options.switchMulticamAngle(Number(event.key)); return;
    }
    if (["j", "k", "l"].includes(event.key.toLowerCase()) && !(event.ctrlKey || event.metaKey || event.altKey)) {
      event.preventDefault(); options.handleShuttleKey(event.key.toLowerCase() as "j" | "k" | "l"); return;
    }
    if (event.key === "ArrowLeft" && !(event.ctrlKey || event.metaKey || event.altKey)) { event.preventDefault(); options.stepProgramFrame(-1); return; }
    if (event.key === "ArrowRight" && !(event.ctrlKey || event.metaKey || event.altKey)) { event.preventDefault(); options.stepProgramFrame(1); return; }
    if (event.key === "Home") { event.preventDefault(); options.seekProgramTo(0); return; }
    if (event.key === "End") {
      event.preventDefault();
      const duration = Math.max(0, ...(options.project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.timeline_start + clip.duration)));
      options.seekProgramTo(duration); return;
    }
    if ((event.key === "Delete" || event.key === "Backspace") && options.selectedTransition) {
      event.preventDefault(); void options.removeSelectedTransition(); return;
    }
    if ((event.key === "Delete" || event.key === "Backspace") && options.selectedClip) {
      event.preventDefault(); void options.removeSelectedClip(); return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "t") { event.preventDefault(); void options.addTitleToTimeline(); return; }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "d") { event.preventDefault(); void options.addTransitionToTimeline(); return; }
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "l") { event.preventDefault(); void options.unlinkSelectedClips(); return; }
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "l") { event.preventDefault(); void options.linkSelectedClips(); return; }
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "g") { event.preventDefault(); void options.decomposeSelectedCompoundClip(); return; }
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "g") {
      event.preventDefault(); options.setCompoundClipNameDraft(`Compound Clip ${(options.project?.sequences?.length ?? 1) + 1}`);
      options.setCompoundClipModalOpen(true); return;
    }
    if ((event.ctrlKey || event.metaKey) && event.altKey && event.key.toLowerCase() === "t") { event.preventDefault(); options.toggleTheme(); return; }
    if (event.shiftKey && ["1", "2", "3", "4", "5", "6", "7"].includes(event.key)) {
      event.preventDefault();
      const spaces: Record<string, DavinciWorkspace> = { "1": "media", "2": "edit", "3": "color", "4": "audio", "5": "effects", "6": "keyframes", "7": "deliver" };
      options.selectDavinciWorkspace(spaces[event.key]); return;
    }
    if (!(event.ctrlKey || event.metaKey || event.altKey)) {
      const tool = options.timelineTools.find((item) => item.shortcut.toLowerCase() === event.key.toLowerCase());
      if (tool) { event.preventDefault(); options.setTimelineTool(tool.id); return; }
      if (event.key.toLowerCase() === "s") { event.preventDefault(); options.setSnapEnabled((enabled) => !enabled); return; }
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") { event.preventDefault(); void getCurrentWindow().close(); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "+") { event.preventDefault(); options.zoomTimeline(1.2); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "-") { event.preventDefault(); options.zoomTimeline(1 / 1.2); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "0") { event.preventDefault(); options.zoomTimeline(72 / options.timelineZoom); return; }
    const pressed = `${event.ctrlKey || event.metaKey ? "Ctrl+" : ""}${event.shiftKey ? "Shift+" : ""}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`;
    const command = options.commands.find((item) => item.shortcut?.toLowerCase() === pressed.toLowerCase());
    if (!command) return;
    event.preventDefault();
    void options.invokeMenuCommand(command.id).catch((error: unknown) => options.setNotice(String(error)));
  }, [options]);
}
