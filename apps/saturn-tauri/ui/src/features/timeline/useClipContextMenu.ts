import { useCallback, useMemo, useState } from "react";
import type { Dispatch, MouseEvent, SetStateAction } from "react";
import type {
  SelectedClip,
  Track,
  Transition,
} from "../../app/projectTypes";
import { TICKS_PER_SECOND } from "./types";
import type { ClipMenuCommand } from "./ClipContextMenu";

const CLIP_CONTEXT_COMMAND_IDS = new Set([
  "timeline.remove_clip",
  "timeline.split_clip",
  "timeline.ripple_delete",
  "timeline.ripple_delete_batch",
  "timeline.rate_stretch",
]);

type ClipMenuState = {
  x: number;
  y: number;
  trackId: number;
  clipIndex: number;
};

export interface UseClipContextMenuOptions {
  tracks: Track[];
  commands: ClipMenuCommand[];
  busy: boolean;
  playhead: number;
  invokeMenuCommand: (id: string) => Promise<void>;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;
  setSelectedClips: Dispatch<SetStateAction<SelectedClip[]>>;
  setSelectedTransition: Dispatch<
    SetStateAction<{ track_id: number; transition: Transition } | null>
  >;
}

/** Right-click target, catalogue menu, and enablement reasons for timeline clips. */
export function useClipContextMenu({
  tracks,
  commands,
  busy,
  playhead,
  invokeMenuCommand,
  setSelectedClip,
  setSelectedClips,
  setSelectedTransition,
}: UseClipContextMenuOptions) {
  const [clipMenu, setClipMenu] = useState<ClipMenuState | null>(null);

  const clipMenuCommands = useMemo(
    () =>
      commands.filter(
        (command) =>
          command.shipped !== false && CLIP_CONTEXT_COMMAND_IDS.has(command.id),
      ),
    [commands],
  );

  const clipMenuClip = clipMenu
    ? (tracks.find((track) => track.id === clipMenu.trackId)?.clips[
        clipMenu.clipIndex
      ] ?? null)
    : null;

  const disabledReasons = useMemo(() => {
    const reasons: Record<string, string | null> = {};
    for (const command of clipMenuCommands) {
      reasons[command.id] = busy ? "Working…" : null;
    }
    if (clipMenuClip) {
      const start = clipMenuClip.timeline_start;
      const end = start + clipMenuClip.duration;
      const playheadTicks = Math.round(playhead * TICKS_PER_SECOND);
      if (playheadTicks <= start || playheadTicks >= end) {
        reasons["timeline.split_clip"] = "Playhead is outside this clip";
      }
    }
    return reasons;
  }, [clipMenuCommands, clipMenuClip, busy, playhead]);

  const openClipContextMenu = useCallback(
    (event: MouseEvent<HTMLDivElement>, trackId: number, clipIndex: number) => {
      event.preventDefault();
      event.stopPropagation();
      const clip = { track_id: trackId, clip_index: clipIndex };
      setSelectedClip(clip);
      setSelectedClips([clip]);
      setSelectedTransition(null);
      setClipMenu({
        x: event.clientX,
        y: event.clientY,
        trackId,
        clipIndex,
      });
    },
    [setSelectedClip, setSelectedClips, setSelectedTransition],
  );

  const selectClipCommand = useCallback(
    (id: string) => {
      setClipMenu(null);
      void invokeMenuCommand(id);
    },
    [invokeMenuCommand],
  );

  const closeClipMenu = useCallback(() => setClipMenu(null), []);

  return {
    clipMenu,
    clipMenuCommands,
    disabledReasons,
    openClipContextMenu,
    selectClipCommand,
    closeClipMenu,
  };
}
