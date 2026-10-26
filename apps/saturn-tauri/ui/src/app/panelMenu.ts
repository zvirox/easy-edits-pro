import { TICKS_PER_SECOND } from "./projectTypes.ts";

/**
 * Project-panel menu support: the File > Project Panel commands. Builders
 * follow the app's native prompt idiom; formatters turn engine results into
 * pool notices. Pure module — testable in node with a window stub.
 */

export const PROJECT_PANEL_COMMANDS = [
  "project.view.get",
  "project.view.set",
  "project.sort",
  "project.items",
  "project.columns.list",
  "project.columns.set",
  "project.columns.resize",
  "project.notes",
  "project.setNotes",
  "project.setMarks",
  "project.selectAll",
  "project.deselectAll",
  "project.viewPreset.save",
  "project.viewPreset.saveAs",
  "project.viewPreset.list",
  "project.viewPreset.restore",
  "project.viewPreset.rename",
  "project.viewPreset.delete",
] as const;

export type ProjectPanelCommand = (typeof PROJECT_PANEL_COMMANDS)[number];

/** Document-mutating panel commands — the project refreshes after these. */
export const PANEL_DOC_COMMANDS: ProjectPanelCommand[] = [
  "project.setNotes",
  "project.setMarks",
];

export interface PanelMenuContext {
  selectedMediaId: number | null;
  notes: string;
}

export interface PanelMenuResult {
  params?: Record<string, unknown> | null;
  notice?: string;
  cancel?: boolean;
}

const DEFAULT_COLUMN_IDS = ["name", "kind", "duration", "fps", "codec", "bin"];

/**
 * Parameter builders for the panel commands (prompts follow the app's native
 * prompt idiom). Returns null for command ids handled elsewhere; the read
 * commands take no params.
 */
export function buildProjectPanelParams(
  id: ProjectPanelCommand,
  ctx: PanelMenuContext
): PanelMenuResult | null {
  switch (id) {
    case "project.view.set": {
      const mode = window.prompt("Project panel view mode (grid or list):", "grid");
      if (mode === null) return { cancel: true };
      if (mode !== "grid" && mode !== "list") {
        return { notice: "Enter 'grid' or 'list'", cancel: true };
      }
      return { params: { view_mode: mode } };
    }
    case "project.sort": {
      const input = window.prompt("Sort by key and direction:", "name asc");
      if (input === null) return { cancel: true };
      const [key, dir = "asc"] = input.trim().split(/[\s,]+/);
      if (!key || (dir !== "asc" && dir !== "desc")) {
        return { notice: "Enter a key plus 'asc' or 'desc', e.g. 'name asc'", cancel: true };
      }
      return { params: { key, dir } };
    }
    case "project.columns.set": {
      const input = window.prompt(
        `Visible columns (comma-separated; of ${DEFAULT_COLUMN_IDS.join(", ")}):`,
        DEFAULT_COLUMN_IDS.join(", ")
      );
      if (input === null) return { cancel: true };
      const ids = input
        .split(/[\s,]+/)
        .map((entry) => entry.trim())
        .filter(Boolean);
      if (ids.length === 0) return { notice: "List at least one column", cancel: true };
      return {
        params: {
          columns: ids.map((columnId) => ({ id: columnId, visible: true })),
        },
      };
    }
    case "project.columns.resize": {
      const input = window.prompt("Column id and width (pixels):", `name 160`);
      if (input === null) return { cancel: true };
      const [columnId, widthText] = input.trim().split(/[\s,]+/);
      const width = Number(widthText);
      if (!columnId || !Number.isFinite(width) || width <= 0) {
        return { notice: "Enter a column id and a positive width, e.g. 'name 160'", cancel: true };
      }
      return { params: { id: columnId, width } };
    }
    case "project.setNotes": {
      const text = window.prompt("Edit project notes:", ctx.notes);
      if (text === null) return { cancel: true };
      return { params: { text } };
    }
    case "project.setMarks": {
      if (ctx.selectedMediaId == null) {
        return { notice: "Select a media item first", cancel: true };
      }
      const input = window.prompt(
        "Source marks in and out, seconds (e.g. '0 4.5'):",
        "0 0"
      );
      if (input === null) return { cancel: true };
      const parts = input.trim().split(/[\s,]+/).map(Number);
      if (parts.length < 2 || !parts.every((value) => Number.isFinite(value) && value >= 0)) {
        return { notice: "Enter two non-negative seconds", cancel: true };
      }
      return {
        params: {
          media_id: ctx.selectedMediaId,
          source_in: Math.round(parts[0] * TICKS_PER_SECOND),
          source_out: Math.round(parts[1] * TICKS_PER_SECOND),
        },
      };
    }
    case "project.viewPreset.save":
    case "project.viewPreset.saveAs":
    case "project.viewPreset.restore": {
      const suffix = id === "project.viewPreset.save" ? "" : " (Save As)";
      const name = window.prompt(`View preset name${suffix}:`, "");
      if (name === null) return { cancel: true };
      const trimmed = name.trim();
      if (trimmed === "" && id === "project.viewPreset.save") {
        return { params: {} };
      }
      if (trimmed === "") {
        return { notice: "Enter a preset name", cancel: true };
      }
      return { params: { name: trimmed } };
    }
    case "project.viewPreset.rename": {
      const input = window.prompt("Rename view preset (from to):", "");
      if (input === null) return { cancel: true };
      const [from, to] = input.trim().split(/[\s,]+/);
      if (!from || !to) {
        return { notice: "Enter the old name then the new name", cancel: true };
      }
      return { params: { from, to } };
    }
    case "project.viewPreset.delete": {
      const name = window.prompt("Delete view preset named:", "");
      if (name === null) return { cancel: true };
      if (name.trim() === "") return { notice: "Enter a preset name", cancel: true };
      return { params: { name: name.trim() } };
    }
    default:
      return null;
  }
}

/** Notice line for a panel command's engine result; null shows nothing. */
export function formatPanelResult(id: string, result: unknown): string | null {
  const value = result as Record<string, unknown> | null;
  if (!value) return null;
  if (id === "project.view.get") {
    const view = (value.view ?? {}) as Record<string, unknown>;
    const sort = (view.sort ?? {}) as Record<string, unknown>;
    const columns = (view.columns ?? []) as unknown[];
    return `Panel view: ${view.view_mode ?? "grid"}, sorted by ${sort.key ?? "name"} ${sort.dir ?? "asc"}, ${columns.length} columns`;
  }
  if (id === "project.columns.list") {
    const columns = (value.columns ?? []) as unknown[];
    return `${columns.length} panel columns configured`;
  }
  if (id === "project.items") {
    const media = (value.media ?? []) as unknown[];
    const bins = (value.bins ?? []) as unknown[];
    return `${media.length} media item(s), ${bins.length} bin(s) in the project panel`;
  }
  if (id === "project.notes") {
    const notes = String(value.notes ?? "");
    return notes ? `Project notes: ${notes.slice(0, 80)}` : "Project notes are empty";
  }
  if (id === "project.viewPreset.list") {
    const presets = (value.presets ?? []) as string[];
    const current = value.current ? ` (current: ${String(value.current)})` : "";
    return presets.length
      ? `${presets.length} view preset(s): ${presets.join(", ")}${current}`
      : "No view presets saved yet";
  }
  if (id === "project.viewPreset.restore" || id === "project.viewPreset.saveAs" || id === "project.viewPreset.save") {
    return typeof value.preset === "string" ? `View preset "${value.preset}" is active` : null;
  }
  if (id === "project.viewPreset.rename") {
    const renamed = (value.renamed ?? {}) as Record<string, unknown>;
    return renamed.to ? `View preset renamed to "${renamed.to}"` : null;
  }
  if (id === "project.viewPreset.delete") {
    return typeof value.deleted === "string" ? `View preset "${value.deleted}" deleted` : null;
  }
  if (id === "project.view.set") {
    const view = (value.view ?? {}) as Record<string, unknown>;
    return view.view_mode ? `Panel view set to ${view.view_mode}` : null;
  }
  if (id === "project.sort") {
    const sort = (value.sort ?? {}) as Record<string, unknown>;
    return sort.key ? `Panel sorted by ${sort.key} ${sort.dir ?? "asc"}` : null;
  }
  if (id === "project.columns.set") {
    const columns = (value.columns ?? []) as unknown[];
    return `${columns.length} panel columns configured`;
  }
  if (id === "project.columns.resize") {
    const column = (value.column ?? {}) as Record<string, unknown>;
    return typeof column.id === "string" ? `Column "${column.id}" resized` : null;
  }
  if (id === "project.setNotes") {
    return "Project notes saved";
  }
  if (id === "project.setMarks") {
    return `Source marks set on media #${String(value.media_id)}`;
  }
  return null;
}
