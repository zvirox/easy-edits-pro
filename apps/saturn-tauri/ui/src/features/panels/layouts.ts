import type { PanelConstraints, PanelRect } from "./types.ts";

/**
 * Default floating layouts per workspace, in viewport fractions. These mirror
 * the old docked grid so the first floating session looks like the editor the
 * user already knows. Every entry must survive clampPanelRect on the smallest
 * supported viewport (1280x720), which the layouts test asserts.
 */
export const EDIT_WORKSPACE_PANELS = ["media", "inspector"] as const;

export const EDIT_DEFAULT_RECTS: Record<string, PanelRect> = {
  media: { x: 0.008, y: 0.012, width: 0.24, height: 0.97 },
  inspector: { x: 0.772, y: 0.012, width: 0.22, height: 0.97 },
  "trim-monitor": { x: 0.3, y: 0.5, width: 0.4, height: 0.34 },
};

export const EDIT_CONSTRAINTS: Record<string, PanelConstraints> = {
  media: { minWidth: 0.16, minHeight: 0.24, maxWidth: 0.4, maxHeight: 0.92 },
  inspector: {
    minWidth: 0.16,
    minHeight: 0.24,
    maxWidth: 0.36,
    maxHeight: 0.92,
  },
  "trim-monitor": {
    minWidth: 0.22,
    minHeight: 0.2,
    maxWidth: 0.6,
    maxHeight: 0.5,
  },
};

export function defaultLayout(
  workspace: string,
): Record<string, PanelRect> | null {
  if (workspace === "edit") return structuredClone(EDIT_DEFAULT_RECTS);
  return null;
}

export function defaultPanels(workspace: string): readonly string[] {
  return workspace === "edit" ? EDIT_WORKSPACE_PANELS : [];
}
