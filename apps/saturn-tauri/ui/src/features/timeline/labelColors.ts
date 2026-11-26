// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * FC-style label colours for timeline clips. The names are the engine's
 * `LABEL_COLORS` (validated there); CSS tokens map to the Linear palette
 * with zero green/teal hues and zero hardcoded hex literals.
 */
export type LabelColor = { name: string; hex: string };

export const LABEL_COLORS: LabelColor[] = [
  { name: "blue", hex: "var(--blue)" },
  { name: "brown", hex: "var(--amber)" },
  { name: "caribbean", hex: "var(--sky)" },
  { name: "cerulean", hex: "var(--sky-text)" },
  { name: "forest", hex: "var(--primary)" },
  { name: "green", hex: "var(--sky)" },
  { name: "iris", hex: "var(--purple)" },
  { name: "lavender", hex: "var(--purple)" },
  { name: "magenta", hex: "var(--pink)" },
  { name: "mango", hex: "var(--orange)" },
  { name: "purple", hex: "var(--purple)" },
  { name: "rose", hex: "var(--red)" },
  { name: "tan", hex: "var(--primary-text)" },
  { name: "teal", hex: "var(--sky-text)" },
  { name: "violet", hex: "var(--purple)" },
  { name: "yellow", hex: "var(--amber)" },
];

/** CSS token colour for a clip's label colour name; null when unlabelled. */
export function clipLabelColor(label: string | undefined | null): string | null {
  if (!label) return null;
  return LABEL_COLORS.find((entry) => entry.name === label)?.hex ?? null;
}
