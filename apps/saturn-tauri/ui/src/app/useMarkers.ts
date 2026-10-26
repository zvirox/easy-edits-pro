import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Marker, MarkerKind, ProjectDocument } from "./projectTypes";
import { PANEL_OPEN_EVENT } from "../features/dock/panelEvents";
import { defaultMarkerRange, splitMarkerTarget, type SplitDomain, type SplitEdge } from "../features/markers/markerList";
import { TICKS_PER_SECOND } from "./projectTypes";

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
  rippleMarkers: (deltaTicks: number) => Promise<void>;
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

  // Shift every marker at or after the playhead by a signed tick amount. Uses
  // marker.ripple so the engine clamps at zero and reports what it moved.
  const rippleMarkers = useCallback(
    async (deltaTicks: number) => {
      if (!deltaTicks) {
        setNotice("Enter a non-zero shift to ripple markers");
        return;
      }
      try {
        const result = await runCommand<{ shifted: number; clamped: number }>(
          "marker.ripple",
          { from: playhead, delta: deltaTicks },
        );
        await refreshProject();
        setNotice(
          result.shifted > 0
            ? `Shifted ${result.shifted} marker${result.shifted === 1 ? "" : "s"}${
                result.clamped ? ` (${result.clamped} clamped to zero)` : ""
              }`
            : "No markers at or after the playhead to shift",
        );
      } catch (error) {
        fail(error);
      }
    },
    [playhead, runCommand, refreshProject, setNotice, fail],
  );

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

  /** The marker a split/split-navigation command acts on. */
  const requireSelectedMarker = useCallback((): Marker | null => {
    if (selectedMarkerId !== null) {
      const marker = markers.find((item) => item.id === selectedMarkerId);
      if (marker) return marker;
    }
    setNotice("Select a marker in the timeline or the Markers panel first");
    return null;
  }, [selectedMarkerId, markers, setNotice]);

  const addKindMarker = useCallback(
    async (kind: Extract<MarkerKind, "chapter" | "flash_cue">) => {
      try {
        const result = await runCommand<{ id: number }>(kind === "chapter" ? "marker.add_chapter" : "marker.add_flash_cue", {
          at: playhead,
        });
        await refreshProject();
        setSelectedMarkerId(result.id);
        setNotice(kind === "chapter" ? "Chapter marker added" : "Flash cue marker added");
      } catch (error) {
        fail(error);
      }
    },
    [playhead, refreshProject, runCommand, setNotice, fail],
  );

  const goToSplitMarker = useCallback(
    async (domain: SplitDomain, edge: SplitEdge) => {
      const marker = requireSelectedMarker();
      if (!marker) return;
      const target = splitMarkerTarget(marker, domain, edge);
      if (target === null) {
        setNotice("That marker has no out position");
        return;
      }
      setSelectedMarkerId(marker.id);
      seek(target);
    },
    [requireSelectedMarker, seek, setNotice],
  );

  const markSplitMarker = useCallback(
    async (domain: SplitDomain, edge: SplitEdge) => {
      const marker = requireSelectedMarker();
      if (!marker) return;
      const id = domain === "audio" ? "marker.set_split_audio" : "marker.set_split_video";
      const outId = domain === "audio" ? "marker.set_split_audio_out" : "marker.set_split_video_out";
      try {
        await runCommand(edge === "in" ? id : outId, {
          id: marker.id,
          at: Math.round(playhead * TICKS_PER_SECOND),
        });
        await refreshProject();
        setNotice(`Marker ${domain} ${edge} marked at the playhead`);
      } catch (error) {
        fail(error);
      }
    },
    [playhead, refreshProject, runCommand, requireSelectedMarker, setNotice, fail],
  );

  const handleMenuCommand = useCallback(
    async (id: string): Promise<boolean> => {
      switch (id) {
        case "marker.add":
          await addMarker();
          return true;
        case "marker.add_chapter":
          await addKindMarker("chapter");
          return true;
        case "marker.add_flash_cue":
          await addKindMarker("flash_cue");
          return true;
        case "marker.add_range":
          await addRangeMarker();
          return true;
        case "marker.add_range_in_out": {
          if (project?.sequence.mark_in == null || project?.sequence.mark_out == null) {
            setNotice("Set both In and Out points before adding a range marker");
            return true;
          }
          try {
            const result = await runCommand<{ id: number }>("marker.add_range_in_out", {});
            await refreshProject();
            setSelectedMarkerId(result.id);
            setNotice("Range marker added for In and Out");
          } catch (error) {
            fail(error);
          }
          return true;
        }
        case "marker.go_to_split_audio_in":
          await goToSplitMarker("audio", "in");
          return true;
        case "marker.go_to_split_audio_out":
          await goToSplitMarker("audio", "out");
          return true;
        case "marker.go_to_split_video_in":
          await goToSplitMarker("video", "in");
          return true;
        case "marker.go_to_split_video_out":
          await goToSplitMarker("video", "out");
          return true;
        case "marker.set_split_audio":
          await markSplitMarker("audio", "in");
          return true;
        case "marker.set_split_video":
          await markSplitMarker("video", "in");
          return true;
        case "marker.set_split_audio_out":
          await markSplitMarker("audio", "out");
          return true;
        case "marker.set_split_video_out":
          await markSplitMarker("video", "out");
          return true;
        case "marker.set_color_filter": {
          const value = window.prompt("Filter markers by colors (comma-separated, empty shows all)", "red, blue");
          if (value === null) return true;
          const colors = value.split(",").map((c) => c.trim()).filter(Boolean);
          try {
            if (colors.length === 0) {
              await runCommand("marker.show_all_colors", {});
              setNotice("Showing all marker colors");
            } else {
              await runCommand("marker.set_color_filter", { colors });
              setNotice(`Filtering markers: ${colors.join(", ")}`);
            }
          } catch (error) {
            fail(error);
          }
          return true;
        }
        case "marker.show_all_colors":
          try {
            await runCommand("marker.show_all_colors", {});
            setNotice("Showing all marker colors");
          } catch (error) {
            fail(error);
          }
          return true;
        case "marker.set_copy_paste_includes": {
          const value = window.prompt("Should copy/paste include sequence markers? (on/off)", "on");
          if (value === null) return true;
          const enabled = value.trim().toLowerCase() !== "off";
          try {
            await runCommand("marker.set_copy_paste_includes", { enabled });
            setNotice(enabled ? "Copy/paste includes sequence markers" : "Copy/paste skips sequence markers");
          } catch (error) {
            fail(error);
          }
          return true;
        }
        case "marker.next":
          await goToNextMarker();
          return true;
        case "marker.previous":
          await goToPreviousMarker();
          return true;
        case "marker.split":
          await splitAtPlayhead();
          return true;
        case "marker.ripple": {
          const value = window.prompt(
            "Shift markers at or after the playhead by (seconds, negative to move earlier)",
            "1",
          );
          if (value === null) return true;
          const seconds = Number(value.trim());
          if (!Number.isFinite(seconds) || seconds === 0) {
            setNotice("Enter a non-zero shift, for example 1 or -0.5");
            return true;
          }
          await rippleMarkers(Math.round(seconds * TICKS_PER_SECOND));
          return true;
        }
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
      addKindMarker,
      addRangeMarker,
      goToSplitMarker,
      markSplitMarker,
      goToNextMarker,
      goToPreviousMarker,
      splitAtPlayhead,
      rippleMarkers,
      exportMarkers,
      importMarkers,
      clearMarkers,
      selectedMarkerId,
      removeMarker,
      openPanel,
      setNotice,
      runCommand,
      fail,
      project,
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
    rippleMarkers,
    exportMarkers,
    importMarkers,
    openPanel,
  };
}
