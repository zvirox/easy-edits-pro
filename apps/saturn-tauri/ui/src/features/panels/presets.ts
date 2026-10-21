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

const edit = (rects: Record<string, PanelRect>, hidden: string[] = []): PresetLayout => ({
  rects,
  hidden: Object.fromEntries(hidden.map((id) => [id, true])),
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
        },
        ["media"],
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

export const PANEL_LAYOUT_EVENT = "eep:panel-layout-changed";
export const PANEL_LAYOUT_STORAGE_PREFIX = "easy-edits-pro.panel-layout.v1";
