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
  "project.select",
  "project.remove",
  "project.viewPreset.save",
  "project.viewPreset.saveAs",
  "project.viewPreset.list",
  "project.viewPreset.restore",
  "project.viewPreset.rename",
  "project.viewPreset.delete",
  "project.freeform.layout",
  "project.freeform.move",
  "project.freeform.resize",
  "project.freeform.alignToGrid",
  "project.freeform.reset",
  "project.freeform.stack",
  "project.freeform.unstack",
  "project.freeform.options",
  "project.freeform.saveArrangement",
  "project.freeform.restoreArrangement",
  "project.freeform.deleteArrangement",
  "project.freeform.arrangements",
  "project.searchBin.set",
  "project.searchBin.delete",
  "project.searchBin.items",
] as const;

export type ProjectPanelCommand = (typeof PROJECT_PANEL_COMMANDS)[number];

/** Document-mutating panel commands — the project refreshes after these. */
export const PANEL_DOC_COMMANDS: ProjectPanelCommand[] = [
  "project.setNotes",
  "project.setMarks",
  "project.remove",
];

export interface PanelMenuContext {
  selectedMediaId: number | null;
  selectedMediaIds: number[];
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
      const mode = window.prompt("Project panel view mode (grid, list, or freeform):", "grid");
      if (mode === null) return { cancel: true };
      if (mode !== "grid" && mode !== "list" && mode !== "freeform") {
        return { notice: "Enter 'grid', 'list', or 'freeform'", cancel: true };
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
    case "project.remove": {
      if (ctx.selectedMediaIds.length === 0) {
        return { notice: "Select media items to clear first", cancel: true };
      }
      return { params: { media_ids: ctx.selectedMediaIds } };
    }
    case "project.freeform.layout":
      return { params: {} };
    case "project.freeform.move": {
      if (ctx.selectedMediaIds.length === 0) {
        return { notice: "Select media items to move first", cancel: true };
      }
      const input = window.prompt("Move clip cards by dx dy (pixels):", "40 0");
      if (input === null) return { cancel: true };
      const [dxText, dyText] = input.trim().split(/[\s,]+/).map(Number);
      if (!Number.isFinite(dxText) || !Number.isFinite(dyText)) {
        return { notice: "Enter two numbers, e.g. '40 0'", cancel: true };
      }
      return { params: { media_ids: ctx.selectedMediaIds, dx: dxText, dy: dyText } };
    }
    case "project.freeform.resize": {
      if (ctx.selectedMediaIds.length === 0) {
        return { notice: "Select media items to resize first", cancel: true };
      }
      const input = window.prompt("Clip size scale (0.2 - 5):", "1.5");
      if (input === null) return { cancel: true };
      const scale = Number(input.trim());
      if (!Number.isFinite(scale)) {
        return { notice: "Enter a number, e.g. 1.5", cancel: true };
      }
      return { params: { media_ids: ctx.selectedMediaIds, scale } };
    }
    case "project.freeform.stack":
    case "project.freeform.unstack": {
      if (ctx.selectedMediaIds.length === 0) {
        return { notice: "Select media items first", cancel: true };
      }
      return { params: { media_ids: ctx.selectedMediaIds } };
    }
    case "project.freeform.alignToGrid":
    case "project.freeform.reset":
    case "project.freeform.arrangements":
      return { params: {} };
    case "project.freeform.options": {
      const input = window.prompt("Freeform grid size (pixels):", "40");
      if (input === null) return { cancel: true };
      const grid = Number(input.trim());
      if (!Number.isFinite(grid) || grid <= 0) {
        return { notice: "Enter a positive grid size in pixels", cancel: true };
      }
      return { params: { grid } };
    }
    case "project.freeform.saveArrangement":
    case "project.freeform.restoreArrangement":
    case "project.freeform.deleteArrangement": {
      const verb = id === "project.freeform.saveArrangement"
        ? "Save"
        : id === "project.freeform.restoreArrangement"
          ? "Restore"
          : "Delete";
      const name = window.prompt(`${verb} arrangement named:`, "");
      if (name === null) return { cancel: true };
      if (name.trim() === "") return { notice: "Enter an arrangement name", cancel: true };
      return { params: { name: name.trim() } };
    }
    case "project.searchBin.set": {
      const input = window.prompt("Search bin as 'name query text':", "");
      if (input === null) return { cancel: true };
      const trimmed = input.trim();
      const separator = trimmed.indexOf(" ");
      const name = separator < 0 ? trimmed : trimmed.slice(0, separator);
      const query = separator < 0 ? "" : trimmed.slice(separator + 1).trim();
      if (name === "" || query === "") {
        return { notice: "Enter a name followed by the query text", cancel: true };
      }
      return { params: { name, query } };
    }
    case "project.searchBin.delete":
    case "project.searchBin.items": {
      const name = window.prompt(
        id === "project.searchBin.delete" ? "Delete search bin named:" : "List search bin named:",
        "",
      );
      if (name === null) return { cancel: true };
      if (name.trim() === "") return { notice: "Enter a search bin name", cancel: true };
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
  if (id === "project.remove") {
    return `${value.removed ?? 0} media item(s) cleared from the panel`;
  }
  if (id === "project.freeform.layout") {
    return value.freeform === true ? "Freeform layout on" : "Freeform layout off (list view)";
  }
  if (id === "project.freeform.move") {
    return `${value.moved ?? 0} clip card(s) moved`;
  }
  if (id === "project.freeform.resize") {
    return `${value.resized ?? 0} clip card(s) resized to ${value.scale}×`;
  }
  if (id === "project.freeform.alignToGrid") {
    return `Clip cards aligned to a ${value.aligned_to_grid}px grid`;
  }
  if (id === "project.freeform.reset") {
    return `Freeform layout reset (${value.cleared ?? 0} card position(s) cleared)`;
  }
  if (id === "project.freeform.stack") {
    return value.stacked === true
      ? `${value.updated ?? 0} clip card(s) stacked`
      : `${value.updated ?? 0} clip card(s) unstacked onto the grid`;
  }
  if (id === "project.freeform.unstack") {
    return `${value.updated ?? 0} clip card(s) unstacked onto the grid`;
  }
  if (id === "project.freeform.options") {
    return typeof value.grid === "number" ? `Freeform grid set to ${value.grid}px` : null;
  }
  if (id === "project.freeform.saveArrangement") {
    const names = (value.arrangements ?? []) as string[];
    return `Arrangement "${String(value.saved)}" saved (${names.length} total)`;
  }
  if (id === "project.freeform.restoreArrangement") {
    return `Arrangement "${String(value.restored)}" restored (${value.items ?? 0} card(s))`;
  }
  if (id === "project.freeform.deleteArrangement") {
    return `Arrangement "${String(value.deleted)}" deleted`;
  }
  if (id === "project.freeform.arrangements") {
    const names = (value.arrangements ?? []) as string[];
    return names.length ? `${names.length} arrangement(s): ${names.join(", ")}` : "No arrangements saved yet";
  }
  if (id === "project.searchBin.set") {
    return `Search bin "${String(value.saved)}" saved: ${String(value.query)}`;
  }
  if (id === "project.searchBin.delete") {
    return `Search bin "${String(value.deleted)}" deleted`;
  }
  if (id === "project.searchBin.items") {
    const matches = (value.matches ?? []) as unknown[];
    return `Search bin "${String(value.bin)}" matched ${matches.length} media item(s)`;
  }
  return null;
}
