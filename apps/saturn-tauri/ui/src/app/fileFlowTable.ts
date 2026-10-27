/** Pure dispatch table for the FC file-family flows (no Tauri imports so it
 * stays node-testable). {@link useFileProjectFlows} consumes this. */

export interface InteropFormat {
  extension: string;
  label: string;
}

/** File > Export > interchange formats backed by the interop engine arms. */
export const INTEROP_EXPORTS: Record<string, InteropFormat> = {
  "interop.export_edl": { extension: "edl", label: "CMX 3600 EDL" },
  "interop.export_fcpxml": { extension: "fcpxml", label: "FCPXML" },
  "interop.export_xmeml_xml": { extension: "xml", label: "Final Cut Pro 7 XML" },
  "interop.export_otio": { extension: "otio", label: "OpenTimelineIO" },
};

/** File > New synthetic media kinds backed by generate_synthetic_media. */
export const SYNTHETIC_KINDS = [
  "media.generate_black_video",
  "media.generate_color_matte",
  "media.generate_transparent_video",
  "media.generate_bars_and_tone",
] as const;

export function syntheticKindOf(id: string): string | null {
  if (!id.startsWith("media.generate_")) return null;
  const kind = id.slice("media.generate_".length);
  return (SYNTHETIC_KINDS as readonly string[]).includes(id) ? kind : null;
}

/** Every id {@link useFileProjectFlows.createFileProjectFlows} handles. */
export const FILE_FLOW_COMMANDS = [
  ...Object.keys(INTEROP_EXPORTS),
  ...SYNTHETIC_KINDS,
  "export.frame",
  "file.saveAs",
  "file.saveCopy",
  "file.saveAll",
  "file.revert",
  "file.close",
  "file.closeProject",
  "file.reveal",
  "project.autosave.now",
  "project.autosave.status",
  "project.recovery.restore",
  "project.recovery.discard",
] as const;

export function isFileFlowCommand(id: string): boolean {
  return (FILE_FLOW_COMMANDS as readonly string[]).includes(id) || id in INTEROP_EXPORTS;
}

/** Filesystem-safe project/file name (same rule as the save dialogs). */
export function sanitizeName(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, "-");
}

/** Human notice for a recovery_status payload (pure formatting). */
export function recoveryNotice(status: {
  available: boolean;
  projectName?: string;
  savedAt?: number;
}): string {
  if (!status.available) return "No recovery snapshot on disk";
  const when = status.savedAt ? new Date(status.savedAt * 1000).toLocaleString() : "unknown time";
  return `Recovery snapshot: "${status.projectName ?? "project"}" saved at ${when}`;
}
