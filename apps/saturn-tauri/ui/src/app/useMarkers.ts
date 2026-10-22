import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Marker, MarkerKind, ProjectDocument } from "./projectTypes";
import { PANEL_OPEN_EVENT } from "../features/panels/presets";
import { defaultMarkerRange } from "../features/markers/markerList";

export const MARKERS_PANEL_ID = "markers";

interface MarkerNavResult {
  target: number | null;
  id?: number;
  name?: string;
}

export interface UseMarkersOptions {
  project: ProjectDocument["project"] | null;
  playhead: number;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
  seek: (tick: number) => void;
}

export interface MarkerOperations {
  markers: Marker[];
  selectedMarkerId: number | null;
  setSelectedMarkerId: (id: number | null) => void;
  selectMarker: (marker: Marker) => void;
  handleMenuCommand: (id: string) => Promise<boolean>;
  addMarker: () => Promise<void>;
  addRangeMarker: () => Promise<void>;
  renameMarker: (id: number, name: string) => Promise<void>;
  setMarkerKind: (id: number, kind: MarkerKind) => Promise<void>;
  removeMarker: (id: number) => Promise<void>;
  clearMarkers: () => Promise<void>;
  splitAtPlayhead: () => Promise<void>;
  goToNextMarker: () => Promise<void>;
  goToPreviousMarker: () => Promise<void>;
  exportMarkers: () => Promise<void>;
  importMarkers: () => Promise<void>;
  openPanel: () => void;
}

/**
 * Marker commands and selection state. Menus, the timeline lane, and the
 * Markers panel all funnel through here so selection and undo behave the same.
 */
export function useMarkers({
  project,
  playhead,
  runCommand,
  refreshProject,
  setNotice,
  seek,
}: UseMarkersOptions): MarkerOperations {
  const [selectedMarkerId, setSelectedMarkerId] = useState<number | null>(null);
  const markers = useMemo(() => project?.sequence.markers ?? [], [project]);

  const fail = useCallback(
    (error: unknown) => setNotice(`Marker command failed: ${String(error)}`),
    [setNotice],
  );

  const addMarker = useCallback(async () => {
    try {
      const result = await runCommand<{ id: number }>("marker.add", { at: playhead });
      await refreshProject();
      setSelectedMarkerId(result.id);
      setNotice("Marker added");
    } catch (error) {
      fail(error);
    }
  }, [playhead, refreshProject, runCommand, setNotice, fail]);

  const addRangeMarker = useCallback(async () => {
    const range = defaultMarkerRange(
      playhead,
      project?.sequence.mark_in,
      project?.sequence.mark_out,
    );
    try {
      const result = await runCommand<{ id: number }>("marker.add_range", range);
      await refreshProject();
      setSelectedMarkerId(result.id);
      setNotice("Range marker added");
    } catch (error) {
      fail(error);
    }
  }, [playhead, project, refreshProject, runCommand, setNotice, fail]);

  const renameMarker = useCallback(
    async (id: number, name: string) => {
      try {
        await runCommand("marker.update", { id, name });
        await refreshProject();
      } catch (error) {
        fail(error);
      }
    },
    [refreshProject, runCommand, fail],
  );

  const setMarkerKind = useCallback(
    async (id: number, kind: MarkerKind) => {
      try {
        await runCommand("marker.update", { id, kind });
        await refreshProject();
      } catch (error) {
        fail(error);
      }
    },
    [refreshProject, runCommand, fail],
  );

  const removeMarker = useCallback(
    async (id: number) => {
      try {
        await runCommand("marker.remove", { id });
        await refreshProject();
        setSelectedMarkerId((current) => (current === id ? null : current));
        setNotice("Marker deleted");
      } catch (error) {
        fail(error);
      }
    },
    [refreshProject, runCommand, setNotice, fail],
  );

  const clearMarkers = useCallback(async () => {
    if (markers.length === 0) {
      setNotice("There are no markers to clear");
      return;
    }
    if (!window.confirm(`Delete all ${markers.length} markers?`)) return;
    try {
      const result = await runCommand<{ cleared: number }>("marker.clear");
      await refreshProject();
      setSelectedMarkerId(null);
      setNotice(`Cleared ${result.cleared} markers`);
    } catch (error) {
      fail(error);
    }
  }, [markers.length, refreshProject, runCommand, setNotice, fail]);

  const splitAtPlayhead = useCallback(async () => {
    try {
      const result = await runCommand<{ split: number }>("marker.split", { at: playhead });
      await refreshProject();
      setNotice(
        result.split > 0 ? "Range marker split at the playhead" : "No range marker at the playhead",
      );
    } catch (error) {
      fail(error);
    }
  }, [playhead, refreshProject, runCommand, setNotice, fail]);

  const navigate = useCallback(
    async (id: "marker.next" | "marker.previous") => {
      try {
        const result = await runCommand<MarkerNavResult>(id, { from: playhead });
        if (result.target === null || result.target === undefined) {
          setNotice(id === "marker.next" ? "No markers after the playhead" : "No markers before the playhead");
          return;
        }
        seek(result.target);
        if (typeof result.id === "number") setSelectedMarkerId(result.id);
      } catch (error) {
        fail(error);
      }
    },
    [playhead, runCommand, seek, setNotice, fail],
  );

  const goToNextMarker = useCallback(() => navigate("marker.next"), [navigate]);
  const goToPreviousMarker = useCallback(() => navigate("marker.previous"), [navigate]);

  const exportMarkers = useCallback(async () => {
    if (markers.length === 0) {
      setNotice("There are no markers to export");
      return;
    }
    try {
      const result = await runCommand<{ csv: string; count: number }>("marker.export_csv");
      const path = await save({
        defaultPath: `${project?.name ?? "Markers"}-markers.csv`,
        filters: [{ name: "Marker CSV", extensions: ["csv"] }],
      });
      if (!path) return;
      await invoke("write_markers_file", { path, csv: result.csv });
      setNotice(`Exported ${result.count} markers`);
    } catch (error) {
      fail(error);
    }
  }, [markers.length, project, runCommand, setNotice, fail]);

  const importMarkers = useCallback(async () => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "Marker CSV", extensions: ["csv"] }],
      });
      if (!path || typeof path !== "string") return;
      const csv = await invoke<string>("read_markers_file", { path });
      const result = await runCommand<{ imported: number }>("marker.import_csv", {
        csv,
        replace: false,
      });
      await refreshProject();
      setNotice(`Imported ${result.imported} markers`);
    } catch (error) {
      fail(error);
    }
  }, [refreshProject, runCommand, setNotice, fail]);

  const openPanel = useCallback(() => {
    const announce = () =>
      window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: MARKERS_PANEL_ID } }));
    announce();
    // The panel is hosted by the Edit workspace. When the caller switches to it
    // in the same tick, the host subscribes a frame later, so re-announce once.
    window.setTimeout(announce, 120);
  }, []);

  const selectMarker = useCallback(
    (marker: Marker) => {
      setSelectedMarkerId(marker.id);
      seek(marker.start_ticks);
    },
    [seek],
  );

  const handleMenuCommand = useCallback(
    async (id: string): Promise<boolean> => {
      switch (id) {
        case "marker.add":
          await addMarker();
          return true;
        case "marker.add_range":
          await addRangeMarker();
          return true;
        case "marker.next":
          await goToNextMarker();
          return true;
        case "marker.previous":
          await goToPreviousMarker();
          return true;
        case "marker.split":
          await splitAtPlayhead();
          return true;
        case "marker.export_csv":
          await exportMarkers();
          return true;
        case "marker.import_csv":
          await importMarkers();
          return true;
        case "marker.clear":
          await clearMarkers();
          return true;
        case "marker.remove":
          if (selectedMarkerId === null) {
            setNotice("Select a marker in the timeline or the Markers panel first");
            return true;
          }
          await removeMarker(selectedMarkerId);
          return true;
        case "marker.panel":
          openPanel();
          return true;
        case "marker.update":
          setNotice("Rename markers in the Markers panel");
          return true;
        default:
          return false;
      }
    },
    [
      addMarker,
      addRangeMarker,
      goToNextMarker,
      goToPreviousMarker,
      splitAtPlayhead,
      exportMarkers,
      importMarkers,
      clearMarkers,
      selectedMarkerId,
      removeMarker,
      openPanel,
      setNotice,
    ],
  );

  return {
    markers,
    selectedMarkerId,
    setSelectedMarkerId,
    selectMarker,
    handleMenuCommand,
    addMarker,
    addRangeMarker,
    renameMarker,
    setMarkerKind,
    removeMarker,
    clearMarkers,
    splitAtPlayhead,
    goToNextMarker,
    goToPreviousMarker,
    exportMarkers,
    importMarkers,
    openPanel,
  };
}
