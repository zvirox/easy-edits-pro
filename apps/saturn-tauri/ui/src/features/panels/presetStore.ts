import { PANEL_LAYOUT_STORAGE_PREFIX, type PresetLayout } from "./presets.ts";

export const CUSTOM_PRESETS_KEY = "easy-edits-pro.workspace-presets.v1";

export interface CustomWorkspacePreset {
  id: string;
  label: string;
  workspace: string;
  layouts: Record<string, PresetLayout>;
}

function isLayout(value: unknown): value is PresetLayout {
  if (!value || typeof value !== "object") return false;
  const layout = value as Partial<PresetLayout>;
  return (
    typeof layout.rects === "object" &&
    layout.rects !== null &&
    typeof layout.hidden === "object" &&
    layout.hidden !== null
  );
}

/** Tolerant parse; malformed entries are dropped rather than breaking startup. */
export function parseCustomPresets(raw: string | null): CustomWorkspacePreset[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry): CustomWorkspacePreset[] => {
      if (!entry || typeof entry !== "object") return [];
      const preset = entry as Partial<CustomWorkspacePreset>;
      if (
        typeof preset.id !== "string" ||
        typeof preset.label !== "string" ||
        typeof preset.workspace !== "string" ||
        !preset.layouts ||
        typeof preset.layouts !== "object"
      ) {
        return [];
      }
      const layouts = Object.fromEntries(
        Object.entries(preset.layouts).filter(([, layout]) => isLayout(layout)),
      );
      if (Object.keys(layouts).length === 0) return [];
      return [
        {
          id: preset.id,
          label: preset.label,
          workspace: preset.workspace,
          layouts,
        },
      ];
    });
  } catch {
    return [];
  }
}

export function slugForLabel(label: string): string {
  const slug = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return slug || "preset";
}

export function makePresetId(
  label: string,
  existingIds: readonly string[],
): string {
  const base = `custom_${slugForLabel(label)}`;
  if (!existingIds.includes(base)) return base;
  let suffix = 2;
  while (existingIds.includes(`${base}_${suffix}`)) suffix += 1;
  return `${base}_${suffix}`;
}

export function createCustomPreset(
  label: string,
  workspace: string,
  layout: PresetLayout,
  existing: readonly CustomWorkspacePreset[],
): CustomWorkspacePreset {
  return {
    id: makePresetId(label, existing.map((preset) => preset.id)),
    label: label.trim(),
    workspace,
    layouts: { [workspace]: layout },
  };
}

function storage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

export function loadCustomPresets(): CustomWorkspacePreset[] {
  return parseCustomPresets(storage()?.getItem(CUSTOM_PRESETS_KEY) ?? null);
}

export function saveCustomPresets(presets: readonly CustomWorkspacePreset[]): void {
  try {
    storage()?.setItem(CUSTOM_PRESETS_KEY, JSON.stringify(presets));
  } catch {
    // Quota/private mode: presets stay in memory for this session.
  }
}

/** Reads the current layout the same way the floating-panel host persists it. */
export function captureWorkspaceLayout(workspace: string): PresetLayout | null {
  try {
    const raw = storage()?.getItem(`${PANEL_LAYOUT_STORAGE_PREFIX}.${workspace}`);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!isLayout(parsed)) return null;
    return { rects: parsed.rects, hidden: parsed.hidden };
  } catch {
    return null;
  }
}
