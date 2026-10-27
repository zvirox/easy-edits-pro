import { useEffect, useSyncExternalStore } from "react";
import { setMultiClips, setPrimaryClip, useClipSelection } from "./selectionStore";
import {
  expandLinkedClip,
  isLinkedSelection,
  isSelectionFollowsPlayhead,
  subscribeToSelectionModes,
} from "./selectionModes";
import { findClipAtPlayhead } from "./playheadNavigation";
import type { ProjectDocument, SelectedClip } from "./projectTypes";

type InnerProject = ProjectDocument["project"];

/** Reactive read of one selection-mode flag. */
export function useLinkedSelection(): boolean {
  return useSyncExternalStore(subscribeToSelectionModes, isLinkedSelection, isLinkedSelection);
}

function sameClip(a: SelectedClip | null, b: SelectedClip | null): boolean {
  return !!a && !!b && a.track_id === b.track_id && a.clip_index === b.clip_index;
}

/**
 * Selection Follows Playhead: while enabled, moving the playhead selects the
 * clip under it (expanded to the link group when Linked Selection is also
 * on). Skips work when the selection already matches — playhead updates flow
 * at frame rate.
 */
export function useSelectionFollowsPlayhead(
  project: InnerProject | null,
  playhead: number,
): void {
  const enabled = useSyncExternalStore(
    subscribeToSelectionModes,
    isSelectionFollowsPlayhead,
    isSelectionFollowsPlayhead,
  );
  const selected = useClipSelection().clip;

  useEffect(() => {
    if (!enabled) return;
    const target = findClipAtPlayhead(project ?? null, playhead);
    if (!target) return;
    if (sameClip(selected, target)) return;

    const linked = isLinkedSelection() ? expandLinkedClip(project ?? null, target) : null;
    if (linked && linked.length > 1) setMultiClips(linked);
    else setPrimaryClip(target);
  }, [enabled, project, playhead, selected]);
}
