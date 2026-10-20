import { createContext, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ProjectStatus } from "./projectTypes";

export type CommandStatus = { enabled: boolean; reason: string | null };
export type MediaAvailability = { mediaId: number; missing: boolean };
type ProjectContextValue = {
  projectState: ProjectStatus | null;
  setProjectState: (value: ProjectStatus | null) => void;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  setUndoStatus: (value: CommandStatus) => void;
  setRedoStatus: (value: CommandStatus) => void;
  missingMediaIds: Set<number>;
  setMissingMediaIds: (value: Set<number> | ((current: Set<number>) => Set<number>)) => void;
  connection: string;
  setConnection: (value: string) => void;
  refreshProject: () => Promise<ProjectStatus>;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
};

const ProjectContext = createContext<ProjectContextValue | null>(null);

export async function runCommand<T = unknown>(id: string, params: unknown = null): Promise<T> {
  return invoke<T>("execute_command", { request: { id, params } });
}

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectState, setProjectState] = useState<ProjectStatus | null>(null);
  const [undoStatus, setUndoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to undo" });
  const [redoStatus, setRedoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to redo" });
  const [missingMediaIds, setMissingMediaIds] = useState<Set<number>>(new Set());
  const [connection, setConnection] = useState("Connecting to Rust core…");

  const refreshProject = useCallback(async () => {
    const [status, undo, redo, availability] = await Promise.all([
      runCommand<ProjectStatus>("project.status"),
      invoke<CommandStatus>("command_status", { id: "edit.undo" }),
      invoke<CommandStatus>("command_status", { id: "edit.redo" }),
      invoke<MediaAvailability[]>("check_media_availability"),
    ]);
    setProjectState(status);
    setUndoStatus(undo);
    setRedoStatus(redo);
    setMissingMediaIds(new Set(availability.filter((item) => item.missing).map((item) => item.mediaId)));
    setConnection("Rust core connected");
    return status;
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;
    void listen("mcp_project_changed", () => {
      void refreshProject().catch((error: unknown) => {
        console.error("Could not refresh after an MCP edit:", error);
      });
    }).then((dispose) => {
      if (active) unlisten = dispose;
      else dispose();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [refreshProject]);

  const value = useMemo(() => ({
    projectState, setProjectState, undoStatus, redoStatus, setUndoStatus, setRedoStatus, missingMediaIds,
    setMissingMediaIds, connection, setConnection, refreshProject, runCommand,
  }), [projectState, undoStatus, redoStatus, missingMediaIds, connection, refreshProject]);
  return <ProjectContext.Provider value={value}>{children}</ProjectContext.Provider>;
}

export { ProjectContext };
