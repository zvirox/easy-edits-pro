import { invoke } from "@tauri-apps/api/core";
import { runCommand } from "./ProjectProvider";

export interface EnqueueExportDeps {
  project: { settings: { width: number; height: number } } | null;
  rendering: boolean;
  exportOutputPath: string;
  exportMarkedRange: boolean;
  setExportOutputPath: (path: string) => void;
  chooseExportDestination: () => Promise<unknown>;
  setNotice: (message: string) => void;
}

export interface ResolvedExportSettings {
  width: number;
  height: number;
  frameRateNumerator: number;
  frameRateDenominator: number;
  useHardware: boolean;
  useMarkRange: boolean;
}

/**
 * Queue the current timeline for a detached background export. The preset
 * name resolves through the engine (export.resolve) into concrete render
 * settings, then `queue_export` hands it to the shell's JobManager.
 * Returns the job id, or null when the user cancelled the file picker.
 * Exported as a factory so useProjectOperations stays under the 800-line
 * source cap.
 */
export function createEnqueueExport({
  project,
  rendering,
  exportOutputPath,
  exportMarkedRange,
  setExportOutputPath,
  chooseExportDestination,
  setNotice,
}: EnqueueExportDeps) {
  return async function enqueueExport(presetName: string): Promise<number | null> {
    if (!project || rendering) return null;
    const path = exportOutputPath || ((await chooseExportDestination()) as string | null);
    if (!path) return null;
    setExportOutputPath(path);
    let resolved: ResolvedExportSettings;
    try {
      resolved = await runCommand<ResolvedExportSettings>("export.resolve", {
        preset: presetName,
        use_mark_range: exportMarkedRange,
      });
    } catch (error) {
      setNotice(String(error));
      return null;
    }
    try {
      const jobId = await invoke<number>("queue_export", {
        outputPath: path,
        useMarkRange: resolved.useMarkRange,
        presetName,
        settings: {
          width: resolved.width,
          height: resolved.height,
          frameRateNumerator: resolved.frameRateNumerator,
          frameRateDenominator: resolved.frameRateDenominator,
          useHardwareEncoding: resolved.useHardware,
        },
      });
      setNotice(`Queued background export #${jobId}: ${path}`);
      return jobId;
    } catch (error) {
      setNotice(`Queue export failed: ${String(error)}`);
      return null;
    }
  };
}
