import { useCallback } from "react";
import {
  PANEL_DOC_COMMANDS,
  buildProjectPanelParams,
  formatPanelResult,
  PROJECT_PANEL_COMMANDS,
  type PanelMenuContext,
  type ProjectPanelCommand,
} from "./panelMenu.ts";

type Runner = <T>(id: string, params?: unknown) => Promise<T>;

type Options = {
  selectedMediaId: number | null;
  setSelectedMediaId: (id: number | null) => void;
  setSelectedMediaIds: (ids: number[]) => void;
  project: { media: { id: number }[] } | null;
  notes: string;
  runCommand: Runner;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
};

/**
 * File > Project Panel menu commands. Engine-backed commands run through the
 * builder (native prompt idiom); read commands surface their result as a
 * pool notice. Media selection is pure UI state.
 */
export function useProjectPanel({
  selectedMediaId,
  setSelectedMediaId,
  setSelectedMediaIds,
  project,
  notes,
  runCommand,
  refreshProject,
  setNotice,
}: Options) {
  return useCallback(
    async (id: string): Promise<boolean> => {
      if (!(PROJECT_PANEL_COMMANDS as readonly string[]).includes(id)) return false;
      const command = id as ProjectPanelCommand;

      if (command === "project.selectAll") {
        const ids = (project?.media ?? []).map((item) => item.id);
        setSelectedMediaIds(ids);
        setSelectedMediaId(ids[0] ?? null);
        setNotice(ids.length ? `All ${ids.length} media item(s) selected` : "The project has no media");
        return true;
      }
      if (command === "project.deselectAll") {
        setSelectedMediaIds([]);
        setSelectedMediaId(null);
        setNotice("Panel selection cleared");
        return true;
      }

      const ctx: PanelMenuContext = { selectedMediaId, notes };
      const built = buildProjectPanelParams(command, ctx);
      if (built) {
        if (built.notice) setNotice(built.notice);
        if (built.cancel) return true;
        try {
          const result = await runCommand<Record<string, unknown>>(command, built.params);
          if (PANEL_DOC_COMMANDS.includes(command)) await refreshProject();
          const notice = formatPanelResult(command, result);
          if (notice) setNotice(notice);
        } catch (error) {
          setNotice(String(error));
        }
        return true;
      }

      // Read-only commands with no params: run and surface the result.
      try {
        const result = await runCommand<Record<string, unknown>>(command, {});
        const notice = formatPanelResult(command, result);
        setNotice(notice ?? `${id} ran`);
      } catch (error) {
        setNotice(String(error));
      }
      return true;
    },
    [selectedMediaId, notes, project, setSelectedMediaId, setSelectedMediaIds, runCommand, refreshProject, setNotice],
  );
}
