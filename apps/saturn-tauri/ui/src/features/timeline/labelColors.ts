/**
 * FC-style label colours for timeline clips. The names are the engine's
 * `LABEL_COLORS` (validated there); the hex values are data-driven accents
 * (same treatment as marker colours), not theme tokens.
 */
export type LabelColor = { name: string; hex: string };

export const LABEL_COLORS: LabelColor[] = [
  { name: "blue", hex: "#3b82f6" },
  { name: "brown", hex: "#a16207" },
  { name: "caribbean", hex: "#0d9488" },
  { name: "cerulean", hex: "#0284c7" },
  { name: "forest", hex: "#166534" },
  { name: "green", hex: "#22c55e" },
  { name: "iris", hex: "#6366f1" },
  { name: "lavender", hex: "#c084fc" },
  { name: "magenta", hex: "#db2777" },
  { name: "mango", hex: "#f59e0b" },
  { name: "purple", hex: "#9333ea" },
  { name: "rose", hex: "#f43f5e" },
  { name: "tan", hex: "#d6bd8f" },
  { name: "teal", hex: "#14b8a6" },
  { name: "violet", hex: "#8b5cf6" },
  { name: "yellow", hex: "#facc15" },
];

/** Hex accent for a clip's label colour name; null when unlabelled. */
export function clipLabelColor(label: string | undefined | null): string | null {
  if (!label) return null;
  return LABEL_COLORS.find((entry) => entry.name === label)?.hex ?? null;
}
