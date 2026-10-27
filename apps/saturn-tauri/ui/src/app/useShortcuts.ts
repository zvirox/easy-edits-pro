import { useCallback, useEffect, type KeyboardEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  EditorWorkspace,
  ProjectStatus,
  SelectedClip,
  TimelineTool,
} from "./projectTypes";

export type { EditorWorkspace, TimelineTool, SelectedClip };
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
  trimActive: boolean;
  handleTrimShuttle: (key: "j" | "k" | "l") => void;
  commitTrim: () => void | Promise<void>;
  cancelTrim: () => void | Promise<void>;
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
  selectEditorWorkspace: (workspace: EditorWorkspace) => void;
  setTimelineTool: (tool: TimelineTool) => void;
  setSnapEnabled: (updater: (enabled: boolean) => boolean) => void;
  zoomTimeline: (factor: number) => void;
  invokeMenuCommand: (id: string) => Promise<unknown>;
  setNotice: (notice: string) => void;
};

export function useShortcuts(options: Options) {
  const handleKeyDown = useCallback((event: KeyboardEvent<HTMLDivElement>) => {
    const target = event.target;
    if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
    if (options.trimActive) {
      if (["j", "k", "l"].includes(event.key.toLowerCase()) && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault(); options.handleTrimShuttle(event.key.toLowerCase() as "j" | "k" | "l"); return;
      }
      if (event.key === "Enter") { event.preventDefault(); void options.commitTrim(); return; }
      if (event.key === "Escape") { event.preventDefault(); void options.cancelTrim(); return; }
    }
    if (event.key === " " || event.code === "Space") { event.preventDefault(); options.toggleProgramPlayback(); return; }
    if (options.multicamMode && ["1", "2", "3", "4"].includes(event.key) && !(event.ctrlKey || event.metaKey || event.altKey)) {
      event.preventDefault(); void options.switchMulticamAngle(Number(event.key)); return;
    }
    if (["j", "k", "l"].includes(event.key.toLowerCase()) && !(event.ctrlKey || event.metaKey || event.altKey)) {
      event.preventDefault(); options.handleShuttleKey(event.key.toLowerCase() as "j" | "k" | "l"); return;
    }
    // Shift+Arrow belongs to the catalogue (marker previous/next), so frame
    // stepping must not swallow the modified chord.
    if (event.key === "ArrowLeft" && !(event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)) { event.preventDefault(); options.stepProgramFrame(-1); return; }
    if (event.key === "ArrowRight" && !(event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)) { event.preventDefault(); options.stepProgramFrame(1); return; }
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
    if (!(event.ctrlKey || event.metaKey || event.altKey)) {
      const toolKey = `${event.shiftKey ? "shift+" : ""}${event.key.toLowerCase()}`;
      const tool = options.timelineTools.find((item) => item.shortcut.toLowerCase() === toolKey);
      if (tool) { event.preventDefault(); options.setTimelineTool(tool.id); return; }
      if (event.key.toLowerCase() === "s") { event.preventDefault(); options.setSnapEnabled((enabled) => !enabled); return; }
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") { event.preventDefault(); void getCurrentWindow().close(); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "+") { event.preventDefault(); options.zoomTimeline(1.2); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "-") { event.preventDefault(); options.zoomTimeline(1 / 1.2); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "0") { event.preventDefault(); options.zoomTimeline(72 / options.timelineZoom); return; }
    // Catalogue shortcuts use the short arrow names ("Left", "Down") while
    // the DOM reports "ArrowLeft"/"ArrowDown"; normalise so both match.
    const keyName = event.key.length === 1
      ? event.key.toUpperCase()
      : event.key.replace(/^Arrow/, "");
    const pressed = `${event.ctrlKey || event.metaKey ? "Ctrl+" : ""}${event.altKey ? "Alt+" : ""}${event.shiftKey ? "Shift+" : ""}${keyName}`;
    const command = options.commands.find((item) => item.shortcut?.toLowerCase() === pressed.toLowerCase());
    if (!command) return;
    event.preventDefault();
    void options.invokeMenuCommand(command.id).catch((error: unknown) => options.setNotice(String(error)));
  }, [options]);

  useEffect(() => {
    const fallback = (event: globalThis.KeyboardEvent) => {
      if (document.activeElement && document.activeElement !== document.body) return;
      handleKeyDown(event as unknown as KeyboardEvent<HTMLDivElement>);
    };
    window.addEventListener("keydown", fallback);
    return () => window.removeEventListener("keydown", fallback);
  }, [handleKeyDown]);

  return handleKeyDown;
}
