/** FC Sequence menu selection modes, editor-held (the engine never sees
 * selection): Linked Selection (clicking a clip selects its whole link
 * group) and Selection Follows Playhead. Pure module: node-tested, the
 * React bindings live beside the consumers. */

export type SelectionModeProject = {
  sequence: {
    tracks: Array<{
      id: number;
      kind: string;
      clips: Array<{ group_id?: number | null }>;
    }>;
  };
} | null;

export type SelectionModeClip = { track_id: number; clip_index: number };

let linkedSelection = false;
let selectionFollowsPlayhead = false;
const modeListeners = new Set<() => void>();

function emitModeChange(): void {
  for (const listener of modeListeners) listener();
}

export function subscribeToSelectionModes(listener: () => void): () => void {
  modeListeners.add(listener);
  return () => modeListeners.delete(listener);
}

export function isLinkedSelection(): boolean {
  return linkedSelection;
}

export function setLinkedSelection(on: boolean): boolean {
  if (linkedSelection !== on) {
    linkedSelection = on;
    emitModeChange();
  }
  return linkedSelection;
}

export function isSelectionFollowsPlayhead(): boolean {
  return selectionFollowsPlayhead;
}

export function setSelectionFollowsPlayhead(on: boolean): boolean {
  if (selectionFollowsPlayhead !== on) {
    selectionFollowsPlayhead = on;
    emitModeChange();
  }
  return selectionFollowsPlayhead;
}

/** Every catalogue id this module owns (captions/media flows live elsewhere). */
const SELECTION_MODE_COMMANDS = new Set([
  "sequence.linked_selection",
  "sequence.selection_follows_playhead",
]);

export function isSelectionModeCommand(id: string): boolean {
  return SELECTION_MODE_COMMANDS.has(id);
}

/** Dispatch one selection-mode command; returns its status notice. */
export function toggleSelectionModeCommand(id: string): string {
  switch (id) {
    case "sequence.linked_selection":
      if (linkedSelection) {
        setLinkedSelection(false);
        return "Linked selection off";
      }
      setLinkedSelection(true);
      return "Linked selection on: selecting a clip selects its link group";
    case "sequence.selection_follows_playhead":
      if (selectionFollowsPlayhead) {
        setSelectionFollowsPlayhead(false);
        return "Selection no longer follows the playhead";
      }
      setSelectionFollowsPlayhead(true);
      return "Selection follows the playhead";
    default:
      return "";
  }
}

/**
 * Linked Selection: all clips sharing the target clip's link group (plus the
 * clip itself as the primary). Returns null when the clip is unlinked or
 * groupless, meaning plain single selection.
 */
export function expandLinkedClip(
  project: SelectionModeProject,
  clip: SelectionModeClip,
): SelectionModeClip[] | null {
  if (!project) return null;
  const track = project.sequence.tracks.find((item) => item.id === clip.track_id);
  const group = track?.clips[clip.clip_index]?.group_id;
  if (group == null) return null;
  const linked: SelectionModeClip[] = [];
  for (const memberTrack of project.sequence.tracks) {
    for (let index = 0; index < memberTrack.clips.length; index += 1) {
      if (memberTrack.clips[index].group_id === group) {
        linked.push({ track_id: memberTrack.id, clip_index: index });
      }
    }
  }
  return linked.length > 1 ? linked : null;
}
