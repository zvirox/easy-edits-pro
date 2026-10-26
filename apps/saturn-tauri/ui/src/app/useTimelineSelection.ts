import { useCallback } from "react";
import { TICKS_PER_SECOND } from "../utils/formatters.ts";
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
  setNotice: (message: string) => void;
};

type TimelineEntry = {
  trackId: number;
  clipIndex: number;
  start: number;
  end: number;
};

export type TimelineSelectionCommand =
  | "timeline.select"
  | "timeline.selectGap"
  | "timeline.selectNextClip"
  | "timeline.selectPrevClip";

export const TIMELINE_SELECTION_COMMANDS: TimelineSelectionCommand[] = [
  "timeline.select",
  "timeline.selectGap",
  "timeline.selectNextClip",
  "timeline.selectPrevClip",
];

const fmt = (ticks: number): string =>
  `${(ticks / TICKS_PER_SECOND).toFixed(2)}s`;

/** A bounded gap between clips on one track, in ticks. */
export type TimelineGap = {
  trackId: number;
  trackName: string;
  start: number;
  end: number;
};

type GapTrack = {
  id: number;
  name: string;
  locked?: boolean;
  clips: Array<{ timeline_start: number; duration: number }>;
};

/**
 * The first bounded gap containing the playhead, scanning tracks in document
 * order and skipping locked ones. Returns null when the playhead sits on a
 * clip or past the end of every track.
 */
export function findGapAtPlayhead(
  tracks: GapTrack[],
  playhead: number,
): TimelineGap | null {
  for (const track of tracks) {
    if (track.locked) continue;
    const clips = [...track.clips].sort(
      (left, right) => left.timeline_start - right.timeline_start,
    );
    let previousEnd = 0;
    for (const clip of clips) {
      if (playhead < clip.timeline_start) {
        if (playhead >= previousEnd) {
          return {
            trackId: track.id,
            trackName: track.name,
            start: previousEnd,
            end: clip.timeline_start,
          };
        }
        break;
      }
      previousEnd = Math.max(
        previousEnd,
        clip.timeline_start + clip.duration,
      );
    }
  }
  return null;
}

/**
 * Timeline-order stepping for Select Next/Previous Clip. With no current
 * index, Next enters at the first clip and Previous at the last; otherwise
 * the step clamps at both ends.
 */
export function stepSelectionIndex(
  currentIndex: number,
  count: number,
  forward: boolean,
): number {
  if (count === 0) return -1;
  if (currentIndex < 0) return forward ? 0 : count - 1;
  return forward
    ? Math.min(currentIndex + 1, count - 1)
    : Math.max(currentIndex - 1, 0);
}

/**
 * Timeline-menu selection helpers: select clips by name or label, mark the
 * gap at the playhead, and step through clips in timeline order. Every
 * command is backend-free (ui_only): it drives existing selection state and,
 * for the gap, the existing mark commands.
 */
export function useTimelineSelection({
  project,
  media,
  playhead,
  selectedClip,
  selectedClips,
  setSelectedClip,
  setSelectedClips,
  runCommand,
  setNotice,
}: Options) {
  return useCallback(
    async (id: TimelineSelectionCommand) => {
      const tracks = project?.sequence.tracks ?? [];

      // Every clip on every track, in timeline order (start time, then
      // document track order).
      const entries: TimelineEntry[] = tracks.flatMap((track) =>
        track.clips.map((clip, clipIndex) => ({
          trackId: track.id,
          clipIndex,
          start: clip.timeline_start,
          end: clip.timeline_start + clip.duration,
        })),
      );
      entries.sort((left, right) => left.start - right.start);

      const entryRef = (entry: TimelineEntry): SelectedClip => ({
        track_id: entry.trackId,
        clip_index: entry.clipIndex,
      });

      if (id === "timeline.select") {
        const query = window.prompt("Select clips by media name or label:");
        if (!query) return;
        const needle = query.toLowerCase();
        const matches = entries.filter((entry) => {
          const track = tracks.find((item) => item.id === entry.trackId);
          const clip = track?.clips[entry.clipIndex];
          if (!clip) return false;
          const asset = media.find((item) => item.id === clip.media_id);
          return `${asset?.name ?? ""} ${clip.label ?? ""}`
            .toLowerCase()
            .includes(needle);
        });
        if (matches.length === 0) {
          setNotice(`No clips match "${query}"`);
          return;
        }
        setSelectedClips(matches.map(entryRef));
        setSelectedClip(entryRef(matches[0]));
        setNotice(`${matches.length} clips match "${query}"`);
        return;
      }

      if (id === "timeline.selectGap") {
        // The first bounded gap that contains the playhead, in track order.
        const gap = findGapAtPlayhead(tracks, playhead);
        if (gap) {
          await runCommand("timeline.set_mark_in", { at: gap.start });
          await runCommand("timeline.set_mark_out", { at: gap.end });
          setNotice(
            `Gap on ${gap.trackName}: ${fmt(gap.start)}–${fmt(gap.end)} (marked in/out)`,
          );
          return;
        }
        setNotice("No gap at the playhead");
        return;
      }

      // Next/previous clip in timeline order, stepping from the current
      // selection. With no selection, Next picks the first clip and
      // Previous picks the last.
      if (entries.length === 0) {
        setNotice("Timeline has no clips");
        return;
      }
      const anchor = selectedClips[0] ?? selectedClip;
      const currentIndex = anchor
        ? entries.findIndex(
            (entry) =>
              entry.trackId === anchor.track_id &&
              entry.clipIndex === anchor.clip_index,
          )
        : -1;
      const targetIndex = stepSelectionIndex(
        currentIndex,
        entries.length,
        id === "timeline.selectNextClip",
      );
      const target = entries[targetIndex];
      setSelectedClips([entryRef(target)]);
      setSelectedClip(entryRef(target));
      setNotice(
        `Clip ${targetIndex + 1} of ${entries.length} (${fmt(target.start)})`,
      );
    },
    [
      project,
      media,
      playhead,
      selectedClip,
      selectedClips,
      setSelectedClip,
      setSelectedClips,
      runCommand,
      setNotice,
    ],
  );
}
