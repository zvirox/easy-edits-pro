import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { CaptionStyle, CaptionTrack, ProjectDocument, SubtitleItem } from "./projectTypes";
import { PANEL_OPEN_EVENT } from "../features/dock/panelEvents";
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
  /** All caption tracks in the sequence, in engine order. */
  captionTracks: CaptionTrack[];
  /** The track the panel edits; falls back to the first track. */
  activeTrackId: number | null;
  setActiveTrackId: (id: number | null) => void;
  track: CaptionTrack | null;
  cues: SubtitleItem[];
  /** Cue ids selected for multi-cue operations, on the active track. */
  selectedCueIds: number[];
  setSelectedCueIds: (ids: number[]) => void;
  /** Selects a cue (toggling when `additive`); seeks to a newly picked cue. */
  selectCue: (id: number | null, additive?: boolean) => void;
  addCue: () => Promise<void>;
  updateCueText: (id: number, text: string) => Promise<void>;
  updateCueSpeaker: (id: number, speaker: string | null) => Promise<void>;
  nudgeCue: (id: number, deltaTicks: number) => Promise<void>;
  removeCue: (id: number) => Promise<void>;
  /** Deletes every selected cue on the active track. */
  deleteSelectedCues: () => Promise<void>;
  /** Inserts a second cue at the playhead where it falls inside a cue. */
  splitCueAtPlayhead: () => Promise<void>;
  /** Joins the selected cues on the active track into one. */
  mergeSelectedCues: () => Promise<void>;
  /** Jumps to the next/previous caption segment and selects it. */
  nextCue: () => Promise<void>;
  previousCue: () => Promise<void>;
  /** Bumped to ask the panel to scroll the active cue into view. */
  revealTick: number;
  revealActiveCue: () => void;
  /** Selects all cues on the active track (the caption.cue_select gesture). */
  selectAllCues: () => void;
  toggleTrackVisible: (trackId: number, visible: boolean) => Promise<void>;
  showAllTracks: () => Promise<void>;
  hideAllTracks: () => Promise<void>;
  addCaptionTrack: () => Promise<void>;
  removeCaptionTrack: (trackId: number) => Promise<void>;
  renameCaptionTrack: (trackId: number, name: string) => Promise<void>;
  /** The track's first reusable style; created on first save. */
  style: CaptionStyle | null;
  saveCaptionStyle: (patch: Partial<CaptionStyle>) => Promise<void>;
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
  const [selectedCueIds, setSelectedCueIds] = useState<number[]>([]);
  // null = follow the first track; the engine keeps ids stable.
  const [activeTrackId, setActiveTrackId] = useState<number | null>(null);
  // Bumped by revealActiveCue(); the panel scrolls the active row into view.
  const [revealTick, setRevealTick] = useState(0);

  const captionTracks = useMemo(() => project?.sequence.caption_tracks ?? [], [project?.sequence]);
  const track = useMemo(() => {
    if (captionTracks.length === 0) return null;
    return captionTracks.find((entry) => entry.id === activeTrackId) ?? captionTracks[0];
  }, [captionTracks, activeTrackId]);
  const cues = useMemo(() => track?.cues ?? [], [track]);

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
      if (res) setSelectedCueIds([res.id]);
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
        setSelectedCueIds((current) => current.filter((entry) => entry !== id));
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, runCommand, refreshProject, setNotice],
  );

  /** Deletes every selected cue on the active track (caption.cue_delete). */
  const deleteSelectedCues = useCallback(async () => {
    if (!track) {
      setNotice("No caption track");
      return;
    }
    const ids = selectedCueIds.filter((id) => track.cues.some((cue) => cue.id === id));
    if (ids.length === 0) {
      setNotice("Select caption cues to delete");
      return;
    }
    try {
      const res = await runCommand<{ removed: number }>("caption.cue_delete", {
        track_id: track.id,
        ids,
      });
      await refreshProject();
      setSelectedCueIds([]);
      setNotice(`Deleted ${res?.removed ?? ids.length} caption cue(s)`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [track, selectedCueIds, runCommand, refreshProject, setNotice]);

  /** Splits the cue under the playhead at the playhead (caption.cue_split). */
  const splitCueAtPlayhead = useCallback(async () => {
    if (!track) {
      setNotice("No caption track");
      return;
    }
    const target =
      track.cues.find(
        (cue) => playhead > cue.start_ticks && playhead < cue.end_ticks,
      ) ?? null;
    if (!target) {
      setNotice("Put the playhead inside a caption cue to split it");
      return;
    }
    try {
      const res = await runCommand<{ first: number; second: number }>("caption.cue_split", {
        track_id: track.id,
        id: target.id,
        at_ticks: playhead,
      });
      await refreshProject();
      if (res) setSelectedCueIds([res.second]);
      setNotice("Caption split at the playhead");
    } catch (error) {
      setNotice(String(error));
    }
  }, [track, playhead, runCommand, refreshProject, setNotice]);

  /** Joins the selected cues into one (caption.cue_merge). */
  const mergeSelectedCues = useCallback(async () => {
    if (!track) {
      setNotice("No caption track");
      return;
    }
    const ids = [...selectedCueIds];
    if (ids.length < 2) {
      setNotice("Select at least two caption cues to merge");
      return;
    }
    try {
      const res = await runCommand<{ id: number }>("caption.cue_merge", {
        track_id: track.id,
        ids,
      });
      await refreshProject();
      if (res) setSelectedCueIds([res.id]);
      setNotice("Caption cues merged");
    } catch (error) {
      setNotice(String(error));
    }
  }, [track, selectedCueIds, runCommand, refreshProject, setNotice]);

  /** Jump navigation across the caption segments (caption.next/previous). */
  const jumpCue = useCallback(
    async (id: "caption.next" | "caption.previous") => {
      try {
        const res = await runCommand<{ target: number | null; id: number | null }>(id, {
          from: playhead,
          ...(track ? { track_id: track.id } : {}),
        });
        if (!res?.target) {
          setNotice(id === "caption.next" ? "No next caption segment" : "No previous caption segment");
          return;
        }
        seek(res.target);
        setSelectedCueIds(res.id ? [res.id] : []);
        setNotice(id === "caption.next" ? "At the next caption segment" : "At the previous caption segment");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [playhead, track, runCommand, seek, setNotice],
  );

  const nextCue = useCallback(() => jumpCue("caption.next"), [jumpCue]);
  const previousCue = useCallback(() => jumpCue("caption.previous"), [jumpCue]);

  /** Selects all cues on the active track (the caption.cue_select gesture). */
  const selectAllCues = useCallback(() => {
    if (cues.length === 0) {
      setNotice("No caption cues to select");
      return;
    }
    setSelectedCueIds(cues.map((cue) => cue.id));
    setNotice(`Selected ${cues.length} caption cue(s)`);
  }, [cues, setNotice]);

  const revealActiveCue = useCallback(() => {
    openPanel();
    setRevealTick((tick) => tick + 1);
  }, [openPanel]);

  /** Track visibility drives the program overlay (caption.tracks_show/hide). */
  const toggleTrackVisible = useCallback(
    async (trackId: number, visible: boolean) => {
      try {
        await runCommand(visible ? "caption.tracks_show" : "caption.tracks_hide", {
          track_ids: [trackId],
        });
        await refreshProject();
        setNotice(visible ? "Caption track shown" : "Caption track hidden");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const showAllTracks = useCallback(async () => {
    try {
      const res = await runCommand<{ shown: number }>("caption.tracks_show", {});
      await refreshProject();
      setNotice(`Shown ${res?.shown ?? 0} caption track(s)`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [runCommand, refreshProject, setNotice]);

  const hideAllTracks = useCallback(async () => {
    try {
      const res = await runCommand<{ hidden: number }>("caption.tracks_hide", {});
      await refreshProject();
      setNotice(`Hidden ${res?.hidden ?? 0} caption track(s)`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [runCommand, refreshProject, setNotice]);

  /** FC Show Active Caption Tracks Only: solo the active track. */
  const showActiveTrackOnly = useCallback(async () => {
    if (activeTrackId == null) {
      setNotice("No active caption track to keep visible");
      return;
    }
    try {
      await runCommand("caption.tracks_hide", {});
      await runCommand<{ shown: number }>("caption.tracks_show", {
        track_ids: [activeTrackId],
      });
      await refreshProject();
      setNotice("Showing the active caption track only");
    } catch (error) {
      setNotice(String(error));
    }
  }, [activeTrackId, runCommand, refreshProject, setNotice]);

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

  const addCaptionTrack = useCallback(async () => {
    try {
      const res = await runCommand<{ id: number }>("caption.track_add", {
        name: `Captions ${captionTracks.length + 1}`,
      });
      await refreshProject();
      if (res) setActiveTrackId(res.id);
      setNotice("Caption track added");
    } catch (error) {
      setNotice(String(error));
    }
  }, [captionTracks.length, runCommand, refreshProject, setNotice]);

  const removeCaptionTrack = useCallback(
    async (trackId: number) => {
      try {
        await runCommand("caption.track_remove", { id: trackId });
        await refreshProject();
        setActiveTrackId((current) => (current === trackId ? null : current));
        setNotice("Caption track removed");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const renameCaptionTrack = useCallback(
    async (trackId: number, name: string) => {
      try {
        await runCommand("caption.track_rename", { id: trackId, name });
        await refreshProject();
        setNotice("Caption track renamed");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  /** The track's first reusable style; created on first save. */
  const style = track?.styles[0] ?? null;

  const saveCaptionStyle = useCallback(
    async (patch: Partial<CaptionStyle>) => {
      if (!track) {
        setNotice("No caption track — add one first");
        return;
      }
      try {
        await runCommand("caption.style_set", {
          track_id: track.id,
          ...(style ? { id: style.id } : {}),
          name: style?.name ?? "Default",
          ...patch,
        });
        await refreshProject();
        setNotice("Caption style saved");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [track, style, runCommand, refreshProject, setNotice],
  );

  /** Panel-gesture entry points for the shipped caption commands. */
  const handlePanelGesture = useCallback(
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
      if (id === "caption.track_add") {
        await addCaptionTrack();
        return true;
      }
      if (id === "caption.track_remove") {
        if (!track) {
          setNotice("No caption track to remove");
          return true;
        }
        await removeCaptionTrack(track.id);
        return true;
      }
      if (id === "caption.track_rename") {
        if (!track) {
          setNotice("No caption track to rename");
          return true;
        }
        const value = window.prompt("Rename caption track", track.name);
        if (value === null) return true;
        const next = value.trim();
        if (next && next !== track.name) await renameCaptionTrack(track.id, next);
        return true;
      }
      if (id === "caption.style_set") {
        // The style fields live in the panel's style section.
        openPanel();
        setNotice("Use the Captions panel style section to edit the caption look");
        return true;
      }
      return false;
    },
    [importCues, exportCues, addCue, addCaptionTrack, removeCaptionTrack, renameCaptionTrack, openPanel, track, setNotice],
  );

  /** Select a cue (togglable with meta/shift) and park the playhead on it. */
  const selectCue = useCallback(
    (id: number | null, additive = false) => {
      if (id == null) {
        setSelectedCueIds([]);
        return;
      }
      setSelectedCueIds((current) => {
        if (!additive) return [id];
        return current.includes(id)
          ? current.filter((entry) => entry !== id)
          : [...current, id];
      });
      const cue = cues.find((entry) => entry.id === id);
      if (cue) seek(cue.start_ticks);
    },
    [cues, seek],
  );

  /** Menu entry points for the shipped caption commands. */
  const handleCaptionMenuCommand = useCallback(
    async (id: string): Promise<boolean> => {
      if (await handlePanelGesture(id)) return true;
      if (id === "caption.list") {
        try {
          const res = await runCommand<{ tracks: unknown[]; cues: number }>("caption.list", {});
          openPanel();
          setNotice(`${res.tracks.length} caption track(s), ${res.cues} cue(s)`);
        } catch (error) {
          setNotice(String(error));
        }
        return true;
      }
      if (id === "caption.cue_delete") {
        await deleteSelectedCues();
        return true;
      }
      if (id === "caption.cue_split") {
        await splitCueAtPlayhead();
        return true;
      }
      if (id === "caption.cue_merge") {
        await mergeSelectedCues();
        return true;
      }
      if (id === "caption.next") {
        await nextCue();
        return true;
      }
      if (id === "caption.previous") {
        await previousCue();
        return true;
      }
      if (id === "caption.tracks_hide") {
        await hideAllTracks();
        return true;
      }
      if (id === "caption.tracks_show") {
        await showAllTracks();
        return true;
      }
      if (id === "caption.show_active_only") {
        await showActiveTrackOnly();
        return true;
      }
      if (id === "caption.cue_select") {
        // UI-only: the selection lives in the panel; the engine stays out.
        selectAllCues();
        return true;
      }
      if (id === "caption.go_to") {
        // UI-only: scroll the active cue into view in the panel.
        revealActiveCue();
        return true;
      }
      return false;
    },
    [
      runCommand,
      openPanel,
      setNotice,
      deleteSelectedCues,
      splitCueAtPlayhead,
      mergeSelectedCues,
      nextCue,
      previousCue,
      hideAllTracks,
      showAllTracks,
      showActiveTrackOnly,
      selectAllCues,
      revealActiveCue,
      handlePanelGesture,
    ],
  );

  return {
    captionTracks,
    activeTrackId,
    setActiveTrackId,
    track,
    cues,
    selectedCueIds,
    setSelectedCueIds,
    selectCue,
    addCue,
    updateCueText,
    updateCueSpeaker,
    nudgeCue,
    removeCue,
    deleteSelectedCues,
    splitCueAtPlayhead,
    mergeSelectedCues,
    nextCue,
    previousCue,
    revealTick,
    revealActiveCue,
    selectAllCues,
    toggleTrackVisible,
    showAllTracks,
    hideAllTracks,
    addCaptionTrack,
    removeCaptionTrack,
    renameCaptionTrack,
    style,
    saveCaptionStyle,
    importCues,
    exportCues,
    handleMenuCommand: handleCaptionMenuCommand,
    openPanel,
  };
}
