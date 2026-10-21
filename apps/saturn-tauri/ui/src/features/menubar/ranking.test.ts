import assert from "node:assert/strict";
import test from "node:test";
import type { CommandSpec } from "../../types.ts";
import { rankCommands } from "./ranking.ts";

function command(
  id: string,
  label: string,
  menu: string[] = ["Edit"],
  shipped = true,
): CommandSpec {
  return {
    id,
    label,
    menu,
    shortcut: null,
    params: "{}",
    category: "Edit",
    mutating: false,
    undoGroup: "none",
    aliases: [],
    since: "0.1.0",
    shipped,
    uiOnly: true,
  };
}

const commands = [
  command("timeline.ripple_delete_gap", "Ripple Delete Gap", ["Sequence"]),
  command("timeline.ripple_delete", "Ripple Delete", ["Edit"]),
  command("timeline.ripple_trim", "Ripple Trim", ["Sequence"]),
  command("tool.ripple", "Ripple Edit Tool", ["Timeline"]),
  command("edit.undo", "Undo", ["Edit"]),
  command("edit.redo", "Redo", ["Edit"], false),
];

test("palette ranks prefix matches before substrings", () => {
  const ranked = rankCommands(commands, "ripple");
  assert.deepEqual(
    ranked.map((entry) => entry.id),
    [
      "timeline.ripple_trim",
      "timeline.ripple_delete",
      "tool.ripple",
      "timeline.ripple_delete_gap",
    ],
  );
});

test("palette ignores unshipped commands and honours the limit", () => {
  const ranked = rankCommands(commands, "re");
  assert.equal(ranked.some((entry) => entry.id === "edit.redo"), false);
  assert.equal(rankCommands(commands, "", 2).length, 2);
});

test("palette falls back to menu paths and returns nothing for misses", () => {
  const ranked = rankCommands(commands, "sequence");
  assert.deepEqual(
    ranked.map((entry) => entry.id),
    ["timeline.ripple_trim", "timeline.ripple_delete_gap"],
  );
  assert.deepEqual(rankCommands(commands, "zzzz"), []);
});
