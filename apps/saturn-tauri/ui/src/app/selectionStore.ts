// F1: single reactive clip-selection store (plan v4 workstream F, §13).
// One source of truth for clip selection across Timeline, Inspector,
// Effects, Color, and Curves. Writers go through setPrimaryClip /
// setMultiClips (the same operations the old orthogonal useStates allowed),
// readers subscribe with useClipSelection.
import { useSyncExternalStore } from "react";
import type { SelectedClip } from "../types";

export interface ClipSelection {
  /** The primary selected clip, or null when nothing is selected. */
  clip: SelectedClip | null;
  /** Every selected clip (multi-select); `[]` when nothing is selected. */
  clips: SelectedClip[];
}

type SelectionProject = {
  sequence: { tracks: Array<{ id: number; kind: string; clips: unknown[] }> };
} | null;

let selection: ClipSelection = { clip: null, clips: [] };
const listeners = new Set<() => void>();

function sameClip(a: SelectedClip | null, b: SelectedClip | null): boolean {
  return a === b || (!!a && !!b && a.track_id === b.track_id && a.clip_index === b.clip_index);
}

function sameClips(a: SelectedClip[], b: SelectedClip[]): boolean {
  return a === b || (a.length === b.length && a.every((item, index) => sameClip(item, b[index])));
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getClipSelection(): ClipSelection {
  return selection;
}

/**
 * Set the primary selected clip. A clip outside the current multi-selection
 * collapses the selection to just that clip; null clears everything.
 * Matches the legacy `selectedClip` setState semantics exactly.
 */
export function setPrimaryClip(clip: SelectedClip | null): void {
  let clips = selection.clips;
  if (clip === null) clips = [];
  else if (!clips.some((existing) => sameClip(existing, clip))) clips = [clip];
  if (sameClip(selection.clip, clip) && sameClips(selection.clips, clips)) return;
  selection = { clip, clips };
  for (const listener of listeners) listener();
}

/**
 * Replace the multi-selection. The current primary clip stays primary when
 * it is still part of the set; otherwise the first entry becomes primary.
 * Matches the legacy `selectedClips` setState semantics exactly.
 */
export function setMultiClips(clips: SelectedClip[]): void {
  const primary =
    selection.clip && clips.some((existing) => sameClip(existing, selection.clip))
      ? selection.clip
      : clips[0] ?? null;
  if (sameClip(selection.clip, primary) && sameClips(selection.clips, clips)) return;
  selection = { clip: primary, clips };
  for (const listener of listeners) listener();
}

/** React hook: re-renders whenever the clip selection changes. */
export function useClipSelection(): ClipSelection {
  return useSyncExternalStore(subscribe, getClipSelection, getClipSelection);
}

/**
 * F1: the first clip of the first video track (V1), or null when the
 * sequence has no clips. Used to auto-select on sequence load so the
 * Inspector, Effects, Color, and Curves docks come up live.
 */
export function pickFirstTimelineClip(project: SelectionProject): SelectedClip | null {
  const firstVideo = project?.sequence.tracks.find((track) => track.kind === "video");
  if (!firstVideo || firstVideo.clips.length === 0) return null;
  return { track_id: firstVideo.id, clip_index: 0 };
}
