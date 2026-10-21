import { useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import type {
  MediaAsset,
  ProjectDocument,
  SelectedClip,
  Transition,
} from "../../app/projectTypes";
import type { SequenceTab } from "./SequenceTabs";

type Perform = (
  id: string,
  params?: unknown,
  successNotice?: string,
) => Promise<unknown>;

export interface UseSequenceTabsOptions {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  perform: Perform;
  switchSequence: (sequenceId: number) => Promise<void>;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;
  setSelectedClips: Dispatch<SetStateAction<SelectedClip[]>>;
  setSelectedTransition: Dispatch<
    SetStateAction<{ track_id: number; transition: Transition } | null>
  >;
  busy: boolean;
}

/**
 * Sequence list and actions for the timeline tab strip. Compound sequences are
 * hidden unless active so users navigate their own sequences, not internals.
 */
export function useSequenceTabs({
  project,
  media,
  perform,
  switchSequence,
  setSelectedClip,
  setSelectedClips,
  setSelectedTransition,
  busy,
}: UseSequenceTabsOptions) {
  const activeId = project?.sequence.id ?? 1;
  const compoundSequenceIds = useMemo(
    () =>
      new Set(
        media.flatMap((asset) =>
          asset.nested_sequence?.id ? [asset.nested_sequence.id] : [],
        ),
      ),
    [media],
  );

  const sequences = useMemo<SequenceTab[]>(() => {
    if (!project) return [];
    return [project.sequence, ...(project.sequences ?? [])]
      .filter(
        (sequence, index, list) =>
          list.findIndex(
            (item) => (item.id ?? 1) === (sequence.id ?? 1),
          ) === index,
      )
      .filter(
        (sequence) =>
          (sequence.id ?? 1) === activeId ||
          !compoundSequenceIds.has(sequence.id ?? 1),
      )
      .sort((left, right) => (left.id ?? 1) - (right.id ?? 1))
      .map((sequence) => {
        const id = sequence.id ?? 1;
        return {
          id,
          name: sequence.name || `Sequence ${id}`,
          isCompound: compoundSequenceIds.has(id),
        };
      });
  }, [project, activeId, compoundSequenceIds]);

  const clearSelection = () => {
    setSelectedClip(null);
    setSelectedClips([]);
    setSelectedTransition(null);
  };

  const createSequence = async () => {
    const fallback = `Sequence ${(project?.sequences?.length ?? 0) + 2}`;
    const requested = window.prompt("New sequence name", fallback);
    if (requested === null) return;
    const name = requested.trim();
    const created = await perform(
      "sequence.create",
      name ? { name } : {},
      name ? `Created sequence “${name}”` : "Created sequence",
    );
    if (created) clearSelection();
  };

  const duplicateSequence = async () => {
    const duplicated = await perform(
      "sequence.duplicate",
      { sequence_id: activeId },
      "Duplicated sequence",
    );
    if (duplicated) clearSelection();
  };

  const renameSequence = async () => {
    const current =
      sequences.find((sequence) => sequence.id === activeId)?.name ??
      project?.sequence.name ??
      "Sequence";
    const requested = window.prompt("Rename sequence", current);
    if (requested === null || !requested.trim() || requested.trim() === current) {
      return;
    }
    await perform(
      "sequence.rename",
      { sequence_id: activeId, name: requested.trim() },
      `Renamed sequence to “${requested.trim()}”`,
    );
  };

  const deleteSequence = async () => {
    if (sequences.length <= 1) return;
    const current =
      sequences.find((sequence) => sequence.id === activeId)?.name ??
      "this sequence";
    if (
      !window.confirm(
        `Delete “${current}”? Clips in this sequence will be removed. This can be undone.`,
      )
    ) {
      return;
    }
    const deleted = await perform(
      "sequence.delete",
      { sequence_id: activeId },
      "Deleted sequence",
    );
    if (deleted) clearSelection();
  };

  return {
    sequences,
    activeId,
    busy,
    onSelect: (sequenceId: number) => void switchSequence(sequenceId),
    onCreate: () => void createSequence(),
    onDuplicate: () => void duplicateSequence(),
    onRename: () => void renameSequence(),
    onDelete: () => void deleteSequence(),
  };
}
