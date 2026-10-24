import { useCallback } from "react";
import type { ProjectDocument } from "./projectTypes";

export interface UseTrackGroupOperationsOptions {
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  project: ProjectDocument["project"] | null;
}

/**
 * Folder-group (track group) operations: create, collapse, rename, delete,
 * and assign/unassign tracks. Backed by `timeline.*_track_group` commands.
 */
export function useTrackGroupOperations({
  perform,
  project,
}: UseTrackGroupOperationsOptions) {
  const handleCreateTrackGroup = useCallback(async () => {
    const name = window.prompt("Enter new folder group name:", "Audio Stems");
    if (!name?.trim()) return;
    const colors = ["#be185d", "#10b981", "#f59e0b", "#ec4899", "#6b21a8", "#6b21a8"];
    const color = colors[Math.floor(Math.random() * colors.length)];
    await perform("timeline.create_track_group", { name: name.trim(), color }, `Folder group "${name}" created`);
  }, [perform]);

  const handleToggleGroupCollapse = useCallback(
    async (groupId: number) => {
      await perform("timeline.toggle_track_group_collapsed", { group_id: groupId });
    },
    [perform]
  );

  const handleEditTrackGroup = useCallback(
    async (groupId: number) => {
      const group = (project?.sequence.track_groups ?? []).find((g) => g.id === groupId);
      if (!group) return;
      const newName = window.prompt("Rename track folder:", group.name);
      if (!newName?.trim()) return;
      await perform("timeline.set_track_group_properties", { group_id: groupId, name: newName.trim(), color: group.color }, `Renamed folder to "${newName}"`);
    },
    [perform, project]
  );

  const handleDeleteTrackGroup = useCallback(
    async (groupId: number) => {
      const group = (project?.sequence.track_groups ?? []).find((g) => g.id === groupId);
      if (!group) return;
      if (!window.confirm(`Delete folder "${group.name}"? Tracks stay — they just leave the folder.`)) return;
      await perform("timeline.delete_track_group", { group_id: groupId }, `Deleted folder "${group.name}"`);
    },
    [perform, project]
  );

  const handleAssignTrackToGroup = useCallback(
    async (trackId: number, groupId: number | null) => {
      await perform(
        "timeline.assign_track_group",
        { track_id: trackId, group_id: groupId },
        groupId != null ? "Track moved into folder" : "Track removed from folder"
      );
    },
    [perform]
  );

  return {
    handleCreateTrackGroup,
    handleToggleGroupCollapse,
    handleEditTrackGroup,
    handleDeleteTrackGroup,
    handleAssignTrackToGroup,
  };
}
