#!/usr/bin/env node
/**
 * Command catalogue gate.
 *
 * Verifies the checked-in catalogue snapshot and the generated command
 * reference. Run `pnpm catalogue:sync` after changing the Rust catalogue.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../../..");
const cataloguePath = resolve(root, "docs/reference/command-catalogue.json");
const docsPath = resolve(root, "docs/COMMANDS.md");

const MENU_BAR = [
  "File",
  "Edit",
  "Clip",
  "Sequence",
  "Markers",
  "Graphics",
  "Audio",
  "Effects",
  "View",
  "Window",
  "Help",
];

const commands = JSON.parse(readFileSync(cataloguePath, "utf8"));
const problems = [];

if (!Array.isArray(commands) || commands.length === 0) {
  problems.push("catalogue is empty or not an array");
}

const ids = new Set();
const shortcuts = new Map();
for (const command of commands) {
  if (!command.id) problems.push("command without an id");
  if (ids.has(command.id)) problems.push(`duplicate command id: ${command.id}`);
  ids.add(command.id);

  if (!command.label || !command.label.trim()) {
    problems.push(`empty label: ${command.id}`);
  }
  if (!Array.isArray(command.menu)) {
    problems.push(`menu is not an array: ${command.id}`);
  } else if (command.menu.length && !MENU_BAR.includes(command.menu[0])) {
    problems.push(`unknown top-level menu ${command.menu[0]}: ${command.id}`);
  }
  if (command.uiOnly && command.menu.length === 0) {
    problems.push(`UI-only command has no menu: ${command.id}`);
  }
  if (command.shipped && command.menu.length === 0) {
    problems.push(`shipped command is unreachable: ${command.id}`);
  }
  if (command.menu.length && typeof command.params !== "string") {
    problems.push(`missing params schema: ${command.id}`);
  }
  if (command.shortcut) {
    const key = command.shortcut.replace("Cmd+", "Ctrl+").toLowerCase();
    if (shortcuts.has(key)) {
      problems.push(
        `shortcut ${command.shortcut} used by ${shortcuts.get(key)} and ${command.id}`,
      );
    }
    shortcuts.set(key, command.id);
  }
}

function renderMarkdown(items) {
  const lines = [
    "# Command reference",
    "",
    "Generated from the command catalogue. Do not edit by hand.",
    "",
    `${items.length} commands.`,
  ];
  for (const menu of MENU_BAR) {
    const group = items.filter((command) => command.menu[0] === menu);
    if (!group.length) continue;
    lines.push("", `## ${menu}`, "");
    lines.push("| Command | ID | Status | Shortcut | Params |", "|---|---|---|---|---|");
    for (const command of group) {
      const shortcut = command.shortcut ?? "";
      const params = (command.params ?? "").replaceAll("|", "\\|");
      const status = command.shipped ? "shipped" : "planned";
      lines.push(
        `| ${command.label} | \`${command.id}\` | ${status} | ${shortcut} | \`${params}\` |`,
      );
    }
  }
  return `${lines.join("\n")}\n`;
}

const expectedDocs = renderMarkdown(commands);
const actualDocs = readFileSync(docsPath, "utf8");
if (actualDocs.trimEnd() !== expectedDocs.trimEnd()) {
  problems.push("docs/COMMANDS.md is out of date; run `pnpm catalogue:sync`");
}

if (problems.length) {
  console.error("Command catalogue check failed:");
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}

const uiOnly = commands.filter((command) => command.uiOnly).length;
const inMenus = commands.filter((command) => command.menu.length).length;
const shipped = commands.filter((command) => command.shipped).length;
console.log(
  `Command catalogue check passed (${commands.length} commands, ${shipped} shipped, ${inMenus} in menus, ${uiOnly} UI-only).`,
);
