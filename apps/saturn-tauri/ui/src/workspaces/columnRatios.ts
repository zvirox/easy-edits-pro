/**
 * §5 column-ratio preset table (GLASS_CARDS_PLAN v4, "One Shell, Many
 * Workspaces"). A workspace switch re-tabs Dock L / Dock R and may pick a
 * different column ratio from this small fixed table — the program monitor,
 * toolbox, timeline, meters and status bar never move or unmount.
 *
 * Each preset is `[left %, center %, right %]`; the plan locks the right dock
 * at 24% and varies the left dock, with the center monitor taking the rest.
 */

/** `[left %, center %, right %]` — the three shell-top column widths. */
export type ColumnRatioPreset = readonly [left: number, center: number, right: number];

/** Plan §5 default split (edit preset); modes without an explicit entry use it. */
const EDIT_PRESET: ColumnRatioPreset = [28, 48, 24];

/**
 * The small fixed table from plan §5. The seven shell modes route through
 * WorkspaceShell; Deliver is a full-page workspace and never uses the shell.
 */
export const WORKSPACE_COLUMN_RATIOS = {
  edit: EDIT_PRESET,
  media: EDIT_PRESET,
  effects: EDIT_PRESET,
  fusion: EDIT_PRESET,
  keyframes: EDIT_PRESET,
  color: [34, 42, 24] as const,
  audio: [40, 36, 24] as const,
} satisfies Record<string, ColumnRatioPreset>;

/** Resolve the preset for a workspace mode, falling back to the edit split. */
export function columnRatiosFor(mode: string): ColumnRatioPreset {
  return (WORKSPACE_COLUMN_RATIOS as Record<string, ColumnRatioPreset>)[mode] ?? EDIT_PRESET;
}
