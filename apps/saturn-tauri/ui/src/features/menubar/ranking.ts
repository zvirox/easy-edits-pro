import type { CommandSpec } from "../../types";

/**
 * Palette ranking: label matches beat id matches, and menu-path matches are the
 * last resort. Ties break on label length so "Ripple Delete" appears before
 * "Ripple Delete Gap" for the query "ripple".
 */
export function rankCommands(
  commands: readonly CommandSpec[],
  query: string,
  limit = 12,
): CommandSpec[] {
  const needle = query.trim().toLowerCase();
  const shipped = commands.filter((command) => command.shipped);
  if (!needle) return shipped.slice(0, limit);
  return shipped
    .map((command) => {
      const label = command.label.toLowerCase();
      const id = command.id.toLowerCase();
      const menu = command.menu.join(" ").toLowerCase();
      const score =
        label === needle
          ? 0
          : label.startsWith(needle)
            ? 1
            : label.includes(needle)
              ? 2
              : id.includes(needle)
                ? 3
                : menu.includes(needle)
                  ? 4
                  : -1;
      return { command, score };
    })
    .filter((entry) => entry.score >= 0)
    .sort((a, b) => a.score - b.score || a.command.label.length - b.command.label.length)
    .slice(0, limit)
    .map((entry) => entry.command);
}
