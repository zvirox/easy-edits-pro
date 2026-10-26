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
  /** Seek the program playhead to a tick position (used by Find). */
  seek: (time: number) => void;
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
  | "edit.select_clip_at_playhead"
  | "edit.paste_insert"
  | "edit.paste_media"
  | "edit.find"
  | "edit.find_next"
  | "edit.select_all_matching"
  | "edit.select_label_group"
  | "edit.edit_original";

export const EDIT_CLIPBOARD_COMMANDS: EditClipboardCommand[] = [
  "edit.cut",
  "edit.copy",
  "edit.paste",
  "edit.delete",
  "edit.select_all",
  "edit.deselect_all",
  "edit.select_clip_at_playhead",
  "edit.paste_insert",
  "edit.paste_media",
  "edit.find",
  "edit.find_next",
  "edit.select_all_matching",
  "edit.select_label_group",
  "edit.edit_original",
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
  seek,
}: Options) {
  const clipboard = useRef<ClipboardItem[]>([]);
  const findSession = useRef<{
    query: string;
    matches: (SelectedClip & { start: number })[];
    index: number;
  } | null>(null);

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

      const tracks = project?.sequence.tracks ?? [];
      const anchorRef = selection()[0];
      const anchorClip = anchorRef
        ? tracks.find((track) => track.id === anchorRef.track_id)?.clips[anchorRef.clip_index]
        : null;
      const anchored = (predicate: (clip: { media_id: number; label?: string }) => boolean) =>
        tracks.flatMap((track) =>
          track.clips
            .map((clip, clip_index) => ({ clip, clip_index, track_id: track.id }))
            .filter(({ clip }) => predicate(clip))
            .map(({ clip_index, track_id }) => ({ track_id, clip_index })),
        );

      if (id === "edit.select_all_matching") {
        if (!anchorClip) {
          setNotice("Select an anchor clip first");
          return;
        }
        const criterion = (clip: { media_id: number; label?: string }) =>
          anchorClip.label
            ? clip.label === anchorClip.label
            : clip.media_id === anchorClip.media_id;
        const matches = anchored(criterion);
        setSelectedClips(matches);
        setSelectedClip(matches[0] ?? null);
        setNotice(matches.length
          ? `${matches.length} clip${matches.length === 1 ? "" : "s"} match the anchor`
          : "No clips match the anchor");
        return;
      }
      if (id === "edit.select_label_group") {
        if (!anchorClip) {
          setNotice("Select an anchor clip first");
          return;
        }
        if (!anchorClip.label) {
          setNotice("The anchor clip has no label colour");
          return;
        }
        const matches = anchored((clip) => clip.label === anchorClip.label);
        setSelectedClips(matches);
        setSelectedClip(matches[0] ?? null);
        setNotice(`${matches.length} clip${matches.length === 1 ? "" : "s"} carry the "${anchorClip.label}" label`);
        return;
      }
      if (id === "edit.paste_insert") {
        if (clipboard.current.length === 0) {
          setNotice("Nothing to paste");
          return;
        }
        try {
          await runCommand("edit.begin_gesture");
          for (const item of clipboard.current) {
            const track = tracks.find(
              (entry) => entry.kind === item.kind && !entry.locked,
            );
            if (!track) throw new Error(`No unlocked ${item.kind} track`);
            await runCommand("timeline.insert_clip", {
              media_id: item.mediaId,
              track_id: track.id,
              at: playhead + item.offset,
              source_in: item.sourceIn,
              duration: item.duration,
            });
          }
          await runCommand("edit.end_gesture");
          await refreshProject();
          setNotice(`${clipboard.current.length} clip(s) paste-inserted at the playhead`);
        } catch (error) {
          await runCommand("edit.end_gesture").catch(() => {});
          setNotice(String(error));
        }
        return;
      }
      if (id === "edit.paste_media") {
        const ids = [...new Set(clipboard.current.map((item) => item.mediaId))];
        if (ids.length === 0) {
          setNotice("Nothing to paste");
          return;
        }
        try {
          for (const mediaId of ids) {
            const asset = media.find((item) => item.id === mediaId);
            if (!asset) throw new Error("Copied media is no longer in the project");
            await runCommand("project.add_media", {
              name: `${asset.name} copy`,
              path: asset.path,
              kind: asset.kind,
            });
          }
          await refreshProject();
          setNotice(`${ids.length} media item(s) pasted into the project`);
        } catch (error) {
          setNotice(String(error));
        }
        return;
      }
      if (id === "edit.edit_original") {
        const asset = anchorClip
          ? media.find((item) => item.id === anchorClip.media_id)
          : null;
        if (!asset) {
          setNotice("Select a clip whose media you want to edit");
          return;
        }
        try {
          await navigator.clipboard.writeText(asset.path);
          setNotice(`Path copied — open it in your external editor: ${asset.path}`);
        } catch {
          setNotice(`Open in an external editor: ${asset.path}`);
        }
        return;
      }
      if (id === "edit.find" || id === "edit.find_next") {
        if (id === "edit.find") {
          const query = window.prompt("Find clips by media name or label:");
          if (!query) return;
          const needle = query.toLowerCase();
          const matches = tracks.flatMap((track) =>
            track.clips
              .map((clip, clip_index) => {
                const asset = media.find((item) => item.id === clip.media_id);
                const haystack = `${asset?.name ?? ""} ${clip.label ?? ""}`.toLowerCase();
                return haystack.includes(needle)
                  ? { track_id: track.id, clip_index, start: clip.timeline_start }
                  : null;
              })
              .filter((entry): entry is SelectedClip & { start: number } => entry !== null),
          );
          if (matches.length === 0) {
            findSession.current = null;
            setNotice(`No clips match "${query}"`);
            return;
          }
          findSession.current = { query, matches, index: 0 };
          setSelectedClips(matches.slice(0, 1));
          setSelectedClip(matches[0]);
          seek(matches[0].start);
          setNotice(`Match 1 of ${matches.length} for "${query}" — Find Next to cycle`);
          return;
        }
        const session = findSession.current;
        if (!session || session.matches.length === 0) {
          setNotice("Use Find to start a search first");
          return;
        }
        session.index = (session.index + 1) % session.matches.length;
        const target = session.matches[session.index];
        setSelectedClips([{ track_id: target.track_id, clip_index: target.clip_index }]);
        setSelectedClip(target);
        seek(target.start);
        setNotice(`Match ${session.index + 1} of ${session.matches.length} for "${session.query}"`);
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
      media,
      seek,
    ],
  );
}
