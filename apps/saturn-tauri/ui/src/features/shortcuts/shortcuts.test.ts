import assert from "node:assert/strict";
import test from "node:test";
import {
  findShortcutConflict,
  formatShortcut,
  parseShortcutImport,
  serializeShortcuts,
} from "./shortcuts.ts";

const rows = [
  { id: "edit.undo", label: "Undo", shortcut: "Ctrl+Z" },
  { id: "edit.redo", label: "Redo", shortcut: "Ctrl+Shift+Z" },
];

test("shortcut labels adapt to the platform without changing the binding", () => {
  assert.equal(formatShortcut("Ctrl+Shift+Z", "other"), "Ctrl+Shift+Z");
  assert.equal(formatShortcut("Ctrl+Shift+Z", "mac"), "⌘⇧Z");
  assert.equal(formatShortcut("Alt+ArrowLeft", "mac"), "⌥ArrowLeft");
});

test("conflict detection ignores the row being rebound", () => {
  assert.equal(findShortcutConflict(rows, "edit.redo", "ctrl+z")?.id, "edit.undo");
  assert.equal(findShortcutConflict(rows, "edit.undo", "Ctrl+Z"), null);
  assert.equal(findShortcutConflict(rows, "edit.undo", "Ctrl+Alt+K"), null);
});

test("import accepts both wrapped and bare maps", () => {
  const valid = new Set(["edit.undo", "edit.redo"]);
  const wrapped = parseShortcutImport(
    JSON.stringify({ version: 1, shortcuts: { "edit.undo": "Ctrl+U" } }),
    valid,
  );
  assert.deepEqual(wrapped, { ok: true, overrides: { "edit.undo": "Ctrl+U" } });

  const bare = parseShortcutImport(JSON.stringify({ "edit.redo": "Ctrl+R" }), valid);
  assert.deepEqual(bare, { ok: true, overrides: { "edit.redo": "Ctrl+R" } });
});

test("import rejects unknown ids, bad values, and malformed json", () => {
  const valid = new Set(["edit.undo"]);
  const unknown = parseShortcutImport(JSON.stringify({ "edit.nope": "Ctrl+Q" }), valid);
  assert.equal(unknown.ok, false);
  assert.match(unknown.ok ? "" : unknown.error, /Unknown command/);

  const badValue = parseShortcutImport(JSON.stringify({ "edit.undo": 4 }), valid);
  assert.equal(badValue.ok, false);

  const malformed = parseShortcutImport("{", valid);
  assert.equal(malformed.ok, false);
  assert.match(malformed.ok ? "" : malformed.error, /valid JSON/);
});

test("export round-trips through import", () => {
  const text = serializeShortcuts({ "edit.undo": "Ctrl+U" });
  const parsed = parseShortcutImport(text, new Set(["edit.undo"]));
  assert.deepEqual(parsed, { ok: true, overrides: { "edit.undo": "Ctrl+U" } });
});
