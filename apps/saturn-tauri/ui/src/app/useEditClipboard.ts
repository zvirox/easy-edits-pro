import { useCallback, useRef } from "react";
import type {
  MediaAsset,
  ProjectDocument,
  SelectedClip,
} from "./projectTypes.ts";

type Runner = <T>(id: string, params?: unknown) => Promise<T>;

type Options = {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  playhead: number;
  selectedClip: SelectedClip | null;
  selectedClips: SelectedClip[];
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  runCommand: Runner;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
};

type ClipboardItem = {
  mediaId: number;
  sourceIn: number;
  duration: number;
  offset: number;
  kind: "video" | "audio";
};

export type EditClipboardCommand =
  | "edit.cut"
  | "edit.copy"
  | "edit.paste"
  | "edit.delete"
  | "edit.select_all"
  | "edit.deselect_all"
  | "edit.select_clip_at_playhead";

export const EDIT_CLIPBOARD_COMMANDS: EditClipboardCommand[] = [
  "edit.cut",
  "edit.copy",
  "edit.paste",
  "edit.delete",
  "edit.select_all",
  "edit.deselect_all",
  "edit.select_clip_at_playhead",
];

/**
 * Edit-menu clipboard and selection. Every multi-clip mutation runs inside one
 * engine gesture, so a cut, paste, or delete is a single undo entry.
 */
export function useEditClipboard({
  project,
  media,
  playhead,
  selectedClip,
  selectedClips,
  setSelectedClip,
  setSelectedClips,
  runCommand,
  refreshProject,
  setNotice,
}: Options) {
  const clipboard = useRef<ClipboardItem[]>([]);

  const selection = useCallback((): SelectedClip[] => {
    if (selectedClips.length > 0) return selectedClips;
    return selectedClip ? [selectedClip] : [];
  }, [selectedClip, selectedClips]);

  const resolved = useCallback(() => {
    return selection().flatMap((reference) => {
      const track = project?.sequence.tracks.find(
        (item) => item.id === reference.track_id,
      );
      const clip = track?.clips[reference.clip_index];
      const asset = clip ? media.find((item) => item.id === clip.media_id) : null;
      return clip && track
        ? [{
            reference,
            clip,
            kind: (asset?.kind === "audio" ? "audio" : "video") as "video" | "audio",
          }]
        : [];
    });
  }, [selection, project, media]);

  const removeAll = useCallback(
    async (items: ReturnType<typeof resolved>, notice: string) => {
      try {
        await runCommand("edit.begin_gesture");
        for (const { reference } of items) {
          await runCommand("timeline.remove_clip", reference);
        }
        await runCommand("edit.end_gesture");
        await refreshProject();
        setSelectedClips([]);
        setSelectedClip(null);
        setNotice(notice);
      } catch (error) {
        await runCommand("edit.end_gesture").catch(() => {});
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setSelectedClip, setSelectedClips, setNotice],
  );

  return useCallback(
    async (id: EditClipboardCommand) => {
      if (id === "edit.select_all") {
        const all = (project?.sequence.tracks ?? []).flatMap((track) =>
          track.clips.map((_, clip_index) => ({
            track_id: track.id,
            clip_index,
          })),
        );
        setSelectedClips(all);
        setSelectedClip(all[0] ?? null);
        setNotice(`${all.length} clips selected`);
        return;
      }
      if (id === "edit.deselect_all") {
        setSelectedClips([]);
        setSelectedClip(null);
        setNotice("Selection cleared");
        return;
      }
      if (id === "edit.select_clip_at_playhead") {
        const found = (project?.sequence.tracks ?? []).flatMap((track) =>
          track.clips
            .map((clip, clip_index) => ({ track, clip, clip_index }))
            .filter(
              ({ clip }) =>
                playhead >= clip.timeline_start &&
                playhead < clip.timeline_start + clip.duration,
            )
            .map(({ track, clip_index }) => ({
              track_id: track.id,
              clip_index,
            })),
        );
        setSelectedClips(found);
        setSelectedClip(found[0] ?? null);
        setNotice(found.length ? "Selected clip at playhead" : "No clip at the playhead");
        return;
      }

      if (id === "edit.paste") {
        if (clipboard.current.length === 0) {
          setNotice("Nothing to paste");
          return;
        }
        try {
          await runCommand("edit.begin_gesture");
          for (const item of clipboard.current) {
            const track = project?.sequence.tracks.find(
              (entry) => entry.kind === item.kind && !entry.locked,
            );
            if (!track) throw new Error(`No unlocked ${item.kind} track`);
            await runCommand("timeline.add_clip", {
              media_id: item.mediaId,
              track_id: track.id,
              timeline_start: Math.max(0, playhead + item.offset),
              source_in: item.sourceIn,
              duration: item.duration,
            });
          }
          await runCommand("edit.end_gesture");
          await refreshProject();
          setNotice(`${clipboard.current.length} clips pasted at the playhead`);
        } catch (error) {
          await runCommand("edit.end_gesture").catch(() => {});
          setNotice(String(error));
        }
        return;
      }

      const items = resolved();
      if (items.length === 0) {
        setNotice("Select one or more clips first");
        return;
      }
      if (id === "edit.copy" || id === "edit.cut") {
        const base = Math.min(...items.map(({ clip }) => clip.timeline_start));
        clipboard.current = items.map(({ clip, kind }) => ({
          mediaId: clip.media_id,
          sourceIn: clip.source_in,
          duration: clip.duration,
          offset: clip.timeline_start - base,
          kind,
        }));
        if (id === "edit.copy") {
          setNotice(`${items.length} clips copied`);
          return;
        }
      }
      await removeAll(
        items,
        id === "edit.cut" ? "Cut to clipboard" : "Selection deleted",
      );
    },
    [
      project,
      playhead,
      resolved,
      removeAll,
      setSelectedClip,
      setSelectedClips,
      setNotice,
      runCommand,
      refreshProject,
    ],
  );
}
