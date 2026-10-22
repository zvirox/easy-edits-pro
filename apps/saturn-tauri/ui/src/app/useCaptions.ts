import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { CaptionTrack, ProjectDocument, SubtitleItem } from "./projectTypes";
import { PANEL_OPEN_EVENT } from "../features/panels/presets";
import { defaultCaptionCues, defaultCaptionTrack } from "./captions";
import { TICKS_PER_SECOND } from "../utils/formatters.ts";

export const CAPTIONS_PANEL_ID = "captions";

export interface UseCaptionsOptions {
  project: ProjectDocument["project"] | null;
  playhead: number;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
  seek: (tick: number) => void;
}

export interface CaptionOperations {
  track: CaptionTrack | null;
  cues: SubtitleItem[];
  selectedCueId: number | null;
  setSelectedCueId: (id: number | null) => void;
  /** Selects the cue and seeks to its start; `null` clears the selection. */
  selectCue: (id: number | null) => void;
  addCue: () => Promise<void>;
  updateCueText: (id: number, text: string) => Promise<void>;
  updateCueSpeaker: (id: number, speaker: string | null) => Promise<void>;
  nudgeCue: (id: number, deltaTicks: number) => Promise<void>;
  removeCue: (id: number) => Promise<void>;
  importCues: () => Promise<void>;
  exportCues: (format: "srt" | "vtt") => Promise<void>;
  /** Handles the caption commands that need UI gestures; false = not ours. */
  handleMenuCommand: (id: string) => Promise<boolean>;
  openPanel: () => void;
}

const SRT_VTT_FILTERS = [
  { name: "Captions (SRT/VTT)", extensions: ["srt", "vtt"] },
];

export function useCaptions({
  project,
  playhead,
  runCommand,
  refreshProject,
  setNotice,
  seek,
}: UseCaptionsOptions): CaptionOperations {
  const [selectedCueId, setSelectedCueId] = useState<number | null>(null);

  const track = useMemo(() => defaultCaptionTrack(project?.sequence), [project?.sequence]);
  const cues = useMemo(() => defaultCaptionCues(project?.sequence), [project?.sequence]);

  const openPanel = useCallback(() => {
    const announce = () =>
      window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: CAPTIONS_PANEL_ID } }));
    announce();
    // The panel is hosted by the Edit workspace. When the caller switches to
    // it in the same tick, the host subscribes a frame later, so re-announce.
    window.setTimeout(announce, 120);
  }, []);

  const addCue = useCallback(async () => {
    const start = playhead;
    const end = playhead + 3 * TICKS_PER_SECOND;
    try {
      const res = await runCommand<{ id: number }>("caption.cue_add", {
        start_ticks: start,
        end_ticks: end,
        text: "New caption",
      });
      await refreshProject();
      if (res) setSelectedCueId(res.id);
      setNotice("Caption cue added at playhead");
    } catch (error) {
      setNotice(String(error));
    }
  }, [playhead, runCommand, refreshProject, setNotice]);

  const updateCueText = useCallback(
    async (id: number, text: string) => {
      if (!track) return;
      try {
        await runCommand("caption.cue_update", { track_id: track.id, id, text });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, runCommand, refreshProject, setNotice],
  );

  const updateCueSpeaker = useCallback(
    async (id: number, speaker: string | null) => {
      if (!track) return;
      try {
        await runCommand("caption.cue_update", { track_id: track.id, id, speaker });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, runCommand, refreshProject, setNotice],
  );

  const nudgeCue = useCallback(
    async (id: number, deltaTicks: number) => {
      if (!track) return;
      try {
        await runCommand("caption.cue_nudge", { track_id: track.id, id, delta_ticks: deltaTicks });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, runCommand, refreshProject, setNotice],
  );

  const removeCue = useCallback(
    async (id: number) => {
      if (!track) return;
      try {
        await runCommand("caption.cue_remove", { track_id: track.id, id });
        await refreshProject();
        if (selectedCueId === id) setSelectedCueId(null);
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, selectedCueId, runCommand, refreshProject, setNotice],
  );

  const importCues = useCallback(async () => {
    const selected = await open({ multiple: false, filters: SRT_VTT_FILTERS });
    if (typeof selected !== "string" || !selected) return;
    try {
      const content = await invoke<string>("read_text_file", { path: selected });
      const result = await runCommand<{ imported: number }>("caption.import", { content });
      await refreshProject();
      setNotice(`Imported ${result?.imported ?? 0} caption cue(s)`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [runCommand, refreshProject, setNotice]);

  const exportCues = useCallback(
    async (format: "srt" | "vtt") => {
      if (cues.length === 0) {
        setNotice("No caption cues to export");
        return;
      }
      const selected = await save({
        defaultPath: `${project?.name ?? "Captions"}.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (typeof selected !== "string" || !selected) return;
      // The Tauri command serialises the default track's current cues.
      await invoke("export_subtitles_file", { path: selected, format });
      setNotice(`Captions exported as ${format.toUpperCase()}`);
    },
    [cues.length, project?.name, setNotice],
  );

  /** Menu entry points for the shipped caption commands (panel gestures). */
  const handleMenuCommand = useCallback(
    async (id: string): Promise<boolean> => {
      if (id === "caption.import") {
        await importCues();
        return true;
      }
      if (id === "caption.export") {
        await exportCues("srt");
        return true;
      }
      if (id === "caption.cue_add") {
        await addCue();
        return true;
      }
      return false;
    },
    [importCues, exportCues, addCue],
  );

  /** Select a cue and park the playhead at its start. */
  const selectCue = useCallback(
    (id: number | null) => {
      setSelectedCueId(id);
      if (id == null) return;
      const cue = cues.find((entry) => entry.id === id);
      if (cue) seek(cue.start_ticks);
    },
    [cues, seek],
  );

  return {
    track,
    cues,
    selectedCueId,
    setSelectedCueId,
    selectCue,
    addCue,
    updateCueText,
    updateCueSpeaker,
    nudgeCue,
    removeCue,
    importCues,
    exportCues,
    handleMenuCommand,
    openPanel,
  };
}
