import type { PanelRect } from "./types";

export interface PresetLayout {
  rects: Record<string, PanelRect>;
  hidden: Record<string, boolean>;
}

export interface WorkspacePreset {
  /** Command suffix: `workspace.preset_<id>`. */
  id: string;
  label: string;
  /** Routed workspace this preset belongs to. */
  workspace: string;
  /** Layouts keyed by panel-host workspace id. */
  layouts: Record<string, PresetLayout>;
}

// Presets opt into panels by name; anything not listed stays closed. The
// Markers, Captions, and Graphics panels are opt-in everywhere, so they are
// part of the hidden set unless a preset names them in `shown`.
const edit = (
  rects: Record<string, PanelRect>,
  hidden: string[] = [],
  shown: string[] = [],
): PresetLayout => ({
  rects,
  hidden: Object.fromEntries(
    ["markers", "captions", "graphics"]
      .filter((id) => !shown.includes(id))
      .concat(hidden)
      .map((id) => [id, true]),
  ),
});

/**
 * Floating-panel layout presets (ADR-03). FilmCraft-style workspaces become
 * layouts over EEP's own panel set rather than new top-level pages.
 */
export const workspacePresets: WorkspacePreset[] = [
  {
    id: "assembly",
    label: "Assembly",
    workspace: "edit",
    layouts: {
      edit: edit(
        {
          media: { x: 0.008, y: 0.012, width: 0.34, height: 0.93 },
          inspector: { x: 0.772, y: 0.012, width: 0.22, height: 0.93 },
        },
        ["inspector"],
      ),
    },
  },
  {
    id: "captions",
    label: "Captions & Graphics",
    workspace: "edit",
    layouts: {
      edit: edit(
        {
          media: { x: 0.008, y: 0.012, width: 0.24, height: 0.93 },
          inspector: { x: 0.744, y: 0.012, width: 0.25, height: 0.93 },
          captions: { x: 0.61, y: 0.09, width: 0.37, height: 0.62 },
          graphics: { x: 0.26, y: 0.09, width: 0.34, height: 0.62 },
        },
        ["media"],
        // The Captions & Graphics workspace exists for these two panels.
        ["captions", "graphics"],
      ),
    },
  },
  {
    id: "review",
    label: "Review",
    workspace: "edit",
    layouts: {
      edit: edit(
        {
          media: { x: 0.008, y: 0.012, width: 0.24, height: 0.93 },
          inspector: { x: 0.772, y: 0.012, width: 0.22, height: 0.93 },
        },
        ["media", "inspector"],
      ),
    },
  },
  {
    id: "learning",
    label: "Learning",
    workspace: "edit",
    layouts: {
      edit: edit({
        media: { x: 0.008, y: 0.012, width: 0.24, height: 0.93 },
        inspector: { x: 0.772, y: 0.012, width: 0.22, height: 0.93 },
      }),
    },
  },
  {
    id: "all_panels",
    label: "All Panels",
    workspace: "edit",
    layouts: {
      edit: edit({
        media: { x: 0.008, y: 0.012, width: 0.3, height: 0.93 },
        inspector: { x: 0.722, y: 0.012, width: 0.27, height: 0.93 },
      }),
    },
  },
];

/** Maps Window-menu panel commands to panel-host events; null when not one. */
export function panelWindowCommand(id: string): string | null {
  if (id === "window.reset_layout") {
    window.dispatchEvent(new Event(PANEL_RESET_EVENT));
    return "Panel layout reset";
  }
  if (id === "window.reopen_panels") {
    window.dispatchEvent(new Event(PANEL_REOPEN_EVENT));
    return "Closed panels reopened";
  }
  return null;
}

export const PANEL_LAYOUT_EVENT = "eep:panel-layout-changed";
export const PANEL_LAYOUT_STORAGE_PREFIX = "easy-edits-pro.panel-layout.v1";
/** Opens a panel by id; used when a gesture needs the Trim Monitor. */
export const PANEL_OPEN_EVENT = "eep:panel-open";
/** Toggles a panel's hidden state from View ▸ Panels. */
export const PANEL_TOGGLE_EVENT = "eep:panel-toggle";
/** Restores the active workspace's default floating layout. */
export const PANEL_RESET_EVENT = "eep:panel-reset";
/** Window ▸ Reopen Closed Panels: every closed panel of the active workspace. */
export const PANEL_REOPEN_EVENT = "eep:panel-reopen";
