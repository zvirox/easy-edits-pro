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
  markers: { x: 0.3, y: 0.06, width: 0.34, height: 0.4 },
  captions: { x: 0.66, y: 0.06, width: 0.34, height: 0.5 },
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
  markers: {
    minWidth: 0.22,
    minHeight: 0.2,
    maxWidth: 0.5,
    maxHeight: 0.7,
  },
  captions: {
    minWidth: 0.2,
    minHeight: 0.2,
    maxWidth: 0.5,
    maxHeight: 0.7,
  },
  graphics: {
    minWidth: 0.2,
    minHeight: 0.25,
    maxWidth: 0.5,
    maxHeight: 0.75,
  },
};

export function defaultLayout(
  workspace: string,
): Record<string, PanelRect> | null {
  if (workspace === "edit") return structuredClone(EDIT_DEFAULT_RECTS);
  if (workspace === "effects") return structuredClone(EFFECTS_DEFAULT_RECTS);
  if (workspace === "color") return structuredClone(COLOR_DEFAULT_RECTS);
  if (workspace === "global") return structuredClone(GLOBAL_DEFAULT_RECTS);
  return null;
}

export function defaultPanels(workspace: string): readonly string[] {
  if (workspace === "edit") return EDIT_WORKSPACE_PANELS;
  if (workspace === "effects") return EFFECTS_WORKSPACE_PANELS;
  if (workspace === "color") return COLOR_WORKSPACE_PANELS;
  if (workspace === "global") return GLOBAL_WORKSPACE_PANELS;
  return [];
}

/**
 * G4c migration: Effects (3), Scopes (4), and Chat (5) panels. Defaults mirror
 * the docked grids they replaced — the effect stack sat right of the monitor,
 * scopes sat right of the program monitor, and the AI drawer was a fixed
 * 520px right sheet.
 */
export const EFFECTS_WORKSPACE_PANELS = ["effects-stack"] as const;

export const EFFECTS_DEFAULT_RECTS: Record<string, PanelRect> = {
  "effects-stack": { x: 0.655, y: 0.015, width: 0.33, height: 0.97 },
};

export const EFFECTS_CONSTRAINTS: Record<string, PanelConstraints> = {
  "effects-stack": {
    minWidth: 0.2,
    minHeight: 0.3,
    maxWidth: 0.5,
    maxHeight: 0.96,
  },
};

export const COLOR_WORKSPACE_PANELS = ["scopes"] as const;

export const COLOR_DEFAULT_RECTS: Record<string, PanelRect> = {
  scopes: { x: 0.505, y: 0.015, width: 0.48, height: 0.62 },
};

export const COLOR_CONSTRAINTS: Record<string, PanelConstraints> = {
  scopes: { minWidth: 0.22, minHeight: 0.24, maxWidth: 0.62, maxHeight: 0.9 },
};

export const GLOBAL_WORKSPACE_PANELS = ["ai-chat"] as const;

export const GLOBAL_DEFAULT_RECTS: Record<string, PanelRect> = {
  "ai-chat": { x: 0.6, y: 0.05, width: 0.38, height: 0.8 },
};

export const GLOBAL_CONSTRAINTS: Record<string, PanelConstraints> = {
  "ai-chat": { minWidth: 0.22, minHeight: 0.3, maxWidth: 0.5, maxHeight: 0.92 },
};
