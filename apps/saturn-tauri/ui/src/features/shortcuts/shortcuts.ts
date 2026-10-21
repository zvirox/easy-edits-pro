/** Shortcut parsing, display, conflict checks, and JSON import/export. */

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "CapsLock", "Tab"]);

export type ShortcutPlatform = "mac" | "other";

export function detectPlatform(): ShortcutPlatform {
  if (typeof navigator === "undefined") return "other";
  return /Mac|iPhone|iPad/.test(navigator.userAgent) ? "mac" : "other";
}

export function comboFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key) || event.key === "Escape") return null;
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  const key =
    event.key === " " ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key;
  parts.push(key);
  return parts.join("+");
}

/** Platform-aware label. Matching always uses the canonical Ctrl+… string. */
export function formatShortcut(
  shortcut: string,
  platform: ShortcutPlatform = detectPlatform(),
): string {
  if (!shortcut) return "";
  if (platform !== "mac") return shortcut;
  const symbols: Record<string, string> = {
    Ctrl: "⌘",
    Alt: "⌥",
    Shift: "⇧",
    Space: "Space",
  };
  return shortcut
    .split("+")
    .map((part) => symbols[part] ?? part)
    .join("");
}

export interface ShortcutRowLike {
  id: string;
  label: string;
  shortcut: string;
}

/** Returns the row already using `combo`, ignoring the row being rebound. */
export function findShortcutConflict<T extends ShortcutRowLike>(
  rows: readonly T[],
  id: string,
  combo: string,
): T | null {
  const needle = combo.trim().toLowerCase();
  if (!needle) return null;
  return rows.find((row) => row.id !== id && row.shortcut.toLowerCase() === needle) ?? null;
}

export interface ShortcutExport {
  version: 1;
  shortcuts: Record<string, string>;
}

export function serializeShortcuts(overrides: Record<string, string>): string {
  const payload: ShortcutExport = {
    version: 1,
    shortcuts: Object.fromEntries(
      Object.entries(overrides).filter(([, value]) => typeof value === "string"),
    ),
  };
  return `${JSON.stringify(payload, null, 2)}\n`;
}

export type ShortcutImportResult =
  | { ok: true; overrides: Record<string, string> }
  | { ok: false; error: string };

/**
 * Accepts `{version, shortcuts}` or a bare `{id: "Ctrl+Z"}` map. Unknown ids are
 * rejected with their names so a typo cannot silently create a dead binding.
 */
export function parseShortcutImport(
  text: string,
  validIds: ReadonlySet<string>,
): ShortcutImportResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    return { ok: false, error: `Not valid JSON: ${String(error)}` };
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return { ok: false, error: "Expected a JSON object of command ids to shortcuts." };
  }
  const record = parsed as Record<string, unknown>;
  const source =
    record.shortcuts && typeof record.shortcuts === "object" && !Array.isArray(record.shortcuts)
      ? (record.shortcuts as Record<string, unknown>)
      : record;
  const overrides: Record<string, string> = {};
  const unknownIds: string[] = [];
  for (const [id, value] of Object.entries(source)) {
    if (!validIds.has(id)) {
      unknownIds.push(id);
      continue;
    }
    if (typeof value !== "string") {
      return { ok: false, error: `Shortcut for ${id} must be a string.` };
    }
    overrides[id] = value.trim();
  }
  if (unknownIds.length > 0) {
    return {
      ok: false,
      error: `Unknown command${unknownIds.length === 1 ? "" : "s"}: ${unknownIds.slice(0, 4).join(", ")}${
        unknownIds.length > 4 ? "…" : ""
      }`,
    };
  }
  return { ok: true, overrides };
}
