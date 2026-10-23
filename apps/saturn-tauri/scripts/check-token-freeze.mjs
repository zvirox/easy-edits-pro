#!/usr/bin/env node
/**
 * Token freeze gate (Plan v4 G1 acceptance: "Zero Color Token Drift").
 *
 * Colour-bearing custom properties in the theme token files are frozen:
 *  - an existing colour token may not change its value,
 *  - a colour token may not be removed,
 *  - a NEW colour token may not appear without running `--update`, committing
 *    the regenerated snapshot, and stating the addition in the commit message.
 *
 * Shape and spacing tokens (durations, easings, blurs, radii, --gap) are not
 * frozen: they carry no colour and are expected to evolve with the layout
 * work. Only values containing a colour function or hex literal are covered.
 *
 * Usage: node scripts/check-token-freeze.mjs [--update]
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const TOKEN_FILES = ["ui/src/styles/00-tokens-glass.css", "ui/src/styles/01-editor.css"];
const SNAPSHOT_PATH = join(APP_ROOT, "scripts/token-freeze.json");
const COLOUR_VALUE = /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(|color-mix\(/;

/** Extract `--name: value;` declarations from CSS custom-property blocks. */
function parseColourTokens(cssText) {
  const tokens = {};
  const declaration = /(--[a-zA-Z0-9-]+)\s*:\s*([^;{}]+);/g;
  let match;
  while ((match = declaration.exec(cssText)) !== null) {
    const [, name, value] = match;
    if (COLOUR_VALUE.test(value)) {
      tokens[name] = value.trim().replace(/\s+/g, " ");
    }
  }
  return tokens;
}

const current = {};
for (const file of TOKEN_FILES) {
  const contents = readFileSync(join(APP_ROOT, file), "utf8");
  // Later files win on name collisions the same way CSS order resolves them;
  // collisions across token files should not exist, so record file-qualified
  // entries to expose any accidental duplicate.
  for (const [name, value] of Object.entries(parseColourTokens(contents))) {
    if (name in current) {
      console.error(`duplicate colour token ${name} (also defined in an earlier token file)`);
      process.exitCode = 1;
    }
    current[name] = value;
  }
}

const updating = process.argv.includes("--update");

if (!existsSync(SNAPSHOT_PATH) && !updating) {
  console.error("token-freeze snapshot missing: scripts/token-freeze.json");
  console.error("Generate it with: node scripts/check-token-freeze.mjs --update");
  process.exit(1);
}

if (updating) {
  writeFileSync(SNAPSHOT_PATH, `${JSON.stringify(current, null, 2)}\n`);
  console.log(`token-freeze snapshot written: ${Object.keys(current).length} colour tokens`);
  process.exit(process.exitCode || 0);
}

const frozen = JSON.parse(readFileSync(SNAPSHOT_PATH, "utf8"));
const problems = [];
for (const [name, value] of Object.entries(current)) {
  if (!(name in frozen)) {
    problems.push(`new colour token ${name}: ${value}`);
  } else if (frozen[name] !== value) {
    problems.push(`colour token ${name} changed:\n  frozen: ${frozen[name]}\n  now:    ${value}`);
  }
}
for (const name of Object.keys(frozen)) {
  if (!(name in current)) {
    problems.push(`colour token ${name} was removed`);
  }
}

if (problems.length > 0) {
  console.error("token freeze violated:");
  for (const problem of problems) console.error(`  - ${problem}`);
  console.error("\nColour tokens are locked by docs/GLASS_CARDS_PLAN.md §3.");
  console.error("If this change is intentional and reviewed, regenerate the snapshot:");
  console.error("  node scripts/check-token-freeze.mjs --update");
  process.exit(1);
}
console.log(`token freeze holds (${Object.keys(frozen).length} colour tokens).`);
