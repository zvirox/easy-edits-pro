import { useCallback, useEffect, useState } from "react";
import type { ProjectDocument, SelectedClip, TimelineTool } from "./projectTypes.ts";
import {
  planTrim,
  type TrimEdge,
  type TrimPlanInput,
  type TrimSession,
} from "./timelineTrim.ts";
import { PANEL_OPEN_EVENT } from "../features/panels/presets.ts";

type TrimInputBuilder = (
  trackId: number,
  clipIndex: number,
  edge: TrimEdge,
  deltaTicks: number,
) => TrimPlanInput | null;

type Options = {
  selectedClip: SelectedClip | null;
  project: ProjectDocument["project"] | null;
  timelineTool: TimelineTool;
  playhead: number;
  beginGesture: () => void;
  endGesture: () => Promise<void>;
  seekProgramTo: (ticks: number) => void;
  handleShuttleKey: (key: "j" | "k" | "l") => void;
  stopPlayback: () => void;
  setNotice: (message: string) => void;
  trimSession: TrimSession | null;
  setTrimSession: (session: TrimSession | null) => void;
  trimInput: TrimInputBuilder;
  commitTrim: (
    trackId: number,
    clipIndex: number,
    edge: TrimEdge,
    deltaTicks: number,
  ) => Promise<void>;
};

/**
 * Keyboard Trim Mode: J/L shuttle the playhead around the cut while the trim
 * delta follows, K stops, Enter commits one undone edit, Escape cancels.
 */
export function useTrimMode({
  selectedClip,
  project,
  timelineTool,
  playhead,
  beginGesture,
  endGesture,
  seekProgramTo,
  handleShuttleKey,
  stopPlayback,
  setNotice,
  trimSession: session,
  setTrimSession: setSession,
  trimInput,
  commitTrim,
}: Options) {
  const [trimMode, setTrimMode] = useState<{
    trackId: number;
    clipIndex: number;
    edge: TrimEdge;
    baseCut: number;
  } | null>(null);

  const enterTrimMode = useCallback(
    (requestedEdge?: TrimEdge) => {
      if (!selectedClip) {
        setNotice("Select a clip before entering Trim Mode");
        return;
      }
      const track = project?.sequence.tracks.find(
        (item) => item.id === selectedClip.track_id,
      );
      const clip = track?.clips[selectedClip.clip_index];
      if (!track || !clip) {
        setNotice("That clip is no longer available");
        return;
      }
      if (track.locked) {
        setNotice(`Track "${track.name}" is locked`);
        return;
      }
      const edge = requestedEdge ?? "end";
      const baseCut =
        edge === "end"
          ? clip.timeline_start + clip.duration
          : clip.timeline_start;
      stopPlayback();
      beginGesture();
      setTrimMode({
        trackId: selectedClip.track_id,
        clipIndex: selectedClip.clip_index,
        edge,
        baseCut,
      });
      setSession({
        trackId: selectedClip.track_id,
        clipIndex: selectedClip.clip_index,
        tool: timelineTool,
        edge,
        deltaTicks: 0,
        dynamic: true,
      });
      seekProgramTo(baseCut);
      window.setTimeout(() => {
        window.dispatchEvent(
          new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "trim-monitor" } }),
        );
      }, 0);
      setNotice(
        `Trim Mode (${edge}) · J/L shuttle · K stop · Enter commit · Esc cancel`,
      );
    },
    [
      selectedClip,
      project,
      beginGesture,
      timelineTool,
      stopPlayback,
      seekProgramTo,
      setNotice,
    ],
  );

  const handleTrimShuttle = useCallback(
    (key: "j" | "k" | "l") => {
      if (!trimMode) return;
      if (key === "k") {
        stopPlayback();
        return;
      }
      handleShuttleKey(key);
    },
    [trimMode, stopPlayback, handleShuttleKey],
  );

  const cancelTrimMode = useCallback(() => {
    if (!trimMode) return;
    stopPlayback();
    setTrimMode(null);
    setSession(null);
    void endGesture();
    seekProgramTo(trimMode.baseCut);
    setNotice("Trim cancelled");
  }, [trimMode, stopPlayback, endGesture, seekProgramTo, setNotice]);

  const commitTrimMode = useCallback(async () => {
    if (!trimMode) return;
    stopPlayback();
    const deltaTicks = session?.deltaTicks ?? 0;
    setTrimMode(null);
    if (Math.abs(deltaTicks) < 1) {
      setSession(null);
      await endGesture();
      setNotice("Trim committed with no change");
      return;
    }
    await commitTrim(trimMode.trackId, trimMode.clipIndex, trimMode.edge, deltaTicks);
  }, [trimMode, session, stopPlayback, endGesture, commitTrim, setNotice]);

  // While Trim Mode is open the playhead is the edit preview: shuttle forward
  // or backward and the trim delta follows, clamped by the same planner as a
  // pointer drag.
  useEffect(() => {
    if (!trimMode) return;
    const deltaTicks = Math.round(playhead - trimMode.baseCut);
    const input = trimInput(
      trimMode.trackId,
      trimMode.clipIndex,
      trimMode.edge,
      deltaTicks,
    );
    if (!input) return;
    const result = planTrim(input);
    if (!result.ok) {
      setNotice(result.reason);
      return;
    }
    setSession({
      trackId: trimMode.trackId,
      clipIndex: trimMode.clipIndex,
      tool: timelineTool,
      edge: trimMode.edge,
      deltaTicks: result.plan.deltaTicks,
      dynamic: true,
    });
  }, [playhead, trimMode, trimInput, timelineTool, setNotice]);

  return {
    trimMode,
    enterTrimMode,
    handleTrimShuttle,
    cancelTrimMode,
    commitTrimMode,
  };
}
